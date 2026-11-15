//! Colors and contrast. Random hues are fine, but every part must stay readable, so lightness is
//! adjusted per part until WCAG-style contrast ratios against its neighbors hold.

#[derive(Clone, Copy, Debug)]
pub struct Hsl {
    pub hue: f32,
    pub saturation: f32,
    pub lightness: f32,
}

impl Hsl {
    pub fn new(hue: f32, saturation: f32, lightness: f32) -> Self {
        Self {
            hue,
            saturation,
            lightness,
        }
    }

    fn rgb(self) -> [u8; 3] {
        let c = (1.0 - (2.0 * self.lightness - 1.0).abs()) * self.saturation;
        let h = self.hue.rem_euclid(360.0) / 60.0;
        let x = c * (1.0 - (h % 2.0 - 1.0).abs());
        let (r, g, b) = match h as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = self.lightness - c / 2.0;
        [r, g, b].map(|v| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8)
    }

    pub fn luminance(self) -> f32 {
        luminance(self.rgb())
    }

    pub fn hex(self) -> String {
        hex(self.rgb())
    }
}

pub fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// Relative luminance as defined by WCAG 2.
pub fn luminance(rgb: [u8; 3]) -> f32 {
    let [r, g, b] = rgb.map(|v| {
        let v = f32::from(v) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

pub fn contrast(a: f32, b: f32) -> f32 {
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

/// A neighbor's luminance and the minimum contrast ratio against it.
pub type Constraint = (f32, f32);

/// Moves the lightness as little as possible (hue and saturation stay) until every constraint
/// holds. Returns the input unchanged if no lightness satisfies them.
pub fn fit(color: Hsl, constraints: &[Constraint]) -> Hsl {
    let holds = |candidate: Hsl| {
        let lum = candidate.luminance();
        constraints
            .iter()
            .all(|&(other, min)| contrast(lum, other) >= min)
    };
    for step in 0..=80 {
        for direction in [-1.0_f32, 1.0] {
            let lightness = color.lightness + direction * step as f32 * 0.01;
            if (0.08..=0.94).contains(&lightness) {
                let candidate = Hsl { lightness, ..color };
                if holds(candidate) {
                    return candidate;
                }
            }
        }
    }
    color
}
