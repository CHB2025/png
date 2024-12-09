pub mod chunk;
pub mod chunk_kind;
pub mod chunk_reader;
pub mod color_kind;
pub mod filter;

use std::{
    io::{self, Read},
    iter,
};

pub use chunk::*;
pub use chunk_kind::*;
pub use color_kind::*;

const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

pub fn read_chunks(mut reader: impl Read) -> io::Result<Vec<Chunk>> {
    let mut sig = [0u8; 8];
    reader.read_exact(&mut sig)?;
    if sig != PNG_SIG {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PNG missing signature",
        ));
    }
    iter::from_fn(|| Some(Chunk::read(&mut reader)))
        .take_while(|c| c.as_ref().is_ok_and(|c| c.kind() != chunk_kind::IEND))
        .chain(iter::once(Ok(Chunk::new(chunk_kind::IEND, Box::new([])))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TINY_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x37,
        0x6e, 0xf9, 0x24, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x01, 0x63, 0x60,
        0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x73, 0x75, 0x01, 0x18, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn test_tiny() {
        let chunks = dbg!(read_chunks(TINY_PNG).expect("Valid png"));
        let expected = [
            Chunk::new(
                chunk_kind::IHDR,
                TINY_PNG[16..29].to_vec().into_boxed_slice(),
            ),
            Chunk::new(
                chunk_kind::IDAT,
                TINY_PNG[39..49].to_vec().into_boxed_slice(),
            ),
            Chunk::new(chunk_kind::IEND, Box::default()),
        ];

        assert_eq!(chunks[0], expected[0]);
    }
}
