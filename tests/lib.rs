use std::{fs::File, num::NonZeroU64};

use matroska_demuxer::{
    ContentEncodingType, Frame, MatrixCoefficients, MatroskaFile, Primaries, TrackEntry, TrackType,
    TransferCharacteristics,
};

#[test]
pub fn parse_simple_mkv() {
    let file = File::open("tests/data/simple.mkv").unwrap();
    let mut mkv = MatroskaFile::open(file).unwrap();

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

    let mut frame = Frame::default();

    let mut count = 0;
    while mkv.next_frame(&mut frame).unwrap() {
        count += 1;
    }
    assert_eq!(count, 74);

    mkv.seek(0).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 0);

    mkv.seek(3).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 3);

    mkv.seek(1_000_000).unwrap();
    assert!(!mkv.next_frame(&mut frame).unwrap());
}

#[test]
pub fn parse_hdr_mkv() {
    let file = File::open("tests/data/hdr.mkv").unwrap();
    let mut mkv = MatroskaFile::open(file).unwrap();

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

    let mut frame = Frame::default();
    let mut count = 0;
    while mkv.next_frame(&mut frame).unwrap() {
        count += 1;
    }
    assert_eq!(count, 9);

    mkv.seek(0).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 0);

    mkv.seek(45).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 45);

    mkv.seek(1_000_000).unwrap();
    assert!(!mkv.next_frame(&mut frame).unwrap());
}

#[test]
pub fn parse_multi_seekhead_mkv() {
    let file = File::open("tests/data/multi_seekhead.mkv").unwrap();
    let mut mkv = MatroskaFile::open(file).unwrap();

    let info = mkv.info();
    assert_eq!(info.title(), Some("Big Buck Bunny"));

    let chapters = mkv.chapters().unwrap()[0].chapter_atoms();
    assert_eq!(chapters[0].uid().get(), 1067995727130785153);
    assert_eq!(chapters[0].time_start(), 0);
    assert_eq!(chapters[0].time_end(), None);
    assert_eq!(chapters[0].displays()[0].string(), "Intro");
    assert_eq!(chapters[0].displays()[0].language(), None);
    assert_eq!(chapters[0].displays()[0].language_ietf(), Some("en"));
    assert_eq!(chapters[0].displays()[0].country(), None);

    let tags = mkv.tags().unwrap();
    assert_eq!(tags[0].simple_tags()[0].name(), "ENCODER");
    assert_eq!(tags[0].simple_tags()[0].string().unwrap(), "Lavf58.76.100");

    let tracks = mkv.tracks();
    assert_eq!(tracks[0].name(), None);
    assert_eq!(tracks[1].name(), Some("Original"));

    let mut frame = Frame::default();

    let mut count = 0;
    while mkv.next_frame(&mut frame).unwrap() {
        count += 1;
    }
    assert_eq!(count, 74);

    mkv.seek(0).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 0);

    mkv.seek(3).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 3);

    mkv.seek(1_000_000).unwrap();
    assert!(!mkv.next_frame(&mut frame).unwrap());
}

#[test]
pub fn parse_subtitle_mkv() {
    let file = File::open("tests/data/subtitles.mkv").unwrap();
    let mut mkv = MatroskaFile::open(file).unwrap();

    let info = mkv.info();
    assert_eq!(info.title(), Some("Big Buck Bunny"));

    let tracks = mkv.tracks();
    assert_eq!(tracks[0].name(), None);
    assert_eq!(tracks[1].name(), None);
    assert_eq!(tracks[1].codec_id(), "S_TEXT/UTF8");
    assert_eq!(tracks[1].language(), None);
    assert!(tracks[1].flag_default());
    assert_eq!(tracks[2].name(), None);
    assert_eq!(tracks[2].codec_id(), "S_TEXT/UTF8");
    assert_eq!(tracks[2].language(), Some("fre"));
    assert!(!tracks[2].flag_default());

    let mut frame = Frame::default();

    let mut count = 0;
    let mut sub_idx = 0;
    while mkv.next_frame(&mut frame).unwrap() {
        if (frame.track == 2 || frame.track == 3) && sub_idx < 2 {
            sub_idx += 1;
            assert_eq!(frame.timestamp, 580);
            assert_eq!(frame.duration, Some(1820));
            assert_eq!(
                std::str::from_utf8(&frame.data).unwrap(),
                "<i>Big Buck Bunny</i>"
            );
        } else if frame.track == 2 && sub_idx == 2 {
            assert_eq!(frame.timestamp, 2540);
            assert_eq!(frame.duration, Some(2220));
            let frame_content = std::str::from_utf8(&frame.data).unwrap();
            assert_eq!(
                frame_content,
                "An animated comedy short film\r\nmade by the Blender Institute."
            );
        } else if frame.track == 3 && sub_idx == 2 {
            assert_eq!(frame.timestamp, 2540);
            assert_eq!(frame.duration, Some(2220));
            let frame_content = std::str::from_utf8(&frame.data).unwrap();
            assert_eq!(
                frame_content,
                "Un court métrage d'annimation\r\ncréer par le Blender Institute."
            );
        }
        count += 1;
    }
    assert_eq!(count, 154);

    mkv.seek(0).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 0);

    mkv.seek(3).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 40); // The second frame is at 40ms

    mkv.seek(1_000_000).unwrap();
    assert!(!mkv.next_frame(&mut frame).unwrap());
}

