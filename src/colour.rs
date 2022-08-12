/// A colour, expressed in RGB or CMYK colour spaces
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Colour {
    /// DeviceRGB colour; r, g, b, range from 0.0 to 1.0
    RGB { r: f32, g: f32, b: f32 },
    /// DeviceCMYK colour; c, m, y, and k range from 0.0 to 1.0
    CMYK { c: f32, m: f32, y: f32, k: f32 },
    /// DeviceGray colour; g ranges from 0.0 to 1.0
    Grey { g: f32 },
}

impl Colour {
    /// Create a new colour in the RGB space. r, g, and b range from 0.0 to 1.0
    pub fn new_rgb(r: f32, g: f32, b: f32) -> Colour {
        Colour::RGB { r, g, b }
    }

    /// Create a new colour in the RGB space. r, g, and b range from 0 to 255
    pub fn new_rgb_bytes(r: u8, g: u8, b: u8) -> Colour {
        Colour::RGB {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
        }
    }

    /// Create a new colour in the CMYK space. c, m, y, and k range from 0.0 to 1.0
    pub fn new_cmyk(c: f32, m: f32, y: f32, k: f32) -> Colour {
        Colour::CMYK { c, m, y, k }
    }

    /// Create a new colour in the CMYK space. c, m, y, and k range from 0 to 255
    pub fn new_cmyk_bytes(c: u8, m: u8, y: u8, k: u8) -> Colour {
        Colour::CMYK {
            c: c as f32 / 255.0,
            m: m as f32 / 255.0,
            y: y as f32 / 255.0,
            k: k as f32 / 255.0,
        }
    }

    /// Create a new colour in the Gray space, g ranges from 0.0 to 1.0
    pub fn new_grey(g: f32) -> Colour {
        Colour::Grey { g }
    }

    /// Create a new colour in the Gray space, g ranges from 0 to 255
    pub fn new_grey_bytes(g: u8) -> Colour {
        Colour::Grey {
            g: g as f32 / 255.0,
        }
    }
}

impl<T: Into<f32>> From<(T, T, T)> for Colour {
    fn from(c: (T, T, T)) -> Self {
        Colour::RGB {
            r: c.0.into(),
            g: c.1.into(),
            b: c.2.into(),
        }
    }
}

impl<T: Into<f32>> From<[T; 3]> for Colour {
    fn from(c: [T; 3]) -> Self {
        let [r, g, b] = c;
        Colour::RGB {
            r: r.into(),
            g: g.into(),
            b: b.into(),
        }
    }
}

impl<T: Into<f32>> From<(T, T, T, T)> for Colour {
    fn from(c: (T, T, T, T)) -> Self {
        Colour::CMYK {
            c: c.0.into(),
            m: c.1.into(),
            y: c.2.into(),
            k: c.3.into(),
        }
    }
}

impl<T: Into<f32>> From<[T; 4]> for Colour {
    fn from(c: [T; 4]) -> Self {
        let [c, m, y, k] = c;
        Colour::CMYK {
            c: c.into(),
            m: m.into(),
            y: y.into(),
            k: k.into(),
        }
    }
}

/// A list of pre-defined colour constants
pub mod colours {
    use super::*;

    pub const BLACK: Colour = Colour::Grey { g: 0.0 };
    pub const WHITE: Colour = Colour::Grey { g: 1.0 };
    pub const RED: Colour = Colour::RGB {
        r: 1.0,
        g: 0.0,
        b: 0.0,
    };
    pub const GREEN: Colour = Colour::RGB {
        r: 0.0,
        g: 1.0,
        b: 0.0,
    };
    pub const BLUE: Colour = Colour::RGB {
        r: 0.0,
        g: 0.0,
        b: 1.0,
    };
    pub const CYAN: Colour = Colour::CMYK {
        c: 1.0,
        m: 0.0,
        y: 0.0,
        k: 0.0,
    };
    pub const MAGENTA: Colour = Colour::CMYK {
        c: 0.0,
        m: 1.0,
        y: 0.0,
        k: 0.0,
    };
    pub const YELLOW: Colour = Colour::CMYK {
        c: 0.0,
        m: 0.0,
        y: 1.0,
        k: 0.0,
    };
}
