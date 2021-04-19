//! Demux errors.

use std::error::Error;

use crate::ebml::ElementData;

/// Errors that can occur when demuxing Matroska files.
#[derive(Debug)]
pub enum DemuxError {
    /// A `std::io::Error`.
    IoError(std::io::Error),
    /// A `std::string::FromUtf8Error`.
    FromUtf8Error(std::string::FromUtf8Error),
    /// An invalid EBML Element ID was found.
    InvalidEbmlElementId,
    /// An invalid EBML data size was found.
    InvalidEbmlDataSize,
    /// An invalid EBML header was found.
    InvalidEbmlHeader(String),
    /// Wrong float size.
    WrongFloatSize(u64),
    /// Unsupported DocType.
    UnsupportedDocType(String),
    /// Unsupported DocTypeReadVersion.
    UnsupportedDocTypeReadVersion(u64),
    /// Unexpected Element found.
    UnexpectedElement((u32, u32)),
    /// Unexpected data type found.
    UnexpectedDataType(ElementData),
}

impl std::fmt::Display for DemuxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DemuxError::IoError(err) => {
                write!(f, "{:?}", err.source())
            }
            DemuxError::FromUtf8Error(err) => {
                write!(f, "{:?}", err.source())
            }
            DemuxError::InvalidEbmlElementId => {
                write!(f, "invalid EBML Element ID was found")
            }
            DemuxError::InvalidEbmlDataSize => {
                write!(f, "invalid EBML data size was found")
            }
            DemuxError::InvalidEbmlHeader(message) => {
                write!(f, "invalid EBML header: {}", message)
            }
            DemuxError::WrongFloatSize(size) => {
                write!(
                    f,
                    "floats need to be either 4 or 7 bytes. found size of: {}",
                    size
                )
            }
            DemuxError::UnsupportedDocType(doctype) => {
                write!(
                    f,
                    "unsupported DocType. Only 'matroska' and 'webm' are supported': {}",
                    doctype
                )
            }
            DemuxError::UnsupportedDocTypeReadVersion(version) => {
                write!(f, "unsupported DocTypeReadVersion: {}", version)
            }
            DemuxError::UnexpectedElement((expected, found)) => {
                write!(
                    f,
                    "unexpected element found. Expected: {} Found: {}",
                    expected, found
                )
            }
            DemuxError::UnexpectedDataType(element) => {
                write!(f, "unexpected data type found: {:?}", element)
            }
        }
    }
}

impl From<std::io::Error> for DemuxError {
    fn from(err: std::io::Error) -> DemuxError {
        DemuxError::IoError(err)
    }
}

impl From<std::string::FromUtf8Error> for DemuxError {
    fn from(err: std::string::FromUtf8Error) -> DemuxError {
        DemuxError::FromUtf8Error(err)
    }
}

impl std::error::Error for DemuxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            DemuxError::IoError(ref e) => Some(e),
            DemuxError::FromUtf8Error(ref e) => Some(e),
            _ => None,
        }
    }
}
