use std::{borrow::Cow, ops::{RangeBounds, RangeInclusive}, str::FromStr};

use itertools::Itertools;

use crate::binary::chunks::{cel::CelChunk, layer::{LayerChunk, LayerFlags}, tags::TagChunk, user_data::UserDataChunk};

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
    pub fn x(&self) -> u32 {
        self.chunk.x as u32
    }
    pub fn y(&self) -> u32 {
        self.chunk.y as u32
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
        // Binary search since they should be sorted
        self.cells
            .binary_search_by(|c| c.layer_index().cmp(&layer_index))
            .ok()
            .map(|i| self.cells[i])
    }
}

#[derive(Debug, Clone)]
pub struct Tag<'a> {
    pub chunk: TagChunk<'a>,
    pub user_data: UserDataChunk<'a>,
    pub parameters: TagParameters,
}

impl Tag<'_> {
    pub fn frame_range(&self) -> RangeInclusive<usize> {
        self.chunk.frames.0 as usize..=self.chunk.frames.1 as usize
    }
    pub fn name(&self) -> &str {
        self.chunk.name
    }
}

#[derive(Debug, Clone)]
pub struct Layer<'a> {
    pub chunk: LayerChunk<'a>,
    pub user_data: UserDataChunk<'a>,
    pub parameters: LayerParameters,
}

impl Layer<'_> {
    pub fn name(&self) -> &str {
        self.chunk.name
    }
    pub fn visible(&self) -> bool {
        self.chunk.flags.contains(LayerFlags::VISIBLE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum_macros::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum LayerParameter {
    Hitbox,
    Invisible,
    //Seperate, //TODO: implement
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum_macros::EnumString)]
pub enum TagParameter {
    // TODO: what do we want here? Velocity-controls maybe? Might be easier to do that kinda thing from code though...
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum_macros::EnumString)]
pub enum FrameParameter {
    // TODO: what do we want here? Being able to tie an action to a particular frame is important
    // Flip visibilty of another layer?
    // 
}

pub type LayerParameters = ahash::AHashMap<LayerParameter, String>;
pub type TagParameters = Vec<(TagParameter, String)>;

impl UserDataChunk<'_> {
    pub fn parse_text_as_layer_parameters(&self) -> LayerParameters {
        self.text
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .flat_map(|s| LayerParameter::from_str(&s))
            .map(|s| (s, "".to_string()))
            .collect()
    }
    pub fn parse_text_as_tag_parameters(&self) -> TagParameters {
        self.text
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .flat_map(|s| TagParameter::from_str(&s))
            .map(|s| (s, "".to_string()))
            .collect()
    }
}

pub trait PixelExt {
    fn r(&self) -> u8;
    fn b(&self) -> u8;
    fn g(&self) -> u8;
    fn a(&self) -> u8;
    fn zeroed() -> Self;
}

impl PixelExt for image::Rgba<u8> {
    fn r(&self) -> u8 {
        self.0[0]
    }

    fn b(&self) -> u8 {
        self.0[1]
    }

    fn g(&self) -> u8 {
        self.0[2]
    }

    fn a(&self) -> u8 {
        self.0[3]
    }

    fn zeroed() -> Self {
        Self([0;4])
    }   
}

