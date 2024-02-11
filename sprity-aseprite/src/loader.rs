use std::{collections::HashMap, ops::Range};

use flate2::Decompress;

use crate::binary::{
    blend_mode::BlendMode,
    chunk::Chunk,
    chunks::{
        cel::CelContent,
        layer::{LayerFlags, LayerType},
        slice::SliceChunk,
        tags::AnimationDirection,
    },
    color_depth::ColorDepth,
    header::Header,
    image::Image,
    palette::{create_palette, Palette},
    raw_file::{parse_raw_file, RawFile},
};

#[derive(Debug)]
pub struct AsepriteFile<'a> {
    pub(crate) header: Header,
    pub(crate) palette: Option<Palette>,
    /// All layers in the file in order
    pub(crate) layers: Vec<Layer>,
    /// All frames in the file in order
    pub(crate) frames: Vec<Frame>,
    /// All tags in the file
    pub(crate) tags: Vec<Tag>,
    /// All images in the file
    pub(crate) images: Vec<Image<'a>>,
    pub(crate) slices: Vec<SliceChunk<'a>>,
}

/// A cell in a frame
/// This is a reference to an image cell
#[derive(Debug, Clone)]
pub struct FrameCell {
    pub origin: (i16, i16),
    pub size: (u16, u16),
    pub layer_index: usize,
    pub image_index: usize,
    pub user_data: String,
}

/// A frame in the file
/// This is a collection of cells for each layer
#[derive(Debug, Clone)]
pub struct Frame {
    pub duration: u16,
    pub origin: (i16, i16),
    pub cells: Vec<FrameCell>,
}

/// A tag in the file
/// This is a range of frames over the frames in the file, ordered by frame index
#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub range: Range<u16>,
    pub direction: AnimationDirection,
    pub repeat: Option<u16>,
    pub user_data: String,
}

/// A layer in the file
#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub opacity: u8,
    pub blend_mode: BlendMode,
    pub visible: bool,
    pub user_data: String,
    pub tileset_ind: Option<usize>,
}

