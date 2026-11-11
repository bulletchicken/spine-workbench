//! T.81 Annex H lossless prediction and Huffman differences. Keeping the
//! reconstruction here makes point transforms and restart predictors exact;
//! DCT JPEG continues to use the pinned JPEG decoder.

use super::super::{PixelHeader, truncated, unsupported};
use shapes::errors::EngineError;

#[derive(Clone)]
struct Huffman {
    counts: [u8; 16],
    first: [u32; 16],
    symbols: Vec<u8>,
}

impl Huffman {
    fn read(bytes: &[u8]) -> Result<(Self, usize), EngineError> {
        let counts: [u8; 16] = bytes.get(..16).ok_or_else(truncated)?.try_into().unwrap();
        let count: usize = counts.iter().map(|&n| usize::from(n)).sum();
        if count == 0 || count > 256 {
            return Err(crate::invalid_film());
        }
        let symbols = bytes.get(16..16 + count).ok_or_else(truncated)?.to_vec();
        if symbols.iter().any(|&value| value > 16) {
            return Err(crate::invalid_film());
        }
        let mut first = [0; 16];
        let mut code = 0u32;
        for i in 0..16 {
            first[i] = code;
            code += u32::from(counts[i]);
            if code >= 1 << (i + 1) {
                return Err(crate::invalid_film());
            }
            code <<= 1;
        }
        Ok((
            Self {
                counts,
                first,
                symbols,
            },
            16 + count,
        ))
    }

    fn symbol(&self, input: &mut Entropy<'_>) -> Result<u8, EngineError> {
        let mut code = 0u32;
        let mut offset = 0;
        for i in 0..16 {
            code = (code << 1) | input.bits(1)?;
            let count = usize::from(self.counts[i]);
            if code >= self.first[i] && code < self.first[i] + count as u32 {
                return Ok(self.symbols[offset + (code - self.first[i]) as usize]);
            }
            offset += count;
        }
        Err(crate::invalid_film())
    }
}

struct Entropy<'a> {
    bytes: &'a [u8],
    at: usize,
    byte: u8,
    remaining: u8,
}

impl Entropy<'_> {
    fn bits(&mut self, count: u8) -> Result<u32, EngineError> {
        let mut value = 0;
        for _ in 0..count {
            if self.remaining == 0 {
                self.byte = *self.bytes.get(self.at).ok_or_else(truncated)?;
                self.at += 1;
                if self.byte == 0xff {
                    if self.bytes.get(self.at) != Some(&0) {
                        return Err(crate::invalid_film());
                    }
                    self.at += 1;
                }
                self.remaining = 8;
            }
            self.remaining -= 1;
            value = (value << 1) | u32::from((self.byte >> self.remaining) & 1);
        }
        Ok(value)
    }

    fn marker(&mut self, expected: u8) -> Result<(), EngineError> {
        self.remaining = 0;
        if self.bytes.get(self.at) != Some(&0xff) {
            return Err(crate::invalid_film());
        }
        while self.bytes.get(self.at) == Some(&0xff) {
            self.at += 1;
        }
        if self.bytes.get(self.at) != Some(&expected) {
            return Err(crate::invalid_film());
        }
        self.at += 1;
        Ok(())
    }
}

