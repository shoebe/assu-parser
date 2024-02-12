use std::ops::Range;

use bitflags::bitflags;
use nom::{bytes::complete::take, combinator::{cond, verify}, multi::count};

use crate::binary::{
    errors::{ParseError, ParseResult},
    palette::PaletteError,
    scalars::{
        dword, parse_color, parse_string, word, Color, Word
    },
};

#[derive(Debug)]
pub struct PaletteChunk<'a> {
    pub first_index: u32,
    pub entries: Vec<PaletteEntry<'a>>,
}

#[derive(Debug)]
pub struct PaletteEntry<'a> {
    pub color: Color,
    pub name: Option<&'a str>,
}

bitflags! {
    pub struct PaletteEntryFlags: Word {
        const HAS_NAME = 0x1;
    }
}

pub fn parse_palette_chunk(input: &[u8]) -> ParseResult<'_, PaletteChunk<'_>> {
    let (input, palette_size) = dword(input)?;
    let (input, first_color_index) = verify(dword, |s| *s < palette_size)(input)?;
    let (input, _) = verify(
        dword, 
        |&last_ind| {
            last_ind <= palette_size && last_ind >= first_color_index && last_ind - first_color_index + 1 == palette_size
        }
    )(input)?;

    let (input, _) = take(8usize)(input)?;
    let (input, entries) = count(parse_palette_entry, palette_size as usize)(input)?;
    Ok((
        input,
        PaletteChunk {
            first_index: first_color_index,
            entries,
        },
    ))
}

pub fn parse_palette_entry(input: &[u8]) -> ParseResult<'_, PaletteEntry<'_>> {
    let (input, flags) = word(input)?;
    let flags = PaletteEntryFlags::from_bits_truncate(flags);
    let (input, color) = parse_color(input)?;
    let (input, name) = cond(flags.contains(PaletteEntryFlags::HAS_NAME), parse_string)(input)?;
    Ok((input, PaletteEntry { color, name }))
}

