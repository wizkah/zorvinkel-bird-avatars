//! SVG rendering: a songbird in side view, facing right, drawn in a 200×200 tile.

use crate::color::{self, Hsl, contrast, fit, luminance};
use crate::{AvatarSpec, Field};
use std::fmt::Write;

const TILE: f32 = 200.0;
const BEAK_COLORS: [[u8; 3]; 5] = [
    [0xf7, 0x94, 0x1d],
    [0xf6, 0xc2, 0x1a],
    [0xe5, 0x53, 0x3d],
    [0xe8, 0xdc, 0xc0],
    [0x3a, 0x3f, 0x47],
];
const DARK: [u8; 3] = [0x1c, 0x1f, 0x26];
const EYE_COLOR: &str = "#1c1f26";
/// Minimum contrast of a body part against the background (WCAG graphics threshold).
const AGAINST_BACKGROUND: f32 = 3.0;
const BODY_CENTER: (f32, f32) = (98.0, 112.0);

struct Colors {
    background: Hsl,
    head: Hsl,
    body: Hsl,
    wing: Hsl,
    tail: Hsl,
    belly: Hsl,
    beak: [u8; 3],
    legs: [u8; 3],
}

struct Palette {
    background: String,
    head: String,
    body: String,
    wing: String,
    tail: String,
    belly: String,
    beak: String,
    legs: String,
}

/// Maps a raw byte onto `lo..=hi`.
fn range(value: u8, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * f32::from(value) / 255.0
}

/// Picks the hues, then fixes lightness so that every part is readable.
fn colors(spec: &AvatarSpec) -> Colors {
    let hues =
        [Field::Hue, Field::HueB, Field::HueC, Field::HueD].map(|f| range(spec.get(f), 0.0, 360.0));
    let saturation = range(spec.get(Field::Saturation), 0.5, 0.85);
    let (background, head, body, wing, tail, belly) = if spec.get(Field::PaletteMode) % 2 == 1 {
        let part = |hue, lightness| Hsl::new(hue, saturation, lightness);
        (
            Hsl::new(hues[3], 0.35, 0.93),
            part(hues[0], 0.42),
            part(hues[1], 0.50),
            part(hues[2], 0.36),
            part(hues[3], 0.40),
            Hsl::new(hues[1], 0.55, 0.88),
        )
    } else {
        let drift = (hues[1] - 180.0) / 6.0;
        let tone = |step: f32, lightness| Hsl::new(hues[0] + drift * step, saturation, lightness);
        (
            Hsl::new(hues[0] + 8.0, 0.40, 0.93),
            tone(0.0, 0.40),
            tone(0.5, 0.50),
            tone(1.0, 0.30),
            tone(1.0, 0.26),
            tone(2.0, 0.86),
        )
    };
    let background_lum = background.luminance();
    let body = fit(body, &[(background_lum, AGAINST_BACKGROUND)]);
    let body_lum = body.luminance();
    let head = fit(
        head,
        &[(background_lum, AGAINST_BACKGROUND), (body_lum, 1.6)],
    );
    let wing = fit(
        wing,
        &[(background_lum, AGAINST_BACKGROUND), (body_lum, 1.8)],
    );
    let tail = fit(
        tail,
        &[(background_lum, AGAINST_BACKGROUND), (body_lum, 1.4)],
    );
    let belly = fit(belly, &[(body_lum, 1.8)]);
    let beak = beak_color(spec, head.luminance(), background_lum);
    let legs = if contrast(luminance(beak), background_lum) >= AGAINST_BACKGROUND {
        beak
    } else {
        DARK
    };
    Colors {
        background,
        head,
        body,
        wing,
        tail,
        belly,
        beak,
        legs,
    }
}

/// The preferred beak color unless it disappears against the head or the background (the beak
/// tip sticks out over it); then the alternative whose weaker contrast is best.
fn beak_color(spec: &AvatarSpec, head_luminance: f32, background_luminance: f32) -> [u8; 3] {
    let preferred = BEAK_COLORS[usize::from(spec.get(Field::BeakColor)) % BEAK_COLORS.len()];
    let weakest = |rgb: [u8; 3]| {
        let lum = luminance(rgb);
        contrast(lum, head_luminance).min(contrast(lum, background_luminance) * 1.4)
    };
    if weakest(preferred) >= 2.2 {
        return preferred;
    }
    *BEAK_COLORS
        .iter()
        .max_by(|a, b| weakest(**a).total_cmp(&weakest(**b)))
        .unwrap_or(&preferred)
}

fn palette(spec: &AvatarSpec) -> Palette {
    let colors = colors(spec);
    Palette {
        background: colors.background.hex(),
        head: colors.head.hex(),
        body: colors.body.hex(),
        wing: colors.wing.hex(),
        tail: colors.tail.hex(),
        belly: colors.belly.hex(),
        beak: color::hex(colors.beak),
        legs: color::hex(colors.legs),
    }
}

#[derive(Clone, Copy)]
struct Point(f32, f32);

impl Point {
    fn offset(self, dx: f32, dy: f32) -> Point {
        Point(self.0 + dx, self.1 + dy)
    }
}

