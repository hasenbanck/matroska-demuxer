//! Implement the parsing of EBML coded files.

use std::convert::TryInto;
use std::io::{Read, Seek, SeekFrom};

use crate::element_id::{ElementId, ElementType, ELEMENT_ID_TO_TYPE, ID_TO_ELEMENT_ID};
use crate::{DemuxError, EbmlHeader, Result};

/// The doc type version this demuxer supports.
const DEMUXER_DOC_TYPE_VERSION: u64 = 4;

/// The data an element can contain.
#[derive(Clone, Debug, PartialEq)]
pub enum ElementData {
    /// Unknown element data.
    Unknown,
    /// Returns the offset and size of the data.
    Location { offset: u64, size: u64 },
    /// Unsigned integer.
    Unsigned(u64),
    /// Signed integer.
    Signed(i64),
    /// Float.
    Float(f64),
    /// Date.
    Date(i64),
    /// String.
    String(String),
}

/// Parses and verifies the EBML header.
pub(crate) fn parse_ebml_header<R: Read + Seek>(r: &mut R) -> Result<EbmlHeader> {
    let (offset, _) = expect_master(r, ElementId::Ebml, None)?;

    let version = expect_unsigned(r, ElementId::EbmlVersion, Some(offset))?;
    let read_version = expect_unsigned(r, ElementId::EbmlReadVersion, None)?;
    let max_id_length = expect_unsigned(r, ElementId::EbmlMaxIdLength, None)?;
    let max_size_length = expect_unsigned(r, ElementId::EbmlMaxSizeLength, None)?;
    let doc_type = expect_string(r, ElementId::DocType, None)?;
    let doc_type_version = expect_unsigned(r, ElementId::DocTypeVersion, None)?;
    let doc_type_read_version = expect_unsigned(r, ElementId::DocTypeReadVersion, None)?;

    if &doc_type != "matroska" && &doc_type != "webm" {
        return Err(DemuxError::UnsupportedDocType(doc_type));
    }

    if doc_type_read_version >= DEMUXER_DOC_TYPE_VERSION {
        return Err(DemuxError::UnsupportedDocTypeReadVersion(
            doc_type_read_version,
        ));
    }

    Ok(EbmlHeader {
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
pub(crate) fn expect_master<R: Read + Seek>(
    r: &mut R,
    expected_id: ElementId,
    from: Option<u64>,
) -> Result<(u64, u64)> {
    let (element_id, size) = parse_element(r, from)?;

    if element_id != expected_id {
        return Err(DemuxError::UnexpectedElement((expected_id, element_id)));
    }

    parse_location(r, size)
}

/// Tries to parse a given Element ID that returns a unsigned integer at the given location of the reader.
pub(crate) fn expect_unsigned<R: Read + Seek>(
    r: &mut R,
    expected_id: ElementId,
    from: Option<u64>,
) -> Result<u64> {
    let (element_id, size) = parse_element(r, from)?;

    if element_id != expected_id {
        return Err(DemuxError::UnexpectedElement((expected_id, element_id)));
    }

    parse_unsigned(r, size)
}

/// Tries to parse a given Element ID that returns a string at the given location of the reader.
pub(crate) fn expect_string<R: Read + Seek>(
    r: &mut R,
    expected_id: ElementId,
    from: Option<u64>,
) -> Result<String> {
    let (element_id, size) = parse_element(r, from)?;

    if element_id != expected_id {
        return Err(DemuxError::UnexpectedElement((expected_id, element_id)));
    }

    parse_string(r, size)
}

/// Parses the next element at the current location of the reader.
pub(crate) fn next_element<R: Read + Seek>(r: &mut R) -> Result<(ElementId, ElementData)> {
    let (element_id, size) = parse_element(r, None)?;

    let element_type = *ELEMENT_ID_TO_TYPE
        .get(&element_id)
        .unwrap_or(&ElementType::Unknown);

    // TODO Default — The default value of the element to use if the parent element is present but this element is not.

    let element_data = match element_type {
        ElementType::Unknown => ElementData::Unknown,
        ElementType::Master | ElementType::Binary => {
            let (offset, size) = parse_location(r, size)?;
            ElementData::Location { offset, size }
        }
        ElementType::Unsigned => {
            let value = parse_unsigned(r, size)?;
            ElementData::Unsigned(value)
        }
        ElementType::Signed => {
            let value = parse_signed(r, size)?;
            ElementData::Signed(value)
        }
        ElementType::Float => {
            let value = parse_float(r, size)?;
            ElementData::Float(value)
        }
        ElementType::Date => {
            let value = parse_date(r, size)?;
            ElementData::Date(value)
        }
        ElementType::String => {
            let value = parse_string(r, size)?;
            ElementData::String(value)
        }
    };

    Ok((element_id, element_data))
}

/// Parses the next element from the given location inside the reader. Returns the Element ID and the size of the data.
pub(crate) fn parse_element<R: Read + Seek>(
    r: &mut R,
    from: Option<u64>,
) -> Result<(ElementId, u64)> {
    if let Some(from) = from {
        let _ = r.seek(SeekFrom::Start(from))?;
    }

    let id = parse_element_id(r)?;
    let size = parse_data_size(r)?;

    let element_id = *ID_TO_ELEMENT_ID.get(&id).unwrap_or(&ElementId::Unknown);

    Ok((element_id, size))
}

/// Parses a variable length EBML Element ID.
fn parse_element_id<R: Read>(r: &mut R) -> Result<u32> {
    let mut bytes = [0u8];
    r.read_exact(&mut bytes)?;
    let element_id = match bytes[0] {
        byte if (byte & 0x80) == 0x80 => byte.into(),
        byte if (byte & 0xC0) == 0x40 => parse_id_value(r, byte, 1)?,
        byte if (byte & 0xE0) == 0x20 => parse_id_value(r, byte, 2)?,
        byte if (byte & 0xF0) == 0x10 => parse_id_value(r, byte, 3)?,
        _ => return Err(DemuxError::InvalidEbmlElementId),
    };
    Ok(element_id)
}

/// Parses a variable length EBML data size.
fn parse_data_size<R: Read>(r: &mut R) -> Result<u64> {
    let mut bytes = [0u8];
    r.read_exact(&mut bytes)?;
    let size = match bytes[0] {
        byte if (byte & 0x80) == 0x80 => (0x7F & byte).into(),
        byte if (byte & 0xC0) == 0x40 => parse_size_value(r, 0x3F & byte, 1)?,
        byte if (byte & 0xE0) == 0x20 => parse_size_value(r, 0x1F & byte, 2)?,
        byte if (byte & 0xF0) == 0x10 => parse_size_value(r, 0x0F & byte, 3)?,
        byte if (byte & 0xF8) == 0x08 => parse_size_value(r, 0x07 & byte, 4)?,
        byte if (byte & 0xFC) == 0x04 => parse_size_value(r, 0x03 & byte, 5)?,
        byte if (byte & 0xFE) == 0x02 => parse_size_value(r, 0x01 & byte, 6)?,
        byte if byte == 0x01 => parse_size_value(r, 0, 7)?,
        _ => return Err(DemuxError::InvalidEbmlDataSize),
    };
    Ok(size)
}

fn parse_id_value<R: Read>(r: &mut R, byte: u8, left: u8) -> Result<u32> {
    let shift: usize = (8 * (3 - left)).into();

    let mut bytes = [byte, 0, 0, 0];
    r.read_exact(&mut bytes[1..=left.into()])?;

    Ok(u32::from_be_bytes(bytes) >> shift)
}

fn parse_size_value<R: Read>(r: &mut R, byte: u8, left: u8) -> Result<u64> {
    let shift: usize = (8 * (7 - left)).into();

    let mut bytes = [byte, 0, 0, 0, 0, 0, 0, 0];
    r.read_exact(&mut bytes[1..=left.into()])?;

    Ok(u64::from_be_bytes(bytes) >> shift)
}

fn parse_location<R: Read + Seek>(r: &mut R, size: u64) -> Result<(u64, u64)> {
    let offset = r.stream_position()?;
    // We skip the data and set the reader to the next element.
    let _ = r.seek(SeekFrom::Start(offset + size))?;

    Ok((offset, size))
}

#[allow(clippy::as_conversions)]
fn parse_unsigned<R: Read>(r: &mut R, size: u64) -> Result<u64> {
    if size == 0 {
        return Ok(0);
    }
    if size > 8 {
        return Err(DemuxError::WrongIntegerSize(size));
    }
    let shift = (8 * (8 - size)) as i64;

    let mut bytes = [0u8; 8];
    r.read_exact(&mut bytes[0..size as usize])?;

    Ok(u64::from_be_bytes(bytes) >> shift)
}

#[allow(clippy::as_conversions)]
fn parse_signed<R: Read>(r: &mut R, size: u64) -> Result<i64> {
    if size == 0 {
        return Ok(0);
    }
    if size > 8 {
        return Err(DemuxError::WrongIntegerSize(size));
    }
    let shift = (8 * (8 - size)) as i64;

    let mut bytes = [0u8; 8];
    r.read_exact(&mut bytes[0..size as usize])?;

    Ok(i64::from_be_bytes(bytes) >> shift)
}

fn parse_float<R: Read>(r: &mut R, size: u64) -> Result<f64> {
    match size {
        0 => Ok(0.0),
        4 => {
            let mut bytes = [0u8; 4];
            r.read_exact(&mut bytes)?;
            Ok(f32::from_be_bytes(bytes).into())
        }
        8 => {
            let mut bytes = [0u8; 8];
            r.read_exact(&mut bytes)?;
            Ok(f64::from_be_bytes(bytes))
        }
        _ => Err(DemuxError::WrongFloatSize(size)),
    }
}

#[allow(clippy::as_conversions)]
fn parse_date<R: Read>(r: &mut R, size: u64) -> Result<i64> {
    if size == 0 {
        return Ok(0);
    }
    if size > 8 {
        return Err(DemuxError::WrongDateSize(size));
    }
    let shift = (8 * (8 - size)) as i64;

    let mut bytes = [0u8; 8];
    r.read_exact(&mut bytes[0..size as usize])?;

    Ok(i64::from_be_bytes(bytes) >> shift)
}

fn parse_string<R: Read>(r: &mut R, size: u64) -> Result<String> {
    if size == 0 {
        return Ok(String::from(""));
    }

    let size: usize = size.try_into()?;
    let mut bytes = vec![0u8; size];
    r.read_exact(&mut bytes[0..size])?;

    Ok(String::from_utf8(bytes)?)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]

    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_parse_master_element() -> Result<()> {
        let data: Vec<u8> = vec![0x1A, 0x45, 0xDF, 0xA3, 0xA2];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::Ebml);
        assert_eq!(
            element_data,
            ElementData::Location {
                offset: 5,
                size: 34,
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_unsigned() -> Result<()> {
        let data: Vec<u8> = vec![0x42, 0x86, 0x81, 0x01];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::EbmlVersion);
        assert_eq!(element_data, ElementData::Unsigned(1));

        Ok(())
    }

    #[test]
    fn test_parse_signed() -> Result<()> {
        let data: Vec<u8> = vec![0xFB, 0x82, 0xFF, 0xFB];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::ReferenceBlock);
        assert_eq!(element_data, ElementData::Signed(-5));

        Ok(())
    }

    #[test]
    fn test_parse_date() -> Result<()> {
        let data: Vec<u8> = vec![0x44, 0x61, 0x84, 0xFF, 0xB3, 0xB4, 0xC0];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::DateUtc);
        assert_eq!(element_data, ElementData::Date(-5_000_000));

        Ok(())
    }

    #[test]
    fn test_parse_float_32() -> Result<()> {
        let data: Vec<u8> = vec![0x44, 0x89, 0x84, 0x43, 0x1C, 0x20, 0x07];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::Duration);
        if let ElementData::Float(x) = element_data {
            assert!((x - 156.1251).abs() < 0.00001)
        } else {
            panic!("parse_element returned the wrong element type");
        }

        Ok(())
    }

    #[test]
    fn test_parse_float_64() -> Result<()> {
        let data: Vec<u8> = vec![
            0x44, 0x89, 0x88, 0x40, 0xA9, 0xE0, 0x43, 0x30, 0xBC, 0x60, 0x6E,
        ];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::Duration);
        if let ElementData::Float(x) = element_data {
            assert!((x - 3312.1312312).abs() < 0.00001)
        } else {
            panic!("parse_element returned the wrong element type");
        }

        Ok(())
    }

    #[test]
    fn test_parse_ascii_string() -> Result<()> {
        let data: Vec<u8> = vec![
            0x42, 0x82, 0x88, 0x6D, 0x61, 0x74, 0x72, 0x6F, 0x73, 0x6B, 0x61,
        ];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::DocType);
        assert_eq!(element_data, ElementData::String("matroska".to_owned()));

        Ok(())
    }

    #[test]
    fn test_parse_utf8_string() -> Result<()> {
        let data: Vec<u8> = vec![
            0x4D, 0x80, 0x95, 0xE3, 0x82, 0x82, 0xE3, 0x81, 0x90, 0xE3, 0x82, 0x82, 0xE3, 0x81,
            0x90, 0xE3, 0x81, 0x8A, 0xE3, 0x81, 0x8B, 0xE3, 0x82, 0x86,
        ];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::MuxingApp);
        assert_eq!(
            element_data,
            ElementData::String("もぐもぐおかゆ".to_owned())
        );

        Ok(())
    }

    #[test]
    fn test_parse_binary() -> Result<()> {
        let data: Vec<u8> = vec![
            0x63, 0xA2, 0x95, 0xE3, 0x82, 0x82, 0xE3, 0x81, 0x90, 0xE3, 0x82, 0x82, 0xE3, 0x81,
            0x90, 0xE3, 0x81, 0x8A, 0xE3, 0x81, 0x8B, 0xE3, 0x82, 0x86,
        ];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::CodecPrivate);
        assert_eq!(
            element_data,
            ElementData::Location {
                offset: 3,
                size: 21,
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_default_unsigned() -> Result<()> {
        let data: Vec<u8> = vec![0x42, 0x86, 0x80];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::EbmlVersion);
        assert_eq!(element_data, ElementData::Unsigned(0));

        Ok(())
    }

    #[test]
    fn test_parse_default_signed() -> Result<()> {
        let data: Vec<u8> = vec![0xFB, 0x80];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::ReferenceBlock);
        assert_eq!(element_data, ElementData::Signed(0));

        Ok(())
    }

    #[test]
    fn test_parse_default_date() -> Result<()> {
        let data: Vec<u8> = vec![0x44, 0x61, 0x80];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::DateUtc);
        assert_eq!(element_data, ElementData::Date(0));

        Ok(())
    }

    #[test]
    fn test_parse_default_float() -> Result<()> {
        let data: Vec<u8> = vec![0x44, 0x89, 0x80];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::Duration);
        if let ElementData::Float(x) = element_data {
            assert!((x).abs() < 0.00001)
        } else {
            panic!("parse_element returned the wrong element type");
        }

        Ok(())
    }

    #[test]
    fn test_parse_default_ascii_string() -> Result<()> {
        let data: Vec<u8> = vec![0x42, 0x82, 0x80];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::DocType);
        assert_eq!(element_data, ElementData::String("".to_owned()));

        Ok(())
    }

    #[test]
    fn test_parse_default_utf8_string() -> Result<()> {
        let data: Vec<u8> = vec![0x4D, 0x80, 0x80];
        let mut cursor = Cursor::new(data);
        let (element_id, element_data) = next_element(&mut cursor)?;
        assert_eq!(element_id, ElementId::MuxingApp);
        assert_eq!(element_data, ElementData::String("".to_owned()));

        Ok(())
    }

    #[test]
    fn test_parse_ebml_header() -> Result<()> {
        let data: Vec<u8> = vec![
            0x1A, 0x45, 0xDF, 0xA3, 0xA2, 0x42, 0x86, 0x81, 0x01, 0x42, 0xF7, 0x81, 0x01, 0x42,
            0xF2, 0x81, 0x04, 0x42, 0xF3, 0x81, 0x08, 0x42, 0x82, 0x88, 0x6D, 0x61, 0x74, 0x72,
            0x6F, 0x73, 0x6B, 0x61, 0x42, 0x87, 0x81, 0x04, 0x42, 0x85, 0x81, 0x02,
        ];
        let mut cursor = Cursor::new(data);
        let ebml_header = parse_ebml_header(&mut cursor)?;
        assert_eq!(ebml_header.version, 1);
        assert_eq!(ebml_header.read_version, 1);
        assert_eq!(ebml_header.max_id_length, 4);
        assert_eq!(ebml_header.max_size_length, 8);
        assert_eq!(&ebml_header.doc_type, "matroska");
        assert_eq!(ebml_header.doc_type_version, 4);
        assert_eq!(ebml_header.doc_type_read_version, 2);

        Ok(())
    }
}
