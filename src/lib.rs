#![warn(missing_docs)]
#![deny(unused_results)]
#![deny(clippy::as_conversions)]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]
//! A Matroska demuxer that can demux Matroska and WebM container files.

use std::collections::HashMap;
use std::convert::TryInto;
use std::io::{Read, Seek, SeekFrom};
use std::num::NonZeroU64;

pub use enums::*;
pub use error::DemuxError;

use crate::ebml::{
    collect_children, expect_master, find_string, find_unsigned, next_element, parse_ebml_header,
    parse_element_header, try_find_date, try_find_float, try_find_string, try_find_unsigned,
    ElementData,
};
use crate::element_id::{ElementId, ID_TO_ELEMENT_ID};

mod ebml;
pub(crate) mod element_id;
mod enums;
mod error;

type Result<T> = std::result::Result<T, DemuxError>;

/// The EBML header of the file.
#[derive(Clone, Debug)]
pub struct EbmlHeader {
    version: Option<u64>,
    read_version: Option<u64>,
    max_id_length: u64,
    max_size_length: u64,
    doc_type: String,
    doc_type_version: u64,
    doc_type_read_version: u64,
}

impl EbmlHeader {
    pub(crate) fn new(fields: &[(ElementId, ElementData)]) -> Result<Self> {
        let version = try_find_unsigned(fields, ElementId::EbmlVersion)?;
        let read_version = try_find_unsigned(fields, ElementId::EbmlReadVersion)?;
        let max_id_length = try_find_unsigned(fields, ElementId::EbmlMaxIdLength)?;
        let max_size_length = try_find_unsigned(fields, ElementId::EbmlMaxSizeLength)?;
        let doc_type = find_string(fields, ElementId::DocType)?;
        let doc_type_version = find_unsigned(fields, ElementId::DocTypeVersion)?;
        let doc_type_read_version = find_unsigned(fields, ElementId::DocTypeReadVersion)?;

        Ok(Self {
            version,
            read_version,
            max_id_length: max_id_length.unwrap_or(4),
            max_size_length: max_size_length.unwrap_or(8),
            doc_type,
            doc_type_version,
            doc_type_read_version,
        })
    }

    /// The EBML version used to create the file.
    pub fn version(&self) -> Option<u64> {
        self.version
    }

    /// The minimum EBML version a parser has to support to read this file.
    pub fn read_version(&self) -> Option<u64> {
        self.read_version
    }

    /// The maximum length of the IDs you'll find in this file (4 or less in Matroska).
    pub fn max_id_length(&self) -> u64 {
        self.max_id_length
    }

    /// The maximum length of the sizes you'll find in this file (8 or less in Matroska).
    pub fn max_size_length(&self) -> u64 {
        self.max_size_length
    }

    /// A string that describes the type of document that follows this EBML header ('matroska' / 'webm').
    pub fn doc_type(&self) -> &str {
        &self.doc_type
    }

    /// The version of DocType interpreter used to create the file.
    pub fn doc_type_version(&self) -> u64 {
        self.doc_type_version
    }

    /// The minimum DocType version an interpreter has to support to read this file.
    pub fn doc_type_read_version(&self) -> u64 {
        self.doc_type_read_version
    }
}

/// An entry in the seek head.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SeekEntry {
    id: ElementId,
    offset: u64,
}

impl SeekEntry {
    pub(crate) fn new(fields: &[(ElementId, ElementData)]) -> Result<SeekEntry> {
        let id: u32 = find_unsigned(fields, ElementId::SeekId)?.try_into()?;
        let id = *ID_TO_ELEMENT_ID.get(&id).unwrap_or(&ElementId::Unknown);
        let offset = find_unsigned(fields, ElementId::SeekPosition)?;

        Ok(Self { id, offset })
    }
}

/// The Info element.
#[derive(Clone, Debug)]
pub struct Info {
    timestamp_scale: NonZeroU64,
    duration: Option<f64>,
    date_utc: Option<i64>,
    title: Option<String>,
    muxing_app: String,
    writing_app: String,
}

