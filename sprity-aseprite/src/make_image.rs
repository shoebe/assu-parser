use crate::{
    binary::blend_mode::BlendMode,
    loader::AsepriteFile, wrappers::PixelExt,
};
use image::{GenericImage, Pixel};
use thiserror::Error;

#[allow(missing_copy_implementations)]
#[derive(Error, Debug)]
pub enum LoadImageError {
    #[error("target buffer too small")]
    TargetBufferTooSmall,
    #[error("missing palette")]
    MissingPalette,
    #[error("unsupported color depth")]
    UnsupportedColorDepth,
    #[error("decompression failed")]
    DecompressError,
    #[error("invalid image data")]
    InvalidImageData,
}

fn blend_channel(first: u8, second: u8, alpha: u8, blend_mode: BlendMode) -> u8 {
    let alpha = alpha as f32 / u8::MAX as f32;
    let first = first as f32 / u8::MAX as f32;
    let second = second as f32 / u8::MAX as f32;

    let result = match blend_mode {
        BlendMode::Normal => second,
        BlendMode::Multiply => first * second,
        BlendMode::Screen => 1.0 - (1.0 - first) * (1.0 - second),
        BlendMode::Darken => first.min(second),
        BlendMode::Lighten => first.max(second),
        BlendMode::Addition => (first + second).min(1.0),
        BlendMode::Subtract => (first - second).max(0.0),
        BlendMode::Difference => (first - second).abs(),
        BlendMode::Overlay => {
            if first < 0.5 {
                2.0 * first * second
            } else {
                1.0 - 2.0 * (1.0 - first) * (1.0 - second)
            }
        }
        // @todo: missing modes
        _ => first,
    };

    let blended = first * (1.0 - alpha) + result * alpha;
    (blended.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[derive(Debug)]
/// This image is not the full canvas size. 
/// Displace it by displacement_x/y before layering it
pub struct CroppedImage {
    pub img: image::RgbaImage,
    pub displacement_x: u32,
    pub displacement_y: u32,
}

impl AsepriteFile<'_> {
    /// Get image loader for a given frame index
    /// This will combine all layers into a single image
    /// It would be a good idea to detect duplicates, some frames could be identical to others
    pub fn combined_frame_image(&self, frame_index: usize) -> Result<image::RgbaImage, LoadImageError> {
        let mut pixels = image::RgbaImage::new(self.canvas_width() as u32, self.canvas_height() as u32);

        let frame = &self.frames[frame_index];

        for cel in frame.cells.iter() {
            let layer = &self.layers[cel.layer_index()];
            if !layer.visible() {
                continue;
            }

            let im = &self.images_decompressed[cel.image_index];

            for (x, y, cel_pixel) in im.enumerate_pixels() {
                let target_pixel = pixels.get_pixel_mut(x + cel.x(), y + cel.y());

                let total_alpha =
                    ((cel_pixel.a() as u16 * layer.chunk.opacity as u16) / u8::MAX as u16) as u8;

                for (target_c, cell_c) in target_pixel.channels_mut().iter_mut().zip(cel_pixel.channels()) {
                    *target_c =
                        blend_channel(*target_c, *cell_c, total_alpha, layer.chunk.blend_mode);
                }
            }
        }

        Ok(pixels)
    }

    pub fn combined_frame_image_cropped(&self, frame_index: usize) -> Result<CroppedImage, LoadImageError> {
        let frame = &self.frames[frame_index];
        let mut min_xy = (u32::MAX,u32::MAX);
        let mut max_xy = (0,0);
        for cel in frame.cells.iter() {
            let layer = &self.layers[cel.layer_index()];
            if !layer.visible() {
                continue;
            }
            let im = &self.images_decompressed[cel.image_index];
            min_xy.0 = u32::min(min_xy.0, cel.x());
            min_xy.1 = u32::min(min_xy.1, cel.y());
            max_xy.0 = u32::max(max_xy.0, cel.x() + im.width());
            max_xy.1 = u32::max(max_xy.1, cel.y() + im.height());
        }

        let offset_xy = min_xy;
        let dims_xy = (max_xy.0 - min_xy.0, max_xy.1 - min_xy.1);

        let mut pixels = image::RgbaImage::new(dims_xy.0, dims_xy.1);

        let frame = &self.frames[frame_index];

        for cel in frame.cells.iter() {
            let layer = &self.layers[cel.layer_index()];
            if !layer.visible() {
                continue;
            }

            let im = &self.images_decompressed[cel.image_index];

            for (x, y, cel_pixel) in im.enumerate_pixels() {
                let target_pixel = pixels.get_pixel_mut(x + cel.x() - offset_xy.0, y + cel.y() - offset_xy.1);

                let total_alpha =
                    ((cel_pixel.a() as u16 * layer.chunk.opacity as u16) / u8::MAX as u16) as u8;

                for (target_c, cell_c) in target_pixel.channels_mut().iter_mut().zip(cel_pixel.channels()) {
                    *target_c =
                        blend_channel(*target_c, *cell_c, total_alpha, layer.chunk.blend_mode);
                }
            }
        }

        Ok(CroppedImage {
            img: pixels,
            displacement_x: offset_xy.0,
            displacement_y: offset_xy.1,
        })
    }

}


/*     pub fn get_image_as_rgba(&self, index: usize) -> Result<DecompressedImage<'_>, LoadImageError> {
        let image = &self.images_decompressed[index];
        let mut pixels = vec![RGBA8::zeroed(); image.pixel_count()];
        let target = pixels.as_bytes_mut();

        match self.header.color_depth {
            ColorDepth::Rgba => target.copy_from_slice(image.data),
            ColorDepth::Grayscale => {
                grayscale_to_rgba(image.data, target)?;
            }
            ColorDepth::Indexed => {
                indexed_to_rgba(
                    image.data,
                    &self.palette,
                    target,
                )?;
            }
            ColorDepth::Unknown(_) => return Err(LoadImageError::UnsupportedColorDepth),
        }
        Ok(SizedImage { pixels, width: image.width as usize, height: image.height as usize })
    } 
    
    fn grayscale_to_rgba(source: &[u8], target: &mut [u8]) -> Result<(), LoadImageError> {
    if target.len() != source.len() * 2 {
        return Err(LoadImageError::InvalidImageData);
    }
    let pixels = target.as_rgba_mut();
    for (i, chunk) in source.chunks(2).enumerate() {
        pixels[i].r = chunk[0];
        pixels[i].g = chunk[0];
        pixels[i].b = chunk[0];
        pixels[i].a = chunk[1];
    }
    Ok(())
}

fn indexed_to_rgba(
    source: &[u8],
    palette: &Palette,
    target: &mut [u8],
) -> Result<(), LoadImageError> {
    if target.len() != source.len() * 4 {
        return Err(LoadImageError::InvalidImageData);
    }
    let pixels = target.as_rgba_mut();
    for (i, px) in source.iter().enumerate() {
        pixels[i] = palette.colors[*px as usize];
    }
    Ok(())
}
    */
