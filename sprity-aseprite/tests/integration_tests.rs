use sprity_aseprite::loader::AsepriteFile;

#[test]
fn test_cell() {
    let path = "tests/aseprite_files/combine.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::from_bytes(&file).unwrap();

    for (frame_i, frame) in file.frames.iter().enumerate() {
        for cel in frame.cells.iter() {
            let img = &file.images_decompressed[cel.image_index];

            std::fs::create_dir_all("tests/generated_pngs").unwrap();
            let path = format!("tests/generated_pngs/cell_f{frame_i}c{}.png", cel.layer_index());
            img.save_with_format(path, image::ImageFormat::Png).unwrap();

            let expected_path = format!("tests/expected_pngs/cell_f{frame_i}c{}.png", cel.layer_index());
            let expected = image::io::Reader::open(expected_path).unwrap().decode().unwrap();
            let expected_rgba = expected.as_rgba8().unwrap();
            assert_eq!(expected_rgba, img);
        }
    }
}

#[test]
fn test_combine() {
    let path = "tests/aseprite_files/combine.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::from_bytes(&file).unwrap();

    for (index, _) in file.frames.iter().enumerate() {
        let img = file.combined_frame_image(index).unwrap();

        std::fs::create_dir_all("tests/generated_pngs").unwrap();
        let path = format!("tests/generated_pngs/combined_{}.png", index);
        img.save_with_format(path, image::ImageFormat::Png).unwrap();
        
        let expected_path = format!("tests/expected_pngs/combined_{}.png", index);
        let expected = image::io::Reader::open(expected_path).unwrap().decode().unwrap();
        let expected_rgba = expected.as_rgba8().unwrap();
        assert_eq!(expected_rgba, &img);
    }
}

#[test]
fn test_combine_cropped() {
    let path = "tests/aseprite_files/combine.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::from_bytes(&file).unwrap();

    for (index, frame) in file.frames.iter().enumerate() {
        let img = frame.combined_frame_image_cropped(&file.layers, &file.images_decompressed).unwrap();

        std::fs::create_dir_all("tests/generated_pngs").unwrap();
        let path = format!("tests/generated_pngs/combined_cropped_{}.png", index);
        img.img.save_with_format(path, image::ImageFormat::Png).unwrap();
        
        let expected_path = format!("tests/expected_pngs/combined_cropped_{}.png", index);
        let expected = image::io::Reader::open(expected_path).unwrap().decode().unwrap();
        let expected_rgba = expected.as_rgba8().unwrap();
        assert!(expected_rgba == &img.img);
    }
}

#[test]
fn test_spritesheet_pack() {
    let path = "tests/aseprite_files/combine.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::from_bytes(&file).unwrap();
    let img = file.packed_spritesheet2().unwrap();
    
    std::fs::create_dir_all("tests/generated_pngs").unwrap();
    let path = "tests/generated_pngs/packed_spritesheet.png";
    img.save_with_format(path, image::ImageFormat::Png).unwrap();
        
    // hashmap/packing is random, need to verify visually
    //let expected_path = "tests/expected_pngs/packed_spritesheet.png";
    //let expected = image::io::Reader::open(expected_path).unwrap().decode().unwrap();
    //let expected_rgba = expected.as_rgba8().unwrap();
    //assert!(expected_rgba == &img);
}

#[test]
fn test_linkedcells() {
    let path = "tests/aseprite_files/linkedcells.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::from_bytes(&file).unwrap();

    for (index, _) in file.frames.iter().enumerate() {
        let img = file.combined_frame_image(index).unwrap();

        std::fs::create_dir_all("tests/generated_pngs").unwrap();
        let path = format!("tests/generated_pngs/linkedcells_{}.png", index);
        img.save_with_format(path, image::ImageFormat::Png).unwrap();

        let expected_path = format!("tests/expected_pngs/linkedcells_{}.png", index);
        let expected = image::io::Reader::open(expected_path).unwrap().decode().unwrap();
        let expected_rgba = expected.as_rgba8().unwrap();
        assert_eq!(expected_rgba, &img);
    }
}

#[test]
fn test_userdata() {
    let path = "tests/aseprite_files/userdata.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::from_bytes(&file).unwrap();

    assert!(file.layers[0].user_data.text.unwrap() == "l1");
    assert!(file.layers[1].user_data.text.unwrap() == "l2");
    assert!(file.layers[2].user_data.text.unwrap() == "l3");

    assert!(file.frames[1].cell_at_layer_index(0).unwrap().user_data.text.unwrap() == "l1f2");
    assert!(file.frames[1].cell_at_layer_index(2).unwrap().user_data.text.unwrap() == "l3f2");
    assert!(file.frames[0].cell_at_layer_index(1).unwrap().user_data.text.unwrap() == "l2f1");
    assert!(file.frames[2].cell_at_layer_index(0).unwrap().user_data.text.unwrap() == "l1f3");
    assert!(file.frames[2].cell_at_layer_index(2).unwrap().user_data.text.unwrap() == "l3f3");

    assert!(file.tags[0].name() == "Tag 13");
    assert!(file.tags[0].user_data.text.unwrap() == "t13");
    assert!(file.tags[1].name() == "Tag 12");
    assert!(file.tags[1].user_data.text.unwrap() == "t12");
    assert!(file.tags[2].name() == "Tag 23");
    assert!(file.tags[2].user_data.text.unwrap() == "t23");
}
