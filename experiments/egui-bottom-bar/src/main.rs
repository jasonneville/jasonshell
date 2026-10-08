use egui_bottom_bar::fixture::FixtureApp;

fn main() -> eframe::Result {
    let smoke = std::env::args().any(|arg| arg == "--smoke-first-frame");
    eframe::run_native(
        "JasonShell bottom bar — Simulated fixture",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 720.0])
                .with_min_inner_size([320.0, 360.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            if smoke {
                if cc.wgpu_render_state.is_none() {
                    return Err("native smoke requires initialized wgpu render state".into());
                }
                Ok(Box::new(FirstFrameSmoke {
                    fixture: FixtureApp::showcase(),
                    frames: 0,
                }))
            } else {
                Ok(Box::new(FixtureApp::showcase()))
            }
        }),
    )
}

// Explicit, finite ordinary-window verification; never used by normal launches.
// The marker comes from native App updates after renderer initialization, not
// from a headless Context or merely surviving startup. No shell services exist.
struct FirstFrameSmoke {
    fixture: FixtureApp,
    frames: u8,
}
impl eframe::App for FirstFrameSmoke {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.fixture.draw(ctx);
        self.frames = self.frames.saturating_add(1);
        if self.frames == 3 {
            println!(
                "Simulated fixture native smoke: initialized wgpu render state; 3 native draw updates; populated showcase {} task groups; closing ordinary fixture",
                self.fixture.view().groups.len()
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.frames < 3 {
            ctx.request_repaint();
        }
    }
}
