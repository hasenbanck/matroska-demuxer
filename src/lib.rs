#![warn(missing_docs)]
#![deny(unused_results)]
#![deny(clippy::as_conversions)]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]
//! A demuxer that can demux Matroska and WebM container files.
//!
//! # Example:
//! ```ignore
//! let file = File::open("test.mkv").unwrap();
//! let mkv = MatroskaFile::open(file).unwrap();
//! assert!(mkv.tracks().len() >= 1);
//! ```

use std::collections::HashMap;
use std::convert::TryInto;
use std::io::{Read, Seek, SeekFrom};
use std::num::NonZeroU64;

use ebml::{
    collect_children, expect_master, find_bool_or, find_custom_type, find_float_or, find_nonzero,
    find_nonzero_or, find_string, find_unsigned, find_unsigned_or, next_element,
    parse_children_at_offset, parse_element_header, try_find_binary, try_find_custom_type,
    try_find_custom_type_or, try_find_date, try_find_float, try_find_nonzero, try_find_string,
    try_find_unsigned, try_parse_child, try_parse_children, ElementData, ParsableElement,
};
pub use element_id::ElementId;
use element_id::ID_TO_ELEMENT_ID;
pub use enums::*;
pub use error::DemuxError;

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

/// Contains general information about the segment.
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

/// Describes a track.
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
        let video = try_parse_child::<_, Video>(r, fields, ElementId::Video)?;

        let content_encodings = try_parse_children::<_, ContentEncoding>(
            r,
            fields,
            ElementId::ContentEncodings,
            ElementId::ContentEncoding,
        )?;

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
            video,
            content_encodings,
        })
    }
}

impl TrackEntry {
    /// The track number as used in the block header.
    pub fn track_number(&self) -> NonZeroU64 {
        self.track_number
    }

    /// A unique ID to identify the track.
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

    /// Settings for several content encoding mechanisms like compression or encryption.
    pub fn content_encodings(&self) -> Option<&[ContentEncoding]> {
        match &self.content_encodings {
            None => None,
            Some(content_encodings) => Some(content_encodings),
        }
    }
}

/// Audio settings.
#[derive(Clone, Debug)]
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

/// Video settings.
#[derive(Clone, Debug)]
pub struct Video {
    flag_interlaced: FlagInterlaced,
    stereo_mode: Option<StereoMode>,
    alpha_mode: Option<u64>,
    pixel_width: NonZeroU64,
    pixel_height: NonZeroU64,
    pixel_crop_bottom: Option<u64>,
    pixel_crop_top: Option<u64>,
    pixel_crop_left: Option<u64>,
    pixel_crop_right: Option<u64>,
    display_width: Option<NonZeroU64>,
    display_height: Option<NonZeroU64>,
    display_unit: Option<DisplayUnit>,
    aspect_ratio_type: Option<AspectRatioType>,
    colour: Option<Colour>,
}

impl<R: Read + Seek> ParsableElement<R> for Video {
    type Output = Self;

    fn new(r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        let flag_interlaced =
            try_find_custom_type_or(fields, ElementId::FlagInterlaced, FlagInterlaced::Unknown)?;
        let stereo_mode = try_find_custom_type(fields, ElementId::StereoMode)?;
        let alpha_mode = try_find_unsigned(fields, ElementId::AlphaMode)?;
        let pixel_width = find_nonzero(fields, ElementId::PixelWidth)?;
        let pixel_height = find_nonzero(fields, ElementId::PixelHeight)?;
        let pixel_crop_bottom = try_find_unsigned(fields, ElementId::PixelCropBottom)?;
        let pixel_crop_top = try_find_unsigned(fields, ElementId::PixelCropTop)?;
        let pixel_crop_left = try_find_unsigned(fields, ElementId::PixelCropLeft)?;
        let pixel_crop_right = try_find_unsigned(fields, ElementId::PixelCropRight)?;
        let display_width = try_find_nonzero(fields, ElementId::DisplayWidth)?;
        let display_height = try_find_nonzero(fields, ElementId::DisplayHeight)?;
        let display_unit = try_find_custom_type(fields, ElementId::DisplayUnit)?;
        let aspect_ratio_type = try_find_custom_type(fields, ElementId::AspectRatioType)?;
        let colour = try_parse_child::<_, Colour>(r, fields, ElementId::Colour)?;

        Ok(Self {
            flag_interlaced,
            stereo_mode,
            alpha_mode,
            pixel_width,
            pixel_height,
            pixel_crop_bottom,
            pixel_crop_top,
            pixel_crop_left,
            pixel_crop_right,
            display_width,
            display_height,
            display_unit,
            aspect_ratio_type,
            colour,
        })
    }
}

