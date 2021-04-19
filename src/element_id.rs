//! Element IDs defines by the EBML and Matroska specifications.

// EBML Basics

pub(crate) const EBML: u32 = 0x1A45DFA3;
pub(crate) const EBML_VERSION: u32 = 0x4286;
pub(crate) const EBML_READ_VERSION: u32 = 0x42F7;
pub(crate) const EBML_MAX_ID_LENGTH: u32 = 0x42F2;
pub(crate) const EBML_MAX_SIZE_LENGTH: u32 = 0x42F3;
pub(crate) const DOC_TYPE: u32 = 0x4282;
pub(crate) const DOC_TYPE_VERSION: u32 = 0x4287;
pub(crate) const DOC_TYPE_READ_VERSION: u32 = 0x4285;

// Global Elements

pub(crate) const VOID: u32 = 0xEC;

// Segment

pub(crate) const SEGMENT: u32 = 0x18538067;

// Meta Seek Information

pub(crate) const SEEK_HEAD: u32 = 0x114D9B74;
pub(crate) const SEEK: u32 = 0x4DBB;
pub(crate) const SEEK_ID: u32 = 0x53AB;
pub(crate) const SEEK_POSITION: u32 = 0x53AC;

// Segment Information

pub(crate) const INFO: u32 = 0x1549A966;
pub(crate) const TIMESTAMP_SCALE: u32 = 0x2AD7B1;
pub(crate) const DURATION: u32 = 0x4489;
pub(crate) const DATE_UTC: u32 = 0x4461;
pub(crate) const TITLE: u32 = 0x7BA9;
pub(crate) const MUXING_APP: u32 = 0x4D80;
pub(crate) const WRITING_APP: u32 = 0x5741;

// Cluster

pub(crate) const CLUSTER: u32 = 0x1F43B675;
pub(crate) const TIMESTAMP: u32 = 0xE7;
pub(crate) const PREV_SIZE: u32 = 0xA7;
pub(crate) const SIMPLE_BLOCK: u32 = 0xA3;
pub(crate) const BLOCK_GROUP: u32 = 0xA0;
pub(crate) const BLOCK: u32 = 0xA1;
pub(crate) const BLOCK_ADDITIONS: u32 = 0x75A1;
pub(crate) const BLOCK_MORE: u32 = 0xA6;
pub(crate) const BLOCK_ADD_ID: u32 = 0xEE;
pub(crate) const BLOCK_ADDITIONAL: u32 = 0xA5;
pub(crate) const BLOCK_DURATION: u32 = 0x9B;
pub(crate) const REFERENCE_BLOCK: u32 = 0xFB;
pub(crate) const DISCARD_PADDING: u32 = 0x75A2;

// Track

pub(crate) const TRACKS: u32 = 0x1654AE6B;
pub(crate) const TRACK_ENTRY: u32 = 0xAE;
pub(crate) const TRACK_NUMBER: u32 = 0xD7;
pub(crate) const TRACK_UID: u32 = 0x73C5;
pub(crate) const TRACK_TYPE: u32 = 0x83;
pub(crate) const FLAG_ENABLED: u32 = 0xB9;
pub(crate) const FLAG_DEFAULT: u32 = 0x88;
pub(crate) const FLAG_FORCED: u32 = 0x55AA;
pub(crate) const FLAG_HEARING_IMPAIRED: u32 = 0x55AB;
pub(crate) const FLAG_VISUAL_IMPAIRED: u32 = 0x55AC;
pub(crate) const FLAG_TEXT_DESCRIPTIONS: u32 = 0x55AD;
pub(crate) const FLAG_ORIGINAL: u32 = 0x55AE;
pub(crate) const FLAG_COMMENTARY: u32 = 0x55AF;
pub(crate) const FLAG_LACING: u32 = 0x9C;
pub(crate) const DEFAULT_DURATION: u32 = 0x23E383;
pub(crate) const NAME: u32 = 0xA7;
pub(crate) const LANGUAGE: u32 = 0x22B59C;
pub(crate) const CODEC_ID: u32 = 0x86;
pub(crate) const CODEC_PRIVATE: u32 = 0x63A2;
pub(crate) const CODEC_NAME: u32 = 0x258688;
pub(crate) const CODEC_DELAY: u32 = 0x56AA;
pub(crate) const SEEK_PRE_ROLL: u32 = 0x56BB;

// Track - Video

pub(crate) const VIDEO: u32 = 0xE0;
pub(crate) const FLAG_INTERLACED: u32 = 0x9A;
pub(crate) const STEREO_MODE: u32 = 0x53B8;
pub(crate) const ALPHA_MODE: u32 = 0x53C0;
pub(crate) const PIXEL_WIDTH: u32 = 0xB0;
pub(crate) const PIXEL_HEIGHT: u32 = 0xBA;
pub(crate) const PIXEL_CROP_BOTTOM: u32 = 0x54AA;
pub(crate) const PIXEL_CROP_TOP: u32 = 0x54BB;
pub(crate) const PIXEL_CROP_LEFT: u32 = 0x54CC;
pub(crate) const PIXEL_CROP_RIGHT: u32 = 0x54DD;
pub(crate) const DISPLAY_WIDTH: u32 = 0x54B0;
pub(crate) const DISPLAY_HEIGHT: u32 = 0x54BA;
pub(crate) const DISPLAY_UNIT: u32 = 0x54B2;
pub(crate) const ASPECT_RATIO_TYPE: u32 = 0x54B3;

// Track - Audio

