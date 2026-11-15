//! Deterministic bird avatars: a seed yields a small, versioned spec that renders to SVG.
//!
//! The *spec* is the durable artifact: a few bytes the device stores once and every client renders
//! identically. It is deliberately stored instead of the seed, so later changes to the generator's
//! random mapping can never change an avatar that already exists.

mod color;
mod svg;

pub use svg::render_svg;

/// Version byte of the encoded spec. Rendering of a version must stay visually stable.
pub const SPEC_VERSION: u8 = 1;
const FIELD_COUNT: usize = 18;
/// Length of the encoded spec in bytes.
pub const ENCODED_LEN: usize = 1 + FIELD_COUNT;

/// One byte per trait; the renderer maps each onto a bounded range, so any byte pattern is a valid
/// (if possibly unattractive) bird.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AvatarSpec([u8; FIELD_COUNT]);

/// Index of each trait inside the spec. The order is part of the stored format.
#[derive(Clone, Copy)]
pub(crate) enum Field {
    /// Even: one base color; odd: every body part has its own color.
    PaletteMode,
    Hue,
    HueB,
    HueC,
    HueD,
    Saturation,
    BodyLength,
    Plumpness,
    HeadSize,
    BeakLength,
    BeakColor,
    WingLength,
    TailLength,
    TailAngle,
    TailFeathers,
    Crest,
    Tilt,
    Belly,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    Length,
    Version,
    Hex,
}

impl AvatarSpec {
    /// Deterministic derivation; the same seed always yields the same spec.
    pub fn from_seed(seed: u64) -> Self {
        let mut stream = SplitMix64(seed);
        Self(std::array::from_fn(|_| stream.next_byte()))
    }

    pub(crate) fn get(&self, field: Field) -> u8 {
        self.0[field as usize]
    }

    pub fn encode(&self) -> [u8; ENCODED_LEN] {
        let mut out = [0; ENCODED_LEN];
        out[0] = SPEC_VERSION;
        out[1..].copy_from_slice(&self.0);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let Some((version, fields)) = bytes.split_first() else {
            return Err(DecodeError::Length);
        };
        let fields: [u8; FIELD_COUNT] = fields.try_into().map_err(|_| DecodeError::Length)?;
        if *version != SPEC_VERSION {
            return Err(DecodeError::Version);
        }
        Ok(Self(fields))
    }

    pub fn to_hex(&self) -> String {
        self.encode().iter().map(|b| format!("{b:02x}")).collect()
    }

    pub fn from_hex(text: &str) -> Result<Self, DecodeError> {
        if !text.is_ascii() || !text.len().is_multiple_of(2) {
            return Err(DecodeError::Hex);
        }
        let bytes = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| DecodeError::Hex))
            .collect::<Result<Vec<_>, _>>()?;
        Self::decode(&bytes)
    }
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_byte(&mut self) -> u8 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 24) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn same_seed_gives_same_spec() {
        assert_eq!(AvatarSpec::from_seed(7), AvatarSpec::from_seed(7));
        assert_ne!(AvatarSpec::from_seed(7), AvatarSpec::from_seed(8));
    }

    #[test]
    fn hex_round_trip_and_validation() {
        let spec = AvatarSpec::from_seed(42);
        assert_eq!(AvatarSpec::from_hex(&spec.to_hex()), Ok(spec));
        assert_eq!(AvatarSpec::from_hex("01ab"), Err(DecodeError::Length));
        assert_eq!(AvatarSpec::from_hex("zz"), Err(DecodeError::Hex));
        let mut wrong_version = spec.encode();
        wrong_version[0] = 9;
        assert_eq!(
            AvatarSpec::decode(&wrong_version),
            Err(DecodeError::Version)
        );
    }

    #[test]
    fn specs_are_practically_unique() {
        let distinct: HashSet<_> = (0..20_000).map(AvatarSpec::from_seed).collect();
        assert_eq!(distinct.len(), 20_000);
    }

    #[test]
    fn any_byte_pattern_renders_finite_svg() {
        for seed in 0..500 {
            let svg = render_svg(&AvatarSpec::from_seed(seed), "t");
            assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
            assert!(!svg.contains("NaN") && !svg.contains("inf"));
        }
        for byte in [0u8, 255] {
            let mut raw = [byte; ENCODED_LEN];
            raw[0] = SPEC_VERSION;
            let svg = render_svg(&AvatarSpec::decode(&raw).unwrap(), "t");
            assert!(!svg.contains("NaN"));
        }
    }
}
