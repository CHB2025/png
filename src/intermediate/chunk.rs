use std::io::{self, ErrorKind, Read};

use super::ChunkKind;

const MAX_CHUNK_LENGTH: u32 = 2u32.pow(31) - 1;
pub(super) const CRC_TABLE: [u32; 256] = make_crc_table();

// Should this deref to slice?
// Should data be mutable?
#[derive(Clone, PartialEq, Eq)]
pub struct Chunk {
    kind: ChunkKind,
    data: Box<[u8]>,
}

impl Chunk {
    pub const fn new(kind: ChunkKind, data: Box<[u8]>) -> Self {
        Chunk { kind, data }
    }

    /// Reads chunk data from a buffered reader.
    pub fn read(reader: &mut impl Read) -> io::Result<Self> {
        let mut len: [u8; 4] = [0; 4];
        reader.read_exact(&mut len)?;
        let len = u32::from_be_bytes(len);
        if len > MAX_CHUNK_LENGTH {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "Chunk length too long",
            ));
        }

        let mut kind: [u8; 4] = [0; 4];
        reader.read_exact(&mut kind)?;
        let kind =
            ChunkKind::try_from(&kind).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;

        // let data = Vec::with_capacity(len as usize);
        let mut data = vec![0; len as usize];
        reader.read_exact(&mut data[..])?;

        let mut crc = [0u8; 4];
        reader.read_exact(&mut crc)?;
        let crc: u32 = u32::from_be_bytes(crc);

        let chunk = Self {
            kind,
            data: data.into(),
        };

        let expected_crc = chunk.crc();

        if expected_crc != crc {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "Mismatched crc values",
            ));
        }

        Ok(chunk)
    }

    /// Raw data of the chunk
    pub fn data(&self) -> &[u8] {
        self.data.as_ref()
    }

    /// Length of the chunk data in bytes
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Chunk type
    pub fn kind(&self) -> ChunkKind {
        self.kind
    }

    /// Cyclic Redundancy Code for the chunk
    pub fn crc(&self) -> u32 {
        // based off of https://www.w3.org/TR/png-3/#D-CRCAppendix
        let mut crc = u32::MAX;
        for &b in self.kind.as_bytes() {
            let lookup_ind = (crc ^ b as u32) as usize & 0xff;
            crc = CRC_TABLE[lookup_ind] ^ (crc >> 8);
        }
        for &b in self.data() {
            let lookup_ind = (crc ^ b as u32) as usize & 0xff;
            crc = CRC_TABLE[lookup_ind] ^ (crc >> 8);
        }

        crc ^ u32::MAX
    }
}

const fn make_crc_table() -> [u32; 256] {
    // based off of https://www.w3.org/TR/png-3/#D-CRCAppendix
    let mut table = [0; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            if c & 1 == 1 {
                c = 0xedb88320u32 ^ (c >> 1);
            } else {
                c = c >> 1;
            }
            k += 1;
        }
        table[n] = c;
        n += 1
    }
    table
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Chunk {{\n    kind: {:?}\n    len: {}\n}}",
            self.kind,
            self.data.len()
        )
    }
}