pub(crate) const AUDIO: u32 = 0xE1;
pub(crate) const SAMPLING_FREQUENCY: u32 = 0xB5;
pub(crate) const OUTPUT_SAMPLING_FREQUENCY: u32 = 0x78B5;
pub(crate) const CHANNELS: u32 = 0x9F;
pub(crate) const BIT_DEPTH: u32 = 0x6264;

// Track - Content

pub(crate) const CONTENT_ENCODINGS: u32 = 0x6D80;
pub(crate) const CONTENT_ENCODING: u32 = 0x6240;
pub(crate) const CONTENT_ENCODING_ORDER: u32 = 0x5031;
pub(crate) const CONTENT_ENCODING_SCOPE: u32 = 0x5032;
pub(crate) const CONTENT_ENCODING_TYPE: u32 = 0x5033;
pub(crate) const CONTENT_ENCRYPTION: u32 = 0x5035;
pub(crate) const CONTENT_ENC_ALGO: u32 = 0x47E1;
pub(crate) const CONTENT_ENC_KEY_ID: u32 = 0x47E2;
pub(crate) const CONTENT_ENC_AESSETTINGS: u32 = 0x47E7;
pub(crate) const AESSETTINGS_CIPHER_MODE: u32 = 0x47E8;

// Colour

pub(crate) const COLOUR: u32 = 0x55B0;
pub(crate) const MATRIX_COEFFICIENTS: u32 = 0x55B1;
pub(crate) const BITS_PER_CHANNEL: u32 = 0x55B2;
pub(crate) const CHROMA_SUBSAMPLING_HORZ: u32 = 0x55B3;
pub(crate) const CHROMA_SUBSAMPLING_VERT: u32 = 0x55B4;
pub(crate) const CB_SUBSAMPLING_HORZ: u32 = 0x55B5;
pub(crate) const CB_SUBSAMPLING_VERT: u32 = 0x55B6;
pub(crate) const CHROMA_SITING_HORZ: u32 = 0x55B7;
pub(crate) const CHROMA_SITING_VERT: u32 = 0x55B8;
pub(crate) const RANGE: u32 = 0x55B9;
pub(crate) const TRANSFER_CHARACTERISTICS: u32 = 0x55BA;
pub(crate) const PRIMARIES: u32 = 0x55BB;
pub(crate) const MAX_CLL: u32 = 0x55BC;
pub(crate) const MAX_FALL: u32 = 0x55BD;
pub(crate) const MASTERING_METADATA: u32 = 0x55D0;
pub(crate) const PRIMARY_RCHROMATICITY_X: u32 = 0x55D1;
pub(crate) const PRIMARY_RCHROMATICITY_Y: u32 = 0x55D2;
pub(crate) const PRIMARY_GCHROMATICITY_X: u32 = 0x55D3;
pub(crate) const PRIMARY_GCHROMATICITY_Y: u32 = 0x55D4;
pub(crate) const PRIMARY_BCHROMATICITY_X: u32 = 0x55D5;
pub(crate) const PRIMARY_BCHROMATICITY_Y: u32 = 0x55D6;
pub(crate) const WHITE_POINT_CHROMATICITY_X: u32 = 0x55D7;
pub(crate) const WHITE_POINT_CHROMATICITY_Y: u32 = 0x55D8;
pub(crate) const LUMINANCE_MAX: u32 = 0x55D9;
pub(crate) const LUMINANCE_MIN: u32 = 0x55DA;

// Cueing Data

pub(crate) const CUES: u32 = 0x1C53BB6B;
pub(crate) const CUE_POINT: u32 = 0xBB;
pub(crate) const CUE_TIME: u32 = 0xB3;
pub(crate) const CUE_TRACK_POSITIONS: u32 = 0xB7;
pub(crate) const CUE_TRACK: u32 = 0xF7;
pub(crate) const CUE_CLUSTER_POSITION: u32 = 0xF1;
pub(crate) const CUE_RELATIVE_POSITION: u32 = 0xF0;
pub(crate) const CUE_DURATION: u32 = 0xB2;
pub(crate) const CUE_BLOCK_NUMBER: u32 = 0x5378;

// Chapters

pub(crate) const CHAPTERS: u32 = 0x1043A770;
pub(crate) const EDITION_ENTRY: u32 = 0x45B9;
pub(crate) const CHAPTER_ATOM: u32 = 0xB6;
pub(crate) const CHAPTER_UID: u32 = 0x73C4;
pub(crate) const CHAPTER_STRING_UID: u32 = 0x5654;
pub(crate) const CHAPTER_TIME_START: u32 = 0x91;
pub(crate) const CHAPTER_TIME_END: u32 = 0x92;
pub(crate) const CHAPTER_DISPLAY: u32 = 0x80;
pub(crate) const CHAP_STRING: u32 = 0x85;
pub(crate) const CHAP_LANGUAGE: u32 = 0x437C;
pub(crate) const CHAP_COUNTRY: u32 = 0x437E;

// Tagging

pub(crate) const TAGS: u32 = 0x1254C367;
pub(crate) const TAG: u32 = 0x7373;
pub(crate) const TARGETS: u32 = 0x63C0;
pub(crate) const TARGET_TYPE_VALUE: u32 = 0x68CA;
pub(crate) const TARGET_TYPE: u32 = 0x63CA;
pub(crate) const TAG_TRACK_UID: u32 = 0x63C5;
pub(crate) const SIMPLE_TAG: u32 = 0x67C8;
pub(crate) const TAG_NAME: u32 = 0x45A3;
pub(crate) const TAG_LANGUAGE: u32 = 0x447A;
pub(crate) const TAG_DEFAULT: u32 = 0x4484;
pub(crate) const TAG_STRING: u32 = 0x4487;
pub(crate) const TAG_BINARY: u32 = 0x4485;
