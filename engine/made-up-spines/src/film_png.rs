//! A painted film as the PNG a case file stores: greyscale, its values
//! left-aligned in 8 or 16 bits with an sBIT chunk naming the bits used, so
//! `value >> (16 − bits)` (or `8 − bits`) gives back each stored value, as
//! `shapes::film::FilmImage` describes.

use crate::film_painter::PaintedFilm;

/// The film's PNG bytes.
pub fn encode(film: &PaintedFilm) -> Vec<u8> {
    let (width, height) = (film.image.width(), film.image.height());
    let bits = film.bits_stored;
    let eight = bits <= 8;
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(if eight {
            png::BitDepth::Eight
        } else {
            png::BitDepth::Sixteen
        });
        let mut writer = encoder
            .write_header()
            .expect("writing to memory cannot fail");
        writer
            .write_chunk(png::chunk::sBIT, &[bits])
            .expect("writing to memory cannot fail");
        let data: Vec<u8> = if eight {
            let shift = 8 - u32::from(bits);
            film.image
                .pixels()
                .iter()
                .map(|&v| (v << shift) as u8)
                .collect()
        } else {
            let shift = 16 - u32::from(bits);
            film.image
                .pixels()
                .iter()
                .flat_map(|&v| (v << shift).to_be_bytes())
                .collect()
        };
        writer
            .write_image_data(&data)
            .expect("the data is exactly one image's");
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkroom::grey_image::{GreyImage, Placement};
    use shapes::film::FilmKind;

    fn film(values: Vec<u16>, width: u32, bits: u8) -> PaintedFilm {
        let height = values.len() as u32 / width;
        PaintedFilm {
            kind: FilmKind::Front,
            image: GreyImage::new(width, height, values, Placement::FILM).expect("an image"),
            bits_stored: bits,
            inverted: false,
            lead_letter: None,
            ruler: None,
            ball: None,
            field: None,
        }
    }

    fn decoded(bytes: &[u8]) -> (png::BitDepth, Option<u8>, Vec<u8>) {
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .expect("a PNG");
        let mut data = vec![0; reader.output_buffer_size().expect("a size")];
        let frame = reader.next_frame(&mut data).expect("one frame");
        data.truncate(frame.buffer_size());
        let info = reader.info();
        (info.bit_depth, info.sbit.as_ref().map(|s| s[0]), data)
    }

    #[test]
    fn twelve_bit_values_come_back_by_shifting() {
        let values = vec![0, 1, 4095, 2048, 77, 3000];
        let (depth, sbit, data) = decoded(&encode(&film(values.clone(), 3, 12)));
        assert_eq!(depth, png::BitDepth::Sixteen);
        assert_eq!(sbit, Some(12));
        let back: Vec<u16> = data
            .chunks(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]) >> 4)
            .collect();
        assert_eq!(back, values);
    }

    #[test]
    fn eight_and_sixteen_bit_films_keep_their_values() {
        let (depth, sbit, data) = decoded(&encode(&film(vec![0, 255, 128, 3], 2, 8)));
        assert_eq!((depth, sbit), (png::BitDepth::Eight, Some(8)));
        assert_eq!(data, vec![0, 255, 128, 3]);
        let (_, sbit, data) = decoded(&encode(&film(vec![65535, 1], 2, 16)));
        assert_eq!(sbit, Some(16));
        assert_eq!(data, vec![255, 255, 0, 1]);
    }
}
