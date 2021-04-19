//! Implement the parsing of EBML coded files.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};

use once_cell::sync::Lazy;

use crate::element_id::*;
use crate::{DemuxError, EBMLHeader, Result};

/// The doc type version this demuxer supports.
const DEMUXER_DOC_TYPE_VERSION: u64 = 4;

static ID_TO_TYPE: Lazy<HashMap<u32, ElementType>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(ElementId::Ebml as u32, ElementType::Master);
    m.insert(ElementId::EbmlVersion as u32, ElementType::UnsignedInteger);
    m.insert(
        ElementId::EbmlReadVersion as u32,
        ElementType::UnsignedInteger,
    );
    m.insert(
        ElementId::EbmlMaxIdLength as u32,
        ElementType::UnsignedInteger,
    );
    m.insert(
        ElementId::EbmlMaxSizeLength as u32,
        ElementType::UnsignedInteger,
    );
    m.insert(ElementId::DocType as u32, ElementType::String);
    m.insert(
        ElementId::DocTypeVersion as u32,
        ElementType::UnsignedInteger,
    );
    m.insert(
        ElementId::DocTypeReadVersion as u32,
        ElementType::UnsignedInteger,
    );
    m.insert(VOID, ElementType::Binary);
    m.insert(SEGMENT, ElementType::Master);
    m.insert(SEEK_HEAD, ElementType::Master);
    m.insert(SEEK, ElementType::Master);
    // This is a binary in the spec, but we convert the IDs to u32.
    m.insert(SEEK_ID, ElementType::UnsignedInteger);
    m.insert(SEEK_POSITION, ElementType::UnsignedInteger);
    m.insert(INFO, ElementType::Master);
    m.insert(TIMESTAMP_SCALE, ElementType::UnsignedInteger);
    m.insert(DURATION, ElementType::Float);
    m.insert(DATE_UTC, ElementType::Date);
    m.insert(TITLE, ElementType::String);
    m.insert(MUXING_APP, ElementType::String);
    m.insert(WRITING_APP, ElementType::String);
    m.insert(CLUSTER, ElementType::Master);
    m.insert(TIMESTAMP, ElementType::UnsignedInteger);
    m.insert(PREV_SIZE, ElementType::UnsignedInteger);
    m.insert(SIMPLE_BLOCK, ElementType::Binary);
    m.insert(BLOCK_GROUP, ElementType::Master);
    m.insert(BLOCK, ElementType::Binary);
    m.insert(BLOCK_ADDITIONS, ElementType::Master);
    m.insert(BLOCK_MORE, ElementType::Master);
    m.insert(BLOCK_ADD_ID, ElementType::UnsignedInteger);
    m.insert(BLOCK_ADDITIONAL, ElementType::Binary);
    m.insert(BLOCK_DURATION, ElementType::UnsignedInteger);
    m.insert(REFERENCE_BLOCK, ElementType::SignedInteger);
    m.insert(DISCARD_PADDING, ElementType::SignedInteger);
    m.insert(TRACKS, ElementType::Master);
    m.insert(TRACK_ENTRY, ElementType::Master);
    m.insert(TRACK_NUMBER, ElementType::UnsignedInteger);
    m.insert(TRACK_UID, ElementType::UnsignedInteger);
    m.insert(TRACK_TYPE, ElementType::UnsignedInteger);
    m.insert(FLAG_ENABLED, ElementType::UnsignedInteger);
    m.insert(FLAG_DEFAULT, ElementType::UnsignedInteger);
    m.insert(FLAG_FORCED, ElementType::UnsignedInteger);
    m.insert(FLAG_HEARING_IMPAIRED, ElementType::UnsignedInteger);
    m.insert(FLAG_VISUAL_IMPAIRED, ElementType::UnsignedInteger);
    m.insert(FLAG_TEXT_DESCRIPTIONS, ElementType::UnsignedInteger);
    m.insert(FLAG_ORIGINAL, ElementType::UnsignedInteger);
    m.insert(FLAG_COMMENTARY, ElementType::UnsignedInteger);
    m.insert(FLAG_LACING, ElementType::UnsignedInteger);
    m.insert(DEFAULT_DURATION, ElementType::UnsignedInteger);
    m.insert(NAME, ElementType::String);
    m.insert(LANGUAGE, ElementType::String);
    m.insert(CODEC_ID, ElementType::String);
    m.insert(CODEC_PRIVATE, ElementType::Binary);
    m.insert(CODEC_NAME, ElementType::String);
    m.insert(CODEC_DELAY, ElementType::UnsignedInteger);
    m.insert(SEEK_PRE_ROLL, ElementType::UnsignedInteger);
    // TODO mappings
    m
});

