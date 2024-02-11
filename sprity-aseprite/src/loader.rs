use std::{
    collections::HashMap,
    ops::RangeBounds,
};

use crate::binary::{
    chunk::Chunk,
    chunks::{
        cel::{CelChunk, CelContent},
        layer::{LayerChunk, LayerFlags},
        slice::SliceChunk,
        tags::TagChunk,
        user_data::UserDataChunk,
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

/// A cel in a frame, there is usually 1 per layer
#[derive(Debug, Clone, Copy)]
pub struct Cel<'a> {
    pub chunk: CelChunk<'a>,
    pub user_data: UserDataChunk<'a>,
    pub image_index: usize,
}

impl Cel<'_> {
    pub fn layer_index(&self) -> usize {
        self.chunk.layer_index as usize
    }
    pub fn x(&self) -> usize {
        self.chunk.x as usize
    }
    pub fn y(&self) -> usize {
        self.chunk.y as usize
    }
    pub fn z_index(&self) -> i16 {
        self.chunk.z_index
    }
}

/// A frame in the file
/// This is a collection of cells for each layer
#[derive(Debug, Clone)]
pub struct Frame<'a> {
    /// In milliseconds
    pub duration: u32,
    pub cells: Vec<Cel<'a>>,
}

impl Frame<'_> {
    pub fn iter_cells(&self) -> impl Iterator<Item = &Cel<'_>> {
        self.cells.iter()
    }
    pub fn cell_at_layer_index(&self, layer_index: usize) -> Option<Cel<'_>> {
        // Binary search should be fast enough
        self.cells
            .binary_search_by(|c| c.layer_index().cmp(&layer_index))
            .ok()
            .map(|i| self.cells[i])
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Tag<'a> {
    pub chunk: TagChunk<'a>,
    pub user_data: UserDataChunk<'a>,
}

impl Tag<'_> {
    pub fn frame_range(&self) -> impl RangeBounds<usize> {
        self.chunk.frames.0 as usize..=self.chunk.frames.1 as usize
    }
    pub fn name(&self) -> &str {
        self.chunk.name
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Layer<'a> {
    pub chunk: LayerChunk<'a>,
    pub user_data: UserDataChunk<'a>,
}

impl Layer<'_> {
    pub fn name(&self) -> &str {
        self.chunk.name
    }
    pub fn visible(&self) -> bool {
        self.chunk.flags.contains(LayerFlags::VISIBLE)
    }
}

#[derive(Debug)]
pub struct AsepriteFile<'a> {
    pub header: Header,
    pub palette: Option<Palette>,
    /// All layers in the file in order
    pub layers: Vec<Layer<'a>>,
    /// All frames in the file in order
    pub frames: Vec<Frame<'a>>,
    /// All tags in the file
    pub tags: Vec<Tag<'a>>,
    /// All images in the file
    pub images: Vec<Image<'a>>,
    pub slices: Vec<SliceChunk<'a>>,
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
                duration: raw_frame.duration as u32,
                cells: Default::default(),
            });
            let mut chunk_it = raw_frame.chunks.into_iter().peekable();
            while let Some(chunk) = chunk_it.next() {
                match chunk {
                    Chunk::Palette0004(_) => {}
                    Chunk::Palette0011(_) => {}
                    Chunk::Layer(chunk) => {
                        // In the first frame, should get all the layer chunks first, then all the actual data in the first frame (cells, etc.)
                        let user_data = if let Some(Chunk::UserData(user_data)) =
                            chunk_it.next_if(Chunk::is_user_data)
                        {
                            user_data
                        } else {
                            Default::default()
                        };
                        self.layers.push(Layer { chunk, user_data });
                    }
                    Chunk::Cel(chunk) => {
                        let user_data = if let Some(Chunk::UserData(user_data)) =
                            chunk_it.next_if(Chunk::is_user_data)
                        {
                            user_data
                        } else {
                            Default::default()
                        };

                        let image_index = match chunk.content {
                            CelContent::Image(image) => {
                                let image_index = self.images.len();
                                self.images.push(image);
                                image_map.insert(
                                    (self.frames.len() - 1, chunk.layer_index),
                                    image_index,
                                );
                                image_index
                            }
                            CelContent::LinkedCel { frame_position } => {
                                image_map[&(frame_position as usize, chunk.layer_index)]
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
                        self.frames.last_mut().unwrap().cells.push(Cel {
                            chunk,
                            user_data,
                            image_index,
                        });
                    }
                    Chunk::CelExtra(_) => {}
                    Chunk::ColorProfile(_) => {}
                    Chunk::ExternalFiles(_) => {}
                    Chunk::Mask(_) => {}
                    Chunk::Path => {}
                    Chunk::Tags(tags_chunk) => {
                        self.tags.extend(tags_chunk.tags.into_iter().map(|chunk| {
                            let user_data = if let Some(Chunk::UserData(user_data)) =
                                chunk_it.next_if(Chunk::is_user_data)
                            {
                                user_data
                            } else {
                                Default::default()
                            };
                            Tag { chunk, user_data }
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

    pub fn canvas_height(&self) -> u16 {
        self.header.height
    }

    pub fn canvas_width(&self) -> u16 {
        self.header.width
    }

    pub fn pixel_count(&self) -> usize {
        self.header.width as usize * self.header.height as usize
    }
}