impl Info {
    pub(crate) fn new(fields: &[(ElementId, ElementData)]) -> Result<Info> {
        let timestamp_scale = try_find_unsigned(fields, ElementId::TimestampScale)?;
        let duration = try_find_float(fields, ElementId::Duration)?;
        let date_utc = try_find_date(fields, ElementId::DateUtc)?;
        let title = try_find_string(fields, ElementId::Title)?;
        let muxing_app = find_string(fields, ElementId::MuxingApp)?;
        let writing_app = find_string(fields, ElementId::WritingApp)?;

        let timestamp_scale = timestamp_scale.unwrap_or(1000000);
        let timestamp_scale = NonZeroU64::new(timestamp_scale)
            .ok_or(DemuxError::NonZeroValueIsZero(ElementId::TimestampScale))?;

        Ok(Self {
            timestamp_scale,
            duration,
            date_utc,
            title,
            muxing_app,
            writing_app,
        })
    }

    /// Timestamp scale in nanoseconds (1_000_000 means all timestamps in the Segment are expressed in milliseconds).
    pub fn timestamp_scale(&self) -> &NonZeroU64 {
        &self.timestamp_scale
    }

    /// Duration of the Segment in nanoseconds based on TimestampScale.
    pub fn duration(&self) -> &Option<f64> {
        &self.duration
    }

    /// The date and time that the Segment was created by the muxing application or library.
    pub fn date_utc(&self) -> &Option<i64> {
        &self.date_utc
    }

    /// General name of the Segment.
    pub fn title(&self) -> &Option<String> {
        &self.title
    }

    /// Muxing application or library.
    pub fn muxing_app(&self) -> &String {
        &self.muxing_app
    }

    /// Writing  application.
    pub fn writing_app(&self) -> &String {
        &self.writing_app
    }
}

// Track
// - Video
// - Audio
// - Colour
// - ContentEncoding

/// Demuxer for Matroska files.
#[derive(Clone, Debug)]
pub struct MatroskaFile<R> {
    file: R,
    ebml_header: EbmlHeader,
    seek_head: HashMap<ElementId, u64>,
    info: Info,
}

impl<R: Read + Seek> MatroskaFile<R> {
    /// Opens a Matroska file.
    pub fn open(mut file: R) -> Result<Self> {
        let ebml_header = parse_ebml_header(&mut file)?;
        let (segment_data_offset, _) = expect_master(&mut file, ElementId::Segment, None)?;
        let optional_seek_head = search_seek_head(&mut file, segment_data_offset)?;

        let mut seek_head = HashMap::new();

        if let Some((seek_head_data_offset, seek_head_data_size)) = optional_seek_head {
            let seek_head_entries =
                collect_children(&mut file, seek_head_data_offset, seek_head_data_size)?;

            for (entry_id, entry_data) in &seek_head_entries {
                if let ElementId::Seek = entry_id {
                    if let ElementData::Location { offset, size } = entry_data {
                        let seek_fields = collect_children(&mut file, *offset, *size)?;
                        if let Ok(seek_entry) = SeekEntry::new(&seek_fields) {
                            let _ = seek_head
                                .insert(seek_entry.id, segment_data_offset + seek_entry.offset);
                        }
                    }
                }
            }
        }

        if seek_head.is_empty() {
            build_seek_head(&mut file, segment_data_offset, &mut seek_head)?;
        }

        if seek_head.get(&ElementId::Cluster).is_none() {
            find_first_cluster_offset(&mut file, segment_data_offset, &mut seek_head)?;
        }

        let info = parse_segment_info(&mut file, &mut seek_head)?;

        // TODO parse the Tracks element
        // TODO parse Cues element

        // TODO how to parse blocks and how to do seeking?
        // TODO we could add a BTreeMap and store the Cues in it. If no Cues have been found, we could (re-)build them too, if asked for (open(file: &mut File, build_cues: Bool)

        // TODO lazy loading: Chapters, Tagging

        Ok(Self {
            file,
            ebml_header,
            seek_head,
            info,
        })
    }

    /// Returns the EBML header.
    pub fn ebml_header(&self) -> &EbmlHeader {
        &self.ebml_header
    }