/// The types of elements a EBML file can have.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ElementType {
    /// Unknown element.
    Unknown,
    /// An element that contains other EBML elements als children.
    Master,
    /// Unsigned integer,
    UnsignedInteger,
    /// Signed integer,
    SignedInteger,
    /// Float,
    Float,
    /// Date,
    Date,
    /// String
    String,
    /// Binary
    Binary,
}

/// The data an element can contain.
#[derive(Clone, Debug, PartialEq)]
pub enum ElementData {
    /// Unknown element. Returns the Element ID.
    Unknown(u32),
    /// Returns the offset and size of the data.
    Location { offset: u64, size: u64 },
    /// Unsigned integer.
    UnsignedInteger(u64),
    /// Signed integer.
    SignedInteger(i64),
    /// Float.
    Float(f64),
    /// Date.
    Date(i64),
    /// String.
    String(String),
}

/// Parses and verifies the EBML header.
pub(crate) fn parse_ebml_header<R: Read + Seek>(r: &mut R) -> Result<EBMLHeader> {
    parse_expected_master(r, EBML)?;
    let version = parse_expected_unsigned_integer(r, EBML_VERSION)?;
    let read_version = parse_expected_unsigned_integer(r, EBML_READ_VERSION)?;
    let max_id_length = parse_expected_unsigned_integer(r, EBML_MAX_ID_LENGTH)?;
    let max_size_length = parse_expected_unsigned_integer(r, EBML_MAX_SIZE_LENGTH)?;
    let doc_type = parse_expected_string(r, DOC_TYPE)?;
    let doc_type_version = parse_expected_unsigned_integer(r, DOC_TYPE_VERSION)?;
    let doc_type_read_version = parse_expected_unsigned_integer(r, DOC_TYPE_READ_VERSION)?;

    if &doc_type != "matroska" && &doc_type != "webm" {
        return Err(DemuxError::UnsupportedDocType(doc_type));
    }

    if doc_type_read_version >= DEMUXER_DOC_TYPE_VERSION {
        return Err(DemuxError::UnsupportedDocTypeReadVersion(
            doc_type_read_version,
        ));
    }

    Ok(EBMLHeader {
        version,
        read_version,
        max_id_length,
        max_size_length,
        doc_type,
        doc_type_version,
        doc_type_read_version,
    })
}

/// Tries to parse a given Element ID that returns a master element at the current location of the reader.
pub(crate) fn parse_expected_master<R: Read + Seek>(
    r: &mut R,
    expected: u32,
) -> Result<(u64, u64)> {
    let element = parse_from(r, Some(expected), None)?;
    if let ElementData::Location { offset, size } = element {
        Ok((offset, size))
    } else {
        Err(DemuxError::UnexpectedDataType(element))
    }
}

/// Tries to parse a given Element ID that returns a unsigned integer at the current location of the reader.
pub(crate) fn parse_expected_unsigned_integer<R: Read + Seek>(
    r: &mut R,
    expected: u32,
) -> Result<u64> {
    let element = parse_from(r, Some(expected), None)?;
    if let ElementData::UnsignedInteger(value) = element {
        Ok(value)
    } else {
        Err(DemuxError::UnexpectedDataType(element))
    }
}