impl Video {
    /// A flag to declare if the video is known to be progressive, or interlaced,
    /// and if applicable to declare details about the interlacement.
    pub fn flag_interlaced(&self) -> FlagInterlaced {
        self.flag_interlaced
    }

    /// Stereo-3D video mode.
    pub fn stereo_mode(&self) -> Option<StereoMode> {
        self.stereo_mode
    }

    /// Alpha Video Mode. Presence of this Element indicates that the
    /// BlockAdditional Element could contain Alpha data.
    pub fn alpha_mode(&self) -> Option<u64> {
        self.alpha_mode
    }

    /// Width of the encoded video frames in pixels.
    pub fn pixel_width(&self) -> NonZeroU64 {
        self.pixel_width
    }

    /// Height of the encoded video frames in pixels.
    pub fn pixel_height(&self) -> NonZeroU64 {
        self.pixel_height
    }

    /// The number of video pixels to remove at the bottom of the image.
    pub fn pixel_crop_bottom(&self) -> Option<u64> {
        self.pixel_crop_bottom
    }

    /// The number of video pixels to remove at the top of the image.
    pub fn pixel_crop_top(&self) -> Option<u64> {
        self.pixel_crop_top
    }

    /// The number of video pixels to remove on the left of the image.
    pub fn pixel_crop_left(&self) -> Option<u64> {
        self.pixel_crop_left
    }

    /// The number of video pixels to remove on the right of the image.
    pub fn pixel_crop_right(&self) -> Option<u64> {
        self.pixel_crop_right
    }

    /// Width of the video frames to display.
    /// Applies to the video frame after cropping (PixelCrop* Elements).
    pub fn display_width(&self) -> Option<NonZeroU64> {
        self.display_width
    }

    /// Height of the video frames to display.
    /// Applies to the video frame after cropping (PixelCrop* Elements).
    pub fn display_height(&self) -> Option<NonZeroU64> {
        self.display_height
    }

    /// How DisplayWidth & DisplayHeight are interpreted.
    pub fn display_unit(&self) -> Option<DisplayUnit> {
        self.display_unit
    }

    /// Specify the possible modifications to the aspect ratio.
    pub fn aspect_ratio_type(&self) -> Option<AspectRatioType> {
        self.aspect_ratio_type
    }

    /// Settings describing the colour format.
    pub fn colour(&self) -> Option<&Colour> {
        self.colour.as_ref()
    }
}

/// Settings describing the colour format.
#[derive(Clone, Debug)]
pub struct Colour {
    matrix_coefficients: Option<MatrixCoefficients>,
    bits_per_channel: Option<u64>,
    chroma_subsampling_horz: Option<u64>,
    chroma_subsampling_vert: Option<u64>,
    cb_subsampling_horz: Option<u64>,
    cb_subsampling_vert: Option<u64>,
    chroma_sitting_horz: Option<ChromaSitingHorz>,
    chroma_sitting_vert: Option<ChromaSitingVert>,
    range: Option<Range>,
    transfer_characteristics: Option<TransferCharacteristics>,
    primaries: Option<Primaries>,
    max_cll: Option<u64>,
    max_fall: Option<u64>,
    mastering_metadata: Option<MasteringMetadata>,
}

