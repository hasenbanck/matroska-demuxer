#![warn(missing_docs)]
//! A simple Matroska demuxer that can demux Matroska and WebM container files.

use std::io::{Read, Seek};

pub use enums::*;
pub use error::DemuxError;

use crate::ebml::parse_ebml_header;

mod ebml;
pub(crate) mod element_id;
mod enums;
mod error;

type Result<T> = std::result::Result<T, DemuxError>;

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
