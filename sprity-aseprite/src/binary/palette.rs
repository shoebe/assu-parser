use thiserror::Error;

use super::scalars::Color;

#[derive(Debug, Default)]
pub struct Palette {
    pub colors: Vec<image::Rgba<u8>>,
}

#[derive(Debug, Copy, Clone, Error)]
pub enum PaletteError {
    #[error("First color index not in range 0..255")]
    FirstColorIndexOutOfBounds,
    #[error("Last color index not in range 0..255")]
    LastColorIndexOutOfBounds,
    #[error("First color index > last color index")]
    FirstColorIndexGreaterThanLastColorIndex,
}