#[test]
pub fn parse_test1_mkv() {
    let file = File::open("tests/data/test1.mkv").unwrap();
    let mut mkv = MatroskaFile::open(file).unwrap();

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

    let mut frame = Frame::default();
    while mkv.next_frame(&mut frame).unwrap() {}

    mkv.seek(0).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 0);

    mkv.seek(180).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 192);

    mkv.seek(1_000_000).unwrap();
    assert!(!mkv.next_frame(&mut frame).unwrap());
}

#[test]
pub fn parse_test2_mkv() {
    let file = File::open("tests/data/test2.mkv").unwrap();
    let mut mkv = MatroskaFile::open(file).unwrap();

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

    let mut frame = Frame::default();
    while mkv.next_frame(&mut frame).unwrap() {}

    mkv.seek(0).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 0);

    // Timescale is "100000"
    mkv.seek(1800).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 3410);

    mkv.seek(1_000_000).unwrap();
    assert!(!mkv.next_frame(&mut frame).unwrap());
}

#[test]
pub fn parse_test3_mkv() {
    let file = File::open("tests/data/test3.mkv").unwrap();
    let mut mkv = MatroskaFile::open(file).unwrap();

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

    let mut frame = Frame::default();
    while mkv.next_frame(&mut frame).unwrap() {}

    mkv.seek(0).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 8);

    mkv.seek(450).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 500);

    mkv.seek(1_000_000).unwrap();
    assert!(!mkv.next_frame(&mut frame).unwrap());
}

#[test]
pub fn parse_test4_mkv() {
    let file = File::open("tests/data/test4.mkv").unwrap();
    let mut mkv = MatroskaFile::open(file).unwrap();

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

    let mut frame = Frame::default();
    while mkv.next_frame(&mut frame).unwrap() {}

    mkv.seek(0).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    // We are seeking in a file based of a live stream. So the first timestamp is "12345".
    assert_eq!(frame.timestamp, 12345);

    mkv.seek(50000).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 50011);

    mkv.seek(1_000_000).unwrap();
    assert!(!mkv.next_frame(&mut frame).unwrap());
}

#[test]
pub fn parse_test5_mkv() {
    let file = File::open("tests/data/test5.mkv").unwrap();
    let mut mkv = MatroskaFile::open(file).unwrap();

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

    let mut frame = Frame::default();
    while mkv.next_frame(&mut frame).unwrap() {}

    mkv.seek(0).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 0);

    mkv.seek(2000).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 2000);

    mkv.seek(1_000_000).unwrap();
    assert!(!mkv.next_frame(&mut frame).unwrap());
}

#[test]
pub fn parse_test6_mkv() {
    let file = File::open("tests/data/test6.mkv").unwrap();
    let mut mkv = MatroskaFile::open(file).unwrap();

    assert_eq!(mkv.ebml_header().max_id_length(), 4);
    assert_eq!(mkv.ebml_header().max_size_length(), 8);

    assert_eq!(
        mkv.info().timestamp_scale(),
        NonZeroU64::new(1000000).unwrap()
    );
    assert!((87336.0 - mkv.info().duration().unwrap()).abs() < f64::EPSILON);
    assert_eq!(mkv.info().date_utc().unwrap(), 304101115000000000);

    assert_eq!(mkv.tracks().len(), 2);

    let mut frame = Frame::default();
    while mkv.next_frame(&mut frame).unwrap() {}

    mkv.seek(0).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 0);

    mkv.seek(1000).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 1000);

    mkv.seek(1_000_000).unwrap();
    assert!(!mkv.next_frame(&mut frame).unwrap());
}

#[test]
pub fn parse_test7_mkv() {
    let file = File::open("tests/data/test7.mkv").unwrap();
    let mut mkv = MatroskaFile::open(file).unwrap();

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

    let mut frame = Frame::default();
    while mkv.next_frame(&mut frame).unwrap() {}

    mkv.seek(0).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 8);

    mkv.seek(2000).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 2000);

    mkv.seek(1_000_000).unwrap();
    assert!(!mkv.next_frame(&mut frame).unwrap());
}

#[test]
pub fn parse_test8_mkv() {
    let file = File::open("tests/data/test8.mkv").unwrap();
    let mut mkv = MatroskaFile::open(file).unwrap();

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

    let mut frame = Frame::default();
    while mkv.next_frame(&mut frame).unwrap() {}

    mkv.seek(0).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 3);

    mkv.seek(750).unwrap();
    assert!(mkv.next_frame(&mut frame).unwrap());
    assert_eq!(frame.timestamp, 750);

    mkv.seek(1_000_000).unwrap();
    assert!(!mkv.next_frame(&mut frame).unwrap());
}