/// Tries to parse a given Element ID that returns a string at the current location of the reader.
pub(crate) fn parse_expected_string<R: Read + Seek>(r: &mut R, expected: u32) -> Result<String> {
    let element = parse_from(r, Some(expected), None)?;
    if let ElementData::String(value) = element {
        Ok(value)
    } else {
        Err(DemuxError::UnexpectedDataType(element))
    }
}

/// Parses the next element at the current location of the reader.
pub(crate) fn parse_next<R: Read + Seek>(r: &mut R) -> Result<ElementData> {
    parse_from(r, None, None)
}

/// Parses the next element from the given location inside the reader.
pub(crate) fn parse_from<R: Read + Seek>(
    r: &mut R,
    expected: Option<u32>,
    from: Option<u64>,
) -> Result<ElementData> {
    if let Some(from) = from {
        r.seek(SeekFrom::Start(from))?;
    }

    let element_id = parse_element_id(r)?;
    let size = parse_data_size(r)?;

    if let Some(expected) = expected {
        if element_id != expected {
            return Err(DemuxError::UnexpectedElement((expected, element_id)));
        }
    }

    let element_type = *ID_TO_TYPE.get(&element_id).unwrap_or(&ElementType::Unknown);

    // TODO Default values are used if the size is "0".
    // https://tools.ietf.org/html/rfc8794

    let element = match element_type {
        ElementType::Unknown => ElementData::Unknown(element_id),
        ElementType::Master => parse_location(r, size)?,
        ElementType::UnsignedInteger => parse_unsigned_integer(r, size)?,
        ElementType::SignedInteger => parse_signed_integer(r, size)?,
        ElementType::Float => parse_float(r, size)?,
        ElementType::Date => parse_date(r, size)?,
        ElementType::String => parse_string(r, size)?,
        ElementType::Binary => parse_location(r, size)?,
    };

    // TODO Default — The default value of the element to use if the parent element is present but this element is not.

    Ok(element)
}

/// Parses a variable length EBML Element ID.
fn parse_element_id<R: Read>(r: &mut R) -> Result<u32> {
    let mut bytes = [0u8];
    r.read_exact(&mut bytes)?;
    let element_id = match bytes[0] {
        byte if (byte & 0x80) == 0x80 => byte as u32,
        byte if (byte & 0xC0) == 0x40 => read_id_value(r, byte, 1)?,
        byte if (byte & 0xE0) == 0x20 => read_id_value(r, byte, 2)?,
        byte if (byte & 0xF0) == 0x10 => read_id_value(r, byte, 3)?,
        _ => return Err(DemuxError::InvalidEbmlElementId),
    };
    Ok(element_id)
}

/// Parses a variable length EBML data size.
fn parse_data_size<R: Read>(r: &mut R) -> Result<u64> {
    let mut bytes = [0u8];
    r.read_exact(&mut bytes)?;
    let size = match bytes[0] {
        byte if (byte & 0x80) == 0x80 => (0x7F & byte as u64),
        byte if (byte & 0xC0) == 0x40 => read_size_value(r, 0x3F & byte, 1)?,
        byte if (byte & 0xE0) == 0x20 => read_size_value(r, 0x1F & byte, 2)?,
        byte if (byte & 0xF0) == 0x10 => read_size_value(r, 0x0F & byte, 3)?,
        byte if (byte & 0xF8) == 0x08 => read_size_value(r, 0x07 & byte, 4)?,
        byte if (byte & 0xFC) == 0x04 => read_size_value(r, 0x03 & byte, 5)?,
        byte if (byte & 0xFE) == 0x02 => read_size_value(r, 0x01 & byte, 6)?,
        byte if byte == 0x01 => read_size_value(r, 0, 7)?,
        _ => return Err(DemuxError::InvalidEbmlDataSize),
    };
    Ok(size)
}

