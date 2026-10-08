//! Frozen source palette from src/app.css (2026-10-07), not themes.ts swatches.
use egui::Color32;

// id, background, text, strong, muted RGBA, raised RGB, sunken RGB, accent
type Palette = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    [u8; 4],
    [u8; 3],
    [u8; 3],
    &'static str,
);
const PALETTES: &[Palette] = &[
    (
        "base-dark",
        "090c13",
        "eef3ff",
        "fbfdff",
        [222, 230, 248, 179],
        [24, 30, 43],
        [7, 10, 17],
        "c0d8e7",
    ),
    (
        "base-light",
        "eef2f8",
        "1e293b",
        "0f172a",
        [45, 55, 72, 184],
        [255, 255, 255],
        [226, 232, 240],
        "2563eb",
    ),
    (
        "monokai",
        "1f1f1c",
        "f8f8f2",
        "fffef6",
        [248, 248, 242, 173],
        [52, 53, 45],
        [25, 26, 22],
        "a6e22e",
    ),
    (
        "atom-one-dark",
        "1d2028",
        "dbe2f1",
        "f1f5ff",
        [171, 178, 191, 194],
        [42, 47, 56],
        [24, 27, 32],
        "61afef",
    ),
    (
        "atom-one-light",
        "eef0f4",
        "383a42",
        "20232a",
        [56, 58, 66, 184],
        [255, 255, 255],
        [228, 231, 237],
        "4078f2",
    ),
    (
        "nord",
        "242933",
        "e5e9f0",
        "ffffff",
        [216, 222, 233, 184],
        [59, 66, 82],
        [36, 41, 51],
        "88c0d0",
    ),
    (
        "dracula",
        "21222c",
        "f8f8f2",
        "ffffff",
        [248, 248, 242, 179],
        [52, 55, 70],
        [30, 31, 41],
        "bd93f9",
    ),
    (
        "solarized-dark",
        "00212b",
        "eee8d5",
        "fdf6e3",
        [147, 161, 161, 209],
        [7, 54, 66],
        [0, 32, 41],
        "268bd2",
    ),
    (
        "solarized-light",
        "eee8d5",
        "586e75",
        "073642",
        [88, 110, 117, 189],
        [255, 251, 235],
        [238, 232, 213],
        "268bd2",
    ),
    (
        "github-dark",
        "0d1117",
        "e6edf3",
        "f0f6fc",
        [125, 133, 144, 230],
        [22, 27, 34],
        [1, 4, 9],
        "2f81f7",
    ),
    (
        "github-light",
        "f6f8fa",
        "24292f",
        "1f2328",
        [87, 96, 106, 199],
        [255, 255, 255],
        [234, 238, 242],
        "0969da",
    ),
    (
        "gruvbox-dark",
        "282018",
        "ebdbb2",
        "fbf1c7",
        [213, 196, 161, 194],
        [60, 56, 54],
        [29, 26, 24],
        "fabd2f",
    ),
    (
        "gruvbox-light",
        "f2e5bc",
        "3c3836",
        "282828",
        [80, 73, 69, 191],
        [255, 248, 214],
        [235, 219, 178],
        "b57614",
    ),
    (
        "tokyo-night",
        "151826",
        "c0caf5",
        "e6edff",
        [169, 177, 214, 194],
        [36, 40, 59],
        [17, 19, 29],
        "7aa2f7",
    ),
    (
        "catppuccin-mocha",
        "181825",
        "cdd6f4",
        "f5e0dc",
        [186, 194, 222, 194],
        [49, 50, 68],
        [17, 17, 27],
        "cba6f7",
    ),
    (
        "ayu-dark",
        "0f1419",
        "e6e1cf",
        "fff8df",
        [191, 184, 162, 189],
        [25, 31, 38],
        [10, 14, 18],
        "ffcc66",
    ),
    (
        "night-owl",
        "011627",
        "d6deeb",
        "ffffff",
        [147, 164, 191, 194],
        [17, 36, 59],
        [7, 17, 31],
        "82aaff",
    ),
    (
        "palenight",
        "171428",
        "d9cbff",
        "fff8ff",
        [194, 178, 232, 194],
        [45, 37, 71],
        [18, 15, 31],
        "c792ea",
    ),
    (
        "one-dark-pro",
        "1b1f2a",
        "cfd7e6",
        "ffffff",
        [162, 172, 189, 194],
        [44, 50, 66],
        [21, 24, 33],
        "61afef",
    ),
    (
        "material-ocean",
        "0f1e27",
        "d7e3eb",
        "f9fdff",
        [171, 188, 197, 194],
        [32, 55, 67],
        [14, 26, 33],
        "80cbc4",
    ),
    (
        "everforest-dark",
        "1e2321",
        "d8e0d0",
        "f3f5ed",
        [181, 190, 171, 194],
        [52, 61, 54],
        [30, 35, 33],
        "a7c080",
    ),
    (
        "rose-pine-moon",
        "1f1d2e",
        "e1dfe9",
        "fffaf5",
        [188, 181, 205, 194],
        [57, 53, 82],
        [29, 27, 40],
        "ebbcba",
    ),
    (
        "oceanic-next",
        "12202a",
        "d7e5ef",
        "f7fcff",
        [172, 186, 199, 194],
        [36, 51, 64],
        [16, 27, 35],
        "74b9ff",
    ),
    (
        "shades-of-purple",
        "1e152a",
        "d6cff8",
        "ffffff",
        [181, 173, 229, 194],
        [59, 45, 85],
        [26, 19, 37],
        "a599e9",
    ),
    (
        "kanagawa-wave",
        "1f1f28",
        "dcd7ba",
        "f6f0da",
        [197, 192, 174, 194],
        [48, 46, 68],
        [22, 22, 30],
        "7e9cd8",
    ),
    (
        "vitesse-dark",
        "121212",
        "dbd7ca",
        "f7f3e8",
        [221, 216, 202, 194],
        [36, 36, 36],
        [14, 14, 14],
        "e2b776",
    ),
    (
        "aura-dark",
        "15141b",
        "edecee",
        "ffffff",
        [196, 193, 203, 194],
        [43, 38, 57],
        [18, 16, 24],
        "a277ff",
    ),
    (
        "horizon",
        "1c1e26",
        "d5d8da",
        "ffffff",
        [188, 191, 194, 194],
        [46, 49, 62],
        [22, 24, 31],
        "e95678",
    ),
    (
        "moonlight-ii",
        "222436",
        "c8d3f5",
        "f5f7ff",
        [176, 188, 225, 194],
        [46, 54, 82],
        [25, 28, 44],
        "82aaff",
    ),
    (
        "synthwave-84",
        "262335",
        "f9f7f3",
        "ffffff",
        [208, 193, 232, 194],
        [55, 49, 84],
        [28, 23, 45],
        "ff7edb",
    ),
    (
        "github-dark-dimmed",
        "22272e",
        "adbac7",
        "f0f6fc",
        [139, 148, 158, 209],
        [45, 51, 59],
        [28, 33, 40],
        "539bf5",
    ),
    (
        "tomorrow-night",
        "1d1f21",
        "c5c8c6",
        "ffffff",
        [181, 184, 181, 194],
        [48, 50, 54],
        [24, 26, 28],
        "81a2be",
    ),
    (
        "noctis",
        "1f2233",
        "dfe3f4",
        "ffffff",
        [196, 202, 225, 194],
        [53, 58, 87],
        [24, 28, 41],
        "7f9cff",
    ),
    (
        "andromeda",
        "23262e",
        "e5e5e5",
        "ffffff",
        [189, 197, 204, 194],
        [46, 49, 59],
        [24, 26, 32],
        "04d9c4",
    ),
];
pub(crate) fn names() -> impl Iterator<Item = &'static str> {
    PALETTES.iter().map(|p| p.0)
}
#[derive(Clone, Copy)]
pub(crate) struct Theme {
    pub bg: Color32,
    pub text: Color32,
    pub strong: Color32,
    pub muted: Color32,
    pub bar: Color32,
    pub sunken: Color32,
    pub accent: Color32,
    pub idle: Color32,
    pub active: Color32,
    pub hover: Color32,
    pub border: Color32,
    pub overlay: Color32,
}
fn hex(s: &str) -> Color32 {
    let n = u32::from_str_radix(s, 16).unwrap_or(0);
    Color32::from_rgb((n >> 16) as u8, (n >> 8) as u8, n as u8)
}
fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}
pub(crate) fn mix(a: Color32, b: Color32, amount: f32) -> Color32 {
    Color32::from_rgb(
        (f32::from(a.r()) * (1.0 - amount) + f32::from(b.r()) * amount).round() as u8,
        (f32::from(a.g()) * (1.0 - amount) + f32::from(b.g()) * amount).round() as u8,
        (f32::from(a.b()) * (1.0 - amount) + f32::from(b.b()) * amount).round() as u8,
    )
}
impl Theme {
    pub(crate) fn is_light(index: usize) -> bool {
        matches!(index, 1 | 4 | 8 | 10 | 12)
    }
    pub(crate) fn get(index: usize) -> Self {
        let p = &PALETTES[index.min(PALETTES.len() - 1)];
        let bg = hex(p.1);
        let text = hex(p.2);
        let accent = hex(p.7);
        let light = matches!(index, 1 | 4 | 8 | 10 | 12);
        let bar = mix(bg, rgb(p.5), if light { 0.99 } else { 0.985 });
        let hover_rgb = if index == 1 || index >= 16 {
            accent
        } else {
            Color32::from_rgb(138, 167, 255)
        };
        let border_rgb = match index {
            1 => Color32::from_rgb(30, 41, 59),
            9 | 18 | 30 => Color32::from_rgb(139, 148, 158),
            10 => Color32::from_rgb(31, 35, 40),
            16 => Color32::from_rgb(125, 146, 173),
            17 => Color32::from_rgb(176, 156, 224),
            19 | 22 => Color32::from_rgb(149, 179, 190),
            20 | 23 | 24 | 25 => accent,
            21 => Color32::from_rgb(188, 181, 205),
            26 => Color32::from_rgb(182, 160, 237),
            27 => Color32::from_rgb(185, 188, 196),
            28 => Color32::from_rgb(176, 188, 225),
            29 => Color32::from_rgb(213, 181, 255),
            31 => Color32::from_rgb(181, 184, 181),
            32 => Color32::from_rgb(174, 183, 255),
            33 => Color32::from_rgb(172, 182, 190),
            _ => Color32::from_rgb(227, 235, 255),
        };
        let sunken_alpha = match index {
            0 | 2 => 0.82,
            1 => 0.74,
            3 | 5 | 11 => 0.84,
            4 | 8 | 10 | 12 => 0.78,
            _ => 0.86,
        };
        let overlay = if index == 1 {
            Color32::from_rgba_unmultiplied(15, 23, 42, 11)
        } else {
            Color32::from_white_alpha(if index >= 16 { 13 } else { 14 })
        };
        Self {
            bg,
            text,
            strong: hex(p.3),
            muted: mix(
                bar,
                rgb([p.4[0], p.4[1], p.4[2]]),
                f32::from(p.4[3]) / 255.0,
            ),
            bar,
            sunken: mix(bar, rgb(p.6), sunken_alpha),
            accent,
            idle: mix(bar, text, 0.07),
            active: mix(bar, accent, 0.24),
            hover: mix(bar, hover_rgb, if index == 1 { 0.10 } else { 0.16 }),
            border: mix(
                bar,
                border_rgb,
                if index == 1 || index == 10 {
                    0.16
                } else if index == 9 {
                    0.22
                } else if index >= 16 {
                    0.18
                } else {
                    0.14
                },
            ),
            overlay,
        }
    }
}
