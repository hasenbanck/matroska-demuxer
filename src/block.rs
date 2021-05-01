//! Parses blocks inside a Matroska file.
use std::collections::VecDeque;
use std::convert::{TryFrom, TryInto};
use std::io::{Read, Seek};
use std::ops::Add;

use crate::ebml::{parse_variable_i64, parse_variable_u64};
use crate::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Lacing {
    None,
    Xiph,
    Ebml,
    FixedSize,
}

impl From<u8> for Lacing {
    fn from(d: u8) -> Self {
        match d {
            1 => Lacing::Xiph,
            2 => Lacing::FixedSize,
            3 => Lacing::Ebml,
            _ => Lacing::None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LacedFrame {
    pub(crate) track: u64,
    pub(crate) timestamp: u64,
    pub(crate) size: u64,
    pub(crate) is_invisible: bool,
    pub(crate) is_keyframe: Option<bool>,
    pub(crate) is_discardable: Option<bool>,
}

pub(crate) fn probe_block_timestamp<R: Read + Seek>(
    r: &mut R,
    cluster_timestamp: u64,
) -> Result<u64> {
    parse_variable_u64(r)?;
    let timestamp = parse_timestamp(r, cluster_timestamp)?;

    Ok(timestamp)
}

pub(crate) fn parse_laced_frames<R: Read + Seek>(
    r: &mut R,
    frames: &mut VecDeque<LacedFrame>,
    block_size: u64,
    cluster_timestamp: u64,
    header_start: u64,
    is_simple_block: bool,
) -> Result<()> {
    let track = parse_variable_u64(r)?;
    let timestamp = parse_timestamp(r, cluster_timestamp)?;

    let mut header_byte = [0_u8];
    r.read_exact(&mut header_byte)?;

    let is_keyframe = if is_simple_block {
        let is_keyframe: bool = ((header_byte[0] & 0x80) >> 7) == 1;
        Some(is_keyframe)
    } else {
        None
    };
    let is_invisible: bool = ((header_byte[0] & 0x08) >> 3) == 1;
    let lacing: Lacing = ((header_byte[0] & 0x06) >> 1).into();
    let is_discardable = if is_simple_block {
        let is_discardable: bool = (header_byte[0] & 0x01) == 1;
        Some(is_discardable)
    } else {
        None
    };

    if lacing == Lacing::None {
        let header_end = r.stream_position()?;
        let header_size = header_end - header_start;
        let data_size = block_size - header_size;

        let frame = LacedFrame {
            track,
            timestamp,
            size: data_size,
            is_invisible,
            is_keyframe,
            is_discardable,
        };

        frames.push_back(frame);
    } else {
        let frame_count = parse_u8_as_u64(r)?.saturating_add(1);

        match lacing {
            /*
                Xiph lacing
                 * Block head (with lacing bits set to 01)
                 * Lacing head: Number of frames in the lace -1 – i.e. 2
                   (the 800 and 500 octets one)
                 * Lacing sizes: only the 2 first ones will be coded, 800 gives 255;255;255;35,
                   500 gives 255;245. The size of the last frame is deduced from the total size
                   of the block.

                A frame with a size multiple of 255 is coded with a 0 at the end of the size
                - for example, 765 is coded 255;255;255;0.
            */
            Lacing::Xiph => {
                let mut encoded_sizes = 0;
                for _ in 0..frame_count - 1 {
                    let size = parse_xiph_frame_size(r)?;
                    encoded_sizes += size;

                    frames.push_back(LacedFrame {
                        track,
                        timestamp,
                        size,
                        is_invisible,
                        is_keyframe,
                        is_discardable,
                    });
                }
                let header_end = r.stream_position()?;
                let header_size = header_end - header_start;
                let data_size = block_size - header_size;
                let size = data_size - encoded_sizes;

                frames.push_back(LacedFrame {
                    track,
                    timestamp,
                    size,
                    is_invisible,
                    is_keyframe,
                    is_discardable,
                });
            }
            /*
                EBML lacing
                 * Block head (with lacing bits set to 11)
                 * Lacing head: Number of frames in the lace -1 – i.e. 2 (the 800 and 500 octets one)
                 * Lacing sizes: only the 2 first ones will be coded, 800 gives 0x320 0x4000 = 0x4320,
                   500 is coded as -300 : - 0x12C + 0x1FFF + 0x4000 = 0x5ED3. The size of the last
                   frame is deduced from the total size of the block.

                In this case, the size is not coded as blocks of 255 bytes, but as a difference with
                the previous size and this size is coded as in EBML. The first size in the lace is
                unsigned as in EBML. The others use a range shifting to get a sign on each value.
            */
            Lacing::Ebml => {
                let mut size = parse_variable_u64(r)?;
                let mut encoded_size = size;

                frames.push_back(LacedFrame {
                    track,
                    timestamp,
                    size,
                    is_invisible,
                    is_keyframe,
                    is_discardable,
                });

                if frame_count > 2 {
                    for _ in 0..frame_count - 2 {
                        let next_offset = parse_variable_i64(r)?;
                        let abs = u64::try_from(next_offset.abs())?;

                        size = if next_offset.is_positive() {
                            size.saturating_add(abs)
                        } else {
                            size.saturating_sub(abs)
                        };
                        encoded_size += size;

                        frames.push_back(LacedFrame {
                            track,
                            timestamp,
                            size,
                            is_invisible,
                            is_keyframe,
                            is_discardable,
                        });
                    }
                }

                let header_end = r.stream_position()?;
                let header_size = header_end - header_start;
                let data_size = block_size - header_size;
                let size = data_size - encoded_size;

                frames.push_back(LacedFrame {
                    track,
                    timestamp,
                    size,
                    is_invisible,
                    is_keyframe,
                    is_discardable,
                });
            }
            /*
                Fixed-size lacing
                 * Block head (with lacing bits set to 10)
                 * Lacing head: Number of frames in the lace -1 – i.e. 2

                In this case, only the number of frames in the lace is saved, the size
                of each frame is deduced from the total size of the Block.
                For example, for 3 frames of 800 octets each.
            */
            Lacing::FixedSize => {
                let header_end = r.stream_position()?;
                let header_size = header_end - header_start;
                let data_size = block_size - header_size;
                let size = data_size / frame_count;

                for _ in 0..frame_count {
                    frames.push_back(LacedFrame {
                        track,
                        timestamp,
                        size,
                        is_invisible,
                        is_keyframe,
                        is_discardable,
                    });
                }
            }
            Lacing::None => { /* Unreachable */ }
        }
    }

    Ok(())
}

fn parse_timestamp<R: Read + Seek>(r: &mut R, cluster_timestamp: u64) -> Result<u64> {
    let timestamp = parse_i16(r)?;

    let abs: u64 = timestamp.abs().try_into()?;
    let timestamp = if timestamp.is_positive() {
        cluster_timestamp.add(abs)
    } else {
        cluster_timestamp.saturating_sub(abs)
    };

    Ok(timestamp)
}

fn parse_xiph_frame_size<R: Read + Seek>(r: &mut R) -> Result<u64> {
    let mut size: u64 = 0;
    loop {
        let val = parse_u8_as_u64(r)?;
        size += val;

        match val {
            255 => continue,
            _ => break,
        }
    }

    Ok(size)
}

fn parse_u8_as_u64<R: Read + Seek>(r: &mut R) -> Result<u64> {
    let mut buffer = [0_u8];
    r.read_exact(&mut buffer)?;
    let frame_count = u64::from(u8::from_be_bytes(buffer));
    Ok(frame_count)
}

fn parse_i16<R: Read + Seek>(r: &mut R) -> Result<i16> {
    let mut bytes = [0u8; 2];
    r.read_exact(&mut bytes)?;
    Ok(i16::from_be_bytes(bytes))
}