    /// Returns the segment info.
    pub fn info(&self) -> &Info {
        &self.info
    }
}

/// Seeks the SeekHead element and returns the offset into to it when present.
///
/// Specification states that the first non CRC-32 element should be a SeekHead if present.
fn search_seek_head<R: Read + Seek>(
    r: &mut R,
    segment_data_offset: u64,
) -> Result<Option<(u64, u64)>> {
    loop {
        let (element_id, size) = parse_element_header(r, Some(segment_data_offset))?;
        match element_id {
            ElementId::SeekHead => {
                let current_pos = r.stream_position()?;
                return Ok(Some((current_pos, size)));
            }
            ElementId::Crc32 => continue,
            _ => return Ok(None),
        }
    }
}

/// Build a SeekHead by parsing the top level entries.
fn build_seek_head<R: Read + Seek>(
    r: &mut R,
    segment_data_offset: u64,
    seek_head: &mut HashMap<ElementId, u64>,
) -> Result<()> {
    let _ = r.seek(SeekFrom::Start(segment_data_offset))?;
    loop {
        let position = r.stream_position()?;
        match next_element(r) {
            Ok((element_id, element_data)) => {
                if element_id == ElementId::Info
                    || element_id == ElementId::Tracks
                    || element_id == ElementId::Chapters
                    || element_id == ElementId::Cues
                    || element_id == ElementId::Tags
                    || element_id == ElementId::Cluster
                {
                    // We only need the first cluster entry.
                    if element_id != ElementId::Cluster
                        || !seek_head.contains_key(&ElementId::Cluster)
                    {
                        let _ = seek_head.insert(element_id, position);
                    }
                }

                if let ElementData::Location { offset, size } = element_data {
                    if size == u64::MAX {
                        // No path left to walk on this level.
                        break;
                    }
                    let _ = r.seek(SeekFrom::Start(offset + size))?;
                }
            }
            Err(_) => {
                // EOF or damaged file. We will stop looking for top level entries.
                break;
            }
        }
    }

    Ok(())
}

/// Tries to find the offset of the first cluster and save it in the SeekHead.
fn find_first_cluster_offset<R: Read + Seek>(
    r: &mut R,
    segment_offset: u64,
    seek_head: &mut HashMap<ElementId, u64>,
) -> Result<()> {
    let (tracks_offset, tracks_size) = if let Some(offset) = seek_head.get(&ElementId::Tracks) {
        expect_master(r, ElementId::Tracks, Some(*offset))?
    } else {
        return Err(DemuxError::CantFindCluster);
    };

    let _ = r.seek(SeekFrom::Start(tracks_offset + tracks_size))?;
    loop {
        match next_element(r) {
            Ok((element_id, element_data)) => {
                if let ElementId::Cluster = element_id {
                    if let ElementData::Location { offset, .. } = element_data {
                        let _ = seek_head.insert(ElementId::Cluster, segment_offset + offset);
                        break;
                    } else {
                        return Err(DemuxError::UnexpectedDataType);
                    }
                }

                if let ElementData::Location { offset, size } = element_data {
                    if size == u64::MAX {
                        // No path left to walk on this level.
                        return Err(DemuxError::CantFindCluster);
                    }
                    let _ = r.seek(SeekFrom::Start(offset + size))?;
                }
            }
            Err(_) => {
                // EOF or damaged file. We will stop looking for top level entries.
                return Err(DemuxError::CantFindCluster);
            }
        }
    }

    Ok(())
}

fn parse_segment_info<R: Read + Seek>(
    mut file: &mut R,
    seek_head: &mut HashMap<ElementId, u64>,
) -> Result<Info> {
    if let Some(info_offset) = seek_head.get(&ElementId::Info) {
        let (info_data_offset, info_data_size) =
            expect_master(&mut file, ElementId::Info, Some(*info_offset))?;
        let children = collect_children(&mut file, info_data_offset, info_data_size)?;
        let info = Info::new(&children)?;
        Ok(info)
    } else {
        Err(DemuxError::ElementNotFound(ElementId::Info))
    }
}
