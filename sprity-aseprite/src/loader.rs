use crate::binary::{
    chunk::Chunk,
    chunks::{
        cel::CelContent, color_profile::ColorProfileChunk,
    },
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
    pub palette: Palette,
    pub color_profile: ColorProfileChunk<'a>,
    /// All layers in the file in order
    pub layers: Vec<Layer<'a>>,
    /// All frames in the file in order
    pub frames: Vec<Frame<'a>>,
    /// All tags in the file
    pub tags: Vec<Tag<'a>>,
    /// All images in the file
    pub images: Vec<Image<'a>>,
}

impl<'a> AsepriteFile<'a> {
    fn new<'b: 'a>(file: RawFile<'b>) -> Result<Self, LoadSpriteError> {
        let mut palette = None;
        let mut color_profile = None;
        let mut frames = Vec::with_capacity(file.frames.len());
        let mut layers = Vec::new();
        let mut images = Vec::new();
        let mut tags = Vec::new();


        let mut image_map = ahash::HashMap::default();

        for raw_frame in file.frames.into_iter() {
            frames.push(Frame {
                duration: raw_frame.duration as u32,
                cells: Default::default(),
            });
            let mut chunk_it = raw_frame.chunks.into_iter().peekable();
            while let Some(chunk) = chunk_it.next() {
                match chunk {
                    Chunk::ColorProfile(profile) => {
                        // Seems to be either normal sRGB, fixed sRGB, or an embedded ICC profile
                        // Might want to use this info for the image making?
                        // This chunk should be in all aseprite files
                        color_profile = Some(profile);
                    } 
                    Chunk::Palette(chunk) => {
                        // this seems to always be present, we only need it for indexed color mode though
                        // This chunk should be in all aseprite files
                        let mut p = Palette::default();
                        for (entry, color_idx) in chunk.entries.iter().zip(chunk.indices.clone()) {
                            p.colors[color_idx as usize] = entry.color;
                        }
                        p.colors[file.header.transparent_index as usize].alpha = 0;
                        if palette.is_some() {
                            return Err(LoadSpriteError::Parse {
                                message: "Aseprite file has 2 Palette chunks! Only 1 expected"
                                    .to_string(),
                            });
                        }
                        palette = Some(p);

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
                        layers.push(Layer { chunk, user_data });
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
                                let image_index = images.len();
                                images.push(image);
                                image_map.insert(
                                    (frames.len() - 1, chunk.layer_index),
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
                        frames.last_mut().unwrap().cells.push(Cel {
                            chunk,
                            user_data,
                            image_index,
                        });
                    }                   
                    Chunk::Tags(tags_chunk) => {
                        tags.extend(tags_chunk.tags.into_iter().map(|chunk| {
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
                    Chunk::Tileset(_) => {
                        todo!()
                    }
                    Chunk::Slice(_) => (), // what are these for?
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

        Ok(Self {
            header: file.header,
            palette: palette.ok_or_else(|| LoadSpriteError::Parse {
                message: "Palette chunk not found".to_string(),
            })?,
            color_profile: color_profile.ok_or_else(|| LoadSpriteError::Parse {
                message: "Color profile chunk not found".to_string(),
            })?,
            layers,
            frames,
            tags,
            images,
        })
    }

    /// Load a aseprite file from a byte slice
    pub fn from_bytes<'b: 'a>(data: &'b [u8]) -> Result<AsepriteFile<'a>, LoadSpriteError> {
        let raw_file = parse_raw_file(data).map_err(|e| LoadSpriteError::Parse {
            message: e.to_string(),
        })?;
        
        let ase = Self::new(raw_file)?;
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