impl<'a> AsepriteFile<'a> {
    fn new(header: Header) -> AsepriteFile<'a> {
        Self {
            header,
            palette: Default::default(),
            layers: Default::default(),
            frames: Default::default(),
            tags: Default::default(),
            images: Default::default(),
            slices: Default::default(),
        }
    }

    fn init<'b: 'a>(&mut self, file: RawFile<'b>) -> Result<(), LoadSpriteError> {
        self.palette = match file.header.color_depth {
            ColorDepth::Indexed => {
                Some(create_palette(&file.header, &file.frames).map_err(|e| {
                    LoadSpriteError::Parse {
                        message: e.to_string(),
                    }
                })?)
            }
            _ => None,
        };

        let mut image_map = HashMap::new();

        for raw_frame in file.frames.into_iter() {
            self.frames.push(Frame {
                duration: raw_frame.duration,
                origin: (0, 0),
                cells: Default::default(),
            });
            let mut chunk_it = raw_frame.chunks.into_iter().peekable();
            while let Some(chunk) = chunk_it.next() {
                match chunk {
                    Chunk::Palette0004(_) => {}
                    Chunk::Palette0011(_) => {}
                    Chunk::Layer(layer) => {
                        // In the first frame, should get all the layer chunks first, then all the actual data in the first frame (cells, etc.)
                        let user_data = if let Some(Chunk::UserData(user_data)) =
                            chunk_it.next_if(Chunk::is_user_data)
                        {
                            user_data.text.unwrap_or_default().to_string()
                        } else {
                            Default::default()
                        };
                        match layer.layer_type {
                            LayerType::Normal | LayerType::Tilemap => {
                                self.layers.push(Layer {
                                    name: layer.name.to_string(),
                                    opacity: layer.opacity,
                                    blend_mode: layer.blend_mode,
                                    visible: layer.flags.contains(LayerFlags::VISIBLE),
                                    user_data,
                                    tileset_ind: layer.tileset_index.map(|a| a as usize),
                                });
                            }
                            LayerType::Group => {
                                todo!()
                            }
                            _ => panic!(),
                        }
                    }
                    Chunk::Cel(cel) => {
                        let user_data = if let Some(Chunk::UserData(user_data)) =
                            chunk_it.next_if(Chunk::is_user_data)
                        {
                            user_data.text.unwrap_or_default().to_string()
                        } else {
                            Default::default()
                        };

                        let image_index = match cel.content {
                            CelContent::Image(image) => {
                                let image_index = self.images.len();
                                self.images.push(image.clone());
                                image_map
                                    .insert((self.frames.len() - 1, cel.layer_index), image_index);
                                image_index
                            }
                            CelContent::LinkedCel { frame_position } => {
                                image_map[&(frame_position as usize, cel.layer_index)]
                            }
                            CelContent::CompressedTilemap { .. } => {
                                return Err(LoadSpriteError::Parse {
                                    message: "CelContent::CompressedTilemap not implemented!"
                                        .to_string(),
                                });
                            }
                            _ => {
                                return Err(LoadSpriteError::Parse {
                                    message: "CelContent not Image or LinkedCel!".to_string(),
                                });
                            }
                        };
                        let im = &self.images[image_index];
                        self.frames.last_mut().unwrap().cells.push(FrameCell {
                            origin: (cel.x, cel.y),
                            size: (im.width, im.height),
                            layer_index: cel.layer_index as usize,
                            image_index,
                            user_data,
                        });
                    }
                    Chunk::CelExtra(_) => {}
                    Chunk::ColorProfile(_) => {}
                    Chunk::ExternalFiles(_) => {}
                    Chunk::Mask(_) => {}
                    Chunk::Path => {}
                    Chunk::Tags(tags_chunk) => {
                        self.tags.extend(tags_chunk.tags.into_iter().map(|tag| {
                            let user_data = if let Some(Chunk::UserData(user_data)) =
                                chunk_it.next_if(Chunk::is_user_data)
                            {
                                user_data.text.unwrap_or_default().to_string()
                            } else {
                                Default::default()
                            };
                            Tag {
                                name: tag.name.to_string(),
                                range: tag.frames.clone(),
                                direction: tag.animation_direction,
                                repeat: if tag.animation_repeat > 0 {
                                    Some(tag.animation_repeat)
                                } else {
                                    None
                                },
                                user_data,
                            }
                        }))
                    }
                    Chunk::Palette(_) => {}
                    Chunk::UserData(_) => {}
                    Chunk::Slice(slice) => self.slices.push(slice),
                    Chunk::Tileset(_) => {
                        todo!()
                    }
                    Chunk::Unsupported(_) => {}
                }
            }
        }

        Ok(())
    }

    /// Load a aseprite file from a byte slice
    pub fn load<'b: 'a>(data: &'b [u8]) -> Result<AsepriteFile<'a>, LoadSpriteError> {
        let raw_file = parse_raw_file(data).map_err(|e| LoadSpriteError::Parse {
            message: e.to_string(),
        })?;
        let mut ase = Self::new(raw_file.header);
        ase.init(raw_file)?;
        Ok(ase)
    }
    /// Get size of the sprite (width, height)
    pub fn size(&self) -> (u16, u16) {
        (self.header.width, self.header.height)
    }
    /// Get tag names
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }
    /// Get layer names
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }
    /// Get the image indices for a given tag and layer
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }
    /// Get image count
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    /// Get image loader for a given frame index
    /// This will combine all layers into a single image
    /// returns a hash describing the image, since cells can be reused in multiple frames
    pub fn combined_frame_image(
        &self,
        frame_index: usize,
        target: &mut [u8],
    ) -> Result<u64, LoadImageError> {
        let mut hash = 0u64;

        let target_size = self.header.width as usize * self.header.height as usize * 4;

        if target.len() < target_size {
            return Err(LoadImageError::TargetBufferTooSmall);
        }

        let frame = &self.frames[frame_index];

        for cell in frame.cells.iter() {
            let layer = &self.layers[cell.layer_index];
            if !layer.visible {
                continue;
            }

            let mut cell_target = vec![0; usize::from(cell.size.0 * cell.size.1) * 4];
            self.load_image(cell.image_index, &mut cell_target).unwrap();
            let layer = &self.layers[cell.layer_index];

            hash += cell.image_index as u64;
            hash += cell.layer_index as u64 * 100;
            hash += cell.origin.0 as u64 * 10000;
            hash += cell.origin.1 as u64 * 1000000;
            hash += cell.size.0 as u64 * 100000000;
            hash += cell.size.1 as u64 * 10000000000;

            for y in 0..cell.size.1 {
                for x in 0..cell.size.0 {
                    let origin_x = x + cell.origin.0 as u16;
                    let origin_y = y + cell.origin.1 as u16;

                    let target_index = (origin_y * self.header.width + origin_x) as usize;
                    let cell_index = (y * cell.size.0 + x) as usize;

                    let target_pixel: &mut [u8] =
                        &mut target[target_index * 4..target_index * 4 + 4];

                    let cell_pixel: &[u8] = &cell_target[cell_index * 4..cell_index * 4 + 4];
                    let cell_alpha = cell_target[cell_index * 4 + 3];

                    let total_alpha = ((cell_alpha as u16 * layer.opacity as u16) / 255) as u8;

                    for i in 0..4 {
                        target_pixel[i] = blend_channel(
                            target_pixel[i],
                            cell_pixel[i],
                            total_alpha,
                            layer.blend_mode,
                        );
                    }
                }
            }
        }

        Ok(hash)
    }

    /// Get image loader for a given image index
    pub fn load_image(&self, index: usize, target: &mut [u8]) -> Result<(), LoadImageError> {
        let image = &self.images[index];
        let target_size = usize::from(image.width * image.height * 4);
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
                let mut buf = vec![0u8; (image.width * image.height * 2).into()];
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
                let mut buf = vec![0u8; (image.width * image.height).into()];
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

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoadSpriteError {
    #[error("parsing failed {message}")]
    Parse { message: String },
    #[error("missing tag: {0}")]
    MissingTag(String),
    #[error("missing layer: {0}")]
    MissingLayer(String),
    #[error("frame index out of range: {0}")]
    FrameIndexOutOfRange(usize),
}

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
    let mut decompressor = Decompress::new(true);
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
    for (i, chunk) in source.chunks(2).enumerate() {
        target[i * 4] = chunk[0];
        target[i * 4 + 1] = chunk[0];
        target[i * 4 + 2] = chunk[0];
        target[i * 4 + 3] = chunk[1];
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
    for (i, px) in source.iter().enumerate() {
        let color = palette.colors[*px as usize];
        target[i * 4] = color.red;
        target[i * 4 + 1] = color.green;
        target[i * 4 + 2] = color.blue;
        target[i * 4 + 3] = color.alpha;
    }
    Ok(())
}

fn blend_channel(first: u8, second: u8, alpha: u8, blend_mode: BlendMode) -> u8 {
    let alpha = alpha as f32 / 255.0;
    let first = first as f32 / 255.0;
    let second = second as f32 / 255.0;

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
    (blended.min(1.0).max(0.0) * 255.0).round() as u8
}
