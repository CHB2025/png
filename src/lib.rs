use std::{
    fmt::{LowerHex, UpperHex},
    iter::FusedIterator,
};

mod intermediate;
pub mod parser;

/// 16 bit representation of rgba color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(u16, u16, u16, u16);

impl Color {
    pub const fn new(red: u16, green: u16, blue: u16, alpha: u16) -> Self {
        Self(red, green, blue, alpha)
    }

    pub const fn new_opaque(red: u16, green: u16, blue: u16) -> Self {
        Self::new(red, green, blue, u16::MAX)
    }

    /// Red channel
    pub const fn red(self) -> u16 {
        self.0
    }
    /// Green channel
    pub const fn green(self) -> u16 {
        self.1
    }
    /// Blue channel
    pub const fn blue(self) -> u16 {
        self.2
    }
    /// Alpha channel
    pub const fn alpha(self) -> u16 {
        self.3
    }
}

impl UpperHex for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Color(r, g, b, a) = self;
        write!(f, "{r:X}{g:X}{b:X}{a:X}")
    }
}

impl LowerHex for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Color(r, g, b, a) = self;
        write!(f, "{r:x}{g:x}{b:x}{a:x}")
    }
}

/// Basically a generic image. Contains no png-specific encocding information
#[derive(Debug, PartialEq, Eq)]
pub struct Png {
    height: u32,
    width: u32,
    pixels: Vec<Color>,
}

impl Png {
    pub fn new(height: u32, width: u32, pixels: Vec<Color>) -> Self {
        Self {
            height,
            width,
            pixels,
        }
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn pixels(
        &self,
    ) -> impl Iterator<Item = &Color> + FusedIterator + ExactSizeIterator + DoubleEndedIterator
    {
        self.pixels.iter()
    }
}

// Below are some of my ideas for storing the various PNG types in a struct. All
// will have to be modified to support Compression and Interlacing methods. An
// alternative to all of these would be to just have rgb with 16-bit colors, no
// matter how they were stored in the png. At that point, it's just an image,
// not really a png, and you could end up with a situation where my library
// encodes it differently than it was originally decoded.

/// Generic Png: Color trait implemented by kinds
/// Pros:
/// * Forces conversion of all pixels if you want to change from one color to
///   another
/// Cons:
/// * Have to know color type at compile time. Doesn't seem feasible
mod generic {
    pub trait Color {
        fn rgba(&self) -> (u8, u8, u8, u8);

        // Does this make sense to add?
        fn rgb(&self) -> (u8, u8, u8) {
            let (r, g, b, _) = self.rgba();
            (r, g, b)
        }

        fn hex(&self) -> String {
            let (r, g, b) = self.rgb();
            format!("#{r:X}{g:X}{b:X}")
        }
    }

    pub struct Png<C: Color> {
        pixels: Vec<C>,
    }
}

/// Global Color Png
/// Have to parse pixels as needed
/// Still not to difficult:
///   pixel n = color_kind.channels() * bit_depth * n -> color_kind.channels() * bit_depth * (n+1)
/// Pixel -> rgb(a) will be a little harder since bit depth
///
/// To convert from one to another a new slice has to be allocated
///   An alternative could be always use u32, regardless of bit depth and channels
///   More memory intensive, especially for lower-quality greyscale/indexed-color
/// Easy to save, since pixel_data should be the same as IDAT data (without compression/interlacing anyways)
///   I don't know that this is true
mod global {
    use super::intermediate::ColorKind;

    pub struct Png {
        color_kind: ColorKind,
        bit_depth: u8,
        pixel_data: Vec<u8>,
    }
}

/// Individual Color Png
/// Pixels are all parsed at creation, but using a struct instead of a generic.
/// This means that each pixel could be a different ColorKind, even though that
/// doesn't make sense.
/// The struct Color storing the data on the heap means that to access the color
/// values you need to access two pointers
mod individual_struct {
    use super::intermediate::ColorKind;

    pub struct Color {
        kind: ColorKind,
        bit_depth: u8,
        data: Box<[u8]>,
    }
    pub struct Png {
        pixels: Vec<Color>,
    }
}

/// This comes with the same memory issues as storing all colors as u32 in the
/// global example.
mod individual_enum {
    pub enum Color {
        GreyAlpha(/* data*/),
        // ...
    }
    pub struct Png {
        pixels: Box<[Color]>,
    }
}

/// Lossy Png - Really just Image, with a Png parser
/// Just use 16 bit rgb values in the struct, leave the decoding/encoding
/// decisions to encoding/decoding time
mod lossy {
    pub struct Color(u16, u16, u16);
    pub struct Png {
        pixels: Vec<Color>,
    }
}
