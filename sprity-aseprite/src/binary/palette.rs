use thiserror::Error;

use super::scalars::Color;

#[derive(Debug)]
pub struct Palette {
    pub colors: [Color; 256],
}

impl Default for Palette {
    fn default() -> Self {
        Palette {
            colors: [Color::default(); 256],
        }
    }
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