fn read_id_value<R: Read>(r: &mut R, byte: u8, left: usize) -> Result<u32> {
    let mut bytes = [byte, 0, 0, 0];
    r.read_exact(&mut bytes[1..1 + left])?;
    Ok(u32::from_be_bytes(bytes) >> (8 * (3 - left as u32)))
}

fn read_size_value<R: Read>(r: &mut R, byte: u8, left: usize) -> Result<u64> {
    let mut bytes = [byte, 0, 0, 0, 0, 0, 0, 0];
    r.read_exact(&mut bytes[1..1 + left])?;
    Ok(u64::from_be_bytes(bytes) >> (8 * (7 - left as u32)))
}

fn parse_location<R: Read + Seek>(r: &mut R, size: u64) -> Result<ElementData> {
    let offset = r.stream_position()?;
    Ok(ElementData::Location { offset, size })
}

fn parse_unsigned_integer<R: Read>(r: &mut R, size: u64) -> Result<ElementData> {
    let mut bytes = [0u8; 8];
    r.read_exact(&mut bytes[0..size as usize])?;
    let value = u64::from_be_bytes(bytes) >> (8 * (8 - size as u32));
    Ok(ElementData::UnsignedInteger(value))
}

fn parse_signed_integer<R: Read>(r: &mut R, size: u64) -> Result<ElementData> {
    let mut bytes = [0u8; 8];
    r.read_exact(&mut bytes[0..size as usize])?;
    let value = i64::from_be_bytes(bytes) >> (8 * (8 - size as u32));
    Ok(ElementData::SignedInteger(value))
}

fn parse_float<R: Read>(r: &mut R, size: u64) -> Result<ElementData> {
    let value = match size {
        4 => {
            let mut bytes = [0u8; 4];
            r.read_exact(&mut bytes)?;
            f32::from_be_bytes(bytes) as f64
        }
        8 => {
            let mut bytes = [0u8; 8];
            r.read_exact(&mut bytes)?;
            f64::from_be_bytes(bytes)
        }
        _ => return Err(DemuxError::WrongFloatSize(size)),
    };
    Ok(ElementData::Float(value))
}

fn parse_date<R: Read>(r: &mut R, size: u64) -> Result<ElementData> {
    let mut bytes = [0u8; 8];
    r.read_exact(&mut bytes[0..size as usize])?;
    let value = i64::from_be_bytes(bytes) >> (8 * (8 - size as u32));
    Ok(ElementData::Date(value))
}

