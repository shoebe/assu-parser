use crate::{
    binary::{
        blend_mode::BlendMode, chunks::slice::SliceChunk, color_depth::ColorDepth, palette::Palette,
    },
    loader::AsepriteFile,
};
use rgb::{ComponentSlice, FromSlice};
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

impl AsepriteFile<'_> {
    /// Get image loader for a given frame index
    /// This will combine all layers into a single image
    /// returns a hash describing the image, since cells can be reused in multiple frames
    pub fn combined_frame_image(
        &self,
        frame_index: usize,
        target: &mut [u8],
    ) -> Result<(), LoadImageError> {
        let target_size = self.size_bytes_rgba();

        if target.len() < target_size {
            return Err(LoadImageError::TargetBufferTooSmall);
        }

        let pixels = target.as_rgba_mut();

        let frame = &self.frames[frame_index];

        for cell in frame.cells.iter() {
            let layer = &self.layers[cell.layer_index];
            if !layer.visible {
                continue;
            }

            let mut cell_target = vec![0; cell.size.0 as usize * cell.size.1 as usize * 4];
            self.load_image(cell.image_index, &mut cell_target).unwrap();
            let cell_pixels = cell_target.as_rgba();
            let layer = &self.layers[cell.layer_index];

            for y in 0..cell.size.1 {
                for x in 0..cell.size.0 {
                    let origin_x = x + cell.origin.0 as u16;
                    let origin_y = y + cell.origin.1 as u16;

                    let target_index = (origin_y * self.header.width + origin_x) as usize;
                    let cell_index = (y * cell.size.0 + x) as usize;

                    let target_pixel = &mut pixels[target_index];

                    let cell_pixel = &cell_pixels[cell_index];

                    let total_alpha =
                        ((cell_pixel.a as u16 * layer.opacity as u16) / u8::MAX as u16) as u8;

                    for (target_c, cell_c) in target_pixel
                        .as_mut_slice()
                        .iter_mut()
                        .zip(cell_pixel.iter())
                    {
                        *target_c = blend_channel(*target_c, cell_c, total_alpha, layer.blend_mode);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get image loader for a given image index
    pub fn load_image(&self, index: usize, target: &mut [u8]) -> Result<(), LoadImageError> {
        let image = &self.images[index];
        let target_size = image.width as usize * image.height as usize * 4;
        if target.len() < target_size {
            return Err(LoadImageError::TargetBufferTooSmall);
        }
        let target = &mut target[..target_size];
        match (self.header.color_depth, image.compressed) {
            (ColorDepth::Rgba, false) => target.copy_from_slice(image.data),
            (ColorDepth::Rgba, true) => decompress(image.data, target)?,
            (ColorDepth::Grayscale, false) => {
                grayscale_to_rgba(image.data, target)?;
            }
            (ColorDepth::Grayscale, true) => {
                let mut buf = vec![0u8; (image.width * image.height * 2) as usize];
                decompress(image.data, &mut buf)?;
                grayscale_to_rgba(&buf, target)?;
            }
            (ColorDepth::Indexed, false) => {
                indexed_to_rgba(
                    image.data,
                    self.palette
                        .as_ref()
                        .ok_or(LoadImageError::MissingPalette)?,
                    target,
                )?;
            }
            (ColorDepth::Indexed, true) => {
                let mut buf = vec![0u8; (image.width * image.height) as usize];
                decompress(image.data, &mut buf)?;
                indexed_to_rgba(
                    &buf,
                    self.palette
                        .as_ref()
                        .ok_or(LoadImageError::MissingPalette)?,
                    target,
                )?;
            }
            (ColorDepth::Unknown(_), _) => return Err(LoadImageError::UnsupportedColorDepth),
        }
        Ok(())
    }
    pub fn slices(&self) -> &[SliceChunk<'_>] {
        &self.slices
    }
}
