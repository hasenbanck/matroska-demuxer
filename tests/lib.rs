use std::fs::File;

use matroska_demux::MatroskaFile;

#[test]
pub fn parse_mkv() {
    let file = File::open("tests/data/simple.mkv").unwrap();
    let _mkv = MatroskaFile::open(file).unwrap();
}