struct Shape {
    body_rx: f32,
    body_ry: f32,
    tilt_deg: f32,
    head_radius: f32,
    beak_length: f32,
    wing_length: f32,
    tail_length: f32,
    tail_angle_deg: f32,
    tail_feathers: usize,
    crest: usize,
    belly: f32,
}

impl Shape {
    fn of(spec: &AvatarSpec) -> Self {
        let get = |field| spec.get(field);
        let body_rx = range(get(Field::BodyLength), 38.0, 50.0);
        Self {
            body_rx,
            body_ry: body_rx * range(get(Field::Plumpness), 0.70, 0.95),
            tilt_deg: range(get(Field::Tilt), -35.0, -12.0),
            head_radius: range(get(Field::HeadSize), 15.0, 23.0),
            beak_length: range(get(Field::BeakLength), 10.0, 26.0),
            wing_length: range(get(Field::WingLength), 34.0, 58.0),
            tail_length: range(get(Field::TailLength), 36.0, 62.0),
            tail_angle_deg: range(get(Field::TailAngle), -12.0, 18.0),
            tail_feathers: 1 + usize::from(get(Field::TailFeathers)) % 3,
            crest: usize::from(get(Field::Crest)) % 4,
            // 0 means no lighter belly at all.
            belly: [0.0, 0.55, 0.7, 0.85][usize::from(get(Field::Belly)) % 4],
        }
    }

    /// Point given in the body's own (tilted) coordinate frame, in tile coordinates.
    fn on_body(&self, local_x: f32, local_y: f32) -> Point {
        let (sin, cos) = self.tilt_deg.to_radians().sin_cos();
        let (x, y) = (local_x * self.body_rx, local_y * self.body_ry);
        Point(
            BODY_CENTER.0 + x * cos - y * sin,
            BODY_CENTER.1 + x * sin + y * cos,
        )
    }

    /// Direction from head to tail along the body axis, in degrees.
    fn rear_angle_deg(&self) -> f32 {
        180.0 + self.tilt_deg
    }
}

/// `id_prefix` keeps element ids unique when several avatars share one document.
pub fn render_svg(spec: &AvatarSpec, id_prefix: &str) -> String {
    let shape = Shape::of(spec);
    let palette = palette(spec);
    let head = shape.on_body(0.72, -0.18).offset(0.0, 0.0);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {TILE} {TILE}\" width=\"{TILE}\" height=\"{TILE}\" id=\"{id_prefix}\">\
         <rect width=\"{TILE}\" height=\"{TILE}\" rx=\"44\" fill=\"{}\"/><g transform=\"translate(2 -12)\">",
        palette.background,
    );
    tail(&mut svg, &shape, &palette);
    legs(&mut svg, &shape, &palette);
    body(&mut svg, &shape, &palette, id_prefix);
    wing(&mut svg, &shape, &palette);
    crest(&mut svg, &shape, &palette, head);
    beak(&mut svg, &shape, &palette, head);
    let _ = write!(
        svg,
        "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\"/>",
        head.0, head.1, shape.head_radius, palette.head
    );
    eye(&mut svg, &shape, head);
    svg.push_str("</g></svg>");
    svg
}

/// A leaf pointing along +x from the origin, `length` long and `width` wide at its widest.
fn leaf(length: f32, width: f32) -> String {
    format!(
        "M 0 0 C {:.1} {:.1} {:.1} {:.1} {length:.1} 0 C {:.1} {:.1} {:.1} {:.1} 0 0 Z",
        length * 0.25,
        -width,
        length * 0.7,
        -width * 0.9,
        length * 0.7,
        width * 0.9,
        length * 0.25,
        width,
    )
}

fn placed_leaf(at: Point, angle_deg: f32, length: f32, width: f32, fill: &str) -> String {
    format!(
        "<path transform=\"translate({:.1} {:.1}) rotate({angle_deg:.1})\" fill=\"{fill}\" d=\"{}\"/>",
        at.0,
        at.1,
        leaf(length, width)
    )
}

fn tail(svg: &mut String, shape: &Shape, palette: &Palette) {
    let root = shape.on_body(-0.55, 0.1);
    let center = shape.rear_angle_deg() + shape.tail_angle_deg;
    let count = shape.tail_feathers;
    for index in 0..count {
        let offset = index as f32 - (count as f32 - 1.0) / 2.0;
        let angle = center + offset * 11.0;
        let length = shape.tail_length * (1.0 - 0.08 * offset.abs());
        svg.push_str(&placed_leaf(root, angle, length, 11.0, &palette.tail));
    }
}

fn legs(svg: &mut String, shape: &Shape, palette: &Palette) {
    let ground = BODY_CENTER.1 + shape.body_ry + 22.0;
    for x in [BODY_CENTER.0 - 4.0, BODY_CENTER.0 + 14.0] {
        let _ = write!(
            svg,
            "<path fill=\"none\" stroke=\"{}\" stroke-width=\"3\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"M {x:.1} {:.1} L {x:.1} {ground:.1} M {:.1} {ground:.1} L {:.1} {ground:.1}\"/>",
            palette.legs,
            BODY_CENTER.1,
            x - 5.0,
            x + 7.0,
        );
    }
}