fn parse_string<R: Read>(r: &mut R, size: u64) -> Result<ElementData> {
    let mut bytes = vec![0u8; size as usize];
    r.read_exact(&mut bytes[0..size as usize])?;
    let value = String::from_utf8(bytes)?;
    Ok(ElementData::String(value))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_parse_master_element() {
        let data: Vec<u8> = vec![0x1A, 0x45, 0xDF, 0xA3, 0xA2];
        let mut cursor = Cursor::new(data);
        let data = parse_next(&mut cursor).unwrap();
        assert_eq!(
            data,
            ElementData::Location {
                offset: 5,
                size: 34,
            }
        );
    }

    #[test]
    fn test_parse_unsigned_integer() {
        let data: Vec<u8> = vec![0x42, 0x86, 0x81, 0x01];
        let mut cursor = Cursor::new(data);
        let data = parse_next(&mut cursor).unwrap();
        assert_eq!(data, ElementData::UnsignedInteger(1));
    }

    #[test]
    fn test_parse_signed_integer() {
        let data: Vec<u8> = vec![0xFB, 0x82, 0xFF, 0xFB];
        let mut cursor = Cursor::new(data);
        let data = parse_next(&mut cursor).unwrap();
        assert_eq!(data, ElementData::SignedInteger(-5));
    }

    #[test]
    fn test_parse_date() {
        let data: Vec<u8> = vec![0x44, 0x61, 0x84, 0xFF, 0xB3, 0xB4, 0xC0];
        let mut cursor = Cursor::new(data);
        let data = parse_next(&mut cursor).unwrap();
        assert_eq!(data, ElementData::Date(-5_000_000));
    }

    #[test]
    fn test_parse_float_32() {
        let data: Vec<u8> = vec![0x44, 0x89, 0x84, 0x43, 0x1C, 0x20, 0x07];
        let mut cursor = Cursor::new(data);
        let data = parse_next(&mut cursor).unwrap();
        if let ElementData::Float(x) = data {
            assert!((x - 156.1251).abs() < 0.00001)
        } else {
            panic!("parse_element returned the wrong element type");
        }
    }

    #[test]
    fn test_parse_float_64() {
        let data: Vec<u8> = vec![
            0x44, 0x89, 0x88, 0x40, 0xA9, 0xE0, 0x43, 0x30, 0xBC, 0x60, 0x6E,
        ];
        let mut cursor = Cursor::new(data);
        let data = parse_next(&mut cursor).unwrap();
        if let ElementData::Float(x) = data {
            assert!((x - 3312.1312312).abs() < 0.00001)
        } else {
            panic!("parse_element returned the wrong element type");
        }
    }

    #[test]
    fn test_parse_ascii_string() {
        let data: Vec<u8> = vec![
            0x42, 0x82, 0x88, 0x6D, 0x61, 0x74, 0x72, 0x6F, 0x73, 0x6B, 0x61,
        ];
        let mut cursor = Cursor::new(data);
        let data = parse_next(&mut cursor).unwrap();
        assert_eq!(data, ElementData::String("matroska".to_owned()));
    }

    #[test]
    fn test_parse_utf8_string() {
        let data: Vec<u8> = vec![
            0x4D, 0x80, 0x95, 0xE3, 0x82, 0x82, 0xE3, 0x81, 0x90, 0xE3, 0x82, 0x82, 0xE3, 0x81,
            0x90, 0xE3, 0x81, 0x8A, 0xE3, 0x81, 0x8B, 0xE3, 0x82, 0x86,
        ];
        let mut cursor = Cursor::new(data);
        let data = parse_next(&mut cursor).unwrap();
        assert_eq!(data, ElementData::String("もぐもぐおかゆ".to_owned()));
    }

    #[test]
    fn test_parse_binary() {
        let data: Vec<u8> = vec![
            0x63, 0xA2, 0x95, 0xE3, 0x82, 0x82, 0xE3, 0x81, 0x90, 0xE3, 0x82, 0x82, 0xE3, 0x81,
            0x90, 0xE3, 0x81, 0x8A, 0xE3, 0x81, 0x8B, 0xE3, 0x82, 0x86,
        ];
        let mut cursor = Cursor::new(data);
        let data = parse_next(&mut cursor).unwrap();
        assert_eq!(
            data,
            ElementData::Location {
                offset: 3,
                size: 21,
            }
        );
    }

    #[test]
    fn test_parse_ebml_header() {
        let data: Vec<u8> = vec![
            0x1A, 0x45, 0xDF, 0xA3, 0xA2, 0x42, 0x86, 0x81, 0x01, 0x42, 0xF7, 0x81, 0x01, 0x42,
            0xF2, 0x81, 0x04, 0x42, 0xF3, 0x81, 0x08, 0x42, 0x82, 0x88, 0x6D, 0x61, 0x74, 0x72,
            0x6F, 0x73, 0x6B, 0x61, 0x42, 0x87, 0x81, 0x04, 0x42, 0x85, 0x81, 0x02,
        ];
        let mut cursor = Cursor::new(data);
        let ebml_header = parse_ebml_header(&mut cursor).unwrap();
        assert_eq!(ebml_header.version, 1);
        assert_eq!(ebml_header.read_version, 1);
        assert_eq!(ebml_header.max_id_length, 4);
        assert_eq!(ebml_header.max_size_length, 8);
        assert_eq!(&ebml_header.doc_type, "matroska");
        assert_eq!(ebml_header.doc_type_version, 4);
        assert_eq!(ebml_header.doc_type_read_version, 2);
    }
}