impl<R: Read + Seek> ParsableElement<R> for Colour {
    type Output = Self;

    fn new(r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        let matrix_coefficients = try_find_custom_type(fields, ElementId::MatrixCoefficients)?;
        let bits_per_channel = try_find_unsigned(fields, ElementId::BitsPerChannel)?;
        let chroma_subsampling_horz = try_find_unsigned(fields, ElementId::ChromaSubsamplingHorz)?;
        let chroma_subsampling_vert = try_find_unsigned(fields, ElementId::ChromaSubsamplingVert)?;
        let cb_subsampling_horz = try_find_unsigned(fields, ElementId::CbSubsamplingHorz)?;
        let cb_subsampling_vert = try_find_unsigned(fields, ElementId::CbSubsamplingVert)?;
        let chroma_sitting_horz = try_find_custom_type(fields, ElementId::ChromaSitingHorz)?;
        let chroma_sitting_vert = try_find_custom_type(fields, ElementId::ChromaSitingVert)?;
        let range = try_find_custom_type(fields, ElementId::Range)?;
        let transfer_characteristics =
            try_find_custom_type(fields, ElementId::TransferCharacteristics)?;
        let primaries = try_find_custom_type(fields, ElementId::Primaries)?;
        let max_cll = try_find_unsigned(fields, ElementId::MatrixCoefficients)?;
        let max_fall = try_find_unsigned(fields, ElementId::MatrixCoefficients)?;
        let mastering_metadata =
            try_parse_child::<_, MasteringMetadata>(r, fields, ElementId::MasteringMetadata)?;

        Ok(Self {
            matrix_coefficients,
            bits_per_channel,
            chroma_subsampling_horz,
            chroma_subsampling_vert,
            cb_subsampling_horz,
            cb_subsampling_vert,
            chroma_sitting_horz,
            chroma_sitting_vert,
            range,
            transfer_characteristics,
            primaries,
            max_cll,
            max_fall,
            mastering_metadata,
        })
    }
}

impl Colour {
    /// The matrix coefficients of the video used to derive luma and chroma values
    /// from red, green, and blue color primaries.
    pub fn matrix_coefficients(&self) -> Option<MatrixCoefficients> {
        self.matrix_coefficients
    }
    /// Number of decoded bits per channel.
    pub fn bits_per_channel(&self) -> Option<u64> {
        self.bits_per_channel
    }

    /// The amount of pixels to remove in the Cr and Cb channels
    /// for every pixel not removed horizontally.
    pub fn chroma_subsampling_horz(&self) -> Option<u64> {
        self.chroma_subsampling_horz
    }

    /// The amount of pixels to remove in the Cr and Cb channels
    /// for every pixel not removed vertically.
    pub fn chroma_subsampling_vert(&self) -> Option<u64> {
        self.chroma_subsampling_vert
    }

    /// The amount of pixels to remove in the Cb channel for every pixel not removed horizontally.
    pub fn cb_subsampling_horz(&self) -> Option<u64> {
        self.cb_subsampling_horz
    }

    /// The amount of pixels to remove in the Cb channel for every pixel not removed vertically.
    pub fn cb_subsampling_vert(&self) -> Option<u64> {
        self.cb_subsampling_vert
    }

    /// How chroma is sub sampled horizontally.
    pub fn chroma_sitting_horz(&self) -> Option<ChromaSitingHorz> {
        self.chroma_sitting_horz
    }

    /// How chroma is sub sampled vertically.
    pub fn chroma_sitting_vert(&self) -> Option<ChromaSitingVert> {
        self.chroma_sitting_vert
    }

    /// Clipping of the color ranges.
    pub fn range(&self) -> Option<Range> {
        self.range
    }

