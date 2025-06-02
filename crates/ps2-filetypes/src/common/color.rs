
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl From<u16> for Color {
    fn from(value: u16) -> Self {
        let r = value & 0x1f;
        let g = (value >> 5) & 0x1f;
        let b = (value >> 10) & 0x1f;
        let a = if value & 0x8000 != 0 { 255 } else { 0 };

        Self {
            r: (r * 255 / 31) as u8,
            g: (g * 255 / 31) as u8,
            b: (b * 255 / 31) as u8,
            a: a as u8,
        }
    }
}

impl Into<u16> for Color {
    fn into(self) -> u16 {
        let r = (self.r as u16 * 31 / 255) & 0x1f;
        let g = (self.g as u16 * 31 / 255) & 0x1f;
        let b = (self.b as u16 * 31 / 255) & 0x1f;
        let a = if self.a > 0 { 0x8000 } else { 0 };

        r | (g << 5) | (b << 10) | a
    }
}

impl Into<[u8; 4]> for Color {
    fn into(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