fn body(svg: &mut String, shape: &Shape, palette: &Palette, id_prefix: &str) {
    let (cx, cy) = BODY_CENTER;
    let clip = format!("{id_prefix}-body");
    let ellipse = format!(
        "<ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"{:.1}\" ry=\"{:.1}\"",
        shape.body_rx, shape.body_ry
    );
    let _ = write!(
        svg,
        "<g transform=\"rotate({:.1} {cx} {cy})\"><clipPath id=\"{clip}\">{ellipse}/></clipPath>{ellipse} fill=\"{}\"/>",
        shape.tilt_deg, palette.body
    );
    if shape.belly > 0.0 {
        let _ = write!(
            svg,
            "<ellipse clip-path=\"url(#{clip})\" cx=\"{:.1}\" cy=\"{:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" fill=\"{}\"/>",
            cx + shape.body_rx * 0.1,
            cy + shape.body_ry * 0.95,
            shape.body_rx * 0.95,
            shape.body_ry * 0.75 * shape.belly,
            palette.belly,
        );
    }
    svg.push_str("</g>");
}

fn wing(svg: &mut String, shape: &Shape, palette: &Palette) {
    let shoulder = shape.on_body(0.1, -0.2);
    let angle = shape.rear_angle_deg() + 4.0;
    svg.push_str(&placed_leaf(
        shoulder,
        angle,
        shape.wing_length,
        15.0,
        &palette.wing,
    ));
    svg.push_str(&format!(
        "<g opacity=\"0.35\">{}</g>",
        placed_leaf(
            shoulder.offset(2.0, -1.0),
            angle - 2.0,
            shape.wing_length * 0.55,
            9.0,
            &palette.belly
        )
    ));
}

fn crest(svg: &mut String, shape: &Shape, palette: &Palette, head: Point) {
    for index in 0..shape.crest {
        let angle = -100.0 - 22.0 * index as f32;
        let (sin, cos) = angle.to_radians().sin_cos();
        let start = head.offset(cos * shape.head_radius * 0.8, sin * shape.head_radius * 0.8);
        svg.push_str(&placed_leaf(start, angle, 20.0, 5.0, &palette.head));
    }
}

fn beak(svg: &mut String, shape: &Shape, palette: &Palette, head: Point) {
    let r = shape.head_radius;
    let top = head.offset(r * 0.55, -r * 0.40);
    let bottom = head.offset(r * 0.70, r * 0.36);
    let tip = head.offset(r + shape.beak_length, r * 0.12);
    let _ = write!(
        svg,
        "<path fill=\"{0}\" stroke=\"{0}\" stroke-width=\"2\" stroke-linejoin=\"round\" d=\"M {1:.1} {2:.1} L {3:.1} {4:.1} L {5:.1} {6:.1} Z\"/>",
        palette.beak, top.0, top.1, tip.0, tip.1, bottom.0, bottom.1,
    );
}

fn eye(svg: &mut String, shape: &Shape, head: Point) {
    let r = shape.head_radius;
    let center = head.offset(r * 0.35, -r * 0.15);
    let _ = write!(
        svg,
        "<circle cx=\"{0:.1}\" cy=\"{1:.1}\" r=\"{2:.1}\" fill=\"#fff\"/>\
         <circle cx=\"{3:.1}\" cy=\"{1:.1}\" r=\"{4:.1}\" fill=\"{EYE_COLOR}\"/>",
        center.0,
        center.1,
        r * 0.27,
        center.0 + r * 0.05,
        r * 0.16,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_part_is_readable_for_many_seeds() {
        let mut worst = f32::MAX;
        for seed in 0..20_000 {
            let c = colors(&AvatarSpec::from_seed(seed));
            let bg = c.background.luminance();
            let body = c.body.luminance();
            for (name, part, min) in [
                ("body", c.body, AGAINST_BACKGROUND),
                ("head", c.head, AGAINST_BACKGROUND),
                ("wing", c.wing, AGAINST_BACKGROUND),
                ("tail", c.tail, AGAINST_BACKGROUND),
            ] {
                let ratio = contrast(part.luminance(), bg);
                worst = worst.min(ratio);
                assert!(
                    ratio >= min - 0.01,
                    "seed {seed}: {name} vs background {ratio:.2}"
                );
            }
            assert!(
                contrast(c.head.luminance(), body) >= 1.59,
                "seed {seed}: head vs body"
            );
            assert!(
                contrast(c.wing.luminance(), body) >= 1.79,
                "seed {seed}: wing vs body"
            );
            assert!(
                contrast(c.belly.luminance(), body) >= 1.79,
                "seed {seed}: belly vs body"
            );
            assert!(
                contrast(luminance(c.beak), c.head.luminance()) >= 1.5,
                "seed {seed}: beak vs head"
            );
            assert!(
                contrast(luminance(c.legs), bg) >= AGAINST_BACKGROUND,
                "seed {seed}: legs"
            );
        }
        assert!(worst >= 2.99);
    }
}
