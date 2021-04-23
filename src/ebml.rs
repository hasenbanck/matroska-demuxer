//! Implement the parsing of EBML coded files.

use std::convert::{TryFrom, TryInto};
use std::io::{Read, Seek, SeekFrom};
use std::num::NonZeroU64;

use crate::element_id::{ElementId, ElementType, ELEMENT_ID_TO_TYPE, ID_TO_ELEMENT_ID};
use crate::{DemuxError, Result};

/// The data an element can contain.
#[derive(Clone, Debug, PartialEq)]
pub enum ElementData {
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

pub(crate) trait ParsableElement<R: Read + Seek> {
    type Output;

    fn new(r: &mut R, fields: &[(ElementId, ElementData)]) -> Result<Self::Output>;
}

/// Tries to parse an element with the given Element ID that returns a master element at the current location of the reader. Leaves the reader at the first byte after the master entry.
pub(crate) fn expect_master<R: Read + Seek>(
    r: &mut R,
    expected_id: ElementId,
    from: Option<u64>,
) -> Result<(u64, u64)> {
    let (element_id, size) = parse_element_header(r, from)?;

    if element_id != expected_id {
        return Err(DemuxError::UnexpectedElement((expected_id, element_id)));
    }

    let offset = r.stream_position()?;
    Ok((offset, size))
}

/// Collects the children of a master element.
pub(crate) fn collect_children<R: Read + Seek>(
    r: &mut R,
    offset: u64,
    size: u64,
) -> Result<Vec<(ElementId, ElementData)>> {
    let mut children = Vec::with_capacity(16);
    let _ = r.seek(SeekFrom::Start(offset))?;
    let end = offset + size;

    while r.stream_position()? < end {
        let (element_id, element_data) = next_element(r)?;

        if let ElementData::Location { offset, size } = element_data {
            if size == u64::MAX {
                break;
            }
            let _ = r.seek(SeekFrom::Start(offset + size))?;
        }

        if element_id != ElementId::Unknown {
            children.push((element_id, element_data))
        }
    }

    Ok(children)
}

/// Parses children of the same kind for the given master element as the given offset.
pub(crate) fn parse_children_for_master<R, T>(
    r: &mut R,
    offset: u64,
    master_id: ElementId,
    child_id: ElementId,
) -> Result<Vec<T::Output>>
where
    R: Read + Seek,
    T: ParsableElement<R>,
{
    let (data_offset, data_size) = expect_master(r, master_id, Some(offset))?;
    let children = parse_children::<_, T>(r, data_offset, data_size, child_id)?;
    Ok(children)
}

/// Parses children of the same kind.
pub(crate) fn parse_children<R, T>(
    r: &mut R,
    offset: u64,
    size: u64,
    child_id: ElementId,
) -> Result<Vec<T::Output>>
where
    R: Read + Seek,
    T: ParsableElement<R>,
{
    let mut children = vec![];
    let master_fields = collect_children(r, offset, size)?;
    for (_, element_data) in master_fields.iter().filter(|(id, _)| *id == child_id) {
        if let ElementData::Location { offset, size } = element_data {
            let child_fields = collect_children(r, *offset, *size)?;
            let track_entry = T::new(r, &child_fields)?;
            children.push(track_entry)
        }
    }
    Ok(children)
}

/// Tries to parse the child with the given Element ID from the given fields and reader.
pub(crate) fn try_parse_child<R, T>(
    r: &mut R,
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
) -> Result<Option<T::Output>>
where
    R: Read + Seek,
    T: ParsableElement<R>,
{
    let audio = if let Some((_, element_data)) = fields.iter().find(|(id, _)| *id == element_id) {
        if let ElementData::Location { offset, size } = element_data {
            let children = collect_children(r, *offset, *size)?;
            let child = T::new(r, &children)?;
            Some(child)
        } else {
            return Err(DemuxError::UnexpectedDataType);
        }
    } else {
        None
    };
    Ok(audio)
}

/// Expects to find element with the an Element ID for an unsigned integer inside a list of children.
pub(crate) fn find_unsigned(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
) -> Result<u64> {
    let value =
        try_find_unsigned(fields, element_id)?.ok_or(DemuxError::ElementNotFound(element_id))?;
    Ok(value)
}

/// Expects to find element with the an Element ID for an unsigned integer inside a list of children, otherwise sets the default value.
pub(crate) fn find_unsigned_or(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
    default: u64,
) -> Result<u64> {
    let value = try_find_unsigned(fields, element_id)?;
    let value = value.unwrap_or(default);
    Ok(value)
}

/// Tries to find an element with the Element ID for an unsigned integer inside a list of children.
pub(crate) fn try_find_unsigned(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
) -> Result<Option<u64>> {
    if let Some((_, data)) = fields.iter().find(|(id, _)| *id == element_id) {
        if let ElementData::Unsigned(value) = data {
            Ok(Some(*value))
        } else {
            Err(DemuxError::UnexpectedDataType)
        }
    } else {
        Ok(None)
    }
}

/// Expects to find element with the an Element ID for an unsigned integer inside a list of children and converts it into a custom type.
pub(crate) fn find_custom_type<T: From<u64>>(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
) -> Result<T> {
    let value =
        try_find_unsigned(fields, element_id)?.ok_or(DemuxError::ElementNotFound(element_id))?;
    Ok(value.into())
}

/// Tries to find an element with the Element ID for a boolean inside a list of children, otherwise sets the default value.
pub(crate) fn find_bool_or(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
    default: bool,
) -> Result<bool> {
    let value = try_find_unsigned(fields, element_id)?;
    match value {
        None => Ok(default),
        Some(value) => match value {
            0 => Ok(false),
            _ => Ok(true),
        },
    }
}

/// Expects to find an element with the Element ID for a non zero unsigned integer inside a list of children.
pub(crate) fn find_nonzero(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
) -> Result<NonZeroU64> {
    let value =
        try_find_unsigned(fields, element_id)?.ok_or(DemuxError::ElementNotFound(element_id))?;
    NonZeroU64::new(value).ok_or(DemuxError::NonZeroValueIsZero(element_id))
}

/// Tries to find an element with the Element ID for an non zero unsigned integer inside a list of children, otherwise sets the default value.
pub(crate) fn find_nonzero_or(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
    default: u64,
) -> Result<NonZeroU64> {
    let value = try_find_unsigned(fields, element_id)?;
    let value = value.unwrap_or(default);
    NonZeroU64::new(value).ok_or(DemuxError::NonZeroValueIsZero(element_id))
}

/// Tries to find an element with the Element ID for an non zero unsigned integer inside a list of children.
pub(crate) fn try_find_nonzero(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
) -> Result<Option<NonZeroU64>> {
    match try_find_unsigned(fields, element_id)? {
        None => Ok(None),
        Some(value) => Ok(Some(
            NonZeroU64::new(value).ok_or(DemuxError::NonZeroValueIsZero(element_id))?,
        )),
    }
}

/// Expects to find an element with the Element ID for a float inside a list of children, otherwise sets the default value.
pub(crate) fn find_float_or(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
    default: f64,
) -> Result<f64> {
    let value = try_find_float(fields, element_id)?;
    let value = value.unwrap_or(default);
    Ok(value)
}

/// Tries to find an element with the Element ID for a float inside a list of children.
pub(crate) fn try_find_float(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
) -> Result<Option<f64>> {
    if let Some((_, data)) = fields.iter().find(|(id, _)| *id == element_id) {
        if let ElementData::Float(value) = data {
            Ok(Some(*value))
        } else {
            Err(DemuxError::UnexpectedDataType)
        }
    } else {
        Ok(None)
    }
}

/// Expects to find an element with the Element ID for a string inside a list of children.
pub(crate) fn find_string(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
) -> Result<String> {
    let value =
        try_find_string(fields, element_id)?.ok_or(DemuxError::ElementNotFound(element_id))?;
    Ok(value)
}

/// Tries to find an element with the Element ID for a string inside a list of children.
pub(crate) fn try_find_string(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
) -> Result<Option<String>> {
    if let Some((_, data)) = fields.iter().find(|(id, _)| *id == element_id) {
        if let ElementData::String(value) = data {
            Ok(Some(value.clone()))
        } else {
            Err(DemuxError::UnexpectedDataType)
        }
    } else {
        Ok(None)
    }
}

/// Tries to find an element with the Element ID for binary inside a list of children.
pub(crate) fn try_find_binary<R: Read + Seek>(
    r: &mut R,
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
) -> Result<Option<Vec<u8>>> {
    if let Some((_, data)) = fields.iter().find(|(id, _)| *id == element_id) {
        if let ElementData::Location { offset: _, size } = data {
            let size = usize::try_from(*size)?;
            let mut data = vec![0_u8; size];
            let _ = r.read_exact(&mut data)?;
            Ok(Some(data))
        } else {
            Err(DemuxError::UnexpectedDataType)
        }
    } else {
        Ok(None)
    }
}

/// Tries to find an element with the Element ID for a date inside a list of children.
pub(crate) fn try_find_date(
    fields: &[(ElementId, ElementData)],
    element_id: ElementId,
) -> Result<Option<i64>> {
    if let Some((_, data)) = fields.iter().find(|(id, _)| *id == element_id) {
        if let ElementData::Date(value) = data {
            Ok(Some(*value))
        } else {
            Err(DemuxError::UnexpectedDataType)
        }
    } else {
        Ok(None)
    }
}

/// Parses the next Element at the current location of the reader and returns it's data.
pub(crate) fn next_element<R: Read + Seek>(r: &mut R) -> Result<(ElementId, ElementData)> {
    let (element_id, size) = parse_element_header(r, None)?;

    let element_type = *ELEMENT_ID_TO_TYPE
        .get(&element_id)
        .unwrap_or(&ElementType::Unknown);

    let element_data = match element_type {
        ElementType::Master | ElementType::Binary | ElementType::Unknown => {
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
pub(crate) fn parse_element_header<R: Read + Seek>(
    r: &mut R,
    from: Option<u64>,
) -> Result<(ElementId, u64)> {
    if let Some(from) = from {
        let _ = r.seek(SeekFrom::Start(from))?;
    }

    let id = parse_element_id(r)?;
    let element_id = *ID_TO_ELEMENT_ID.get(&id).unwrap_or(&ElementId::Unknown);

    let size = parse_data_size(r)?;

    Ok((element_id, size))
}

/// Parses a variable length EBML Element ID.
fn parse_element_id<R: Read>(r: &mut R) -> Result<u32> {
    loop {
        let mut bytes = [0u8];
        r.read_exact(&mut bytes)?;
        let element_id = match bytes[0] {
            // We keep reading bytes until we find a valid Element ID.
            byte if (byte & 0xF0) == 0x00 => continue,
            byte if (byte & 0x80) == 0x80 => byte.into(),
            byte if (byte & 0xC0) == 0x40 => parse_id_value(r, byte, 1)?,
            byte if (byte & 0xE0) == 0x20 => parse_id_value(r, byte, 2)?,
            byte if (byte & 0xF0) == 0x10 => parse_id_value(r, byte, 3)?,
            _ => return Err(DemuxError::InvalidEbmlElementId),
        };
        return Ok(element_id);
    }
}

/// Parses a variable length EBML data size.
fn parse_data_size<R: Read>(r: &mut R) -> Result<u64> {
    let mut bytes = [0u8];
    r.read_exact(&mut bytes)?;
    let size = match bytes[0] {
        byte if byte == 0xFF => u64::MAX,
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

    // We skip the data and set the reader to the next element, if the size is known.
    if size != u64::MAX {
        let _ = r.seek(SeekFrom::Start(offset + size))?;
    }

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
}
