use std::{borrow::Cow, collections::HashMap, mem::zeroed};

use crate::{binary::{
    chunk::Chunk, chunks::{
        cel::CelContent, color_profile::ColorProfileChunk, tileset::TilesetChunk,
    }, color_depth::ColorDepth, header::Header, image::Image, palette::Palette, raw_file::{parse_raw_file, RawFile}
}, make_image::{CroppedImage, LoadImageError}};

use crate::wrappers::*;

use itertools::Itertools;
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
    pub images_decompressed: Vec<image::RgbaImage>,
    pub tilesets: Vec<TilesetChunk<'a>>,
}

impl<'a> AsepriteFile<'a> {
    fn new<'b: 'a>(file: RawFile<'b>) -> Result<Self, LoadSpriteError> {
        let mut color_profile = None;
        let mut palette = Palette::default();
        let mut frames = Vec::with_capacity(file.frames.len());
        let mut layers = Vec::new();
        let mut images = Vec::new();
        let mut tags = Vec::new();
        let mut tilesets = Vec::new();


        let mut image_map = ahash::HashMap::default();

        for raw_frame in file.frames.into_iter() {
            frames.push(Frame {
                duration: raw_frame.duration as u32,
                cells: Default::default(),
            });
            let mut chunk_it = raw_frame.chunks.into_iter().peekable();
            while let Some(chunk) = chunk_it.next() {
                match chunk {
                    // Should get the chunks below in the first frame
                    Chunk::ColorProfile(profile) => {
                        // Seems to be either normal sRGB, fixed sRGB, or an embedded ICC profile
                        // Might want to use this info for the image making?
                        // This chunk should be in all aseprite files
                        color_profile = Some(profile);
                    } 
                    Chunk::Palette(chunk) => {
                        // This seems to always be present, only needed for indexed color mode though
                        // documentation says: 
                        //    "Color palettes are in FLI color chunks (it could be type=11 or type=4). For color depths more than 8bpp, palettes are optional."
                        //    Guessing type=11/4 is referring to the old palette chunks? This one is 0x2019
                        let req_len = chunk.first_index as usize + chunk.entries.len();
                        if palette.colors.len() < req_len {
                            palette.colors.resize(req_len, image::Rgba::<u8>::zeroed());
                        }

                        for (idx, entry) in chunk.entries.iter().enumerate() {
                            let c = &mut palette.colors[chunk.first_index as usize + idx]; 
                            c.0 = [entry.color.red, entry.color.green, entry.color.blue, entry.color.alpha];
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
                        layers.push(Layer { 
                            chunk, 
                            parameters: user_data.parse_text_as_layer_parameters(), 
                            user_data,
                        });
                    }
                    Chunk::Tileset(t) => {
                        tilesets.push(t);
                    }
                    // Everything below shows up after the above in the first frame, or in any frame after
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
                                // "data" has all the tiles. A "tile" is a "bits_per_tile" bitmask, apparently always 32-bit right now.
                                // & it with "bitmask_tile_id" to get the tile id, etc. for flips
                                // To get the associated tileset -> get layer of cel -> layer should have "tileset index" -> index tilesets gotten in first frame
                                todo!()
                            }
                            CelContent::Unknown(_) => {
                                return Err(LoadSpriteError::Parse {
                                    message: "CelContent has unknown type!".to_string(),
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
                            Tag { chunk, parameters: user_data.parse_text_as_tag_parameters(), user_data }
                        }))
                    }
                    // below aren't needed for current functionality
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

        if file.header.color_depth != ColorDepth::Rgba {
            return Err(LoadSpriteError::Parse {
                message: format!("Expecting color depth to be Rgba, not {:?}", file.header.color_depth),
            })
        }

        let mut decompressor = flate2::Decompress::new(true);
        let images_decompressed: Result<Vec<_>, _> = images.iter().map(|image| {
            let img = if image.compressed {
                // Pretty sure the images are always compressed
                //let mut buf = vec![0; image.pixel_count() * 4];
                let mut buf = image::RgbaImage::new(image.width as u32, image.height as u32);
                decompressor.reset(true);
                decompressor.decompress(image.data, &mut buf, flate2::FlushDecompress::Finish)
                    .map_err(|e| 
                        LoadSpriteError::Parse {
                            message: format!("failed to decompress: {e}"),
                        }
                    )?;
                buf
            } else {
                image::RgbaImage::from_raw(image.width as u32, image.height as u32, image.data.to_owned())
                    .ok_or_else(|| LoadSpriteError::Parse {
                        message:"image::RgbaImage::from_raw error".to_string(),
                    })?
            };

            Ok(img)
        }).collect();

        let images_decompressed = images_decompressed?;

        Ok(Self {
            header: file.header,
            color_profile: color_profile.ok_or_else(|| LoadSpriteError::Parse {
                message: "Color profile chunk not found".to_string(),
            })?,
            palette,
            layers,
            frames,
            tags,
            images,
            images_decompressed,
            tilesets,
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
