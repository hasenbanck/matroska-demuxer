use std::fs::File;
use std::num::NonZeroU64;

use matroska_demux::{
    ContentEncodingType, MatrixCoefficients, MatroskaFile, Primaries, TrackEntry, TrackType,
    TransferCharacteristics,
};

#[test]
pub fn parse_simple_mkv() {
    let file = File::open("tests/data/simple.mkv").unwrap();
    let mkv = MatroskaFile::open(file).unwrap();

    let chapters = mkv.chapters().unwrap()[0].chapter_atoms();
    assert_eq!(chapters[0].uid().get(), 1067995727130785153);
    assert_eq!(chapters[0].time_start(), 0);
    assert_eq!(chapters[0].time_end(), None);
    assert_eq!(chapters[0].displays()[0].string(), "Intro");
    assert_eq!(chapters[0].displays()[0].language(), None);
    assert_eq!(chapters[0].displays()[0].language_ietf(), Some("en"));
    assert_eq!(chapters[0].displays()[0].country(), None);

    let tags = mkv.tags().unwrap();
    assert_eq!(tags[0].targets().unwrap().target_type_value().unwrap(), 50);
    assert_eq!(tags[0].simple_tags()[0].name(), "ENCODER");
    assert_eq!(tags[0].simple_tags()[0].string().unwrap(), "Lavf58.76.100");
}

#[test]
pub fn parse_hdr_mkv() {
    let file = File::open("tests/data/hdr.mkv").unwrap();
    let mkv = MatroskaFile::open(file).unwrap();

    let video_tracks: Vec<TrackEntry> = mkv
        .tracks()
        .iter()
        .filter(|t| t.track_type() == TrackType::Video)
        .cloned()
        .collect();

    let video = video_tracks[0].video().unwrap();

    assert_eq!(video.pixel_width().get(), 3840);
    assert_eq!(video.pixel_height().get(), 2160);

    let colour = video.colour().unwrap();

    assert_eq!(
        colour.transfer_characteristics().unwrap(),
        TransferCharacteristics::Bt2100
    );
    assert_eq!(
        colour.matrix_coefficients().unwrap(),
        MatrixCoefficients::Bt2020Ncl
    );
    assert_eq!(colour.primaries().unwrap(), Primaries::Bt2020);

    let metadata = colour.mastering_metadata().unwrap();

    assert!((1000.0 - metadata.luminance_max().unwrap()).abs() < f64::EPSILON);
    assert!((0.009999999776482582 - metadata.luminance_min().unwrap()).abs() < f64::EPSILON);
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

    assert_eq!(
        mkv.info().timestamp_scale(),
        NonZeroU64::new(1000000).unwrap()
    );
    assert!((87336.0 - mkv.info().duration().unwrap()).abs() < f64::EPSILON);
    assert_eq!(
        mkv.info().muxing_app(),
        "libebml2 v0.10.0 + libmatroska2 v0.10.1"
    );
    assert_eq!(mkv.info().date_utc().unwrap(), 304068183000000000);
    assert_eq!(mkv.info().writing_app(), "mkclean 0.5.5 ru from libebml v1.0.0 + libmatroska v1.0.0 + mkvmerge v4.1.1 ('Bouncin' Back') built on Jul  3 2010 22:54:08");

    assert_eq!(mkv.tracks().len(), 2);
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

    assert_eq!(
        mkv.info().timestamp_scale(),
        NonZeroU64::new(100000).unwrap()
    );
    assert!((475090.0 - mkv.info().duration().unwrap()).abs() < f64::EPSILON);
    assert_eq!(mkv.info().date_utc().unwrap(), 328711520000000000);

    assert_eq!(mkv.tracks().len(), 2);
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

    assert_eq!(
        mkv.info().timestamp_scale(),
        NonZeroU64::new(1000000).unwrap()
    );
    assert!((49064.0 - mkv.info().duration().unwrap()).abs() < f64::EPSILON);
    assert_eq!(mkv.info().date_utc().unwrap(), 304119805000000000);

    assert_eq!(mkv.tracks().len(), 2);

    assert_eq!(mkv.tracks()[0].content_encodings().unwrap().len(), 1);
    assert_eq!(mkv.tracks()[1].content_encodings().unwrap().len(), 1);

    assert_eq!(
        mkv.tracks()[0].content_encodings().unwrap()[0].encoding_type(),
        ContentEncodingType::Compression
    );
    assert_eq!(
        mkv.tracks()[1].content_encodings().unwrap()[0].encoding_type(),
        ContentEncodingType::Compression
    );
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

    assert_eq!(
        mkv.info().timestamp_scale(),
        NonZeroU64::new(1000000).unwrap()
    );
    assert_eq!(mkv.info().date_utc().unwrap(), 304072935000000000);

    assert_eq!(mkv.tracks().len(), 2);
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

    assert_eq!(
        mkv.info().timestamp_scale(),
        NonZeroU64::new(1000000).unwrap()
    );
    assert!((46665.0 - mkv.info().duration().unwrap()).abs() < f64::EPSILON);
    assert_eq!(mkv.info().date_utc().unwrap(), 304106803000000000);

    assert_eq!(mkv.tracks().len(), 11);
    assert_eq!(
        mkv.tracks()
            .iter()
            .filter(|t| t.track_type() == TrackType::Audio)
            .count(),
        2
    );
    assert_eq!(
        mkv.tracks()
            .iter()
            .filter(|t| t.track_type() == TrackType::Subtitle)
            .count(),
        8
    );

    let audio_tracks: Vec<TrackEntry> = mkv
        .tracks()
        .iter()
        .filter(|t| t.track_type() == TrackType::Audio)
        .cloned()
        .collect();

    assert!((48000.0 - audio_tracks[0].audio().unwrap().sampling_frequency()).abs() < f64::EPSILON);
    assert_eq!(audio_tracks[0].audio().unwrap().channels().get(), 2);

    assert!((22050.0 - audio_tracks[1].audio().unwrap().sampling_frequency()).abs() < f64::EPSILON);
    assert!(
        (44100.0
            - audio_tracks[1]
                .audio()
                .unwrap()
                .output_sampling_frequency()
                .unwrap())
        .abs()
            < f64::EPSILON
    );
}

#[test]
pub fn parse_test6_mkv() {
    let file = File::open("tests/data/test6.mkv").unwrap();
    let mkv = MatroskaFile::open(file).unwrap();

    assert_eq!(mkv.ebml_header().max_id_length(), 4);
    assert_eq!(mkv.ebml_header().max_size_length(), 8);

    assert_eq!(
        mkv.info().timestamp_scale(),
        NonZeroU64::new(1000000).unwrap()
    );
    assert!((87336.0 - mkv.info().duration().unwrap()).abs() < f64::EPSILON);
    assert_eq!(mkv.info().date_utc().unwrap(), 304101115000000000);

    assert_eq!(mkv.tracks().len(), 2);
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

    assert_eq!(
        mkv.info().timestamp_scale(),
        NonZeroU64::new(1000000).unwrap()
    );
    assert!((37043.0 - mkv.info().duration().unwrap()).abs() < f64::EPSILON);
    assert_eq!(mkv.info().date_utc().unwrap(), 304102823000000000);

    assert_eq!(mkv.tracks().len(), 2);
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

    assert_eq!(
        mkv.info().timestamp_scale(),
        NonZeroU64::new(1000000).unwrap()
    );
    assert!((47341.0 - mkv.info().duration().unwrap()).abs() < f64::EPSILON);
    assert_eq!(mkv.info().date_utc().unwrap(), 304104134000000000);

    assert_eq!(mkv.tracks().len(), 2);
}
