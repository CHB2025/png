pub const IHDR: ChunkKind = ChunkKind(*b"IHDR");
pub const PLTE: ChunkKind = ChunkKind(*b"PLTE");
pub const IDAT: ChunkKind = ChunkKind(*b"IDAT");
pub const IEND: ChunkKind = ChunkKind(*b"IEND");

const SIG_BIT: u8 = 0b100000;

/// Specifies the type of chunk. Should maybe be enum with Unkown variant?
///
/// Required to understand:
/// IHDR
/// PLTE
/// IDAT
/// IEND
/// Others are optional
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ChunkKind([u8; 4]);

impl ChunkKind {
    /// Returns a reference to the raw chunk type
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    /// Indicates that this chunk is critical for the successful display of
    /// the png. If the decoder finds an unknown chunk that is critical, it
    /// should not display the image
    pub fn critical(&self) -> bool {
        self.0[0] & SIG_BIT == 0
    }

    /// Indicates that this chunk is defined in the International Standard or is
    /// registered in the list of PNG special-purpose public chunk types
    pub fn public(&self) -> bool {
        self.0[1] & SIG_BIT == 0
    }

    /// Indicates that this chunk is safe to copy if the datastream is changed
    /// even if the editor doesn't recognize the type.
    ///
    /// From the standard:
    /// 1. If a chunk's safe-to-copy bit is 1, the chunk may be copied to a modified PNG datastream whether or not the PNG editor recognizes the chunk type, and regardless of the extent of the datastream modifications.
    /// 2. If a chunk's safe-to-copy bit is 0, it indicates that the chunk depends on the image data. If the program has made any changes to critical chunks, including addition, modification, deletion, or reordering of critical chunks, then unrecognized unsafe chunks shall not be copied to the output PNG datastream. (Of course, if the program does recognize the chunk, it can choose to output an appropriately modified version.)
    /// 3. A PNG editor is always allowed to copy all unrecognized ancillary chunks if it has only added, deleted, modified, or reordered ancillary chunks. This implies that it is not permissible for ancillary chunks to depend on other ancillary chunks.
    /// 4. PNG editors shall terminate on encountering an unrecognized critical chunk type, because there is no way to be certain that a valid datastream will result from modifying a datastream containing such a chunk. (Simply discarding the chunk is not good enough, because it might have unknown implications for the interpretation of other chunks.) The safe/unsafe mechanism is intended for use with ancillary chunks. The safe-to-copy bit will always be 0 for critical chunks.
    pub fn copy_safe(&self) -> bool {
        // A bit weird, since this is opposite of the other two
        self.0[3] & SIG_BIT == SIG_BIT
    }
}

impl std::fmt::Debug for ChunkKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            std::str::from_utf8(&self.0).expect("Always valid ascii characters")
        )
    }
}

impl TryFrom<&[u8; 4]> for ChunkKind {
    type Error = &'static str; // TODO: better error type

    fn try_from(value: &[u8; 4]) -> Result<Self, Self::Error> {
        // TODO: Check validity:
        // Should be ascii characters (65-90, 97-122)
        if value
            .iter()
            .all(|&v| (v >= 65 && v <= 90) || (v >= 97 || v <= 122))
        {
            Ok(Self(*value))
        } else {
            Err("Invalid chunk type")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consts() {
        assert!(IHDR.critical());
        assert!(PLTE.critical());
        assert!(IDAT.critical());
        assert!(IEND.critical());

        assert!(IHDR.public());
        assert!(PLTE.public());
        assert!(IDAT.public());
        assert!(IEND.public());

        assert!(!IHDR.copy_safe());
        assert!(!PLTE.copy_safe());
        assert!(!IDAT.copy_safe());
        assert!(!IEND.copy_safe());
    }

    #[test]
    fn test_unknown() {
        let e1 = ChunkKind::try_from(b"cHnk").unwrap();
        assert!(!e1.critical());
        assert!(e1.public());
        assert!(e1.copy_safe());

        let e2 = ChunkKind::try_from(b"AaAA").unwrap();
        assert!(e2.critical());
        assert!(!e2.public());
        assert!(!e2.copy_safe());
    }
}
