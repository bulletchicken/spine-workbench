//! Bounded DICOM structure reading. Unlisted values are skipped as bytes;
//! they are never decoded as text, retained in an element, or put in errors.

use std::borrow::Cow;

use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{IntakeFlag, TransferSyntax};

use super::pixel_data::transfer_syntax;
use crate::{MAX_FILE_BYTES, allocate, invalid_film, too_large};

type Tag = (u16, u16);

pub(super) struct Input<'a> {
    pub bytes: Cow<'a, [u8]>,
    pub syntax: TransferSyntax,
    pub flags: Vec<IntakeFlag>,
}

#[derive(Clone, Copy)]
pub(super) struct Encoding {
    pub explicit: bool,
    pub big_endian: bool,
}

impl Encoding {
    pub fn for_syntax(syntax: TransferSyntax) -> Self {
        Self {
            explicit: syntax != TransferSyntax::ImplicitLittleEndian,
            big_endian: syntax == TransferSyntax::ExplicitBigEndian,
        }
    }

    pub fn word(self, bytes: &[u8]) -> u16 {
        if self.big_endian {
            u16::from_be_bytes([bytes[0], bytes[1]])
        } else {
            u16::from_le_bytes([bytes[0], bytes[1]])
        }
    }

    fn length(self, bytes: &[u8]) -> u32 {
        let array = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if self.big_endian {
            u32::from_be_bytes(array)
        } else {
            u32::from_le_bytes(array)
        }
    }
}

pub(super) struct Element<'a> {
    pub tag: Tag,
    pub vr: Option<[u8; 2]>,
    pub bytes: &'a [u8],
    pub items: Vec<Dataset<'a>>,
    pub encapsulated: bool,
}

pub(super) struct Dataset<'a> {
    pub elements: Vec<Element<'a>>,
    pub encoding: Encoding,
}

impl<'a> Dataset<'a> {
    pub fn get(&self, tag: Tag) -> Option<&Element<'a>> {
        self.elements.iter().find(|element| element.tag == tag)
    }

    pub fn text(&self, tag: Tag) -> Option<&str> {
        let value = std::str::from_utf8(self.get(tag)?.bytes).ok()?;
        Some(value.trim_matches([' ', '\0']))
    }

    pub fn numbers(&self, tag: Tag) -> Option<Vec<f64>> {
        let text = self.text(tag)?;
        let values: Option<Vec<f64>> = text
            .split('\\')
            .map(|value| {
                let value = value.trim().parse::<f64>().ok()?;
                value.is_finite().then_some(value)
            })
            .collect();
        values.filter(|values| !values.is_empty())
    }

    pub fn number(&self, tag: Tag) -> Option<f64> {
        let value = self
            .text(tag)?
            .split('\\')
            .next()?
            .trim()
            .parse::<f64>()
            .ok()?;
        value.is_finite().then_some(value)
    }

    pub fn word(&self, tag: Tag) -> Option<u16> {
        let bytes = self.get(tag)?.bytes;
        (bytes.len() == 2).then(|| self.encoding.word(bytes))
    }

    pub fn required_word(&self, tag: Tag) -> Result<u16, EngineError> {
        self.word(tag).ok_or_else(invalid_film)
    }
}

