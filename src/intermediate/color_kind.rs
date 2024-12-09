use crate::Color;

pub struct PngColor {
    kind: ColorKind,
    depth: u8,
}

impl PngColor {
    pub fn new(kind: ColorKind, depth: u8) -> Result<Self, &'static str> {
        if depth.count_ones() != 1 || kind.allowed_bit_depth() & depth != depth {
            return Err("Invalid color type/bit depth combination");
        }

        Ok(Self { kind, depth })
    }

    pub const fn channels(&self) -> u8 {
        self.kind.channels()
    }

    pub const fn channel_mask(&self) -> u16 {
        match self.depth {
            0b10000 => u16::MAX,
            0b01000 => u8::MAX as u16,
            0b00100 => 15,
            0b00010 => 3,
            0b00001 => 1,
            _ => panic!("Invalid bit depth"),
        }
    }

    pub const fn data_len(&self) -> usize {
        self.channels() as usize * self.depth as usize
    }

    pub fn parse(&self, data: &[u8]) -> Result<Vec<Color>, &'static str> {
        // Not sure how to handle bit depths < 8 (1,2,4)
        let mut colors = Vec::new();
        for i in 0..data.len() * 8 / self.data_len() {
            // i = starting bit position of color
            let mut raw: Vec<u16> = Vec::new();
            for c in (0..self.channels()).rev() {
                // higher shift first
                let start_bit = (i * self.data_len()) + (c * self.depth) as usize;
                let u16_to_check = start_bit / 16;
                let shift = start_bit % 16;
                let mask = self.channel_mask() << shift;

                // Not necessarily even in length (evenly divides into u16s)
                let d = u16::from_be_bytes(
                    *data[u16_to_check..]
                        .first_chunk::<2>()
                        .unwrap_or(&[data[u16_to_check], 0]),
                );
                let mut channel = (d & mask) >> shift;
                let mut t = self.depth;
                while t < 16 {
                    channel |= channel << t;
                    t *= 2;
                }
                raw.push(channel)
            }
            match self.kind {
                ColorKind::Grey(false) => colors.push(Color::new(raw[0], raw[0], raw[0], u16::MAX)),
                ColorKind::Grey(true) => colors.push(Color::new(raw[0], raw[0], raw[0], raw[1])),
                ColorKind::True(false) => colors.push(Color::new(raw[0], raw[1], raw[2], u16::MAX)),
                ColorKind::True(true) => colors.push(Color::new(raw[0], raw[1], raw[2], raw[3])),
                ColorKind::Indexed => todo!(),
            }
        }
        Ok(colors)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorKind {
    /// Greyscale (with alpha)
    Grey(bool),
    /// Truecolor (with alpha)
    True(bool),
    /// Indexed-color
    Indexed, // Where are the indexes to be stored?
}

impl ColorKind {
    /// Returns all allowed bit depths for the given color type. The allowed bit
    /// depths are all powers of two, so all can stored in a single u8.
    pub const fn allowed_bit_depth(self) -> u8 {
        use ColorKind::*;
        match self {
            Grey(false) => 0b11111,
            True(_) | Grey(true) => 0b11000,
            Indexed => 0b1111,
        }
    }

    pub const fn channels(self) -> u8 {
        match self {
            Self::Grey(false) => 1,
            Self::Grey(true) => 2,
            Self::True(false) => 3,
            Self::True(true) => 4,
            Self::Indexed => 1,
        }
    }
}

impl TryFrom<u8> for ColorKind {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Grey(false)),
            2 => Ok(Self::True(false)),
            3 => Ok(Self::Indexed),
            4 => Ok(Self::Grey(true)),
            6 => Ok(Self::True(true)),
            _ => Err("Unknown color kind"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: Color = Color::new(u16::MAX, u16::MAX, u16::MAX, u16::MAX);
    const B: Color = Color::new(0, 0, 0, u16::MAX);

    #[test]
    fn test_allowed_bit_depth() {
        let ck = ColorKind::try_from(0).unwrap(); // Greyscale - 1,2,4,8,16 allowed
        let abd = ck.allowed_bit_depth();
        assert!(abd & 16 == 16);
        assert!(abd & 32 == 0);

        let ck = ColorKind::try_from(6).unwrap(); // Truecolor with alpha - 8, 16 allowed
        let abd = ck.allowed_bit_depth();
        assert!(abd & 16 == 16);
        assert!(abd & 2 != 2);
    }

    #[test]
    fn test_single_greyscale() {
        let ck = ColorKind::Grey(false);
        let color = PngColor::new(ck, 1).unwrap();
        let data = [0b10011111u8];

        let colors = color.parse(&data).unwrap();
        let expected = [W, B, B, W, W, W, W, W];
        for (c, e) in colors.iter().zip(expected.iter()) {
            println!("#{c:X}, #{e:X}");
        }
        assert_eq!(&colors, &expected);
    }

    #[test]
    fn test_two_greyscale() {
        let ck = ColorKind::Grey(false);
        let color = PngColor::new(ck, 2).unwrap();
        let data = [0b10011100u8];
        let a = 0x5555;
        let b = 0xAAAA;
        let ac = Color::new(a, a, a, u16::MAX);
        let bc = Color::new(b, b, b, u16::MAX);

        let colors = color.parse(&data).unwrap();
        let expected = [bc, ac, W, B];
        for (c, e) in colors.iter().zip(expected.iter()) {
            println!("#{c:X}, #{e:X}");
        }

        assert_eq!(&colors, &expected);
    }

    #[test]
    fn test_alpha_greyscale() {
        let ck = ColorKind::Grey(true);
        let color = PngColor::new(ck, 8).unwrap();
        let data = [u8::MAX, u8::MAX, 0, u8::MAX, u8::MAX, 0, 0, 0];
        let mut tw = W;
        tw.3 = 0;
        let mut tb = W;
        tb.3 = 0;

        let colors = color.parse(&data).unwrap();
        let expected = [W, B, tw, tb];
        for (c, e) in colors.iter().zip(expected.iter()) {
            println!("#{c:X}, #{e:X}");
        }

        assert_eq!(&colors, &expected);
    }
}
