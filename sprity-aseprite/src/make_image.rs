use crate::{
    binary::{
        blend_mode::BlendMode, color_depth::ColorDepth, palette::Palette,
    },
    loader::AsepriteFile,
};
use rgb::{ComponentBytes, ComponentSlice, FromSlice, Zeroable, RGBA8};
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

fn decompress(data: &[u8], target: &mut [u8]) -> Result<(), LoadImageError> {
    let mut decompressor = flate2::Decompress::new(true);
    match decompressor.decompress(data, target, flate2::FlushDecompress::Finish) {
        Ok(flate2::Status::Ok | flate2::Status::BufError) => Err(LoadImageError::DecompressError),
        Ok(flate2::Status::StreamEnd) => Ok(()),
        Err(_) => Err(LoadImageError::DecompressError),
    }
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
        let color = palette.colors[*px as usize];
        pixels[i].r = color.red;
        pixels[i].g = color.green;
        pixels[i].b = color.blue;
        pixels[i].a = color.alpha;
    }
    Ok(())
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
pub struct SizedImage {
    pub pixels: Vec<RGBA8>,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug)]
/// This image is not the full canvas size. 
/// Displace it by displacement_x/y before layering it
pub struct CroppedImage {
    pub pixels: Vec<RGBA8>,
    pub width: usize,
    pub height: usize,
    pub displacement_x: u32,
    pub displacement_y: u32,
}

impl AsepriteFile<'_> {
    /// Get image loader for a given frame index
    /// This will combine all layers into a single image
    /// It would be a good idea to detect duplicates, some frames could be identical to others
    /// This outputs an image the size of the aseprite canvas
    pub fn combined_frame_image(&self, frame_index: usize) -> Result<SizedImage, LoadImageError> {
        let mut pixels = vec![RGBA8::zeroed(); self.pixel_count()];

        let frame = &self.frames[frame_index];

        for cel in frame.cells.iter() {
            let layer = &self.layers[cel.layer_index()];
            if !layer.visible() {
                continue;
            }

            let cel_img = self.load_image(cel.image_index).unwrap();
            let im = &self.images[cel.image_index];

            for (pixel_ind, cel_pixel) in cel_img.pixels.iter().enumerate() {
                let x = pixel_ind % im.width as usize;
                let y = pixel_ind / im.width as usize;

                let x_target = x + cel.x();
                let y_target = y + cel.y();

                let target_index = y_target * self.header.width as usize + x_target;

                let target_pixel = &mut pixels[target_index];

                let total_alpha =
                    ((cel_pixel.a as u16 * layer.chunk.opacity as u16) / u8::MAX as u16) as u8;

                for (target_c, cell_c) in target_pixel.as_mut_slice().iter_mut().zip(cel_pixel.iter()) {
                    *target_c =
                        blend_channel(*target_c, cell_c, total_alpha, layer.chunk.blend_mode);
                }
            }
        }

        Ok(SizedImage {
            pixels,
            width: self.canvas_width() as usize,
            height: self.canvas_height() as usize,
        })
    }

    pub fn combined_frame_image_cropped(&self, frame_index: usize) -> Result<SizedImage, LoadImageError> {
        let mut pixels = vec![RGBA8::zeroed(); self.pixel_count()];

        let frame = &self.frames[frame_index];

        for cel in frame.cells.iter() {
            let layer = &self.layers[cel.layer_index()];
            if !layer.visible() {
                continue;
            }

            let cel_img = self.load_image(cel.image_index).unwrap();
            let im = &self.images[cel.image_index];

            for (pixel_ind, cel_pixel) in cel_img.pixels.iter().enumerate() {
                let x = pixel_ind % im.width as usize;
                let y = pixel_ind / im.width as usize;

                let x_target = x + cel.x();
                let y_target = y + cel.y();

                let target_index = y_target * self.header.width as usize + x_target;

                let target_pixel = &mut pixels[target_index];

                let total_alpha =
                    ((cel_pixel.a as u16 * layer.chunk.opacity as u16) / u8::MAX as u16) as u8;

                for (target_c, cell_c) in target_pixel.as_mut_slice().iter_mut().zip(cel_pixel.iter()) {
                    *target_c =
                        blend_channel(*target_c, cell_c, total_alpha, layer.chunk.blend_mode);
                }
            }
        }

        Ok(SizedImage {
            pixels,
            width: self.canvas_width() as usize,
            height: self.canvas_height() as usize,
        })
    }

    /// Get image loader for a given image index
    pub fn load_image(&self, index: usize) -> Result<SizedImage, LoadImageError> {
        let image = &self.images[index];
        let mut pixels = vec![RGBA8::zeroed(); image.pixel_count()];
        let target = pixels.as_bytes_mut();

        match (self.header.color_depth, image.compressed) {
            (ColorDepth::Rgba, false) => target.copy_from_slice(image.data),
            (ColorDepth::Rgba, true) => decompress(image.data, target)?,
            (ColorDepth::Grayscale, false) => {
                grayscale_to_rgba(image.data, target)?;
            }
            (ColorDepth::Grayscale, true) => {
                let mut buf = vec![0u8; image.pixel_count()];
                decompress(image.data, &mut buf)?;
                grayscale_to_rgba(&buf, target)?;
            }
            (ColorDepth::Indexed, false) => {
                indexed_to_rgba(
                    image.data,
                    &self.palette,
                    target,
                )?;
            }
            (ColorDepth::Indexed, true) => {
                let mut buf = vec![0u8; image.pixel_count()];
                decompress(image.data, &mut buf)?;
                indexed_to_rgba(
                    &buf,
                    &self.palette,
                    target,
                )?;
            }
            (ColorDepth::Unknown(_), _) => return Err(LoadImageError::UnsupportedColorDepth),
        }
        Ok(SizedImage { pixels, width: image.width as usize, height: image.height as usize })
    }
}
