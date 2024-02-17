use std::{fs::File, io::Write};

use sprity_aseprite::{loader::AsepriteFile, output::AnimationSet};


#[test]
#[ignore = "only run to dump the content"]
fn test_dump_file() {
    let path = "tests/aseprite_files/animated.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::from_bytes(&file).unwrap();

    std::fs::create_dir_all("tests/generated_pngs").unwrap();
    let mut f = File::create("tests/generated_pngs/dump.txt").unwrap();
    let s = format!("{file:#?}");
    f.write_all(s.as_bytes()).unwrap();
}

#[test]
#[ignore = "packing is random, must verify visually"]
fn test_spritesheet_pack() {
    let path = "tests/aseprite_files/animated.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::from_bytes(&file).unwrap();

    let config = texture_packer::TexturePackerConfig {
        max_width: 512,
        max_height: 512,
        allow_rotation: false,
        texture_outlines: false,
        border_padding: 0,
        force_max_dimensions: false,
        texture_padding: 3,
        texture_extrusion: 0,
        trim: false, // should already be trimmed but just in case, don't want to mess up offsets
    };
    let mut packer = texture_packer::MultiTexturePacker::new_skyline(config);
    let anim_set = AnimationSet::from_ase(file, "test", &mut packer).unwrap();
    let mut f = File::create("tests/generated_pngs/dump.txt").unwrap();
    let s = format!("{anim_set:#?}");
    f.write_all(s.as_bytes()).unwrap();
    
    std::fs::create_dir_all("tests/generated_pngs").unwrap();
    for (i, f) in packer.get_pages().iter().enumerate() {
        let path = format!("tests/generated_pngs/packed_spritesheet{i}.png");
        let img = texture_packer::exporter::ImageExporter::export(f).map_err(|s| anyhow::anyhow!(s)).unwrap();
        img.save_with_format(path, image::ImageFormat::Png).unwrap();
    }
    // hashmap/packing is random, need to verify visually
}