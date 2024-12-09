use std::io::{self, Error, ErrorKind, Read, Seek};

use flate2::read::{DeflateDecoder, ZlibDecoder};

use crate::{
    intermediate::{
        self,
        chunk_reader::ChunkReader,
        filter::{Filter, FilterKind},
        Chunk, ChunkKind, ColorKind, PngColor,
    },
    Color, Png,
};

const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// Struct for parsing a png
/// https://www.w3.org/TR/png-3
///
/// E           D
/// | interlace ^
/// | filter    |
/// | compress  |
/// v chunk     |
pub struct PngParser<R> {
    reader: ZlibDecoder<ChunkReader<R>>,
    width: u32,
    height: u32,
    color: PngColor,
    interlace_method: u8,
    filter: Filter,
    compression_method: u8,
}

impl<R> PngParser<R> {
    fn scanline_length(&self) -> usize {
        // TODO: change for interlace method and pass #
        self.width as usize * self.color.data_len().div_ceil(8) + 1
    }
}

impl<R> PngParser<R>
where
    R: Read + Seek,
{
    pub fn new(mut reader: R) -> io::Result<Self> {
        let mut sig = [0u8; 8];
        reader.read_exact(&mut sig)?;
        if sig != PNG_SIG {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "PNG missing signature",
            ));
        }

        let header = Chunk::read(&mut reader)?;
        if header.kind() != intermediate::IHDR || header.len() != 13 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "PNG didn't start with expected header",
            ));
        }

        let header_data: &[u8; 13] = header.data().try_into().expect("Checked length already");
        let width = u32::from_be_bytes(*header_data.first_chunk::<4>().expect("Checked above"));
        let height =
            u32::from_be_bytes(*header_data[4..].first_chunk::<4>().expect("Checked above"));

        let bit_depth = header_data[8];
        let color_kind = ColorKind::try_from(header_data[9])
            .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

        let color = PngColor::new(color_kind, bit_depth)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

        let interlace_method = header_data[12];
        let filter =
            Filter::try_from(header_data[11]).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

        let compression_method = header_data[10];
        assert!(compression_method == 0); // Panic for compressed pngs for now

        // read chunks (and ignore) until first IDAT chunk
        let mut kind_bytes = [0u8; 4];
        reader.seek_relative(4)?; // Skip length
        reader.read_exact(&mut kind_bytes)?;
        let mut chunk_kind =
            ChunkKind::try_from(&kind_bytes).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
        reader.seek_relative(-8)?; // Should be always safe

        while chunk_kind != intermediate::IDAT {
            assert!(chunk_kind.critical()); // Throwing away, so can't be critical
            println!("Throwing away {:?}", chunk_kind);

            _ = Chunk::read(&mut reader)?;

            reader.seek_relative(4)?; // Skip length
            reader.read_exact(&mut kind_bytes)?;
            chunk_kind = ChunkKind::try_from(&kind_bytes)
                .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
            reader.seek_relative(-8)?; // Should be always safe
        }
        // next chunk up is IDAT

        Ok(Self {
            reader: ZlibDecoder::new(ChunkReader::new(reader)?),
            width,
            height,
            color,
            interlace_method,
            filter,
            compression_method,
        })
    }
}

impl<R> PngParser<R>
where
    R: Read,
{
    /// E           D
    /// | interlace ^
    /// | filter    |
    /// | compress  |
    /// v chunk     |
    pub fn parse(mut self) -> Result<Png, io::Error> {
        // De-filter

        let mut pixels: Vec<Color> = Vec::new();

        // TODO: change for interlace method and pass #
        let mut prev = vec![0; self.scanline_length()];
        let mut line = vec![0; self.scanline_length()];

        for _ in 0..self.height {
            self.reader.read_exact(&mut line)?;
            dbg!(&line);
            let (filter_kind, data) = line
                .split_first()
                .expect("Line must be self.scanline_length()");
            let filter_kind = FilterKind::try_from(*filter_kind)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            assert_eq!(filter_kind, FilterKind::default()); // TODO: replace with filtering

            pixels.extend_from_slice(&self.color.parse(data).unwrap()[..self.width as usize]);

            std::mem::swap(&mut prev, &mut line);
        }
        dbg!(pixels);

        // De-interlace
        // Could also be done after converting bytes to colors
        //  - makes sense when using progressive parser

        // Convert bytes to colors

        todo!()
    }
}

impl<R> Iterator for PngParser<R>
where
    R: Read,
{
    type Item = Png;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.interlace_method {
            0 => (1, Some(1)),
            1 => (7, Some(7)),
            _ => (0, Some(0)),
        }
    }
}

impl<R> ExactSizeIterator for PngParser<R> where R: Read {}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::Color;

    use super::*;

    const TINY_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x37,
        0x6e, 0xf9, 0x24, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x01, 0x63, 0x60,
        0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x73, 0x75, 0x01, 0x18, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    // #[test]
    // fn test_tiny() {
    //     let mut parser = PngParser::new(Cursor::new(TINY_PNG)).unwrap();
    //     let image = parser.next().unwrap();
    //     assert_eq!(parser.next(), None);

    //     let mut pixels = image.pixels();
    //     let pixel = pixels.next().unwrap();

    //     assert_eq!(*pixel, Color::new_opaque(0, 0, 0));
    //     assert_eq!(pixels.next(), None);
    // }

    #[test]
    fn test_parse_tiny() {
        let parser = PngParser::new(Cursor::new(TINY_PNG)).unwrap();
        let image = parser.parse().unwrap();

        let mut pixels = image.pixels();
        let pixel = pixels.next().unwrap();

        assert_eq!(*pixel, Color::new_opaque(0, 0, 0));
        assert_eq!(pixels.next(), None);
    }
}
