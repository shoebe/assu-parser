use sprity_aseprite::loader::AsepriteFile;

#[test]
fn test_cell() {
    let path = "tests/aseprite_files/combine.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::load(&file).unwrap();

    for (frame_i, frame) in file.frames.iter().enumerate() {
        for cel in frame.cells.iter() {
            let img = file.load_image(cel.image_index).unwrap();

            std::fs::create_dir_all("tests/generated_pngs").unwrap();
            let path = format!("tests/generated_pngs/cell_f{frame_i}c{}.png", cel.layer_index());
            lodepng::encode32_file(path, &img.pixels, img.width, img.height).unwrap();

            let expected_path = format!("tests/expected_pngs/cell_f{frame_i}c{}.png", cel.layer_index());
            let expected = lodepng::decode32_file(expected_path).unwrap();
            assert_eq!(expected.height, img.height);
            assert_eq!(expected.width, img.width);
            assert_eq!(expected.buffer, img.pixels);
        }
    }
}

#[test]
fn test_combine() {
    let path = "tests/aseprite_files/combine.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::load(&file).unwrap();

    for (index, _) in file.frames.iter().enumerate() {
        let img = file.combined_frame_image(index).unwrap();

        std::fs::create_dir_all("tests/generated_pngs").unwrap();
        let path = format!("tests/generated_pngs/combined_{}.png", index);
        lodepng::encode32_file(path, &img.pixels, img.width, img.height).unwrap();
        
        let expected_path = format!("tests/expected_pngs/combined_{}.png", index);
        let expected = lodepng::decode32_file(expected_path).unwrap();
        assert_eq!(expected.height, img.height);
        assert_eq!(expected.width, img.width);
        assert_eq!(expected.buffer, img.pixels);
    }
}

#[test]
fn test_linkedcells() {
    let path = "tests/aseprite_files/linkedcells.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::load(&file).unwrap();

    for (index, _) in file.frames.iter().enumerate() {
        let img = file.combined_frame_image(index).unwrap();

        std::fs::create_dir_all("tests/generated_pngs").unwrap();
        let path = format!("tests/generated_pngs/linkedcells_{}.png", index);
        lodepng::encode32_file(path, &img.pixels, img.width, img.height).unwrap();

        let expected_path = format!("tests/expected_pngs/linkedcells_{}.png", index);
        let expected = lodepng::decode32_file(expected_path).unwrap();
        assert_eq!(expected.height, img.height);
        assert_eq!(expected.width, img.width);
        assert_eq!(expected.buffer, img.pixels);
    }
}

#[test]
fn test_userdata() {
    let path = "tests/aseprite_files/userdata.aseprite";
    let file = std::fs::read(path).unwrap();
    let file = AsepriteFile::load(&file).unwrap();

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