    /// The transfer characteristics of the video.
    pub fn transfer_characteristics(&self) -> Option<TransferCharacteristics> {
        self.transfer_characteristics
    }

    /// The colour primaries of the video.
    pub fn primaries(&self) -> Option<Primaries> {
        self.primaries
    }

    /// Maximum brightness of a single pixel (cd/m^2^).
    pub fn max_cll(&self) -> Option<u64> {
        self.max_cll
    }

    /// Maximum brightness of a single full frame (cd/m^2^).
    pub fn max_fall(&self) -> Option<u64> {
        self.max_fall
    }

    /// SMPTE 2086 mastering data.
    pub fn mastering_metadata(&self) -> Option<&MasteringMetadata> {
        self.mastering_metadata.as_ref()
    }
}

/// SMPTE 2086 mastering data.
#[derive(Clone, Debug)]
pub struct MasteringMetadata {
    primary_r_chromaticity_x: Option<f64>,
    primary_r_chromaticity_y: Option<f64>,
    primary_g_chromaticity_x: Option<f64>,
    primary_g_chromaticity_y: Option<f64>,
    primary_b_chromaticity_x: Option<f64>,
    primary_b_chromaticity_y: Option<f64>,
    white_point_chromaticity_x: Option<f64>,
    white_point_chromaticity_y: Option<f64>,
    luminance_max: Option<f64>,
    luminance_min: Option<f64>,
}

impl<R: Read + Seek> ParsableElement<R> for MasteringMetadata {
    type Output = Self;

    fn new(_r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        let primary_r_chromaticity_x = try_find_float(fields, ElementId::PrimaryRChromaticityX)?;
        let primary_r_chromaticity_y = try_find_float(fields, ElementId::PrimaryRChromaticityX)?;
        let primary_g_chromaticity_x = try_find_float(fields, ElementId::PrimaryGChromaticityX)?;
        let primary_g_chromaticity_y = try_find_float(fields, ElementId::PrimaryGChromaticityX)?;
        let primary_b_chromaticity_x = try_find_float(fields, ElementId::PrimaryBChromaticityX)?;
        let primary_b_chromaticity_y = try_find_float(fields, ElementId::PrimaryBChromaticityX)?;
        let white_point_chromaticity_x =
            try_find_float(fields, ElementId::WhitePointChromaticityX)?;
        let white_point_chromaticity_y =
            try_find_float(fields, ElementId::WhitePointChromaticityY)?;
        let luminance_max = try_find_float(fields, ElementId::LuminanceMax)?;
        let luminance_min = try_find_float(fields, ElementId::LuminanceMin)?;

        Ok(Self {
            primary_r_chromaticity_x,
            primary_r_chromaticity_y,
            primary_g_chromaticity_x,
            primary_g_chromaticity_y,
            primary_b_chromaticity_x,
            primary_b_chromaticity_y,
            white_point_chromaticity_x,
            white_point_chromaticity_y,
            luminance_max,
            luminance_min,
        })
    }
}

impl MasteringMetadata {
    /// Red X chromaticity coordinate, as defined by CIE 1931.
    pub fn primary_r_chromaticity_x(&self) -> Option<f64> {
        self.primary_r_chromaticity_x
    }

    /// Red Y chromaticity coordinate, as defined by CIE 1931.
    pub fn primary_r_chromaticity_y(&self) -> Option<f64> {
        self.primary_r_chromaticity_y
    }

    /// Green X chromaticity coordinate, as defined by CIE 1931.
    pub fn primary_g_chromaticity_x(&self) -> Option<f64> {
        self.primary_g_chromaticity_x
    }

    /// Green Y chromaticity coordinate, as defined by CIE 1931
    pub fn primary_g_chromaticity_y(&self) -> Option<f64> {
        self.primary_g_chromaticity_y
    }

    /// Blue X chromaticity coordinate, as defined by CIE 1931.
    pub fn primary_b_chromaticity_x(&self) -> Option<f64> {
        self.primary_g_chromaticity_x
    }

