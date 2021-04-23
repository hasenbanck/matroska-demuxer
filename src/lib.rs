#![warn(missing_docs)]
#![deny(unused_results)]
#![deny(clippy::as_conversions)]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]
//! A demuxer that can demux Matroska and WebM container files.

use std::collections::HashMap;
use std::convert::TryInto;
use std::io::{Read, Seek, SeekFrom};
use std::num::NonZeroU64;

pub use enums::*;
pub use error::DemuxError;

use crate::ebml::{
    collect_children, expect_master, find_bool_or, find_custom_type, find_float_or, find_nonzero,
    find_nonzero_or, find_string, find_unsigned, find_unsigned_or, next_element,
    parse_element_header, try_find_binary, try_find_date, try_find_float, try_find_nonzero,
    try_find_string, try_find_unsigned, try_parse_child, ElementData, ParsableElement,
};
use crate::element_id::{ElementId, ID_TO_ELEMENT_ID};

mod ebml;
pub(crate) mod element_id;
mod enums;
mod error;

/// The doc type version this demuxer supports.
const DEMUXER_DOC_TYPE_VERSION: u64 = 4;

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

impl<R: Read + Seek> ParsableElement<R> for EbmlHeader {
    type Output = Self;

    fn new(_r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        let version = try_find_unsigned(fields, ElementId::EbmlVersion)?;
        let read_version = try_find_unsigned(fields, ElementId::EbmlReadVersion)?;
        let max_id_length = find_unsigned_or(fields, ElementId::EbmlMaxIdLength, 4)?;
        let max_size_length = find_unsigned_or(fields, ElementId::EbmlMaxSizeLength, 8)?;
        let doc_type = find_string(fields, ElementId::DocType)?;
        let doc_type_version = find_unsigned(fields, ElementId::DocTypeVersion)?;
        let doc_type_read_version = find_unsigned(fields, ElementId::DocTypeReadVersion)?;

        if doc_type != "matroska" && doc_type != "webm" {
            return Err(DemuxError::InvalidEbmlHeader(format!(
                "unsupported DocType: {}",
                doc_type
            )));
        }

        if doc_type_read_version >= DEMUXER_DOC_TYPE_VERSION {
            return Err(DemuxError::InvalidEbmlHeader(format!(
                "unsupported DocTypeReadVersion: {}",
                doc_type_read_version
            )));
        }

        if max_id_length > 4 {
            return Err(DemuxError::InvalidEbmlHeader(format!(
                "unsupported MaxIdLength: {}",
                max_id_length
            )));
        }

        if max_size_length > 8 {
            return Err(DemuxError::InvalidEbmlHeader(format!(
                "unsupported MaxSizeLength: {}",
                max_size_length
            )));
        }

        Ok(Self {
            version,
            read_version,
            max_id_length,
            max_size_length,
            doc_type,
            doc_type_version,
            doc_type_read_version,
        })
    }
}

impl EbmlHeader {
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

impl<R: Read + Seek> ParsableElement<R> for SeekEntry {
    type Output = Self;

