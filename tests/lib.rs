use std::fs::File;

use matroska_demux::MatroskaFile;

#[test]
pub fn parse_simple_mkv() {
    let file = File::open("tests/data/simple.mkv").unwrap();
    let _mkv = MatroskaFile::open(file).unwrap();
}

#[test]
pub fn parse_test1_mkv() {
    let file = File::open("tests/data/test1.mkv").unwrap();
    let mkv = MatroskaFile::open(file).unwrap();

    assert_eq!(mkv.ebml_header().version(), None);
    assert_eq!(mkv.ebml_header().read_version(), None);
    assert_eq!(mkv.ebml_header().doc_type(), "matroska");
    assert_eq!(mkv.ebml_header().doc_type_version(), 2);
    assert_eq!(mkv.ebml_header().doc_type_read_version(), 2);
    assert_eq!(mkv.ebml_header().max_id_length(), 4);
    assert_eq!(mkv.ebml_header().max_size_length(), 8);
}

#[test]
pub fn parse_test2_mkv() {
    let file = File::open("tests/data/test2.mkv").unwrap();
    let mkv = MatroskaFile::open(file).unwrap();

    assert_eq!(mkv.ebml_header().version(), None);
    assert_eq!(mkv.ebml_header().read_version(), None);
    assert_eq!(mkv.ebml_header().doc_type(), "matroska");
    assert_eq!(mkv.ebml_header().doc_type_version(), 2);
    assert_eq!(mkv.ebml_header().doc_type_read_version(), 2);
    assert_eq!(mkv.ebml_header().max_id_length(), 4);
    assert_eq!(mkv.ebml_header().max_size_length(), 8);
}

#[test]
pub fn parse_test3_mkv() {
    let file = File::open("tests/data/test3.mkv").unwrap();
    let mkv = MatroskaFile::open(file).unwrap();

    assert_eq!(mkv.ebml_header().version(), None);
    assert_eq!(mkv.ebml_header().read_version(), None);
    assert_eq!(mkv.ebml_header().doc_type(), "matroska");
    assert_eq!(mkv.ebml_header().doc_type_version(), 2);
    assert_eq!(mkv.ebml_header().doc_type_read_version(), 2);
    assert_eq!(mkv.ebml_header().max_id_length(), 4);
    assert_eq!(mkv.ebml_header().max_size_length(), 8);
}

#[test]
pub fn parse_test4_mkv() {
    let file = File::open("tests/data/test4.mkv").unwrap();
    let mkv = MatroskaFile::open(file).unwrap();

    assert_eq!(mkv.ebml_header().version(), None);
    assert_eq!(mkv.ebml_header().read_version(), None);
    assert_eq!(mkv.ebml_header().doc_type(), "matroska");
    assert_eq!(mkv.ebml_header().doc_type_version(), 1);
    assert_eq!(mkv.ebml_header().doc_type_read_version(), 1);
    assert_eq!(mkv.ebml_header().max_id_length(), 4);
    assert_eq!(mkv.ebml_header().max_size_length(), 8);
}

#[test]
pub fn parse_test5_mkv() {
    let file = File::open("tests/data/test5.mkv").unwrap();
    let mkv = MatroskaFile::open(file).unwrap();

    assert_eq!(mkv.ebml_header().version(), Some(1));
    assert_eq!(mkv.ebml_header().read_version(), Some(1));
    assert_eq!(mkv.ebml_header().doc_type(), "matroska");
    assert_eq!(mkv.ebml_header().doc_type_version(), 2);
    assert_eq!(mkv.ebml_header().doc_type_read_version(), 2);
    assert_eq!(mkv.ebml_header().max_id_length(), 4);
    assert_eq!(mkv.ebml_header().max_size_length(), 8);
}

#[test]
pub fn parse_test6_mkv() {
    let file = File::open("tests/data/test6.mkv").unwrap();
    let mkv = MatroskaFile::open(file).unwrap();

    assert_eq!(mkv.ebml_header().max_id_length(), 4);
    assert_eq!(mkv.ebml_header().max_size_length(), 8);
}

#[test]
pub fn parse_test7_mkv() {
    let file = File::open("tests/data/test7.mkv").unwrap();
    let mkv = MatroskaFile::open(file).unwrap();

    assert_eq!(mkv.ebml_header().version(), None);
    assert_eq!(mkv.ebml_header().read_version(), None);
    assert_eq!(mkv.ebml_header().doc_type(), "matroska");
    assert_eq!(mkv.ebml_header().doc_type_version(), 2);
    assert_eq!(mkv.ebml_header().doc_type_read_version(), 2);
    assert_eq!(mkv.ebml_header().max_id_length(), 4);
    assert_eq!(mkv.ebml_header().max_size_length(), 8);
}

#[test]
pub fn parse_test8_mkv() {
    let file = File::open("tests/data/test8.mkv").unwrap();
    let mkv = MatroskaFile::open(file).unwrap();

    assert_eq!(mkv.ebml_header().version(), None);
    assert_eq!(mkv.ebml_header().read_version(), None);
    assert_eq!(mkv.ebml_header().doc_type(), "matroska");
    assert_eq!(mkv.ebml_header().doc_type_version(), 2);
    assert_eq!(mkv.ebml_header().doc_type_read_version(), 2);
    assert_eq!(mkv.ebml_header().max_id_length(), 4);
    assert_eq!(mkv.ebml_header().max_size_length(), 8);
}
