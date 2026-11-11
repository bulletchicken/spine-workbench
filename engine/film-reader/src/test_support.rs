//! Writes test DICOM files (feature `test-support`): tests and
//! `spine make-up --format dicom` use it. Never part of the browser build.

use std::collections::BTreeMap;
use std::io::Write;

/// An in-memory DICOM writer for deliberately synthetic films. Numeric
/// element bytes are supplied little-endian and swapped when writing a
/// big-endian dataset. Text and unknown tags can be added to test stripping.
pub struct DicomBuilder {
    syntax: String,
    elements: BTreeMap<(u16, u16), ([u8; 2], Vec<u8>)>,
    fragments: Option<(Vec<Vec<u8>>, Vec<u32>)>,
}

impl DicomBuilder {
    pub fn new(width: u16, height: u16, bits: u8, pixels: &[u16]) -> Self {
        let allocated = if bits <= 8 { 8 } else { 16 };
        let mut builder = Self {
            syntax: "1.2.840.10008.1.2.1".into(),
            elements: BTreeMap::new(),
            fragments: None,
        }
        .text(0x0008, 0x0016, *b"UI", "1.2.840.10008.5.1.4.1.1.1")
        .text(0x0008, 0x0060, *b"CS", "CR")
        .photometric("MONOCHROME2")
        .us(0x0028, 0x0002, 1)
        .us(0x0028, 0x0010, height)
        .us(0x0028, 0x0011, width)
        .us(0x0028, 0x0100, allocated)
        .us(0x0028, 0x0101, u16::from(bits))
        .us(0x0028, 0x0102, u16::from(bits - 1))
        .signed(false);
        let raw = if allocated == 8 {
            pixels.iter().map(|&value| value as u8).collect()
        } else {
            pixels
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect()
        };
        builder = builder.raw_pixels(raw);
        builder
    }

    pub fn syntax(mut self, uid: &str) -> Self {
        self.syntax = uid.to_owned();
        self
    }

    pub fn signed(self, signed: bool) -> Self {
        self.us(0x0028, 0x0103, u16::from(signed))
    }

    pub fn photometric(self, value: &str) -> Self {
        self.text(0x0028, 0x0004, *b"CS", value)
    }

    pub fn text(self, group: u16, element: u16, vr: [u8; 2], value: &str) -> Self {
        self.element(group, element, vr, value.as_bytes().to_vec())
    }

    pub fn us(self, group: u16, element: u16, value: u16) -> Self {
        self.element(group, element, *b"US", value.to_le_bytes().to_vec())
    }

    pub fn element(mut self, group: u16, element: u16, vr: [u8; 2], value: Vec<u8>) -> Self {
        self.elements.insert((group, element), (vr, value));
        self
    }

    pub fn raw_pixels(self, bytes: Vec<u8>) -> Self {
        let vr = if self
            .elements
            .get(&(0x0028, 0x0100))
            .is_some_and(|entry| entry.1 == [8, 0])
        {
            *b"OB"
        } else {
            *b"OW"
        };
        self.element(0x7fe0, 0x0010, vr, bytes)
    }

    pub fn encapsulated(mut self, fragments: Vec<Vec<u8>>, offsets: Vec<u32>) -> Self {
        self.fragments = Some((fragments, offsets));
        self.elements.remove(&(0x7fe0, 0x0010));
        self
    }

    /// Dataset bytes without a preamble or file-meta header; deflation is
    /// applied only by `build`, so tests can also make a zlib-wrapped variant.
    pub fn dataset(&self) -> Vec<u8> {
        let big = self.syntax == "1.2.840.10008.1.2.2";
        let implicit = self.syntax == "1.2.840.10008.1.2";
        let mut output = Vec::new();
        for (&tag, (vr, value)) in &self.elements {
            let mut value = value.clone();
            if big {
                let word_bytes = match vr {
                    b"US" | b"SS" | b"OW" => 2,
                    b"UL" | b"SL" | b"FL" | b"OF" | b"OL" => 4,
                    b"FD" | b"OD" | b"OV" | b"UV" | b"SV" => 8,
                    _ => 1,
                };
                for word in value.chunks_exact_mut(word_bytes) {
                    word.reverse();
                }
            }
            write_element(&mut output, tag, *vr, &value, big, implicit);
        }
        if let Some((fragments, offsets)) = &self.fragments {
            output.extend([0xe0, 0x7f, 0x10, 0x00, b'O', b'B', 0, 0]);
            output.extend(u32::MAX.to_le_bytes());
            let table: Vec<u8> = offsets
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect();
            item(&mut output, &table);
            for fragment in fragments {
                item(&mut output, fragment);
            }
            output.extend([0xfe, 0xff, 0xdd, 0xe0, 0, 0, 0, 0]);
        }
        output
    }