pub(super) fn input(bytes: &[u8]) -> Result<Input<'_>, EngineError> {
    let mut pos = if bytes.get(128..132) == Some(b"DICM") {
        132
    } else {
        0
    };
    let meta_encoding = Encoding {
        explicit: true,
        big_endian: false,
    };
    let mut syntax = None;
    let mut meta = false;
    while bytes.get(pos..pos + 2) == Some(&[2, 0]) {
        meta = true;
        let header = header(bytes, &mut pos, meta_encoding)?;
        let value = take(bytes, &mut pos, header.len as usize)?;
        match header.tag {
            (2, 2) if trimmed(value) == Some("1.2.840.10008.1.3.10") => {
                return Err(EngineError::new(
                    ErrorCode::FilmIsADirectory,
                    "a DICOM directory is not a film",
                ));
            }
            (2, 0x10) => {
                syntax = Some(transfer_syntax(trimmed(value).ok_or_else(invalid_film)?)?);
            }
            _ => {}
        }
    }
    let mut flags = Vec::new();
    let syntax = match (meta, syntax) {
        (true, Some(syntax)) => syntax,
        (true, None) => return Err(invalid_film()),
        (false, _) => {
            flags.push(IntakeFlag::NoFileMeta);
            if bytes.get(pos + 4..pos + 6).is_some_and(known_vr) {
                TransferSyntax::ExplicitLittleEndian
            } else {
                TransferSyntax::ImplicitLittleEndian
            }
        }
    };
    let data = &bytes[pos..];
    let bytes = if syntax == TransferSyntax::DeflatedLittleEndian {
        let limit = 4 * MAX_FILE_BYTES;
        match miniz_oxide::inflate::decompress_to_vec_with_limit(data, limit) {
            Ok(inflated) => Cow::Owned(inflated),
            Err(error) if error.status == miniz_oxide::inflate::TINFLStatus::HasMoreOutput => {
                return Err(too_large());
            }
            Err(_)
                if data.len() >= 2
                    && data[0] & 0x0f == 8
                    && (u16::from(data[0]) * 256 + u16::from(data[1])) % 31 == 0 =>
            {
                let inflated = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(data, limit)
                    .map_err(|error| {
                        if error.status == miniz_oxide::inflate::TINFLStatus::HasMoreOutput {
                            too_large()
                        } else {
                            invalid_film()
                        }
                    })?;
                flags.push(IntakeFlag::NonstandardDeflate);
                Cow::Owned(inflated)
            }
            Err(_) => return Err(invalid_film()),
        }
    } else {
        Cow::Borrowed(data)
    };
    Ok(Input {
        bytes,
        syntax,
        flags,
    })
}

fn trimmed(bytes: &[u8]) -> Option<&str> {
    Some(std::str::from_utf8(bytes).ok()?.trim_matches([' ', '\0']))
}

struct Header {
    tag: Tag,
    len: u32,
    sequence: bool,
    vr: Option<[u8; 2]>,
}

fn header(bytes: &[u8], pos: &mut usize, encoding: Encoding) -> Result<Header, EngineError> {
    let head = take(bytes, pos, 8)?;
    let tag = (encoding.word(&head[..2]), encoding.word(&head[2..4]));
    let vr = (encoding.explicit && tag.0 != 0xfffe).then_some([head[4], head[5]]);
    let (len, sequence) = if !encoding.explicit || tag.0 == 0xfffe {
        (encoding.length(&head[4..]), false)
    } else {
        let vr = &head[4..6];
        if !known_vr(vr) {
            return Err(invalid_film());
        }
        let long = matches!(
            vr,
            b"OB"
                | b"OD"
                | b"OF"
                | b"OL"
                | b"OV"
                | b"OW"
                | b"SQ"
                | b"UC"
                | b"UR"
                | b"UT"
                | b"UN"
                | b"SV"
                | b"UV"
        );
        let len = if long {
            encoding.length(take(bytes, pos, 4)?)
        } else {
            u32::from(encoding.word(&head[6..]))
        };
        (len, vr == b"SQ")
    };
    Ok(Header {
        tag,
        len,
        sequence: sequence || matches!(tag, (0x28, 0x3000 | 0x3010)),
        vr,
    })
}

fn known_vr(vr: &[u8]) -> bool {
    matches!(
        vr,
        b"AE"
            | b"AS"
            | b"AT"
            | b"CS"
            | b"DA"
            | b"DS"
            | b"DT"
            | b"FD"
            | b"FL"
            | b"IS"
            | b"LO"
            | b"LT"
            | b"OB"
            | b"OD"
            | b"OF"
            | b"OL"
            | b"OV"
            | b"OW"
            | b"PN"
            | b"SH"
            | b"SL"
            | b"SQ"
            | b"SS"
            | b"ST"
            | b"SV"
            | b"TM"
            | b"UC"
            | b"UI"
            | b"UL"
            | b"UN"
            | b"UR"
            | b"US"
            | b"UT"
            | b"UV"
    )
}

