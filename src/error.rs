//! Demux errors.

use std::error::Error;

use crate::element_id::ElementId;

/// Errors that can occur when demuxing Matroska files.
#[derive(Debug)]
pub enum DemuxError {
    /// A `std::io::Error`.
    IoError(std::io::Error),
    /// A `std::string::FromUtf8Error`.
    FromUtf8Error(std::string::FromUtf8Error),
    /// A `TryFromIntError`.
    TryFromIntError(std::num::TryFromIntError),
    /// An invalid EBML Element ID was found.
    InvalidEbmlElementId,
    /// An invalid EBML data size was found.
    InvalidEbmlDataSize,
    /// An invalid EBML header was found.
    InvalidEbmlHeader(String),
    /// Wrong float size.
    WrongFloatSize(u64),
    /// Wrong integer size.
    WrongIntegerSize(u64),
    /// Wrong date size.
    WrongDateSize(u64),
    /// Unsupported DocType.
    UnsupportedDocType(String),
    /// Unsupported DocTypeReadVersion.
    UnsupportedDocTypeReadVersion(u64),
    /// Unexpected element found.
    UnexpectedElement((ElementId, ElementId)),
    /// Unexpected data type found.
    UnexpectedDataType,
    /// Can't find the expected element.
    ElementNotFound(ElementId),
    /// Can't find a cluster element.
    CantFindCluster,
    /// A value that should not be zero was zero.
    NonZeroValueIsZero(ElementId),
    /// A value that should be positive is not positive.
    PositiveValueIsNotPositive,
}

impl std::fmt::Display for DemuxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(err) => {
                write!(f, "{:?}", err.source())
            }
            Self::FromUtf8Error(err) => {
                write!(f, "{:?}", err.source())
            }
            Self::TryFromIntError(err) => {
                write!(f, "{:?}", err.source())
            }
            Self::InvalidEbmlElementId => {
                write!(f, "invalid EBML Element ID was found")
            }
            Self::InvalidEbmlDataSize => {
                write!(f, "invalid EBML data size was found")
            }
            Self::InvalidEbmlHeader(message) => {
                write!(f, "invalid EBML header: {message}",)
            }
            Self::WrongFloatSize(size) => {
                write!(
                    f,
                    "floats need to be either 4 or 7 bytes. Found size of: {size}",
                )
            }
            Self::WrongIntegerSize(size) => {
                write!(f, "integers can be at most 8 bytes. Found size of: {size}",)
            }
            Self::WrongDateSize(size) => {
                write!(f, "date can be at most 8 bytes. Found size of: {size}",)
            }
            Self::UnsupportedDocType(doctype) => {
                write!(
                    f,
                    "unsupported DocType. Only 'matroska' and 'webm' are supported': {doctype}",
                )
            }
            Self::UnsupportedDocTypeReadVersion(version) => {
                write!(f, "unsupported DocTypeReadVersion: {version}")
            }
            Self::UnexpectedElement((expected, found)) => {
                write!(
                    f,
                    "unexpected element found. Expected: {expected:?} Found: {found:?}",
                )
            }
            Self::UnexpectedDataType => {
                write!(f, "unexpected data type found")
            }
            Self::ElementNotFound(element_id) => {
                write!(f, "can't find Element: {element_id:?}",)
            }
            Self::CantFindCluster => {
                write!(f, "can't find the first cluster element")
            }
            Self::NonZeroValueIsZero(element_id) => {
                write!(
                    f,
                    "a value that should not be zero was zero: {element_id:?}",
                )
            }
            Self::PositiveValueIsNotPositive => {
                write!(f, "a value that should be positive is not positive")
            }
        }
    }
}

impl From<std::io::Error> for DemuxError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<std::string::FromUtf8Error> for DemuxError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Self::FromUtf8Error(err)
    }
}

impl From<std::num::TryFromIntError> for DemuxError {
    fn from(err: std::num::TryFromIntError) -> Self {
        Self::TryFromIntError(err)
    }
}

impl Error for DemuxError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            Self::IoError(ref e) => Some(e),
            Self::FromUtf8Error(ref e) => Some(e),
            Self::TryFromIntError(ref e) => Some(e),
            _ => None,
        }
    }
}
