

use nom::{bytes::complete::take, multi::count};
use strum_macros::FromRepr;

use crate::binary::{
    errors::{ParseError, ParseResult},
    scalars::{byte, parse_string, word, Byte, Word},
};

#[derive(Debug)]
/// After the tags chunk, you can write one user data chunk for each tag. E.g. if there are 10 tags, you can then write 10 user data chunks one for each tag.
pub struct TagsChunk<'a> {
    pub tags: Vec<TagChunk<'a>>,
}

/// A tag in the file
/// This is a range of frames over the frames in the file, ordered by frame index
#[derive(Debug, Clone, Copy)]
pub struct TagChunk<'a> {
    /// Both Inclusive
    pub frames: (Word, Word),
    pub animation_direction: AnimationDirection,
    /// Repeat N times. Play this animation section N times:
    ///   0 = Doesn't specify (plays infinite in UI, once on export,
    ///       for ping-pong it plays once in each direction)
    ///   1 = Plays once (for ping-pong, it plays just in one direction)
    ///   2 = Plays twice (for ping-pong, it plays once in one direction,
    ///       and once in reverse)
    ///   n = Plays N times
    pub animation_repeat: Word,
    pub name: &'a str,
}

#[derive(FromRepr, Debug, Copy, Clone)]
pub enum AnimationDirection {
    Forward,
    Reverse,
    PingPong,
    PingPongReverse,
    Unknown(Byte),
}

impl From<Byte> for AnimationDirection {
    fn from(byte: Byte) -> Self {
        AnimationDirection::from_repr(byte.into()).unwrap_or(AnimationDirection::Unknown(byte))
    }
}

pub fn parse_tags_chunk(input: &[u8]) -> ParseResult<'_, TagsChunk<'_>> {
    let (input, number_of_tags) = word(input)?;
    let (input, _) = take(8usize)(input)?;
    let (input, tags) = count(parse_tag, number_of_tags.into())(input)?;
    Ok((input, TagsChunk { tags }))
}

pub fn parse_tag(input: &[u8]) -> ParseResult<'_, TagChunk<'_>> {
    let (input, from_frame) = word(input)?;
    let (input, to_frame) = word(input)?;
    if from_frame > to_frame {
        return Err(nom::Err::Failure(ParseError::InvalidFrameRange(
            from_frame, to_frame,
        )));
    }
    let (input, animation_direction) = byte(input)?;
    let animation_direction = AnimationDirection::from(animation_direction);
    let (input, animation_repeat) = word(input)?;
    let (input, _) = take(6usize)(input)?;
    let (input, color) = take(3usize)(input)?;
    let _ = color; // color of the tag, is deprecated, color in userdata used instead
    let (input, _) = byte(input)?;
    let (input, name) = parse_string(input)?;
    Ok((
        input,
        TagChunk {
            frames: (from_frame, to_frame),
            animation_direction,
            animation_repeat,
            name,
        },
    ))
}

#[test]
fn test_tags() {
    use crate::loader::AsepriteFile;
    let input = std::fs::read("tests/aseprite_files/tags.aseprite").unwrap();
    let file = AsepriteFile::from_bytes(&input).unwrap();
    assert_eq!(file.frames.len(), 1);
    assert_eq!(file.frames[0].duration, 100);
    assert_eq!(file.tags.len(), 3);
    assert_eq!(file.tags[0].name(), "Tag 1");
    assert_eq!(file.tags[1].name(), "Tag 2");
    assert_eq!(file.tags[2].name(), "Tag 3");
}
