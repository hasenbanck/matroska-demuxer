#![warn(missing_docs)]
//! A simple Matroska demuxer that can demux Matroska and WebM container files.

pub use error::MkvDemuxError;

mod error;

type Result<T> = std::result::Result<T, MkvDemuxError>;
