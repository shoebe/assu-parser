#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::loader::AsepriteFile;
    use image::RgbaImage;

    #[test]
    fn test_cell() {
        let path = "tests/combine.aseprite";
        let file = std::fs::read(path).unwrap();
        let file = AsepriteFile::load(&file).unwrap();

        for frame in file.frames().iter() {
            for (i, cell) in frame.cells.iter().enumerate() {
                let (width, height) = cell.size;
                println!("width: {}, height: {}", width, height);

                let mut target = vec![0; usize::from(width * height) * 4];
                file.load_image(cell.image_index, &mut target).unwrap();

                let image =
                    RgbaImage::from_raw(u32::from(width), u32::from(height), target).unwrap();

                //save
                std::fs::create_dir_all("out").unwrap();
                let path = format!("out/cell_{}.png", i);
                image.save(path).unwrap();
            }
        }
    }

    #[test]
    fn test_combine() {
        let path = "tests/combine.aseprite";
        let file = std::fs::read(path).unwrap();
        let file = AsepriteFile::load(&file).unwrap();

        let (width, height) = file.size();
        for (index, _) in file.frames().iter().enumerate() {
            let mut target = vec![0; usize::from(width * height) * 4];
            let _ = file.combined_frame_image(index, &mut target).unwrap();
            let image = RgbaImage::from_raw(u32::from(width), u32::from(height), target).unwrap();
            std::fs::create_dir_all("out").unwrap();
            let path = format!("out/combined_{}.png", index);
            image.save(path).unwrap();
        }
    }

    #[test]
    fn test_linkedcells() {
        let path = "tests/linkedcells.aseprite";
        let file = std::fs::read(path).unwrap();
        let file = AsepriteFile::load(&file).unwrap();

        let (width, height) = file.size();
        for (index, _) in file.frames().iter().enumerate() {
            let mut target = vec![0; usize::from(width * height) * 4];
            let _ = file.combined_frame_image(index, &mut target).unwrap();
            let image = RgbaImage::from_raw(u32::from(width), u32::from(height), target).unwrap();
            std::fs::create_dir_all("out").unwrap();
            let path = format!("out/linkedcells_{}.png", index);
            image.save(path).unwrap();
        }
    }

    #[test]
    fn test_userdata() {
        let path = "tests/userdata.aseprite";
        let file = std::fs::read(path).unwrap();
        let file = AsepriteFile::load(&file).unwrap();

        assert!(file.layers[0].user_data == "l1");
        assert!(file.layers[1].user_data == "l2");
        assert!(file.layers[2].user_data == "l3");

        assert!(file.frames[1].cells[0].user_data == "l1f2");
        //assert!(file.frames[1].cells[2].user_data == "l1f2"); // can't do since layer 2 don't have a frame, would need to iterate through the array and find layer_ind=2
        assert!(file.frames[0].cells[1].user_data == "l2f1");
        assert!(file.frames[2].cells[0].user_data == "l1f3");
        assert!(file.frames[2].cells[2].user_data == "l3f3");
    }
}