    pub fn build(&self) -> Vec<u8> {
        let mut meta = Vec::new();
        write_element(&mut meta, (2, 1), *b"OB", &[0, 1], false, false);
        let sop = self
            .elements
            .get(&(8, 0x16))
            .map(|entry| entry.1.as_slice())
            .unwrap_or(b"1.2.840.10008.5.1.4.1.1.1");
        write_element(&mut meta, (2, 2), *b"UI", sop, false, false);
        write_element(
            &mut meta,
            (2, 3),
            *b"UI",
            b"1.2.826.0.1.3680043.10.999.1",
            false,
            false,
        );
        write_element(
            &mut meta,
            (2, 0x10),
            *b"UI",
            self.syntax.as_bytes(),
            false,
            false,
        );
        write_element(
            &mut meta,
            (2, 0x12),
            *b"UI",
            b"1.2.826.0.1.3680043.10.999.2",
            false,
            false,
        );
        let mut output = vec![0; 128];
        output.extend(b"DICM");
        write_element(
            &mut output,
            (2, 0),
            *b"UL",
            &(meta.len() as u32).to_le_bytes(),
            false,
            false,
        );
        output.extend(meta);
        let dataset = self.dataset();
        if self.syntax == "1.2.840.10008.1.2.1.99" {
            let mut encoder =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            encoder
                .write_all(&dataset)
                .expect("in-memory compression succeeds");
            output.extend(encoder.finish().expect("in-memory compression succeeds"));
        } else {
            output.extend(dataset);
        }
        output
    }
}

fn write_element(
    output: &mut Vec<u8>,
    tag: (u16, u16),
    vr: [u8; 2],
    value: &[u8],
    big: bool,
    implicit: bool,
) {
    let u16_bytes = |value: u16| {
        if big {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        }
    };
    let u32_bytes = |value: u32| {
        if big {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        }
    };
    output.extend(u16_bytes(tag.0));
    output.extend(u16_bytes(tag.1));
    let length = value.len().next_multiple_of(2);
    if implicit {
        output.extend(u32_bytes(length as u32));
    } else {
        output.extend(vr);
        if matches!(
            &vr,
            b"OB"
                | b"OD"
                | b"OF"
                | b"OL"
                | b"OV"
                | b"OW"
                | b"SQ"
                | b"UC"
                | b"UN"
                | b"UR"
                | b"UT"
                | b"SV"
                | b"UV"
        ) {
            output.extend([0, 0]);
            output.extend(u32_bytes(length as u32));
        } else {
            output.extend(u16_bytes(length as u16));
        }
    }
    output.extend(value);
    if value.len() != length {
        output.push(if matches!(&vr, b"UI" | b"OB" | b"OW" | b"UN") {
            0
        } else {
            b' '
        });
    }
}

fn item(output: &mut Vec<u8>, data: &[u8]) {
    output.extend([0xfe, 0xff, 0x00, 0xe0]);
    output.extend((data.len().next_multiple_of(2) as u32).to_le_bytes());
    output.extend(data);
    if !data.len().is_multiple_of(2) {
        output.push(0);
    }
}

/// Independent literal-only RLE writer. Byte planes are most significant
/// first, with components ordered in the same way as the interleaved input.
pub fn rle_frame(pixels: &[u16], bits_allocated: u8, components: u8) -> Vec<u8> {
    let bytes = usize::from(bits_allocated / 8);
    let components = usize::from(components);
    let mut frame = vec![0; 64];
    frame[..4].copy_from_slice(&((bytes * components) as u32).to_le_bytes());
    for segment in 0..bytes * components {
        let offset = frame.len() as u32;
        frame[4 + 4 * segment..8 + 4 * segment].copy_from_slice(&offset.to_le_bytes());
        let component = segment / bytes;
        let shift = 8 * (bytes - 1 - segment % bytes);
        let plane: Vec<u8> = pixels
            .iter()
            .skip(component)
            .step_by(components)
            .map(|value| (value >> shift) as u8)
            .collect();
        for chunk in plane.chunks(128) {
            frame.push((chunk.len() - 1) as u8);
            frame.extend(chunk);
        }
        if !frame.len().is_multiple_of(2) {
            frame.push(0);
        }
    }
    frame
}

