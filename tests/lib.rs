use std::fs::File;

#[test]
pub fn parse_mkv() {
    let _file = File::open("tests/data/simple.mkv").unwrap();
}