fn take<'a>(bytes: &'a [u8], pos: &mut usize, length: usize) -> Result<&'a [u8], EngineError> {
    let end = pos.checked_add(length).ok_or_else(invalid_film)?;
    let value = bytes.get(*pos..end).ok_or_else(|| {
        EngineError::new(
            ErrorCode::FilmTruncated,
            "the film ends before its data is complete",
        )
    })?;
    *pos = end;
    Ok(value)
}

pub(super) fn dataset(bytes: &[u8], syntax: TransferSyntax) -> Result<Dataset<'_>, EngineError> {
    read_dataset(bytes, &mut 0, Encoding::for_syntax(syntax), 0, true, false)
}

fn read_dataset<'a>(
    bytes: &'a [u8],
    pos: &mut usize,
    encoding: Encoding,
    depth: u8,
    retain: bool,
    delimited: bool,
) -> Result<Dataset<'a>, EngineError> {
    if depth > 32 {
        return Err(too_large());
    }
    let mut elements = Vec::new();
    let mut count = 0;
    while *pos < bytes.len() {
        count += 1;
        if count > 100_000 {
            return Err(too_large());
        }
        let head = header(bytes, pos, encoding)?;
        if head.tag == (0xfffe, 0xe00d) && delimited && head.len == 0 {
            return Ok(Dataset { elements, encoding });
        }
        if head.tag.0 == 0xfffe {
            return Err(invalid_film());
        }
        let keep = retain && allowed(head.tag);
        let start = *pos;
        let encapsulated = head.tag == (0x7fe0, 0x10) && head.len == u32::MAX;
        let mut items = Vec::new();
        if encapsulated {
            loop {
                let item = header(bytes, pos, encoding)?;
                if item.tag == (0xfffe, 0xe0dd) && item.len == 0 {
                    break;
                }
                if item.tag != (0xfffe, 0xe000) || item.len == u32::MAX {
                    return Err(invalid_film());
                }
                take(bytes, pos, item.len as usize)?;
            }
        } else if (head.sequence && keep) || head.len == u32::MAX {
            let end = if head.len == u32::MAX {
                None
            } else {
                Some(
                    start
                        .checked_add(head.len as usize)
                        .ok_or_else(invalid_film)?,
                )
            };
            let sequence_bytes = if let Some(end) = end {
                bytes.get(..end).ok_or_else(invalid_film)?
            } else {
                bytes
            };
            loop {
                if end == Some(*pos) {
                    break;
                }
                let item = header(sequence_bytes, pos, encoding)?;
                if item.tag == (0xfffe, 0xe0dd) && item.len == 0 && end.is_none() {
                    break;
                }
                if item.tag != (0xfffe, 0xe000) {
                    return Err(invalid_film());
                }
                let object = if item.len == u32::MAX {
                    read_dataset(sequence_bytes, pos, encoding, depth + 1, keep, true)?
                } else {
                    let value = take(sequence_bytes, pos, item.len as usize)?;
                    read_dataset(value, &mut 0, encoding, depth + 1, keep, false)?
                };
                if keep {
                    items.push(object);
                }
                if items.len() > 1024 {
                    return Err(too_large());
                }
            }
        } else {
            take(bytes, pos, head.len as usize)?;
        }
        if keep {
            if elements
                .iter()
                .any(|element: &Element<'_>| element.tag == head.tag)
            {
                return Err(invalid_film());
            }
            if head.tag != (0x7fe0, 0x10) && *pos - start > 1024 * 1024 {
                return Err(too_large());
            }
            elements.push(Element {
                tag: head.tag,
                vr: head.vr,
                bytes: if head.sequence {
                    &[]
                } else {
                    &bytes[start..*pos]
                },
                items,
                encapsulated,
            });
        }
    }
    if delimited {
        return Err(invalid_film());
    }
    Ok(Dataset { elements, encoding })
}

