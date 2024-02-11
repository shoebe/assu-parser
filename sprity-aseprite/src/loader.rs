use std::{collections::HashMap, ops::Range};

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

impl<'a> AsepriteFile<'a> {
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
        let mut ase = Self {
            header: raw_file.header,
            palette: Default::default(),
            layers: Default::default(),
            frames: Default::default(),
            tags: Default::default(),
            images: Default::default(),
            slices: Default::default(),
        };
        ase.init(raw_file)?;
        Ok(ase)
    }
    /// Get size of the sprite (width, height)
    pub fn size(&self) -> (u16, u16) {
        (self.header.width, self.header.height)
    }
    pub fn size_bytes_rgba(&self) -> usize {
        self.header.width as usize * self.header.height as usize * 4
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
}
