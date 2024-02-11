use std::{fs::File, io::Write};

use sprity_aseprite::loader::AsepriteFile;


#[test]
#[ignore = "only run to dump the content"]
fn test_dump_indexed() {
    let path = "tests/aseprite_files/userdata.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::from_bytes(&file).unwrap();

    std::fs::create_dir_all("tests/generated_pngs").unwrap();
    let mut f = File::create("tests/generated_pngs/dump.txt").unwrap();
    let s = format!("{file:#?}");
    f.write_all(s.as_bytes()).unwrap();
}