// Decode-only fields and lookup tables stay inside this parser's callers.
// Only keep_list constructs the public KeptTags value.
fn allowed(tag: Tag) -> bool {
    matches!(
        tag,
        (0x0008, 0x0016 | 0x0060 | 0x0068 | 0x0070 | 0x1090)
            | (
                0x0018,
                0x1110
                    | 0x1111
                    | 0x1114
                    | 0x1164
                    | 0x1600
                    | 0x1602
                    | 0x1604
                    | 0x1606
                    | 0x1608
                    | 0x1610
                    | 0x1612
                    | 0x1620
                    | 0x2010
                    | 0x5101
            )
            | (0x0020, 0x0020 | 0x0062)
            | (
                0x0028,
                0x0002
                    | 0x0004
                    | 0x0006
                    | 0x0008
                    | 0x0010
                    | 0x0011
                    | 0x0030
                    | 0x0034
                    | 0x0100
                    | 0x0101
                    | 0x0102
                    | 0x0103
                    | 0x0120
                    | 0x0121
                    | 0x0301
                    | 0x0a02
                    | 0x1050
                    | 0x1051
                    | 0x1052
                    | 0x1053
                    | 0x1056
                    | 0x1101
                    | 0x1102
                    | 0x1103
                    | 0x1201
                    | 0x1202
                    | 0x1203
                    | 0x2110
                    | 0x2112
                    | 0x3000
                    | 0x3002
                    | 0x3006
                    | 0x3010
            )
            | (0x2050, 0x0020)
            | (0x7fe0, 0x0001 | 0x0002 | 0x0010)
    )
}

/// Extract frame zero. Offset tables count item headers as well as bytes.
pub(super) fn frame<'a>(
    data: &'a Dataset<'a>,
    syntax: TransferSyntax,
    frames: u32,
) -> Result<Cow<'a, [u8]>, EngineError> {
    let pixel = data.get((0x7fe0, 0x10)).ok_or_else(|| {
        EngineError::new(ErrorCode::FilmNoPixels, "the DICOM contains no pixel data")
    })?;
    if !pixel.encapsulated {
        return Ok(Cow::Borrowed(pixel.bytes));
    }
    let bytes = pixel.bytes;
    let encoding = Encoding {
        explicit: false,
        big_endian: false,
    };
    let mut pos = 0;
    let bot = header(bytes, &mut pos, encoding)?;
    if bot.tag != (0xfffe, 0xe000) || bot.len % 4 != 0 {
        return Err(invalid_film());
    }
    let offsets = take(bytes, &mut pos, bot.len as usize)?;
    let extended = data.get((0x7fe0, 1)).map(|element| element.bytes);
    let end = if frames > 1 {
        if let Some(offsets) = extended {
            if offsets.len() < 16 || offsets.len() % 8 != 0 {
                return Err(invalid_film());
            }
            Some(
                usize::try_from(u64::from_le_bytes(
                    offsets[8..16].try_into().map_err(|_| invalid_film())?,
                ))
                .map_err(|_| too_large())?,
            )
        } else if offsets.len() >= 8 {
            Some(encoding.length(&offsets[4..8]) as usize)
        } else {
            None
        }
    } else {
        None
    };
    if offsets.len() >= 4 && encoding.length(&offsets[..4]) != 0 {
        return Err(invalid_film());
    }
    if extended.is_some_and(|offsets| offsets.len() < 8 || offsets[..8] != [0; 8]) {
        return Err(invalid_film());
    }
    if end == Some(0) {
        return Err(invalid_film());
    }
    let first_item = pos;
    let mut fragments = Vec::new();
    while pos < bytes.len() {
        if end == Some(pos - first_item) {
            break;
        }
        if end.is_some_and(|end| pos - first_item > end) {
            return Err(invalid_film());
        }
        let item = header(bytes, &mut pos, encoding)?;
        if item.tag == (0xfffe, 0xe0dd) {
            if end.is_some() {
                return Err(invalid_film());
            }
            break;
        }
        if item.tag != (0xfffe, 0xe000) {
            return Err(invalid_film());
        }
        fragments.push(take(bytes, &mut pos, item.len as usize)?);
        if frames > 1 && end.is_none() {
            break;
        }
    }
    if fragments.is_empty() || (syntax == TransferSyntax::RleLossless && fragments.len() != 1) {
        return Err(invalid_film());
    }
    if fragments.len() == 1 {
        return Ok(Cow::Borrowed(fragments[0]));
    }
    let len = fragments.iter().try_fold(0usize, |len, fragment| {
        len.checked_add(fragment.len()).ok_or_else(too_large)
    })?;
    let mut joined = allocate(len)?;
    for fragment in fragments {
        joined.extend_from_slice(fragment);
    }
    Ok(Cow::Owned(joined))
}
