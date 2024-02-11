use crate::binary::{
    chunk::Chunk,
    chunks::{
        cel::CelContent, color_profile::ColorProfileChunk, slice::SliceChunk
    },
    color_depth::ColorDepth,
    header::Header,
    image::Image,
    palette::Palette,
    raw_file::{parse_raw_file, RawFile},
};

use crate::wrappers::*;

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

#[derive(Debug)]
pub struct AsepriteFile<'a> {
    pub header: Header,
    /// Used for indexed-to-RGB conversion
    pub palette: Option<Palette>,
    pub color_profile: Option<ColorProfileChunk<'a>>,
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
        let mut image_map = ahash::HashMap::default();

        for raw_frame in file.frames.into_iter() {
            self.frames.push(Frame {
                duration: raw_frame.duration as u32,
                cells: Default::default(),
            });
            let mut chunk_it = raw_frame.chunks.into_iter().peekable();
            while let Some(chunk) = chunk_it.next() {
                match chunk {
                    Chunk::ColorProfile(profile) => {
                        // Seems to be either normal sRGB, fixed sRGB, or an embedded ICC profile
                        // Might want to use this info for the image making?
                        self.color_profile = Some(profile);
                    } 
                    Chunk::Palette(chunk) => {
                        // this seems to always be present, is only used for indexed color mode though
                        let mut palette = Palette::default();
                        palette.colors[self.header.transparent_index as usize].alpha = 0;
                        for (entry, color_idx) in chunk.entries.iter().zip(chunk.indices.clone()) {
                            palette.colors[color_idx as usize] = entry.color;
                        }
                    } 
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
                    Chunk::Slice(slice) => self.slices.push(slice),
                    Chunk::Tileset(_) => {
                        todo!()
                    }
                    Chunk::ExternalFiles(_) => {} // Not sure in what situations external files are used
                    Chunk::UserData(_) => {} // we parse all of the ones we want in their respective sections
                    // Above might be useful
                    Chunk::CelExtra(_) => {} // Not sure what this is for (precise position? width/height scaled in real time?)
                    // below is old/deprecated
                    Chunk::Palette0004(_) => {} // only used by old versions of ase
                    Chunk::Palette0011(_) => {} // only used by old versions of ase
                    Chunk::Mask(_) => {} // deprecated by ase
                    Chunk::Path => {} // unused by ase
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
            color_profile: Default::default(),
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