    /// Blue Y chromaticity coordinate, as defined by CIE 1931.
    pub fn primary_b_chromaticity_y(&self) -> Option<f64> {
        self.primary_g_chromaticity_y
    }

    /// White X chromaticity coordinate, as defined by CIE 1931.
    pub fn white_point_chromaticity_x(&self) -> Option<f64> {
        self.primary_g_chromaticity_x
    }

    /// White Y chromaticity coordinate, as defined by CIE 1931.
    pub fn white_point_chromaticity_y(&self) -> Option<f64> {
        self.primary_g_chromaticity_y
    }

    /// Maximum luminance. Represented in candelas per square meter (cd/m^2^).
    pub fn luminance_max(&self) -> Option<f64> {
        self.luminance_max
    }

    /// Minimum luminance. Represented in candelas per square meter (cd/m^2^).
    pub fn luminance_min(&self) -> Option<f64> {
        self.luminance_min
    }
}

/// Settings for one content encoding like compression or encryption.
#[derive(Clone, Debug)]
pub struct ContentEncoding {
    order: u64,
    scope: u64,
    encoding_type: ContentEncodingType,
    encryption: Option<ContentEncryption>,
}

impl<R: Read + Seek> ParsableElement<R> for ContentEncoding {
    type Output = Self;

    fn new(r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        let order = find_unsigned_or(fields, ElementId::ContentEncodingOrder, 0)?;
        let scope = find_unsigned_or(fields, ElementId::ContentEncodingScope, 1)?;

        let encoding_type = try_find_custom_type_or(
            fields,
            ElementId::ContentEncodingType,
            ContentEncodingType::Compression,
        )?;

        let encryption =
            try_parse_child::<_, ContentEncryption>(r, fields, ElementId::ContentEncryption)?;

        Ok(Self {
            order,
            scope,
            encoding_type,
            encryption,
        })
    }
}

impl ContentEncoding {
    /// Tells when this modification was used during encoding / muxing starting
    /// with 0 and counting upwards.
    pub fn order(&self) -> u64 {
        self.order
    }

    /// A bit field that describes which Elements have been modified in this way.
    ///
    /// Values (big-endian) can be OR'ed:
    ///
    /// 1 - All frame contents, excluding lacing data.
    /// 2 - The track's private data.
    /// 4 - The next ContentEncoding.
    pub fn scope(&self) -> u64 {
        self.scope
    }

    /// Describes what kind of transformation is applied.
    pub fn encoding_type(&self) -> ContentEncodingType {
        self.encoding_type
    }

    /// Settings describing the encryption used.
    pub fn encryption(&self) -> Option<&ContentEncryption> {
        self.encryption.as_ref()
    }
}

/// Settings describing the encryption used.
#[derive(Clone, Debug)]
pub struct ContentEncryption {
    algo: ContentEncAlgo,
    key_id: Option<Vec<u8>>,
    aes_settings: Option<ContentEncAesSettings>,
}

impl<R: Read + Seek> ParsableElement<R> for ContentEncryption {
    type Output = Self;

    fn new(r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        let algo = try_find_custom_type_or(
            fields,
            ElementId::ContentEncAlgo,
            ContentEncAlgo::NotEncrypted,
        )?;
        let key_id = try_find_binary(r, fields, ElementId::ContentEncKeyId)?;
        let aes_settings = try_parse_child::<_, ContentEncAesSettings>(
            r,
            fields,
            ElementId::ContentEncAesSettings,
        )?;

        Ok(Self {
            algo,
            key_id,
            aes_settings,
        })
    }
}

impl ContentEncryption {
    /// The encryption algorithm used.
    pub fn algo(&self) -> ContentEncAlgo {
        self.algo
    }

