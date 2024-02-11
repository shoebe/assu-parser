use std::ops::RangeBounds;

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