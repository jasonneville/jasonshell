//! egui presentation and event translation, shared by native and RawInput frames.
use crate::{
    fixture::*,
    theme::{self, Theme},
};
use egui::{
    Align2, Color32, FontId, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo,
    WidgetType, pos2, vec2,
};
use std::time::Duration;

// Only engaged overflow labels retain state. LRU cap covers simultaneous focus/
// hover without retaining the 1000-window fixture's entire label inventory.
const DRAW_TICKER_LIMIT: usize = 64;
pub(crate) struct DrawTicker {
    signature: Id,
    overflow: f32,
    start: u64,
    seen: u64,
}

impl eframe::App for FixtureApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.draw(ctx);
    }
}

impl FixtureApp {
    pub(crate) fn draw_ui(&mut self, ctx: &egui::Context) {
        self.rects.clear();
        self.draw_frame = self.draw_frame.saturating_add(1);
        self.draw_animation_pending = false;
        if self.view.disposed {
            return;
        }
        let time = ctx.input(|i| i.time);
        self.tick((time.max(0.0) * 1000.0) as u64);
        let bounds = ctx.screen_rect();
        self.input(Input::Viewport {
            width: bounds.width().min(584.0),
            height: (bounds.height() - self.view.height - 16.0).min(328.0),
        });
        if !ctx.input(|i| i.focused) {
            self.input(Input::Blur);
        }
        let t = Theme::get(self.theme);
        let mut style = (*ctx.style()).clone();
        style.visuals = if Theme::is_light(self.theme) {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        style.visuals.override_text_color = Some(t.text);
        style.visuals.panel_fill = t.bg;
        style.visuals.window_fill = t.bar;
        style.visuals.selection.stroke = Stroke::new(2.0_f32, t.accent);
        style.animation_time = 0.0;
        ctx.set_style(style);
        if let Some(id) = self.restore_focus.take() {
            ctx.memory_mut(|m| m.request_focus(id));
        }
        self.keyboard(ctx);
        if ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
            self.input(Input::DragEnd);
        } else if !ctx.input(|i| i.pointer.primary_down()) {
            self.input(Input::DragCancel);
        }
        self.bar(ctx, t, bounds);
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.bg).inner_margin(24)).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Bottom bar / safe interactive fixture");
                ui.label(egui::RichText::new("SIMULATED • No live tasks, pins, launches, or process actions").color(t.accent));
                ui.add_space(16.0);
                ui.label("Demo controls — outside the bar; all changes last only this session.");
                let old = self.scenario;
                egui::ComboBox::from_label("Scenario").selected_text(scenario_name(self.scenario)).show_ui(ui, |ui| {
                    for i in 0..if self.showcase_enabled {7} else {6} { ui.selectable_value(&mut self.scenario, i, scenario_name(i)); }
                });
                if old != self.scenario { if self.scenario == 6 { *self = Self::showcase(); } else { self.load_scenario([0,1,2,3,7,1000][self.scenario]); } }
                egui::ComboBox::from_label("Source theme").selected_text(theme::names().nth(self.theme).unwrap_or("base-dark")).show_ui(ui, |ui| {
                    for (i, name) in theme::names().enumerate() { ui.selectable_value(&mut self.theme, i, name); }
                });
                egui::ComboBox::from_label("Next simulated launch outcome").selected_text(format!("{:?}", self.launch_outcome)).show_ui(ui, |ui| {
                    for outcome in [LaunchOutcome::Success, LaunchOutcome::Failed, LaunchOutcome::Canceled, LaunchOutcome::UnelevatedFallback] { ui.selectable_value(&mut self.launch_outcome, outcome, format!("{outcome:?}")); }
                });
                let mut locked = self.locked; if ui.checkbox(&mut locked, "Lock bar height").changed() { self.input(Input::HeightLocked(locked)); }
                let mut reduced = self.reduced; if ui.checkbox(&mut reduced, "Reduced motion (static titles)").changed() { self.input(Input::ReducedMotion(reduced)); }
                ui.label(format!("Height {:.1} logical px • image cache 0 / {} bytes • no native thumbnail destinations", self.view.height, CACHE_LIMIT_BYTES));
                ui.add_space(12.0);
                ui.label("Tab / Shift+Tab: controls. Arrows / Home / End: open list or gallery. Enter: selected action. Shift+F10: menu. Escape: dismiss or cancel gesture.");
                ui.label("Drag task tiles or Alt+Left/Right on a focused tile to reorder. Unlock height, then drag the top edge (or focus it and use Up/Down/Home/End); Escape cancels. Demo illustrations are generated, never captured/live.");
                if let Some(diagnostic) = &self.view.diagnostic { ui.separator(); ui.label(egui::RichText::new(diagnostic).color(t.strong)); }
                ui.add_space(20.0); ui.separator(); ui.heading("Simulated action ledger");
                if self.ledger().is_empty() { ui.label("No actions recorded."); }
                for entry in self.ledger().iter().rev().take(12) { ui.monospace(format!("Simulated {:?} · {}{}", entry.action, entry.target.as_deref().unwrap_or("fixture height"), entry.elevated.map_or(String::new(), |e| format!(" · elevated: {e}")))); }
                ui.label("Ledger retains latest 256 entries. Close records intent only; tasks remain authoritative.");
            });
        });
        self.quick_launch(ctx, t, bounds);
        self.gallery(ctx, t, bounds);
        self.preview(ctx, t, bounds);
        self.menu_ui(ctx, t, bounds);
        self.process_popup(ctx, t, bounds);
        self.draw_tickers
            .retain(|_, ticker| ticker.seen == self.draw_frame);
        self.view.animation_pending |= self.draw_animation_pending;
        if self.visible
            && let Some(deadline) = self.next_deadline()
        {
            ctx.request_repaint_after(Duration::from_millis(deadline.saturating_sub(self.now)));
        }
        if self.view.animation_pending {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
    fn keyboard(&mut self, ctx: &egui::Context) {
        if self.view.process_manager.is_some() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.input(Input::Key(Key::Escape));
            }
            return;
        }
        let overlay =
            self.view.gallery.is_some() || self.view.quick_launch.is_some() || self.view.menu_open;
        for (native, key) in [
            (egui::Key::Escape, Key::Escape),
            (egui::Key::ArrowUp, Key::Up),
            (egui::Key::ArrowDown, Key::Down),
            (egui::Key::ArrowLeft, Key::Left),
            (egui::Key::ArrowRight, Key::Right),
            (egui::Key::Home, Key::Home),
            (egui::Key::End, Key::End),
        ] {
            if (overlay || native == egui::Key::Escape)
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, native))
            {
                self.input(Input::Key(key));
            }
        }
        if overlay && ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::F10)) {
            self.input(Input::Key(Key::ContextMenu));
        }
        // Enter for Quick Launch is consumed here; gallery buttons own Enter/Space to
        // preserve the separate selected activation/Close focus pair.
        if self.view.quick_launch.is_some()
            && self.menu.is_none()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            self.input(Input::Key(Key::Enter));
        }
    }
    fn remember(&mut self, name: &str, response: &Response) {
        if response.interact_rect.contains_rect(response.rect)
            && response.rect.intersects(response.ctx.screen_rect())
        {
            self.rects.insert(name.into(), response.rect);
        }
    }
    // Keep the measured widget contract together: name, face, focus and theme.
    #[allow(clippy::too_many_arguments)]
    fn button(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        id: Id,
        name: &str,
        copy: &str,
        selected: bool,
        focusable: bool,
        t: Theme,
    ) -> Response {
        let sense = if focusable {
            Sense::click()
        } else {
            Sense::click().difference(Sense::FOCUSABLE)
        };
        let r = ui.interact(rect, id, sense);
        r.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), name));
        let fill = if r.hovered() {
            t.hover
        } else if selected {
            t.active
        } else {
            t.idle
        };
        ui.painter().rect_filled(rect, 2, fill);
        ui.painter().rect_stroke(
            rect,
            2,
            Stroke::new(
                if r.has_focus() { 2.0_f32 } else { 1.0_f32 },
                if r.has_focus() { t.accent } else { t.border },
            ),
            StrokeKind::Inside,
        );
        ui.painter().line_segment(
            [
                rect.left_top() + vec2(1.0, 1.0),
                rect.right_top() + vec2(-1.0, 1.0),
            ],
            Stroke::new(1.0_f32, t.overlay),
        );
        let font = FontId::proportional(10.0);
        let galley = ui.painter().layout_no_wrap(copy.into(), font, t.text);
        let label_rect = rect.shrink2(vec2(6.0, 2.0));
        let offset = if !copy.is_empty() {
            self.local_ticker_offset(
                r.id,
                copy,
                galley.size().x,
                label_rect.width(),
                r.hovered() || r.has_focus(),
            )
        } else {
            0.0
        };
        ui.painter()
            .with_clip_rect(label_rect.intersect(ui.clip_rect()))
            .galley(
                pos2(
                    label_rect.left() + offset,
                    label_rect.center().y - galley.size().y / 2.0,
                ),
                galley,
                t.text,
            );
        self.remember(name, &r);
        r
    }
    fn local_ticker_offset(
        &mut self,
        id: Id,
        text: &str,
        text_width: f32,
        viewport_width: f32,
        engaged: bool,
    ) -> f32 {
        let overflow = (text_width - viewport_width).max(0.0);
        if !engaged || !self.visible || self.reduced || !overflow.is_finite() || overflow <= 0.0 {
            self.draw_tickers.remove(&id);
            return 0.0;
        }
        if !self.draw_tickers.contains_key(&id)
            && self.draw_tickers.len() >= DRAW_TICKER_LIMIT
            && let Some(oldest) = self
                .draw_tickers
                .iter()
                .min_by_key(|(_, ticker)| ticker.seen)
                .map(|(id, _)| *id)
        {
            self.draw_tickers.remove(&oldest);
        }
        let signature = Id::new(text);
        let ticker = self.draw_tickers.entry(id).or_insert(DrawTicker {
            signature,
            overflow,
            start: self.now,
            seen: self.draw_frame,
        });
        if ticker.signature != signature || ticker.overflow != overflow {
            ticker.signature = signature;
            ticker.overflow = overflow;
            ticker.start = self.now;
        }
        ticker.seen = self.draw_frame;
        self.draw_animation_pending = true;
        let elapsed = self.now.saturating_sub(ticker.start) as f32;
        if elapsed < 600.0 {
            return 0.0;
        }
        let travel = overflow / 0.04;
        let phase = (elapsed - 600.0) % (travel * 2.0 + 5000.0);
        if phase < travel {
            -phase * 0.04
        } else if phase < travel + 2000.0 {
            -overflow
        } else if phase < 2.0 * travel + 2000.0 {
            -overflow + (phase - travel - 2000.0) * 0.04
        } else {
            0.0
        }
    }
    fn bar(&mut self, ctx: &egui::Context, t: Theme, bounds: Rect) {
        let mut task_hover = None;
        let mut group_hover = None;
        let mut centers = Vec::new();
        let mut drag_start = None;
        let mut keyboard_reorder = None;
        egui::TopBottomPanel::bottom("fixture-bar")
            .exact_height(self.view.height)
            .frame(egui::Frame::NONE.fill(t.bar))
            .show(ctx, |ui| {
                let bar = ui.max_rect();
                let h = bar.height() - 4.0;
                let quick_rect = Rect::from_min_size(bar.min + vec2(4.0, 2.0), vec2(88.0, h));
                let process_rect =
                    Rect::from_min_size(pos2(bar.right() - 28.8, bar.top() + 2.0), vec2(24.8, h));
                let q = self.button(
                    ui,
                    quick_rect,
                    Id::new("quick-launch-toggle"),
                    "Quick Launch",
                    "Quick Launch",
                    self.view.quick_launch.is_some(),
                    true,
                    t,
                );
                if q.is_pointer_button_down_on()
                    && ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
                {
                    self.input(Input::QuickLaunchButtonDown);
                }
                if q.clicked() {
                    self.origin_focus = Some(q.id);
                    self.input(Input::QuickLaunchButtonClick);
                }
                let p = self.button(
                    ui,
                    process_rect,
                    Id::new("processes"),
                    "Processes",
                    "P",
                    false,
                    true,
                    t,
                );
                if p.clicked() {
                    self.origin_focus = Some(p.id);
                    self.input(Input::Processes);
                }
                let strip = Rect::from_min_max(
                    pos2(95.0, bar.top() + 2.0),
                    pos2(process_rect.left() - 3.0, bar.bottom() - 2.0),
                );
                let groups = self.view.groups.clone();
                let n = groups.len();
                let slot = if n == 0 {
                    160.0
                } else {
                    ((strip.width() - 3.0 * (n.saturating_sub(1)) as f32) / n as f32)
                        .clamp(48.0, 160.0)
                };
                self.drag_slot = slot + 3.0;
                self.input(Input::StripMetrics {
                    available: strip.width(),
                    required: n as f32 * (slot + 3.0) - 3.0,
                });
                if n == 0 {
                    ui.painter().text(
                        strip.left_center() + vec2(4.0, 0.0),
                        Align2::LEFT_CENTER,
                        "No simulated windows",
                        FontId::proportional(10.0),
                        t.muted,
                    );
                }
                // Keyboard focus scrolls hidden slots without retaining a separate task source.
                let offset =
                    (self.task_focus as f32 * (slot + 3.0) + slot - strip.width()).max(0.0);
                let old_clip = ui.clip_rect();
                ui.set_clip_rect(strip.intersect(old_clip));
                for (i, g) in groups.iter().enumerate() {
                    let Some(w) = g
                        .members
                        .first()
                        .and_then(|id| self.windows.iter().find(|w| &w.id == id))
                        .cloned()
                    else {
                        continue;
                    };
                    let mut name = if g.capsule {
                        format!("{}, {} windows", w.process.trim(), g.members.len())
                    } else {
                        w.title.clone()
                    };
                    if g.active {
                        name.push_str(", active");
                    }
                    if g.minimized {
                        name.push_str(", minimized");
                    }
                    if g.attention {
                        name.push_str(", needs attention");
                    }
                    if g.toasts > 0 {
                        name.push_str(&format!(", {} toasts", g.toasts));
                    }
                    if g.busy {
                        name.push_str(", activity detected");
                    }
                    let rect = Rect::from_min_size(
                        pos2(strip.left() + i as f32 * (slot + 3.0) - offset, strip.top()),
                        vec2(slot, h),
                    );
                    let tile_id = if g.capsule {
                        Id::new(("task-group", &g.key, "tile"))
                    } else {
                        Id::new(("task", &w.id, w.identity, "tile"))
                    };
                    let r = self.button(ui, rect, tile_id, &name, "", g.active, true, t);
                    centers.push((g.key.clone(), r.rect.center().x));
                    let icon = Rect::from_min_size(
                        pos2(rect.left() + 6.0, rect.center().y - 5.9),
                        vec2(11.8, 11.8),
                    );
                    ui.painter().rect_stroke(
                        icon,
                        1,
                        Stroke::new(1.0_f32, t.muted),
                        StrokeKind::Inside,
                    );
                    if self.showcase_enabled {
                        ui.painter().text(
                            icon.center(),
                            Align2::CENTER_CENTER,
                            w.process.chars().next().unwrap_or('A'),
                            FontId::proportional(9.0),
                            t.accent,
                        );
                    }
                    let count_width = if g.capsule { 24.0 } else { 0.0 };
                    let label_rect = Rect::from_min_max(
                        pos2(icon.right() + 4.5, rect.top() + 3.0),
                        pos2(rect.right() - 6.0 - count_width, rect.bottom() - 3.0),
                    );
                    let title = if g.capsule {
                        w.process.trim()
                    } else {
                        &w.title
                    };
                    let galley = ui.painter().layout_no_wrap(
                        title.into(),
                        FontId::proportional(9.92),
                        t.text,
                    );
                    let ticker_offset = self.local_ticker_offset(
                        r.id,
                        title,
                        galley.size().x,
                        label_rect.width(),
                        r.hovered() || r.has_focus(),
                    );
                    ui.painter()
                        .with_clip_rect(label_rect.intersect(ui.clip_rect()))
                        .galley(
                            pos2(
                                label_rect.left() + ticker_offset,
                                label_rect.center().y - galley.size().y / 2.0,
                            ),
                            galley,
                            t.text,
                        );
                    if g.capsule {
                        ui.painter().text(
                            pos2(rect.right() - 15.0, rect.center().y),
                            Align2::CENTER_CENTER,
                            g.members.len().to_string(),
                            FontId::proportional(8.64),
                            t.text,
                        );
                    }
                    if r.has_focus() {
                        self.task_focus = i;
                        let reorder = ctx.input(|input| {
                            input.modifiers.alt
                                && (input.key_pressed(egui::Key::ArrowLeft)
                                    || input.key_pressed(egui::Key::ArrowRight))
                        });
                        if reorder {
                            let right = ctx.input(|input| input.key_pressed(egui::Key::ArrowRight));
                            keyboard_reorder = Some((g.key.clone(), right));
                        }
                        let next = ctx.input(|input| {
                            if input.modifiers.alt {
                                return None;
                            }
                            if input.key_pressed(egui::Key::Home) {
                                Some(0)
                            } else if input.key_pressed(egui::Key::End) {
                                Some(n - 1)
                            } else if input.key_pressed(egui::Key::ArrowRight) {
                                Some((i + 1).min(n - 1))
                            } else if input.key_pressed(egui::Key::ArrowLeft) {
                                Some(i.saturating_sub(1))
                            } else {
                                None
                            }
                        });
                        if let Some(next) = next {
                            self.task_focus = next;
                            self.focus_task_next = true;
                            ctx.request_repaint();
                        }
                        if ctx.input(|input| {
                            input.key_pressed(egui::Key::F10) && input.modifiers.shift
                        }) {
                            if g.capsule {
                                self.input(Input::GroupMenu(g.key.clone()));
                            } else {
                                self.input(Input::TaskMenu(w.id.clone()));
                            }
                        }
                    }
                    if self.focus_task_next && i == self.task_focus {
                        r.request_focus();
                        self.focus_task_next = false;
                    }
                    if r.hovered() {
                        if g.capsule {
                            group_hover = Some(g.key.clone());
                        } else {
                            task_hover = Some(w.id.clone());
                        }
                    }
                    if r.secondary_clicked() {
                        self.origin_focus = Some(r.id);
                        if g.capsule {
                            self.input(Input::GroupMenu(g.key.clone()));
                        } else {
                            self.input(Input::TaskMenu(w.id.clone()));
                        }
                    }
                    if r.is_pointer_button_down_on()
                        && ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
                        && let Some(pos) = ctx.input(|i| i.pointer.interact_pos())
                    {
                        drag_start = Some((g.key.clone(), pos.x));
                    }
                    if r.clicked() {
                        self.origin_focus = Some(r.id);
                        if ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
                            self.input(Input::TrailingClick(g.key.clone()));
                        } else {
                            self.activate_group(&g.key);
                        }
                    }
                    if g.attention || g.toasts > 0 {
                        ui.painter().line_segment(
                            [
                                rect.left_top() + vec2(1.0, 1.0),
                                rect.right_top() + vec2(-1.0, 1.0),
                            ],
                            Stroke::new(2.0_f32, Color32::from_rgb(255, 213, 79)),
                        );
                    }
                    if g.busy {
                        ui.painter().line_segment(
                            [rect.left_bottom(), rect.right_bottom()],
                            Stroke::new(2.0_f32, t.accent),
                        );
                    }
                    if g.capsule && g.members.len() <= 3 {
                        for mark in 0..g.members.len().min(3) {
                            let x = rect.right() - 6.0 - mark as f32 * 11.0;
                            ui.painter().line_segment(
                                [
                                    pos2(x - 8.0, rect.bottom() - 3.0),
                                    pos2(x, rect.bottom() - 3.0),
                                ],
                                Stroke::new(2.0_f32, if mark == 0 { t.accent } else { t.muted }),
                            );
                        }
                    } else if g.capsule {
                        for dot in 0..4 {
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    pos2(
                                        rect.right() - 10.0 + (dot % 2) as f32 * 4.0,
                                        rect.bottom() - 10.0 + (dot / 2) as f32 * 4.0,
                                    ),
                                    vec2(3.0, 3.0),
                                ),
                                0,
                                if dot == 0 { t.accent } else { t.muted },
                            );
                        }
                    }
                }
                ui.set_clip_rect(old_clip);
                if !self.locked {
                    let handle = Rect::from_min_size(bar.min, vec2(bar.width(), 5.0));
                    let r = ui.interact(handle, Id::new("Resize bar height"), Sense::drag());
                    r.widget_info(|| {
                        WidgetInfo::labeled(
                            WidgetType::Slider,
                            true,
                            "Resize bar height (24–120 logical pixels)",
                        )
                    });
                    self.remember("Resize bar height", &r);
                    if r.hovered() || r.has_focus() {
                        ui.painter().rect_filled(handle, 0, t.accent);
                    }
                    if r.has_focus() {
                        let height = ctx.input(|i| {
                            if i.key_pressed(egui::Key::ArrowUp) {
                                Some(self.view.height + 1.0)
                            } else if i.key_pressed(egui::Key::ArrowDown) {
                                Some(self.view.height - 1.0)
                            } else if i.key_pressed(egui::Key::Home) {
                                Some(24.0)
                            } else if i.key_pressed(egui::Key::End) {
                                Some(120.0)
                            } else {
                                None
                            }
                        });
                        if let Some(height) = height {
                            self.input(Input::ResizeStart { y: 0.0 });
                            self.input(Input::ResizeMove {
                                y: self.view.height - height,
                            });
                            self.input(Input::ResizeEnd);
                        }
                    }
                    if r.drag_started()
                        && let Some(pos) = r.interact_pointer_pos()
                    {
                        self.input(Input::ResizeStart {
                            y: pos.y - r.drag_delta().y,
                        });
                    }
                    if r.dragged()
                        && let Some(pos) = r.interact_pointer_pos()
                    {
                        self.input(Input::ResizeMove { y: pos.y });
                    }
                    if r.drag_stopped() {
                        self.input(Input::ResizeEnd);
                    }
                }
            });
        // Capture this frame's actual widget centers before admitting a gesture;
        // never recompute thresholds from reordered or resized later frames.
        self.measured_group_centers = centers;
        if let Some((group, x)) = drag_start {
            self.input(Input::DragStart { group, x });
        }
        if let Some((group, right)) = keyboard_reorder
            && let Some(source) = self
                .measured_group_centers
                .iter()
                .position(|(key, _)| key == &group)
        {
            let neighbor = if right {
                source
                    .checked_add(1)
                    .filter(|index| *index < self.measured_group_centers.len())
            } else {
                source.checked_sub(1)
            };
            if let Some(neighbor) = neighbor {
                let delta = self.measured_group_centers[neighbor].1
                    - self.measured_group_centers[source].1
                    + if right { 0.1 } else { -0.1 };
                self.reorder_from_keyboard(group, delta);
            }
        }
        if let Some(pos) = ctx.input(|i| i.pointer.interact_pos())
            && ctx.input(|i| i.pointer.primary_down())
        {
            self.input(Input::DragMove { x: pos.x });
        }
        if task_hover != self.hovered_task {
            self.hovered_task = task_hover.clone();
            self.input(Input::HoverTask(task_hover));
        }
        if group_hover != self.hovered_group {
            self.hovered_group = group_hover.clone();
            self.input(Input::HoverGroup(group_hover));
        }
        let _ = bounds;
    }
    fn quick_launch(&mut self, ctx: &egui::Context, t: Theme, bounds: Rect) {
        let Some(q) = self.view.quick_launch.clone() else {
            return;
        };
        let pos = pos2(
            4.0,
            (bounds.bottom() - self.view.height - 8.0 - 220.0).max(8.0),
        );
        egui::Area::new(Id::new("quick-launch-panel"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(t.bar)
                    .inner_margin(8)
                    .corner_radius(8)
                    .show(ui, |ui| {
                        ui.set_min_width(260.0_f32.min(bounds.width() - 32.0));
                        ui.label("Quick Launch · Simulated");
                        if q.pins.is_empty() {
                            ui.label("No simulated pins. Choose a demo scenario.");
                        }
                        for (i, id) in q.pins.iter().enumerate() {
                            let Some(pin) = self.pins.iter().find(|p| &p.id == id).cloned() else {
                                continue;
                            };
                            let (rect, _) = ui.allocate_exact_size(
                                vec2(ui.available_width(), 32.0),
                                Sense::hover(),
                            );
                            let r = self.button(
                                ui,
                                rect,
                                Id::new(("quick-launch-pin", &pin.id)),
                                &format!("Launch {}", pin.name),
                                &pin.name,
                                q.selected == Some(i),
                                true,
                                t,
                            );
                            if r.gained_focus()
                                && let Some(q) = &mut self.view.quick_launch
                            {
                                q.selected = Some(i);
                            }
                            if r.clicked() {
                                if let Some(q) = &mut self.view.quick_launch {
                                    q.selected = Some(i);
                                }
                                self.input(Input::Key(Key::Enter));
                            }
                            if r.secondary_clicked() {
                                if let Some(q) = &mut self.view.quick_launch {
                                    q.selected = Some(i);
                                }
                                self.input(Input::Key(Key::ContextMenu));
                            }
                        }
                        ui.label("Shift+F10: Run as administrator (simulated)");
                        if ui.button("Dismiss Quick Launch").clicked() {
                            self.input(Input::Key(Key::Escape));
                        }
                    });
            });
    }
    fn gallery(&mut self, ctx: &egui::Context, t: Theme, bounds: Rect) {
        let Some(g) = self.view.gallery.clone() else {
            self.last_gallery_selected = None;
            return;
        };
        let members = self.members();
        let changed_selection = self.last_gallery_selected != Some(g.selected);
        self.last_gallery_selected = Some(g.selected);
        // An index shift still updates scroll geometry, never action focus.
        let request_activation_focus = std::mem::take(&mut self.gallery_activation_focus_pending);
        let panel_width = g.columns as f32 * 180.0
            + (g.columns - 1) as f32 * 8.0
            + 16.0
            + if g.mode == LayoutMode::Matrix {
                12.0
            } else {
                0.0
            };
        let viewport_height = g.visible_rows as f32 * 136.0 + (g.visible_rows - 1) as f32 * 8.0;
        let height = viewport_height + 48.0;
        let pos = pos2(
            8.0,
            (bounds.bottom() - self.view.height - height - 8.0).max(8.0),
        );
        let response = egui::Area::new(Id::new("gallery-panel"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .default_size(vec2(panel_width.min(bounds.width() - 16.0), height))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(t.bar)
                    .inner_margin(8)
                    .corner_radius(8)
                    .show(ui, |ui| {
                        ui.set_width(panel_width.min(bounds.width() - 16.0) - 16.0);
                        ui.set_height(height - 16.0);
                        ui.horizontal(|ui| {
                            ui.set_min_height(24.0);
                            ui.label(format!(
                                "{} windows · {:?} · Simulated",
                                members.len(),
                                g.mode
                            ));
                            if ui.button("Dismiss gallery").clicked() {
                                self.input(Input::Key(Key::Escape));
                            }
                        });
                        ui.add_space(8.0);
                        let mut scroll = egui::ScrollArea::vertical()
                            .animated(false)
                            .auto_shrink([false, false]);
                        if changed_selection {
                            scroll = scroll.vertical_scroll_offset(
                                (g.selected / g.columns).saturating_sub(g.visible_rows - 1) as f32
                                    * 144.0,
                            );
                        }
                        scroll
                            .max_height(viewport_height.min(bounds.height() - 120.0))
                            .show(ui, |ui| {
                                for (row, items) in members.chunks(g.columns).enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                                        for (col, w) in items.iter().enumerate() {
                                            let index = row * g.columns + col;
                                            let card_width = ((panel_width
                                                .min(bounds.width() - 16.0)
                                                - 16.0
                                                - (g.columns - 1) as f32 * 8.0
                                                - if g.mode == LayoutMode::Matrix {
                                                    12.0
                                                } else {
                                                    0.0
                                                })
                                                / g.columns as f32)
                                                .min(180.0);
                                            let (card, _) = ui.allocate_exact_size(
                                                vec2(card_width, 136.0),
                                                Sense::hover(),
                                            );
                                            ui.painter().rect_filled(
                                                card,
                                                2,
                                                if w.active { t.active } else { t.idle },
                                            );
                                            let image = Rect::from_min_size(
                                                card.min + vec2(4.0, 4.0),
                                                vec2(card.width() - 8.0, 96.0),
                                            );
                                            ui.painter().rect_filled(image, 0, t.sunken);
                                            if self.showcase_enabled {
                                                self.preview_art(
                                                    ui,
                                                    image,
                                                    w,
                                                    t,
                                                    &format!("Simulated gallery artwork {}", w.id),
                                                );
                                            } else {
                                                ui.painter().text(
                                                    image.center(),
                                                    Align2::CENTER_CENTER,
                                                    "Preview unavailable",
                                                    FontId::proportional(10.0),
                                                    t.muted,
                                                );
                                            }
                                            let activate = Rect::from_min_max(
                                                card.min,
                                                pos2(card.right() - 32.0, card.bottom()),
                                            );
                                            let name = format!("Switch to {}", w.title);
                                            let r = ui.interact(
                                                activate,
                                                Id::new(("gallery", &w.id, w.identity, "activate")),
                                                if index == g.selected {
                                                    Sense::click()
                                                } else {
                                                    Sense::click().difference(Sense::FOCUSABLE)
                                                },
                                            );
                                            r.widget_info(|| {
                                                WidgetInfo::labeled(WidgetType::Button, true, &name)
                                            });
                                            self.remember(&name, &r);
                                            if index == g.selected && request_activation_focus {
                                                r.request_focus();
                                            }
                                            if r.has_focus() {
                                                ui.painter().rect_stroke(
                                                    card,
                                                    2,
                                                    Stroke::new(2.0_f32, t.accent),
                                                    StrokeKind::Inside,
                                                );
                                            }
                                            if r.clicked() {
                                                if let Some(g) = &mut self.view.gallery {
                                                    g.selected = index;
                                                }
                                                self.input(Input::GalleryAction(Action::Focus));
                                            }
                                            if r.secondary_clicked() {
                                                self.input(Input::TaskMenu(w.id.clone()));
                                            }
                                            let footer = Rect::from_min_max(
                                                pos2(card.left() + 4.0, card.top() + 104.0),
                                                pos2(card.right() - 32.0, card.bottom() - 4.0),
                                            );
                                            let galley = ui.painter().layout_no_wrap(
                                                w.title.clone(),
                                                FontId::proportional(10.0),
                                                t.text,
                                            );
                                            let offset = self.local_ticker_offset(
                                                r.id,
                                                &w.title,
                                                galley.size().x,
                                                footer.width(),
                                                r.hovered() || r.has_focus(),
                                            );
                                            ui.painter()
                                                .with_clip_rect(footer.intersect(ui.clip_rect()))
                                                .galley(
                                                    footer.min + vec2(offset, 0.0),
                                                    galley,
                                                    t.text,
                                                );
                                            let close = Rect::from_min_size(
                                                pos2(card.right() - 28.0, card.top() + 104.0),
                                                vec2(24.0, 24.0),
                                            );
                                            let r = self.button(
                                                ui,
                                                close,
                                                Id::new(("gallery", &w.id, w.identity, "close")),
                                                &format!("Close {}", w.title),
                                                "×",
                                                false,
                                                index == g.selected,
                                                t,
                                            );
                                            if r.clicked() {
                                                if let Some(g) = &mut self.view.gallery {
                                                    g.selected = index;
                                                }
                                                self.input(Input::GalleryAction(Action::Close));
                                            }
                                            if r.has_focus() {
                                                ui.painter().rect_stroke(
                                                    close,
                                                    2,
                                                    Stroke::new(2.0_f32, t.accent),
                                                    StrokeKind::Inside,
                                                );
                                            }
                                        }
                                    });
                                    ui.add_space(8.0);
                                }
                            });
                    });
            });
        let hover = response.response.contains_pointer();
        if hover != self.hovered_gallery {
            self.hovered_gallery = hover;
            self.input(Input::HoverGallery(hover));
        }
    }
    fn preview(&mut self, ctx: &egui::Context, t: Theme, bounds: Rect) {
        let Some(w) = self
            .view
            .preview
            .as_ref()
            .and_then(|id| self.windows.iter().find(|w| &w.id == id))
            .cloned()
        else {
            self.hovered_preview = false;
            // Measure the pending popup during its real hover delay, so the
            // first visible frame paints immediately rather than Area's sizing pass.
            if self.hovered_task.is_some() {
                egui::Area::new(Id::new("preview-panel"))
                    .interactable(false)
                    .default_size(vec2(332.0_f32.min(bounds.width() - 16.0), 228.0))
                    .fixed_pos(pos2(
                        8.0,
                        (bounds.bottom() - self.view.height - 236.0).max(8.0),
                    ))
                    .show(ctx, |ui| {
                        ui.allocate_exact_size(
                            vec2(332.0_f32.min(bounds.width() - 16.0), 228.0),
                            Sense::hover(),
                        );
                    });
            }
            return;
        };
        let pos = pos2(8.0, (bounds.bottom() - self.view.height - 236.0).max(8.0));
        let response = egui::Area::new(Id::new("preview-panel"))
            .fade_in(false)
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .default_size(vec2(332.0_f32.min(bounds.width() - 16.0), 228.0))
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(
                    vec2(332.0_f32.min(bounds.width() - 16.0), 228.0),
                    Sense::hover(),
                );
                ui.painter().rect_filled(rect, 8, t.bar);
                let header = Rect::from_min_size(
                    rect.min + vec2(12.0, 12.0),
                    vec2(rect.width() - 60.0, 28.0),
                );
                let r = self.button(
                    ui,
                    header,
                    Id::new(("preview", &w.id, w.identity, "header-maximize")),
                    &format!("Maximize {}", w.title),
                    &w.title,
                    w.active,
                    true,
                    t,
                );
                if r.clicked() {
                    self.input(Input::PreviewAction(Action::Maximize));
                }
                let close = Rect::from_min_size(
                    pos2(rect.right() - 40.0, rect.top() + 12.0),
                    vec2(28.0, 25.0),
                );
                if self
                    .button(
                        ui,
                        close,
                        Id::new(("preview", &w.id, w.identity, "close")),
                        &format!("Close preview {}", w.title),
                        "×",
                        false,
                        true,
                        t,
                    )
                    .clicked()
                {
                    self.input(Input::PreviewAction(Action::Close));
                }
                let image =
                    Rect::from_min_max(rect.min + vec2(12.0, 48.0), rect.max - vec2(12.0, 12.0));
                let r = self.button(
                    ui,
                    image,
                    Id::new(("preview", &w.id, w.identity, "image-maximize")),
                    &format!("Activate preview {}", w.title),
                    if self.showcase_enabled {
                        ""
                    } else {
                        "Preview unavailable · Simulated maximize"
                    },
                    false,
                    true,
                    t,
                );
                if r.clicked() {
                    self.input(Input::PreviewAction(Action::Maximize));
                }
                if self.showcase_enabled {
                    self.preview_art(ui, image, &w, t, "Simulated preview artwork");
                }
            });
        let hover = response.response.contains_pointer();
        if hover != self.hovered_preview {
            self.hovered_preview = hover;
            self.input(Input::HoverPreview(hover));
        }
    }
    fn menu_ui(&mut self, ctx: &egui::Context, t: Theme, bounds: Rect) {
        let Some(menu) = self.menu.clone() else {
            self.menu_had_focus = false;
            return;
        };
        let take_focus = !self.menu_had_focus;
        if take_focus {
            self.menu_origin_focus = ctx.memory(|m| m.focused());
        }
        self.menu_had_focus = true;
        let actions: Vec<(&str, Action, bool)> = match &menu {
            Menu::Pin(_) => vec![("Run as administrator · Simulated", Action::RunAs, true)],
            Menu::Group(_) => vec![
                (
                    if self.showcase_enabled {
                        "Open Simulated Process Manager"
                    } else {
                        "Process Manager · unavailable"
                    },
                    Action::Process,
                    true,
                ),
                (
                    "Close all windows · Simulated requests",
                    Action::CloseAll,
                    true,
                ),
            ],
            Menu::Task(id, identity) => {
                let w = self
                    .windows
                    .iter()
                    .find(|w| &w.id == id && w.identity == *identity);
                vec![
                    ("Restore / Switch · Simulated", Action::Focus, w.is_some()),
                    (
                        "Minimize · Simulated",
                        Action::Minimize,
                        w.is_some_and(|w| !w.minimized),
                    ),
                    (
                        if self.showcase_enabled {
                            "Open Simulated Process Manager"
                        } else {
                            "Process Manager · unavailable"
                        },
                        Action::Process,
                        w.is_some_and(|w| w.pid.is_some()),
                    ),
                    ("Close · Simulated intent", Action::Close, w.is_some()),
                ]
            }
        };
        egui::Area::new(Id::new("fixture-menu"))
            .order(egui::Order::Tooltip)
            .fixed_pos(pos2(
                16.0,
                (bounds.bottom() - self.view.height - 180.0).max(8.0),
            ))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(t.bar)
                    .inner_margin(8)
                    .corner_radius(4)
                    .show(ui, |ui| {
                        ui.label("Simulated menu");
                        for (i, (label, action, enabled)) in actions.into_iter().enumerate() {
                            let response = ui.add_enabled(enabled, egui::Button::new(label));
                            self.remember(label, &response);
                            if i == 0 && take_focus && enabled {
                                response.request_focus();
                            }
                            if response.clicked() {
                                self.input(Input::MenuAction(action));
                            }
                        }
                        if ui.button("Dismiss menu").clicked() {
                            self.input(Input::Key(Key::Escape));
                        }
                    });
            });
    }
    fn preview_art(&mut self, ui: &mut Ui, rect: Rect, w: &TaskWindow, t: Theme, label: &str) {
        let response = ui.interact(rect, Id::new(("art", &w.id, w.identity)), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Image, true, label));
        self.remember(label, &response);
        let p = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        p.rect_filled(rect, 2, t.bg);
        let toolbar = Rect::from_min_size(rect.min, vec2(rect.width(), 18.0));
        p.rect_filled(toolbar, 0, t.idle);
        p.text(
            toolbar.left_center() + vec2(5.0, 0.0),
            Align2::LEFT_CENTER,
            format!("{} · Simulated preview", w.process),
            FontId::proportional(9.0),
            t.text,
        );
        let content = Rect::from_min_max(rect.min + vec2(6.0, 24.0), rect.max - vec2(6.0, 6.0));
        if w.process == "Browser" {
            p.rect_filled(
                Rect::from_min_size(content.min, vec2(content.width() * 0.22, content.height())),
                2,
                t.sunken,
            );
            for i in 0..3 {
                let row = Rect::from_min_size(
                    content.min + vec2(content.width() * 0.27, i as f32 * content.height() / 3.0),
                    vec2(content.width() * 0.7, content.height() / 3.0 - 4.0),
                );
                p.rect_filled(row, 2, t.idle);
                p.text(
                    row.left_center() + vec2(4.0, 0.0),
                    Align2::LEFT_CENTER,
                    ["DL Release.zip", "DL Theme pack", "OK Notes.pdf"][i],
                    FontId::proportional(9.0),
                    t.text,
                );
            }
        } else {
            p.rect_filled(content, 2, t.sunken);
            let sidebar =
                Rect::from_min_size(content.min, vec2(content.width() * 0.18, content.height()));
            p.rect_filled(sidebar, 0, t.idle);
            for i in 0..4 {
                let line = Rect::from_min_size(
                    content.min + vec2(content.width() * 0.23, 6.0 + i as f32 * 12.0),
                    vec2(content.width() * (0.45 + (i % 2) as f32 * 0.15), 3.0),
                );
                p.rect_filled(line, 1, if i == 0 { t.accent } else { t.muted });
            }
            p.text(
                content.min + vec2(content.width() * 0.23, content.height() - 8.0),
                Align2::LEFT_BOTTOM,
                if w.process == "Terminal" {
                    "> cargo build · success"
                } else {
                    "{}  fixture / workspace"
                },
                FontId::monospace(8.0),
                t.text,
            );
        }
    }
    fn process_popup(&mut self, ctx: &egui::Context, t: Theme, bounds: Rect) {
        let Some(pm) = self.view.process_manager.clone() else {
            return;
        };
        let width = (bounds.width() - 16.0).min(620.0);
        egui::Area::new(Id::new("simulated-process-manager"))
            .fade_in(false)
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(
                (bounds.width() - width) / 2.0,
                80.0_f32.min(bounds.height() * 0.1),
            ))
            .default_size(vec2(width, 360.0))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(t.bar)
                    .stroke(Stroke::new(1.0_f32, t.border))
                    .inner_margin(12)
                    .corner_radius(4)
                    .show(ui, |ui| {
                        ui.set_width((width - 24.0).max(100.0));
                        ui.heading("Simulated Process Manager");
                        ui.label(
                            egui::RichText::new(
                                "Deterministic fake metrics • No OS process access",
                            )
                            .color(t.muted),
                        );
                        ui.separator();
                        let filter_label = ui.label("Filter simulated processes");
                        let r = ui
                            .add(
                                egui::TextEdit::singleline(&mut self.demo_filter)
                                    .id(Id::new("demo-process-filter"))
                                    .hint_text("Filter by name")
                                    .char_limit(256)
                                    .desired_width(ui.available_width()),
                            )
                            .labelled_by(filter_label.id);
                        self.remember("Filter simulated processes", &r);
                        if std::mem::take(&mut self.demo_focus_pending) {
                            r.request_focus();
                        }
                        if r.changed() {
                            self.input(Input::DemoProcessFilter(self.demo_filter.clone()));
                        }
                        ui.horizontal_wrapped(|ui| {
                            let r = ui.button("Refresh simulated processes");
                            self.remember("Refresh simulated processes", &r);
                            if r.clicked() {
                                self.input(Input::DemoProcessRefresh);
                            }
                            if ui.button("CPU descending").clicked() {
                                self.input(Input::DemoProcessSort(DemoProcessSort::CpuDescending));
                            }
                            ui.label(format!("Fake snapshot {}", pm.revision));
                        });
                        ui.separator();
                        if width >= 500.0 {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [220.0, 20.0],
                                    egui::Label::new("Process / fake identity"),
                                );
                                ui.add_sized([130.0, 20.0], egui::Label::new("CPU %"));
                                ui.label("Memory / status");
                            });
                        }
                        egui::ScrollArea::vertical()
                            .max_height((bounds.height() - 300.0).clamp(70.0, 240.0))
                            .show(ui, |ui| {
                                if pm.rows.is_empty() {
                                    ui.label("No simulated processes match the filter.");
                                }
                                for row in &pm.rows {
                                    ui.push_id((&row.id, row.identity), |ui| {
                                        if width >= 500.0 {
                                            ui.horizontal(|ui| {
                                                let r = ui.add_sized(
                                                    [220.0, 26.0],
                                                    egui::Button::selectable(
                                                        pm.selected.as_ref() == Some(&row.id),
                                                        format!("{} · {}", row.name, row.id),
                                                    ),
                                                );
                                                if r.clicked() {
                                                    self.input(Input::DemoProcessSelect(
                                                        row.id.clone(),
                                                    ));
                                                }
                                                ui.add_sized(
                                                    [130.0, 18.0],
                                                    egui::ProgressBar::new(row.cpu_percent / 100.0)
                                                        .fill(t.active)
                                                        .text(format!("{:.1}%", row.cpu_percent)),
                                                );
                                                ui.label(format!(
                                                    "{} MiB{}",
                                                    row.memory_mib,
                                                    if row.killable { "" } else { " · Protected" }
                                                ));
                                            });
                                        } else {
                                            let r = ui.selectable_label(
                                                pm.selected.as_ref() == Some(&row.id),
                                                format!(
                                                    "{}    {:.1}% CPU    {} MiB{}",
                                                    row.name,
                                                    row.cpu_percent,
                                                    row.memory_mib,
                                                    if row.killable { "" } else { "    Protected" }
                                                ),
                                            );
                                            if r.clicked() {
                                                self.input(Input::DemoProcessSelect(
                                                    row.id.clone(),
                                                ));
                                            }
                                        }
                                    });
                                }
                            });
                        ui.separator();
                        let can_end = pm
                            .rows
                            .iter()
                            .any(|r| Some(&r.id) == pm.selected.as_ref() && r.killable);
                        if pm.confirmation.is_some() {
                            ui.label(
                                "Remove this fake row? Authoritative task tiles remain unchanged.",
                            );
                            ui.horizontal_wrapped(|ui| {
                                let r = ui.button("Confirm simulated end");
                                self.remember("Confirm simulated end", &r);
                                if r.clicked() {
                                    self.input(Input::DemoProcessConfirm(true));
                                }
                                if ui.button("Cancel simulated end").clicked() {
                                    self.input(Input::DemoProcessConfirm(false));
                                }
                            });
                        } else {
                            let r =
                                ui.add_enabled(can_end, egui::Button::new("End simulated process"));
                            self.remember("End simulated process", &r);
                            if r.clicked() {
                                self.input(Input::DemoProcessEnd);
                            }
                        }
                        let r = ui.button("Dismiss Simulated Process Manager");
                        self.remember("Dismiss Simulated Process Manager", &r);
                        if r.clicked() {
                            self.input(Input::DemoProcessClose);
                        }
                    });
            });
    }
}
fn scenario_name(i: usize) -> &'static str {
    [
        "Empty (safe default)",
        "Single preview",
        "2 windows / Ribbon",
        "3 windows / Ribbon",
        "7 windows / Matrix + states",
        "1000-window stress",
        "Mixed simulated showcase",
    ]
    .get(i)
    .copied()
    .unwrap_or("Empty")
}
