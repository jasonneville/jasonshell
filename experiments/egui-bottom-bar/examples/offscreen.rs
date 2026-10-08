//! Offscreen CPU raster of the real egui mesh/atlas. Not a desktop/GPU screenshot.
use egui::epaint::Primitive;
use egui::{Color32, Context, ImageData, RawInput, Rect, TextureId, pos2, vec2};
use egui_bottom_bar::fixture::{FixtureApp, Input, TaskWindow};
use std::{collections::HashMap, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    let width: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1280);
    let scale: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let theme = args.get(3).map_or("base-dark", String::as_str);
    let count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(7);
    let output = args.get(5).map_or("offscreen.png", String::as_str);
    let showcase = args.get(6).map(String::as_str);
    if !(320..=1920).contains(&width) || !(1.0..=2.0).contains(&scale) || count > 1000 {
        return Err("invalid evidence dimensions/count".into());
    }
    let ctx = Context::default();
    ctx.enable_accesskit();
    ctx.set_pixels_per_point(scale);
    let mut app = if showcase.is_some() {
        FixtureApp::showcase()
    } else {
        FixtureApp::default()
    };
    if !app.set_demo_theme(theme) {
        return Err("unknown theme".into());
    }
    if showcase.is_none() {
        app.input(Input::Snapshot {
            sequence: 1,
            windows: (0..count)
                .map(|i| TaskWindow {
                    id: format!("w{i}"),
                    identity: 1,
                    title: format!("Document {i}"),
                    process: "Editor".into(),
                    pid: Some(100),
                    active: i == 0,
                    minimized: i == 1,
                    attention: i == 2,
                    busy: false,
                    toasts: if i == 2 { 7 } else { 0 },
                })
                .collect(),
        });
        if count > 1 {
            app.input(Input::OpenGallery("editor".into()));
        }
    }
    match showcase {
        Some("gallery") => app.input(Input::OpenGallery("editor".into())),
        Some("processes") => app.input(Input::Processes),
        Some("bar" | "preview") | None => {}
        _ => return Err("unknown showcase evidence mode".into()),
    }
    let height = 720u32;
    let mut textures: HashMap<TextureId, egui::ColorImage> = HashMap::new();
    let mut last = None;
    for frame in 0..4 {
        let full = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(width as f32, height as f32),
                )),
                time: Some(
                    frame as f64
                        * if showcase == Some("preview") {
                            0.2
                        } else {
                            0.016
                        },
                ),
                ..Default::default()
            },
            |ctx| app.draw(ctx),
        );
        for (id, delta) in &full.textures_delta.set {
            let ImageData::Color(image) = &delta.image;
            if let Some([x, y]) = delta.pos {
                if let Some(atlas) = textures.get_mut(id) {
                    for row in 0..image.height() {
                        let start = (y + row) * atlas.width() + x;
                        atlas.pixels[start..start + image.width()].copy_from_slice(
                            &image.pixels[row * image.width()..(row + 1) * image.width()],
                        );
                    }
                }
            } else {
                textures.insert(*id, (**image).clone());
            }
        }
        last = Some(full);
        if frame == 0 && showcase == Some("preview") {
            app.input(Input::HoverTask(Some("demo-browser".into())));
        }
    }
    let Some(full) = last else {
        return Err("no frames".into());
    };
    let pixels_per_point = full.pixels_per_point;
    let w = (width as f32 * pixels_per_point).round() as u32;
    let h = (height as f32 * pixels_per_point).round() as u32;
    let mut image = image::RgbaImage::from_pixel(w, h, image::Rgba([0, 0, 0, 255]));
    for primitive in ctx.tessellate(full.shapes, pixels_per_point) {
        let Primitive::Mesh(mesh) = primitive.primitive else {
            continue;
        };
        let Some(texture) = textures.get(&mesh.texture_id) else {
            continue;
        };
        for triangle in mesh.indices.chunks_exact(3) {
            let v = [
                mesh.vertices[triangle[0] as usize],
                mesh.vertices[triangle[1] as usize],
                mesh.vertices[triangle[2] as usize],
            ];
            let p = v.map(|v| v.pos * pixels_per_point);
            let clip = primitive.clip_rect * pixels_per_point;
            let minx = p
                .iter()
                .map(|p| p.x)
                .fold(f32::INFINITY, f32::min)
                .max(clip.left())
                .max(0.0)
                .floor() as u32;
            let maxx = p
                .iter()
                .map(|p| p.x)
                .fold(0.0, f32::max)
                .min(clip.right())
                .min(w as f32)
                .ceil() as u32;
            let miny = p
                .iter()
                .map(|p| p.y)
                .fold(f32::INFINITY, f32::min)
                .max(clip.top())
                .max(0.0)
                .floor() as u32;
            let maxy = p
                .iter()
                .map(|p| p.y)
                .fold(0.0, f32::max)
                .min(clip.bottom())
                .min(h as f32)
                .ceil() as u32;
            let area = edge(p[0], p[1], p[2]);
            if area.abs() < 0.001 {
                continue;
            }
            for y in miny..maxy {
                for x in minx..maxx {
                    let at = pos2(x as f32 + 0.5, y as f32 + 0.5);
                    let b = [
                        edge(p[1], p[2], at) / area,
                        edge(p[2], p[0], at) / area,
                        edge(p[0], p[1], at) / area,
                    ];
                    if b.iter().any(|b| *b < 0.0) {
                        continue;
                    }
                    let uv = v[0].uv.to_vec2() * b[0]
                        + v[1].uv.to_vec2() * b[1]
                        + v[2].uv.to_vec2() * b[2];
                    let tx = (uv.x * texture.width() as f32)
                        .floor()
                        .clamp(0.0, (texture.width() - 1) as f32)
                        as usize;
                    let ty = (uv.y * texture.height() as f32)
                        .floor()
                        .clamp(0.0, (texture.height() - 1) as f32)
                        as usize;
                    let tex = texture.pixels[ty * texture.width() + tx];
                    let vertex = interpolate(v.map(|v| v.color), b);
                    let alpha = f32::from(tex.a()) * vertex[3] / 65025.0;
                    let old = image.get_pixel(x, y).0;
                    let mut out = [0, 0, 0, 255];
                    for c in 0..3 {
                        let source = f32::from(tex[c]) * vertex[c] / 255.0;
                        out[c] =
                            (source + f32::from(old[c]) * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
                    }
                    image.put_pixel(x, y, image::Rgba(out));
                }
            }
        }
    }
    image.save(output)?;
    let count: usize = app.view().groups.iter().map(|g| g.members.len()).sum();
    println!(
        "Offscreen CPU mesh raster: {output}; {width}×720 logical, {pixels_per_point}×; theme={theme}; windows={count}; desktop/GPU/Narrator unobserved"
    );
    Ok(())
}
fn edge(a: egui::Pos2, b: egui::Pos2, p: egui::Pos2) -> f32 {
    (p.x - a.x) * (b.y - a.y) - (p.y - a.y) * (b.x - a.x)
}
fn interpolate(colors: [Color32; 3], b: [f32; 3]) -> [f32; 4] {
    std::array::from_fn(|i| {
        f32::from(colors[0][i]) * b[0]
            + f32::from(colors[1][i]) * b[1]
            + f32::from(colors[2][i]) * b[2]
    })
}
