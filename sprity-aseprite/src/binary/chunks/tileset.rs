use bitflags::bitflags;
use nom::{
    bytes::complete::take,
    combinator::{cond, flat_map, verify}, error::{make_error, ErrorKind},
};

use crate::binary::{
    errors::ParseResult,
    scalars::{dword, parse_string, short, word, Dword, Short, Word},
};

#[derive(Debug, Clone, Copy)]
pub struct TilesetChunk<'a> {
    /// Tileset ID
    pub id: Dword,
    /// Tileset flags
    pub flags: TilesetFlags,
    /// Number of tiles
    pub number_of_tiles: Dword,
    /// Tile Width
    pub width: Word,
    /// Tile Height
    pub height: Word,
    /// Base Index: Number to show in the screen from the tile with
    /// index 1 and so on (by default this is field is 1, so the data
    /// that is displayed is equivalent to the data in memory). But it
    /// can be 0 to display zero-based indexing (this field isn't used
    /// for the representation of the data in the file, it's just for
    /// UI purposes).
    pub base_index: Short,
    /// Name of the tileset
    pub name: &'a str,
    /// Tiles inside this file
    pub tiles: TilesetTiles<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum TilesetTiles<'a> {
    /// Compressed Tileset image (see NOTE.3):
    /// (Tile Width) x (Tile Height x Number of Tiles)
    CompressedTiles(&'a [u8]),
    TilesetExternalFile{
        /// ID of the external file. This ID is one entry
        /// of the the External Files Chunk.
        external_file_id: Dword,
        /// Tileset ID in the external file
        tileset_id: Dword,
    },
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct TilesetFlags: Dword {
        /// 1 - Include link to external file
        const EXTERNAL_FILE = 1;
        /// 2 - Include tiles inside this file
        const TILES = 2;
        /// 4 - Tilemaps using this tileset use tile ID=0 as empty tile
        /// (this is the new format). In rare cases this bit is off,
        /// and the empty tile will be equal to 0xffffffff (used in
        /// internal versions of Aseprite)
        const TILE_0_EMPTY = 4;
        /// 8 - Aseprite will try to match modified tiles with their X
        /// flipped version automatically in Auto mode when using
        /// this tileset.
        const XFLIP = 8;
        /// 16 - Same for Y flips
        const YFLIP = 16;
        /// 32 - Same for D(iagonal) flips
        const DFLIP = 32;
    }
}

pub fn parse_tileset_chunk(input: &[u8]) -> ParseResult<'_, TilesetChunk<'_>> {
    let (input, id) = dword(input)?;
    let (input, flags) = verify(
        map(dword, TilesetFlags::from_bits_truncate), 
        |flags| flags.contains(TilesetFlags::EXTERNAL_FILE) ^ flags.contains(TilesetFlags::TILES)
    )(input)?;

    let (input, number_of_tiles) = dword(input)?;
    let (input, width) = word(input)?;
    let (input, height) = word(input)?;
    let (input, base_index) = short(input)?;
    let (input, _) = take(14usize)(input)?;
    let (input, name) = parse_string(input)?;

    let (input, tiles) = if flags.contains(TilesetFlags::TILES) {
        parse_tiles(input)?
    } else {
        parse_external_file(input)?
    };
    Ok((
        input,
        TilesetChunk {
            id,
            flags,
            number_of_tiles,
            width,
            height,
            base_index,
            name,
            tiles,
        },
    ))
}

pub fn parse_external_file(input: &[u8]) -> ParseResult<'_, TilesetTiles<'_>> {
    let (input, external_file_id) = dword(input)?;
    let (input, tileset_id) = dword(input)?;
    Ok((
        input,
        TilesetTiles::TilesetExternalFile {
            external_file_id,
            tileset_id,
        },
    ))
}

use nom::combinator::map;

pub fn parse_tiles(input: &[u8]) -> ParseResult<'_, TilesetTiles<'_>> {
    map(flat_map(dword, take), |data| TilesetTiles::CompressedTiles(data))(input)
}