    /// The encryption algorithm used.
    pub fn key_id(&self) -> Option<&[u8]> {
        match &self.key_id {
            None => None,
            Some(key_id) => Some(key_id),
        }
    }

    /// The encryption algorithm used.
    pub fn aes_settings(&self) -> Option<&ContentEncAesSettings> {
        self.aes_settings.as_ref()
    }
}

/// Settings describing the encryption algorithm used.
#[derive(Clone, Debug)]
pub struct ContentEncAesSettings {
    aes_settings_cipher_mode: Option<AesSettingsCipherMode>,
}

impl<R: Read + Seek> ParsableElement<R> for ContentEncAesSettings {
    type Output = Self;

    fn new(_r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self> {
        let aes_settings_cipher_mode =
            try_find_custom_type(fields, ElementId::AesSettingsCipherMode)?;

        Ok(Self {
            aes_settings_cipher_mode,
        })
    }
}

impl ContentEncAesSettings {
    /// The AES cipher mode used in the encryption.
    pub fn aes_settings_cipher_mode(&self) -> Option<AesSettingsCipherMode> {
        self.aes_settings_cipher_mode
    }
}

/// Contains all information relative to a seek point in the segment.
#[derive(Clone, Debug)]
pub struct CuePoint {
    time: u64,
    track_position: CueTrackPositions,
}

impl<R: Read + Seek> ParsableElement<R> for CuePoint {
    type Output = Self;

    fn new(_r: &mut R, _fields: &[(ElementId, ElementData)]) -> Result<Self> {
        unimplemented!()
    }
}

/// Contain positions for different tracks corresponding to the timestamp.
#[derive(Clone, Debug)]
pub struct CueTrackPositions {
    track: u64,
    cluster_position: u64,
    relative_position: Option<u64>,
    cue_duration: Option<u64>,
    cue_block_number: Option<u64>,
}

impl<R: Read + Seek> ParsableElement<R> for CueTrackPositions {
    type Output = Self;

    fn new(_r: &mut R, _fields: &[(ElementId, ElementData)]) -> Result<Self> {
        unimplemented!()
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
    cue_points: Option<Vec<CuePoint>>,
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

        let tracks = if let Some(offset) = seek_head.get(&ElementId::Tracks) {
            parse_children_at_offset::<_, TrackEntry>(
                &mut file,
                *offset,
                ElementId::Tracks,
                ElementId::TrackEntry,
            )?
        } else {
            return Err(DemuxError::ElementNotFound(ElementId::Tracks));
        };

        let cue_points = if let Some(offset) = seek_head.get(&ElementId::Cues) {
            let cue_points = parse_children_at_offset::<_, CuePoint>(
                &mut file,
                *offset,
                ElementId::Cues,
                ElementId::CuePoint,
            )?;
            Some(cue_points)
        } else {
            None
        };

        // TODO implement parsing of blocks (with an iterator? Or a nextBlock() function?)
        /* TODO Implement seeking

            How to search for a seek point:
            let s = [0,50,90,100,150];
            let seek = 95;
            dbg!(s.binary_search_by(|e| e.cmp(&seek));
            Err(3) => Position 2 is the next smaller.
            Ok(2) => Position 2 is an exact fit.
            Err(s.len()) => seek time not in slice.

            To handle the case of seeking "out of the duration" we simply do a:

            let seek_pos = match err = {
                Ok(value) => value,
                Err(value) => value - 1,
            };
            let seek_pos = cues.len().min(seek_pos)

            With his seek_pos we have a starting point of a cluster (and maybe inside it too), from
            which we need to do the linear search until we found the first frame after the timestamp
            we want to seek to.

            If we don't have this seek_pos, we will start the linear search from the start or the end
            (start = timestamp is < duration / 2, end = timestamp is >= duration/2).
        */

        // TODO parse Chapters
        // TODO parse Tags

        Ok(Self {
            file,
            ebml_header,
            seek_head,
            info,
            tracks,
            cue_points,
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