pub(super) fn decode(
    bytes: &[u8],
    header: PixelHeader,
    precision: u8,
    first_order: bool,
) -> Result<Vec<u16>, EngineError> {
    let components = usize::from(header.samples_per_pixel);
    let mut tables: [Option<Huffman>; 4] = [None, None, None, None];
    let mut component_ids = Vec::new();
    let mut at = 2;
    let mut interval = 0usize;
    let (selectors, predictor, transform, scan_start) = loop {
        if bytes.get(at) != Some(&0xff) {
            return Err(crate::invalid_film());
        }
        while bytes.get(at) == Some(&0xff) {
            at += 1;
        }
        let marker = *bytes.get(at).ok_or_else(truncated)?;
        at += 1;
        let length_bytes = bytes.get(at..at + 2).ok_or_else(truncated)?;
        let length = usize::from(u16::from_be_bytes(length_bytes.try_into().unwrap()));
        if length < 2 {
            return Err(crate::invalid_film());
        }
        let segment = bytes.get(at + 2..at + length).ok_or_else(truncated)?;
        at += length;
        match marker {
            0xc3 => {
                if !component_ids.is_empty() || segment.len() != 6 + 3 * components {
                    return Err(crate::invalid_film());
                }
                for component in segment[6..].as_chunks::<3>().0 {
                    if component[1] != 0x11 || component_ids.contains(&component[0]) {
                        return Err(unsupported("Subsampled lossless JPEG is not supported."));
                    }
                    component_ids.push(component[0]);
                }
            }
            0xc4 => {
                let mut start = 0;
                while start < segment.len() {
                    let index = usize::from(segment[start]);
                    if index >= tables.len() {
                        return Err(crate::invalid_film());
                    }
                    let (table, used) = Huffman::read(&segment[start + 1..])?;
                    tables[index] = Some(table);
                    start += used + 1;
                }
            }
            0xdd => {
                if segment.len() != 2 {
                    return Err(crate::invalid_film());
                }
                interval = usize::from(u16::from_be_bytes(segment.try_into().unwrap()));
            }
            0xda => {
                if component_ids.len() != components
                    || segment.len() != 4 + 2 * components
                    || usize::from(segment[0]) != components
                {
                    return Err(unsupported("Lossless JPEG requires one interleaved scan."));
                }
                let mut selectors = Vec::new();
                for (i, component) in segment[1..1 + 2 * components]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .enumerate()
                {
                    let table = usize::from(component[1] >> 4);
                    if component[0] != component_ids[i]
                        || component[1] & 15 != 0
                        || table >= tables.len()
                        || tables[table].is_none()
                    {
                        return Err(crate::invalid_film());
                    }
                    selectors.push(table);
                }
                let predictor = segment[1 + 2 * components];
                let transform = segment[3 + 2 * components];
                if !(1..=7).contains(&predictor)
                    || (first_order && predictor != 1)
                    || segment[2 + 2 * components] != 0
                    || transform >= precision
                {
                    return Err(crate::invalid_film());
                }
                break (selectors, predictor, transform, at);
            }
            0xe0..=0xef | 0xfe => {}
            _ => return Err(crate::invalid_film()),
        }
    };
    let width = header.width as usize;
    if interval != 0 && !interval.is_multiple_of(width) {
        return Err(crate::invalid_film());
    }
    let count = width * header.height as usize;
    let reduced_bits = precision - transform;
    let mask = (1i32 << reduced_bits) - 1;
    let mut reduced = crate::allocate(count * components)?;
    let mut input = Entropy {
        bytes: &bytes[scan_start..],
        at: 0,
        byte: 0,
        remaining: 0,
    };
    let mut restart = 0;
    for pixel in 0..count {
        let reset = pixel == 0 || (interval > 0 && pixel.is_multiple_of(interval));
        if pixel != 0 && reset {
            input.marker(0xd0 + (restart % 8))?;
            restart = (restart + 1) % 8;
        }
        for (component, &table) in selectors.iter().enumerate() {
            let category = tables[table].as_ref().unwrap().symbol(&mut input)?;
            let difference = match category {
                0 => 0,
                16 => 32768,
                bits => {
                    let value = input.bits(bits)? as i32;
                    if value < 1 << (bits - 1) {
                        value + 1 - (1 << bits)
                    } else {
                        value
                    }
                }
            };
            let x = pixel % width;
            let y = pixel / width;
            let prediction = if reset {
                1 << (reduced_bits - 1)
            } else if y == 0 || (interval != 0 && pixel % interval < width) {
                i32::from(reduced[(pixel - 1) * components + component])
            } else if x == 0 {
                i32::from(reduced[(pixel - width) * components + component])
            } else {
                let a = i32::from(reduced[(pixel - 1) * components + component]);
                let b = i32::from(reduced[(pixel - width) * components + component]);
                let c = i32::from(reduced[(pixel - width - 1) * components + component]);
                match predictor {
                    1 => a,
                    2 => b,
                    3 => c,
                    4 => a + b - c,
                    5 => a + ((b - c) >> 1),
                    6 => b + ((a - c) >> 1),
                    7 => (a + b) >> 1,
                    _ => unreachable!(),
                }
            };
            reduced.push(((prediction + difference) & mask) as u16);
        }
    }
    input.marker(0xd9)?;
    for value in &mut reduced {
        *value <<= transform;
    }
    Ok(reduced)
}
