//! Writes an SVG contact sheet: `cargo run --example gallery -- <out.svg> [count]`.
use zorvinkel_bird_avatars::{AvatarSpec, render_svg};

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("usage: gallery <out.svg> [count]");
    let count: u64 = args.next().map_or(36, |c| c.parse().expect("count"));
    let columns = count.isqrt() + u64::from(count.isqrt().pow(2) < count);
    let rows = count.div_ceil(columns);
    let mut sheet = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\"><rect width=\"100%\" height=\"100%\" fill=\"#fff\"/>",
        columns * 210 + 10,
        rows * 210 + 10,
        columns * 210 + 10,
        rows * 210 + 10
    );
    for seed in 0..count {
        let spec = AvatarSpec::from_seed(seed);
        let inner = render_svg(&spec, &format!("a{seed}"));
        let (x, y) = (10 + (seed % columns) * 210, 10 + (seed / columns) * 210);
        sheet.push_str(&format!("<g transform=\"translate({x} {y})\">{inner}</g>"));
    }
    sheet.push_str("</svg>");
    std::fs::write(out, sheet).expect("write");
}
