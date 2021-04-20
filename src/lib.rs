#![warn(missing_docs)]
//! A simple Matroska demuxer that can demux Matroska and WebM container files.

use std::io::{Read, Seek};

pub use error::DemuxError;

use crate::ebml::parse_ebml_header;

mod ebml;
pub(crate) mod element_id;
mod error;

type Result<T> = std::result::Result<T, DemuxError>;

/// The Matrix Coefficients of the video used to derive luma and chroma values
/// from red, green, and blue color primaries. For clarity, the value and meanings
/// for MatrixCoefficients are adopted from Table 4 of ISO/IEC 23001-8:2016 or ITU-T H.273.
pub enum MatrixCoefficients {
    /// Unknown,
    Unknown,
    /// Identity.
    Identity,
    /// ITU-R BT.709.
    Bt709,
    /// Unspecified.
    Unspecified,
    /// Reserved.
    Reserved,
    /// US FCC 73.682.
    Fcc73682,
    /// ITU-R BT.470BG.
    Bt470,
    /// SMPTE 170M.
    Smpte170,
    /// SMPTE 240M.
    Smpte240,
    /// YCoCg.
    YCoCg,
    /// BT2020 Non-constant Luminance.
    Bt2020Ncl,
    /// BT2020 Constant Luminance.
    Bt2020Cl,
    /// SMPTE ST 2085.
    SmpteSt2085,
    /// Chroma-derived Non-constant Luminance.
    ChromaDerivedNcl,
    /// Chroma-derived Constant Luminance.
    ChromaDerivedCl,
    /// ITU-R BT.2100-0.
    Bt2100,
}

impl From<u64> for MatrixCoefficients {
    fn from(d: u64) -> Self {
        match d {
            0 => MatrixCoefficients::Identity,
            1 => MatrixCoefficients::Bt709,
            2 => MatrixCoefficients::Unspecified,
            3 => MatrixCoefficients::Reserved,
            4 => MatrixCoefficients::Fcc73682,
            5 => MatrixCoefficients::Bt470,
            6 => MatrixCoefficients::Smpte170,
            7 => MatrixCoefficients::Smpte240,
            8 => MatrixCoefficients::YCoCg,
            9 => MatrixCoefficients::Bt2020Ncl,
            10 => MatrixCoefficients::Bt2020Cl,
            11 => MatrixCoefficients::SmpteSt2085,
            12 => MatrixCoefficients::ChromaDerivedNcl,
            13 => MatrixCoefficients::ChromaDerivedCl,
            14 => MatrixCoefficients::Bt2100,
            _ => MatrixCoefficients::Unknown,
        }
    }
}

/// How DisplayWidth & DisplayHeight are interpreted.
pub enum DisplayUnit {
    /// In pixels.
    Pixels,
    /// In centimeters.
    Centimeters,
    /// In inches.
    Inches,
    /// By using the aspect ratio.
    DisplayAspectRatio,
    /// Unknown.
    Unknown,
}

impl From<u64> for DisplayUnit {
    fn from(d: u64) -> Self {
        match d {
            0 => DisplayUnit::Pixels,
            1 => DisplayUnit::Centimeters,
            2 => DisplayUnit::Inches,
            3 => DisplayUnit::DisplayAspectRatio,
            _ => DisplayUnit::Unknown,
        }
    }
}

/// Specify the possible modifications to the aspect ratio.
pub enum AspectRatioType {
    /// Unknown.
    Unknown,
    /// Allow free resizing.
    FreeResizing,
    /// Keep the aspect ratio.
    KeepAspectRatio,
    /// Fixed size.
    Fixed,
}

impl From<u64> for AspectRatioType {
    fn from(d: u64) -> Self {
        match d {
            0 => AspectRatioType::FreeResizing,
            1 => AspectRatioType::KeepAspectRatio,
            2 => AspectRatioType::Fixed,
            _ => AspectRatioType::Unknown,
        }
    }
}

/// Type of the track.
pub enum TrackType {
    /// Unknown.
    Unknown,
    /// Video track.
    Video,
    /// Audio track.
    Audio,
    /// A complex track.
    Complex,
    /// A logo.
    Logo,
    /// Subtitles.
    Subtitle,
    /// Buttons.
    Buttons,
    /// Controls.
    Control,
    /// Metadata.
    Metadata,
}

impl From<u64> for TrackType {
    fn from(d: u64) -> Self {
        match d {
            1 => TrackType::Video,
            2 => TrackType::Audio,
            3 => TrackType::Complex,
            16 => TrackType::Logo,
            17 => TrackType::Subtitle,
            18 => TrackType::Buttons,
            32 => TrackType::Control,
            33 => TrackType::Metadata,
            _ => TrackType::Unknown,
        }
    }
}

// TODO enum FlagInterlaced
// TODO enum StereoMode
// TODO enum ChromaSitingHorz
// TODO enum ChromaSitingVert
// TODO enum Range
// TODO enum TransferCharacteristics
// TODO enum Primaries
// TODO enum ContentEncodingScope
// TODO enum ContentEncodingType
// TODO enum ContentEncAlgo
// TODO enum AESSettingsCipherMode

/// The value of a simple tag.
pub enum SimpleTagValue {
    /// Unicode string.
    String(String),
    /// Binary data.
    Binary(Vec<u8>),
}

/// The EBML header of the file.
#[derive(Clone, Debug)]
pub struct EbmlHeader {
    version: u64,
    read_version: u64,
    max_id_length: u64,
    max_size_length: u64,
    doc_type: String,
    doc_type_version: u64,
    doc_type_read_version: u64,
}

impl EbmlHeader {
    /// The EBML version used to create the file.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// The minimum EBML version a parser has to support to read this file.
    pub fn read_version(&self) -> u64 {
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

/// Demuxer for Matroska files.
#[derive(Clone, Debug)]
pub struct MatroskaFile<R> {
    file: R,
    ebml_header: EbmlHeader,
}

impl<R: Read + Seek> MatroskaFile<R> {
    /// Opens a Matroska file. Also verifies the EBML header.
    pub fn open(mut file: R) -> Result<Self> {
        let ebml_header = parse_ebml_header(&mut file)?;
        Ok(Self { file, ebml_header })
    }

    /// Returns the EBML header.
    pub fn ebml_header(&self) -> &EbmlHeader {
        &self.ebml_header
    }
}
