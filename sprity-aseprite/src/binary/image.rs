use super::scalars::Word;

#[derive(Debug, Clone, Copy)]
pub struct Image<'a> {
    /// Width in pixels
    pub width: Word,
    /// Height in pixels
    pub height: Word,
    /// Raw pixel data: row by row from top to bottom,
    /// for each scanline read pixels from left to right.
    /// --or--
    /// "Raw Cel" data compressed with ZLIB method (see NOTE.3)
    pub data: &'a [u8],
    /// True if the cel data is compressed
    /// Generally you'll not find uncompressed images in .aseprite files (only in very old .aseprite files). (from ase doc)
    pub compressed: bool,
}

impl Image<'_> {
    pub fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }
}