/// One-component T.81 Annex H test encoder. All difference categories use
/// a five-bit Huffman code, so the written stream is easy to inspect.
pub fn lossless_jpeg(
    pixels: &[u16],
    width: u16,
    height: u16,
    precision: u8,
    predictor: u8,
    point_transform: u8,
    restart_interval: u16,
) -> Vec<u8> {
    assert_eq!(pixels.len(), usize::from(width) * usize::from(height));
    assert!(restart_interval == 0 || restart_interval.is_multiple_of(width));
    let mut output = vec![0xff, 0xd8];
    let mut frame = vec![precision];
    frame.extend(height.to_be_bytes());
    frame.extend(width.to_be_bytes());
    frame.extend([1, 1, 0x11, 0]);
    jpeg_segment(&mut output, 0xc3, &frame);
    let mut huffman = vec![0; 17];
    huffman[5] = 17;
    huffman.extend(0..=16);
    jpeg_segment(&mut output, 0xc4, &huffman);
    if restart_interval != 0 {
        jpeg_segment(&mut output, 0xdd, &restart_interval.to_be_bytes());
    }
    jpeg_segment(&mut output, 0xda, &[1, 1, 0, predictor, 0, point_transform]);
    let mut entropy = JpegBits {
        output: &mut output,
        byte: 0,
        used: 0,
    };
    let interval = usize::from(restart_interval);
    let width = usize::from(width);
    let mut marker = 0;
    for (i, &sample) in pixels.iter().enumerate() {
        let reset = i == 0 || (interval != 0 && i.is_multiple_of(interval));
        if i != 0 && reset {
            entropy.flush();
            entropy.output.extend([0xff, 0xd0 + marker]);
            marker = (marker + 1) % 8;
        }
        let at = |index: usize| i32::from(pixels[index] >> point_transform);
        let prediction = if reset {
            1 << (precision - point_transform - 1)
        } else if i < width || (interval != 0 && i % interval < width) {
            at(i - 1)
        } else if i.is_multiple_of(width) {
            at(i - width)
        } else {
            let (a, b, c) = (at(i - 1), at(i - width), at(i - width - 1));
            match predictor {
                1 => a,
                2 => b,
                3 => c,
                4 => a + b - c,
                5 => a + ((b - c) >> 1),
                6 => b + ((a - c) >> 1),
                7 => (a + b) >> 1,
                _ => panic!("test predictor is outside Annex H"),
            }
        };
        let difference = (i32::from(sample >> point_transform) - prediction) as i16;
        let magnitude = i32::from(difference).unsigned_abs();
        let category = (32 - magnitude.leading_zeros()) as u8;
        entropy.push(u32::from(category), 5);
        if category != 0 && category != 16 {
            let value = if difference < 0 {
                ((1i32 << category) - 1 + i32::from(difference)) as u32
            } else {
                difference as u32
            };
            entropy.push(value, category);
        }
    }
    entropy.flush();
    output.extend([0xff, 0xd9]);
    output
}

fn jpeg_segment(output: &mut Vec<u8>, marker: u8, bytes: &[u8]) {
    output.extend([0xff, marker]);
    output.extend(((bytes.len() + 2) as u16).to_be_bytes());
    output.extend(bytes);
}

struct JpegBits<'a> {
    output: &'a mut Vec<u8>,
    byte: u8,
    used: u8,
}

impl JpegBits<'_> {
    fn push(&mut self, value: u32, bits: u8) {
        for i in (0..bits).rev() {
            self.byte = (self.byte << 1) | ((value >> i) & 1) as u8;
            self.used += 1;
            if self.used == 8 {
                self.output.push(self.byte);
                if self.byte == 0xff {
                    self.output.push(0);
                }
                self.byte = 0;
                self.used = 0;
            }
        }
    }

    fn flush(&mut self) {
        if self.used != 0 {
            self.push(0xff, 8 - self.used);
        }
    }
}