    fn new(_r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
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

impl<R: Read + Seek> ParsableElement<R> for Info {
    type Output = Self;

    fn new(_r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        let timestamp_scale = find_nonzero_or(fields, ElementId::TimestampScale, 1000000)?;
        let duration = try_find_float(fields, ElementId::Duration)?;
        let date_utc = try_find_date(fields, ElementId::DateUtc)?;
        let title = try_find_string(fields, ElementId::Title)?;
        let muxing_app = find_string(fields, ElementId::MuxingApp)?;
        let writing_app = find_string(fields, ElementId::WritingApp)?;

        if let Some(duration) = duration {
            if duration < 0.0 {
                return Err(DemuxError::PositiveValueIsNotPositive);
            }
        }

        Ok(Self {
            timestamp_scale,
            duration,
            date_utc,
            title,
            muxing_app,
            writing_app,
        })
    }
}

impl Info {
    /// Timestamp scale in nanoseconds (1_000_000 means all timestamps in the Segment are expressed in milliseconds).
    pub fn timestamp_scale(&self) -> NonZeroU64 {
        self.timestamp_scale
    }

    /// Duration of the Segment in nanoseconds based on TimestampScale.
    pub fn duration(&self) -> Option<f64> {
        self.duration
    }

    /// The date and time that the Segment was created by the muxing application or library.
    pub fn date_utc(&self) -> Option<i64> {
        self.date_utc
    }

    /// General name of the Segment.
    pub fn title(&self) -> Option<&str> {
        match &self.title {
            None => None,
            Some(title) => Some(title),
        }
    }

    /// Muxing application or library.
    pub fn muxing_app(&self) -> &str {
        &self.muxing_app
    }

    /// Writing  application.
    pub fn writing_app(&self) -> &str {
        &self.writing_app
    }
}

/// The TrackEntry element.
#[derive(Clone, Debug)]
pub struct TrackEntry {
    track_number: NonZeroU64,
    track_uid: NonZeroU64,
    track_type: TrackType,
    flag_enabled: bool,
    flag_default: bool,
    flag_forced: bool,
    flag_lacing: bool,
    default_duration: Option<NonZeroU64>,
    name: Option<String>,
    language: Option<String>,
    codec_id: String,
    codec_private: Option<Vec<u8>>,
    codec_name: Option<String>,
    codec_delay: Option<u64>,
    seek_pre_roll: Option<u64>,
    audio: Option<Audio>,
    video: Option<Video>,
    content_encodings: Option<Vec<ContentEncoding>>,
}

impl<R: Read + Seek> ParsableElement<R> for TrackEntry {
    type Output = Self;

    fn new(r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        let track_number = find_nonzero(fields, ElementId::TrackNumber)?;
        let track_uid = find_nonzero(fields, ElementId::TrackUid)?;
        let track_type = find_custom_type(fields, ElementId::TrackType)?;
        let flag_enabled = find_bool_or(fields, ElementId::FlagEnabled, true)?;
        let flag_default = find_bool_or(fields, ElementId::FlagDefault, true)?;
        let flag_forced = find_bool_or(fields, ElementId::FlagForced, false)?;
        let flag_lacing = find_bool_or(fields, ElementId::FlagLacing, false)?;
        let default_duration = try_find_nonzero(fields, ElementId::DefaultDuration)?;
        let name = try_find_string(fields, ElementId::Name)?;
        let language = try_find_string(fields, ElementId::Language)?;
        let codec_id = find_string(fields, ElementId::CodecId)?;
        let codec_private = try_find_binary(r, fields, ElementId::CodecPrivate)?;
        let codec_name = try_find_string(fields, ElementId::CodecName)?;
        let codec_delay = try_find_unsigned(fields, ElementId::CodecDelay)?;
        let seek_pre_roll = try_find_unsigned(fields, ElementId::SeekPreRoll)?;

        let audio = try_parse_child::<_, Audio>(r, fields, ElementId::Audio)?;

        // TODO parse Video
        // TODO parse ContentEncoding

        Ok(Self {
            track_number,
            track_uid,
            track_type,
            flag_enabled,
            flag_default,
            flag_forced,
            flag_lacing,
            default_duration,
            name,
            language,
            codec_id,
            codec_private,
            codec_name,
            codec_delay,
            seek_pre_roll,
            audio,
            video: None,
            content_encodings: None,
        })
    }
}

impl TrackEntry {
    /// The track number as used in the Block Header.
    pub fn track_number(&self) -> NonZeroU64 {
        self.track_number
    }

    /// A unique ID to identify the Track.
    pub fn track_uid(&self) -> NonZeroU64 {
        self.track_uid
    }

    /// The type of the track.
    pub fn track_type(&self) -> TrackType {
        self.track_type
    }

    /// Indicates if a track is usable. It is possible to turn a not usable track
    /// into a usable track using chapter codecs or control tracks.
    pub fn flag_enabled(&self) -> bool {
        self.flag_enabled
    }

    /// Set if that track (audio, video or subs) should be eligible
    /// for automatic selection by the player.
    pub fn flag_default(&self) -> bool {
        self.flag_default
    }

    /// Applies only to subtitles. Set if that track should be eligible for automatic selection
    /// by the player if it matches the user's language preference, even if the user's preferences
    /// would normally not enable subtitles with the selected audio track.
    pub fn flag_forced(&self) -> bool {
        self.flag_forced
    }

    /// Indicates if the track may contain blocks using lacing.
    pub fn flag_lacing(&self) -> bool {
        self.flag_lacing
    }

    /// Number of nanoseconds (not scaled via TimestampScale) per frame (one Element put into a (Simple)Block).
    pub fn default_duration(&self) -> Option<NonZeroU64> {
        self.default_duration
    }

    /// A human-readable track name.
    pub fn name(&self) -> Option<&str> {
        match &self.name {
            None => None,
            Some(name) => Some(name),
        }
    }

    /// Specifies the language of the track.
    pub fn language(&self) -> Option<&str> {
        match &self.language {
            None => None,
            Some(language) => Some(language),
        }
    }

    /// An ID corresponding to the codec.
    pub fn codec_id(&self) -> &str {
        &self.codec_id
    }

    /// Private data only known to the codec.
    pub fn codec_private(&self) -> Option<&[u8]> {
        match &self.codec_private {
            None => None,
            Some(data) => Some(data),
        }
    }

    /// A human-readable string specifying the codec.
    pub fn codec_name(&self) -> Option<&str> {
        match &self.codec_name {
            None => None,
            Some(codec_name) => Some(codec_name),
        }
    }

    /// CodecDelay is ehe codec-built-in delay in nanoseconds.
    /// This value must be subtracted from each block timestamp in order to get the actual timestamp.
    pub fn codec_delay(&self) -> Option<u64> {
        self.codec_delay
    }

    /// After a discontinuity, SeekPreRoll is the duration in nanoseconds of the data the decoder
    /// must decode before the decoded data is valid.
    pub fn seek_pre_roll(&self) -> Option<u64> {
        self.seek_pre_roll
    }

    /// Video settings.
    pub fn video(&self) -> Option<&Video> {
        self.video.as_ref()
    }

    /// Audio settings.
    pub fn audio(&self) -> Option<&Audio> {
        self.audio.as_ref()
    }

    /// Audio settings.
    pub fn content_encodings(&self) -> Option<&[ContentEncoding]> {
        match &self.content_encodings {
            None => None,
            Some(content_encodings) => Some(content_encodings),
        }
    }
}

/// The Audio element.
#[derive(Clone, Copy, Debug)]
pub struct Audio {
    sampling_frequency: f64,
    output_sampling_frequency: Option<f64>,
    channels: NonZeroU64,
    bit_depth: Option<NonZeroU64>,
}

impl<R: Read + Seek> ParsableElement<R> for Audio {
    type Output = Self;

    fn new(_r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        let sampling_frequency = find_float_or(fields, ElementId::SamplingFrequency, 8000.0)?;
        let output_sampling_frequency = try_find_float(fields, ElementId::OutputSamplingFrequency)?;
        let channels = find_nonzero_or(fields, ElementId::Channels, 1)?;
        let bit_depth = try_find_nonzero(fields, ElementId::BitDepth)?;

        if sampling_frequency < 0.0 {
            return Err(DemuxError::PositiveValueIsNotPositive);
        }

        if let Some(output_sampling_frequency) = output_sampling_frequency {
            if output_sampling_frequency < 0.0 {
                return Err(DemuxError::PositiveValueIsNotPositive);
            }
        }

        Ok(Audio {
            sampling_frequency,
            output_sampling_frequency,
            channels,
            bit_depth,
        })
    }
}

impl Audio {
    /// Sampling frequency in Hz.
    pub fn sampling_frequency(&self) -> f64 {
        self.sampling_frequency
    }

    /// Real output sampling frequency in Hz.
    pub fn output_sampling_frequency(&self) -> Option<f64> {
        self.output_sampling_frequency
    }

    /// Numbers of channels in the track.
    pub fn channels(&self) -> NonZeroU64 {
        self.channels
    }

    /// Bits per sample.
    pub fn bit_depth(&self) -> Option<NonZeroU64> {
        self.bit_depth
    }
}

/// The Video element.
#[derive(Clone, Copy, Debug)]
pub struct Video {
    // FlagInterlaced
    // StereoMode
    // AlphaMode
    // PixelWidth
    // PixelHeight
    // PixelCropBottom
    // PixelCropTop
    // PixelCropLeft
    // PixelCropRight
    // DisplayWidth
    // DisplayHeight
    // DisplayUnit
    // AspectRatioType
    color: Option<Colour>,
}

impl<R: Read + Seek> ParsableElement<R> for Video {
    type Output = Self;

    fn new(_r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        Ok(Self { color: None })
    }
}

/// The Colour element.
#[derive(Clone, Copy, Debug)]
pub struct Colour {
    // MatrixCoefficients
// BitsPerChannel
// ChromaSubsamplingHorz
// ChromaSubsamplingVert
// CbSubsamplingHorz
// CbSubsamplingVert
// ChromaSitingHorz
// ChromaSitingVert
// Range
// TransferCharacteristics
// Primaries
// MaxCLL
// MaxFALL
// Vec<MasteringMetadata>
}

impl<R: Read + Seek> ParsableElement<R> for Colour {
    type Output = Self;

    fn new(_r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        Ok(Self {})
    }
}

/// The MasteringMetadata element.
#[derive(Clone, Copy, Debug)]
pub struct MasteringMetadata {
    // PrimaryRChromaticityX
// PrimaryRChromaticityY
// PrimaryGChromaticityX
// PrimaryGChromaticityY
// PrimaryBChromaticityX
// PrimaryBChromaticityY
// WhitePointChromaticityX
// WhitePointChromaticityY
// LuminanceMax
// LuminanceMin
}

impl<R: Read + Seek> ParsableElement<R> for MasteringMetadata {
    type Output = Self;

    fn new(_r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        Ok(Self {})
    }
}

/// The ContentEncoding element.
#[derive(Clone, Copy, Debug)]
pub struct ContentEncoding {
    // ContentEncodingOrder
// ContentEncodingScope
// ContentEncodingType
// ContentEncryption
// ContentEncAlgo
// ContentEncKeyID
// ContentEncAESSettings
// AESSettingsCipherMode
}

impl<R: Read + Seek> ParsableElement<R> for ContentEncoding {
    type Output = Self;

    fn new(_r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        Ok(Self {})
    }
}

/// Demuxer for Matroska files.
#[derive(Clone, Debug)]
pub struct MatroskaFile<R> {
    file: R,
    ebml_header: EbmlHeader,
    seek_head: HashMap<ElementId, u64>,
    info: Info,
    tracks: Vec<TrackEntry>,
}

impl<R: Read + Seek> MatroskaFile<R> {
    /// Opens a Matroska file.
    pub fn open(mut file: R) -> Result<Self> {
        let ebml_header = parse_ebml_header(&mut file)?;

        let (segment_data_offset, _) = expect_master(&mut file, ElementId::Segment, None)?;
        let optional_seek_head = search_seek_head(&mut file, segment_data_offset)?;

        let mut seek_head = parse_seek_head(&mut file, segment_data_offset, optional_seek_head)?;

        if seek_head.is_empty() {
            build_seek_head(&mut file, segment_data_offset, &mut seek_head)?;
        }

        if seek_head.get(&ElementId::Cluster).is_none() {
            find_first_cluster_offset(&mut file, segment_data_offset, &mut seek_head)?;
        }

        let info = parse_segment_info(&mut file, &mut seek_head)?;
        let tracks = parse_tracks(&mut file, &mut seek_head)?;

        // TODO parse Cues element
        // TODO how to parse blocks and how to do seeking?
        // TODO we could add a BTreeMap and store the Cues in it. If no Cues have been found, we could (re-)build them too, if asked for (open(file: &mut File, build_cues: Bool)
        // TODO lazy loading: Chapters, Tagging

        Ok(Self {
            file,
            ebml_header,
            seek_head,
            info,
            tracks,
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

    /// Returns the tracks of the file.
    pub fn tracks(&self) -> &[TrackEntry] {
        &self.tracks
    }
}

/// Parses and verifies the EBML header.
fn parse_ebml_header<R: Read + Seek>(r: &mut R) -> Result<EbmlHeader> {
    let (master_offset, master_size) = expect_master(r, ElementId::Ebml, None)?;
    let master_children = collect_children(r, master_offset, master_size)?;
    let header = EbmlHeader::new(r, &master_children)?;
    Ok(header)
}

/// Parses the seek head if present.
fn parse_seek_head<R: Read + Seek>(
    mut file: &mut R,
    segment_data_offset: u64,
    optional_seek_head: Option<(u64, u64)>,
) -> Result<HashMap<ElementId, u64>> {
    let mut seek_head = HashMap::new();

    if let Some((seek_head_data_offset, seek_head_data_size)) = optional_seek_head {
        let seek_head_entries =
            collect_children(&mut file, seek_head_data_offset, seek_head_data_size)?;

        for (entry_id, entry_data) in &seek_head_entries {
            if let ElementId::Seek = entry_id {
                if let ElementData::Location { offset, size } = entry_data {
                    let seek_fields = collect_children(&mut file, *offset, *size)?;
                    if let Ok(seek_entry) = SeekEntry::new(&mut file, &seek_fields) {
                        let _ = seek_head
                            .insert(seek_entry.id, segment_data_offset + seek_entry.offset);
                    }
                }
            }
        }
    }

    Ok(seek_head)
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
    r: &mut R,
    seek_head: &mut HashMap<ElementId, u64>,
) -> Result<Info> {
    if let Some(offset) = seek_head.get(&ElementId::Info) {
        let (info_data_offset, info_data_size) = expect_master(r, ElementId::Info, Some(*offset))?;
        let children = collect_children(r, info_data_offset, info_data_size)?;
        let info = Info::new(r, &children)?;
        Ok(info)
    } else {
        Err(DemuxError::ElementNotFound(ElementId::Info))
    }
}

fn parse_tracks<R: Read + Seek>(
    r: &mut R,
    seek_head: &mut HashMap<ElementId, u64>,
) -> Result<Vec<TrackEntry>> {
    let mut tracks = vec![];
    if let Some(offset) = seek_head.get(&ElementId::Tracks) {
        let (data_offset, data_size) = expect_master(r, ElementId::Tracks, Some(*offset))?;
        let children = collect_children(r, data_offset, data_size)?;
        for (element_id, element_data) in children {
            if let ElementId::TrackEntry = element_id {
                if let ElementData::Location { offset, size } = element_data {
                    let children = collect_children(r, offset, size)?;
                    let track_entry = TrackEntry::new(r, &children)?;
                    tracks.push(track_entry)
                }
            }
        }
        Ok(tracks)
    } else {
        Err(DemuxError::ElementNotFound(ElementId::Info))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]

    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_parse_ebml_header() -> Result<()> {
        let data: Vec<u8> = vec![
            0x1A, 0x45, 0xDF, 0xA3, 0xA2, 0x42, 0x86, 0x81, 0x01, 0x42, 0xF7, 0x81, 0x01, 0x42,
            0xF2, 0x81, 0x04, 0x42, 0xF3, 0x81, 0x08, 0x42, 0x82, 0x88, 0x6D, 0x61, 0x74, 0x72,
            0x6F, 0x73, 0x6B, 0x61, 0x42, 0x87, 0x81, 0x04, 0x42, 0x85, 0x81, 0x02,
        ];
        let mut cursor = Cursor::new(data);
        let ebml_header = parse_ebml_header(&mut cursor)?;
        assert_eq!(ebml_header.version, Some(1));
        assert_eq!(ebml_header.read_version, Some(1));
        assert_eq!(ebml_header.max_id_length, 4);
        assert_eq!(ebml_header.max_size_length, 8);
        assert_eq!(&ebml_header.doc_type, "matroska");
        assert_eq!(ebml_header.doc_type_version, 4);
        assert_eq!(ebml_header.doc_type_read_version, 2);

        Ok(())
    }
}
