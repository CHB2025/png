use std::io::{self, ErrorKind, Read};

use super::{chunk_kind, ChunkKind, CRC_TABLE};

/// Bytes for CRC + length + kind
const BOUND_LEN: usize = 12;
const INITIAL_CRC: u32 = 3394304481;

/// Lazily parses data chunks of a PNG datastream
#[derive(Debug)]
pub struct ChunkReader<R> {
    reader: R,
    /// Remaining bytes in current chunk
    leftover: usize,
    /// CRC of current chunk calculated on the fly
    crc: u32,
}

impl<R> ChunkReader<R> {
    pub fn is_done(&self) -> bool {
        self.leftover == 0
    }
}

impl<R: Read> ChunkReader<R> {
    pub fn new(mut reader: R) -> std::io::Result<Self> {
        let mut len: [u8; 4] = [0; 4];
        reader.read_exact(&mut len)?;
        let mut len = u32::from_be_bytes(len) as usize;

        let mut kind: [u8; 4] = [0; 4];
        reader.read_exact(&mut kind)?;
        let kind =
            ChunkKind::try_from(&kind).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
        match kind {
            chunk_kind::IDAT => (),
            chunk_kind::IEND => {
                // should be 0 anyway
                len = 0;
            }
            c => panic!("Unexpected chunk kind in chunk reader: {:?}", c),
        }

        Ok(Self {
            reader,
            leftover: len,
            crc: INITIAL_CRC,
        })
    }
}

impl<R: Read> Read for ChunkReader<R> {
    // Right now violates the error condition in the docs:
    // If this function encounters any form of I/O or other error, an error
    // variant will be returned. If an error is returned then it must be
    // guaranteed that no bytes were read.
    //
    // The easiest way I can see to fix this would be to buffer it and save and
    //restore the cursor position in case of an error
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Done reading. IEND recieved
        if self.leftover == 0 {
            return Ok(0);
        }

        let mut bc = self.reader.read(buf)?;
        let mut used = 0;
        while self.leftover != 0 && bc - used >= self.leftover {
            let cb_start = self.leftover + used;
            let cb_end = (cb_start + BOUND_LEN).min(bc);
            let to_read = BOUND_LEN.saturating_sub(bc - cb_start);

            let mut chunk_bound = [0u8; BOUND_LEN];
            // Fill what we can from already read chunks
            buf[cb_start..cb_end]
                .iter()
                .zip(chunk_bound.iter_mut())
                .for_each(|(o, b)| *b = *o);
            // Get the rest from the reader
            if to_read > 0 {
                self.reader
                    .read_exact(&mut chunk_bound[BOUND_LEN - to_read..])?;
            }

            // Move the chunk boundary to the end
            buf[cb_start..bc].rotate_left(BOUND_LEN - to_read);
            buf[bc - (cb_end - cb_start)..bc].fill(0); // Probably not necessary, but clean up

            // Adjust the byte count
            bc -= cb_end - cb_start;

            // Update the crc and check it
            for &b in &buf[used..cb_start] {
                let lookup_ind = (self.crc ^ b as u32) as usize & 0xff;
                self.crc = CRC_TABLE[lookup_ind] ^ (self.crc >> 8);
            }
            let found_crc = u32::from_be_bytes(*chunk_bound.first_chunk::<4>().expect("12 > 4"));
            if found_crc != self.crc ^ u32::MAX {
                // Could this be recoverable?
                self.leftover = 0;
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "Mismatched crc. Error somewhere in transit/processing",
                ));
            }

            // Reset the leftover and crc
            used += self.leftover;
            self.crc = INITIAL_CRC;
            self.leftover =
                u32::from_be_bytes(*chunk_bound[4..].first_chunk::<4>().expect("8 > 4")) as usize;
            let kind = ChunkKind::try_from(chunk_bound[8..].first_chunk::<4>().expect("4 = 4"))
                .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
            match kind {
                chunk_kind::IDAT => (),
                chunk_kind::IEND => {
                    // should be 0 anyway
                    self.leftover = 0;
                    bc = used; // cut off IEND length and crc
                }
                c => panic!("Unexpected chunk kind in chunk reader: {:?}", c),
            }
        }

        // update crc with remaining bytes
        for &b in &buf[used..bc] {
            let lookup_ind = (self.crc ^ b as u32) as usize & 0xff;
            self.crc = CRC_TABLE[lookup_ind] ^ (self.crc >> 8);
        }

        self.leftover -= bc - used;
        Ok(bc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINGLE_CHUNK: &[u8] = &[
        0x00, 0x00, 0x00, 0x0a, // len
        0x49, 0x44, 0x41, 0x54, // IDAT
        0x78, 0x01, 0x63, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, // data
        0x73, 0x75, 0x01, 0x18, // crc
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    const MULTI_CHUNK: &[u8] = &[
        0x00, 0x00, 0x00, 0x0a, // len
        0x49, 0x44, 0x41, 0x54, // IDAT
        0x78, 0x01, 0x63, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, // data
        0x73, 0x75, 0x01, 0x18, // crc
        0x00, 0x00, 0x00, 0x0a, // len
        0x49, 0x44, 0x41, 0x54, // IDAT
        0x78, 0x01, 0x63, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, // data
        0x73, 0x75, 0x01, 0x18, // crc
        0x00, 0x00, 0x00, 0x0a, // len
        0x49, 0x44, 0x41, 0x54, // IDAT
        0x78, 0x01, 0x63, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, // data
        0x73, 0x75, 0x01, 0x18, // crc
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn test_single_chunk() {
        let mut reader = ChunkReader::new(SINGLE_CHUNK).unwrap();

        let mut data = Vec::new();
        let length = reader.read_to_end(&mut data).unwrap();
        assert_eq!(length, 10);
        assert_eq!(data[..], SINGLE_CHUNK[8..18]);
    }

    #[test]
    fn test_multi_chunk() {
        let mut reader = ChunkReader::new(MULTI_CHUNK).unwrap();

        let mut data = Vec::new();
        let length = reader.read_to_end(&mut data).unwrap();
        assert_eq!(length, 30);
        assert_eq!(data[..10], MULTI_CHUNK[8..18]);
        assert_eq!(data[10..20], MULTI_CHUNK[30..40]);
        assert_eq!(data[20..30], MULTI_CHUNK[52..62]);
    }
}
