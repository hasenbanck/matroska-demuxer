//! MKV demux errors.

use std::error::Error;

/// Errors that can occur when demuxing Matroska files.
#[derive(Debug)]
pub enum MkvDemuxError {
    /// A `std::io::Error`.
    IoError(std::io::Error),
}

impl std::fmt::Display for MkvDemuxError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MkvDemuxError::IoError(err) => {
                write!(f, "{:?}", err.source())
            }
        }
    }
}

impl From<std::io::Error> for MkvDemuxError {
    fn from(err: std::io::Error) -> MkvDemuxError {
        MkvDemuxError::IoError(err)
    }
}

impl std::error::Error for MkvDemuxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            MkvDemuxError::IoError(ref e) => Some(e),
            _ => None,
        }
    }
}
