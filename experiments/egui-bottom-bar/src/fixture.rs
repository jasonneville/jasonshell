//! Deterministic simulated state machine. IDs are fixture strings, never OS authority.
use std::collections::{HashMap, HashSet};

/// Reserved decoded-image capacity (4 MiB). M1 has no images and allocates zero.
pub const CACHE_LIMIT_BYTES: usize = 4 * 1024 * 1024;
const LEDGER_LIMIT: usize = 256;

#[derive(Debug, Clone)]
pub struct DemoProcess {
    pub id: String,
    pub identity: u64,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mib: u32,
    pub killable: bool,
}
#[derive(Debug, Clone)]
pub struct DemoProcessView {
    pub rows: Vec<DemoProcess>,
    pub selected: Option<String>,
    pub target: Option<String>,
    pub confirmation: Option<String>,
    pub revision: u64,
}
#[derive(Debug, Clone, Copy)]
pub enum DemoProcessSort {
    CpuDescending,
}

#[derive(Debug, Clone)]
pub struct TaskWindow {
    pub id: String,
    pub identity: u64,
    pub title: String,
    pub process: String,
    pub pid: Option<u32>,
    pub active: bool,
    pub minimized: bool,
    pub attention: bool,
    pub busy: bool,
    pub toasts: u32,
}
#[derive(Debug, Clone)]
pub struct Pin {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Focus,
    Minimize,
    Maximize,
    Close,
    RequestClose,
    CloseAll,
    Process,
    RunAs,
    Resize,
    Launch,
    EndDemoProcess,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Simulated,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchOutcome {
    Success,
    Failed,
    Canceled,
    UnelevatedFallback,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Enter,
    Escape,
    ContextMenu,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Ribbon,
    Matrix,
}
#[derive(Debug, Clone)]
pub struct LedgerEntry {
    pub action: Action,
    pub target: Option<String>,
    pub status: Outcome,
    pub elevated: Option<bool>,
}
#[derive(Debug, Clone)]
pub struct GroupView {
    pub key: String,
    pub members: Vec<String>,
    pub toasts: u32,
    pub active: bool,
    pub minimized: bool,
    pub attention: bool,
    pub busy: bool,
    pub capsule: bool,
}
#[derive(Debug, Clone)]
pub struct GalleryView {
    pub mode: LayoutMode,
    pub columns: usize,
    pub visible_rows: usize,
    pub close_size: f32,
    pub selected: usize,
}
#[derive(Debug, Clone)]
pub struct QuickLaunchView {
    pub pins: Vec<String>,
    pub selected: Option<usize>,
}
#[derive(Debug, Clone)]
pub struct View {
    pub process_manager: Option<DemoProcessView>,
    pub sequence: u64,
    pub groups: Vec<GroupView>,
    pub pressure: bool,
    pub height: f32,
    pub preview: Option<String>,
    pub gallery: Option<GalleryView>,
    pub quick_launch: Option<QuickLaunchView>,
    pub menu_open: bool,
    pub diagnostic: Option<String>,
    pub ticker_offset: f32,
    pub animation_pending: bool,
    pub live_capabilities: bool,
    pub pending_snapshots: usize,
    pub thumbnail_destinations: usize,
    pub cache_bytes: usize,
    pub disposed: bool,
}
#[derive(Debug, Clone)]
pub enum Input {
    DemoProcessFilter(String),
    DemoProcessSort(DemoProcessSort),
    DemoProcessSelect(String),
    DemoProcessRefresh,
    DemoProcessEnd,
    DemoProcessConfirm(bool),
    DemoProcessClose,
    DemoProcesses(Vec<DemoProcess>),
    Snapshot {
        sequence: u64,
        windows: Vec<TaskWindow>,
    },
    Pins(Vec<Pin>),
    StripMetrics {
        available: f32,
        required: f32,
    },
    HeightLocked(bool),
    ResizeStart {
        y: f32,
    },
    ResizeMove {
        y: f32,
    },
    ResizeEnd,
    ResizeCancel,
    TileActivate {
        id: String,
        pre_click_active: bool,
    },
    HoverTask(Option<String>),
    HoverPreview(bool),
    PreviewAction(Action),
    Viewport {
        width: f32,
        height: f32,
    },
    OpenGallery(String),
    HoverGroup(Option<String>),
    HoverGallery(bool),
    Key(Key),
    GalleryAction(Action),
    GroupMenu(String),
    TaskMenu(String),
    MenuAction(Action),
    ToggleQuickLaunch,
    QuickLaunchButtonDown,
    QuickLaunchButtonClick,
    LaunchOutcome(LaunchOutcome),
    Blur,
    DragStart {
        group: String,
        x: f32,
    },
    DragMove {
        x: f32,
    },
    DragEnd,
    DragCancel,
    TrailingClick(String),
    TickerMeasure {
        text_width: f32,
        viewport_width: f32,
    },
    TickerEngage {
        hover: bool,
        focus: bool,
    },
    ReducedMotion(bool),
    Visible(bool),
    Processes,
}

#[derive(Debug, Clone)]
pub(crate) enum Menu {
    Task(String, u64),
    Group(Vec<(String, u64)>),
    Pin(String),
}
struct Drag {
    key: String,
    x: f32,
    delta: f32,
    crossed: bool,
    order: Vec<String>,
    centers: Vec<f32>,
}
struct Resize {
    y: f32,
    height: f32,
    dirty: bool,
}

pub struct FixtureApp {
    pub(crate) showcase_enabled: bool,
    pub(crate) demo_filter: String,
    pub(crate) demo_focus_pending: bool,
    pub(crate) demo_origin_focus: Option<egui::Id>,
    demo_rows: Vec<DemoProcess>,
    demo_confirmation: Option<(String, u64)>,
    pub(crate) view: View,
    pub(crate) windows: Vec<TaskWindow>,
    pub(crate) pins: Vec<Pin>,
    ledger: Vec<LedgerEntry>,
    pending: Option<(u64, Vec<TaskWindow>)>,
    pub(crate) now: u64,
    pub(crate) locked: bool,
    resize: Option<Resize>,
    drag: Option<Drag>,
    suppress_click: Option<String>,
    suppress_quick: bool,
    preview_show: Option<(u64, String)>,
    preview_hide: Option<u64>,
    gallery_show: Option<(u64, String)>,
    gallery_hide: Option<u64>,
    pub(crate) gallery_key: Option<String>,
    pub(crate) menu: Option<Menu>,
    closed: HashSet<(String, u64)>,
    pub(crate) viewport: [f32; 2],
    pub(crate) launch_outcome: LaunchOutcome,
    ticker_overflow: f32,
    ticker_start: Option<u64>,
    ticker_engaged: bool,
    pub(crate) reduced: bool,
    pub(crate) visible: bool,
    pub(crate) rects: HashMap<String, egui::Rect>,
    pub(crate) theme: usize,
    pub(crate) scenario: usize,
    pub(crate) task_focus: usize,
    pub(crate) restore_focus: Option<egui::Id>,
    pub(crate) origin_focus: Option<egui::Id>,
    pub(crate) menu_origin_focus: Option<egui::Id>,
    pub(crate) drag_slot: f32,
    pub(crate) measured_group_centers: Vec<(String, f32)>,
    pub(crate) hovered_task: Option<String>,
    pub(crate) hovered_group: Option<String>,
    pub(crate) hovered_preview: bool,
    pub(crate) hovered_gallery: bool,
    pub(crate) last_gallery_selected: Option<usize>,
    pub(crate) gallery_activation_focus_pending: bool,
    pub(crate) menu_had_focus: bool,
    pub(crate) focus_task_next: bool,
    pub(crate) draw_tickers: HashMap<egui::Id, crate::ui::DrawTicker>,
    pub(crate) draw_frame: u64,
    pub(crate) draw_animation_pending: bool,
}
impl Default for FixtureApp {
    fn default() -> Self {
        Self {
            showcase_enabled: false,
            demo_filter: String::new(),
            demo_focus_pending: false,
            demo_origin_focus: None,
            demo_rows: vec![],
            demo_confirmation: None,
            view: View {
                process_manager: None,
                sequence: 0,
                groups: vec![],
                pressure: false,
                height: 32.4,
                preview: None,
                gallery: None,
                quick_launch: None,
                menu_open: false,
                diagnostic: None,
                ticker_offset: 0.0,
                animation_pending: false,
                live_capabilities: false,
                pending_snapshots: 0,
                thumbnail_destinations: 0,
                cache_bytes: 0,
                disposed: false,
            },
            windows: vec![],
            pins: vec![],
            ledger: vec![],
            pending: None,
            now: 0,
            locked: true,
            resize: None,
            drag: None,
            suppress_click: None,
            suppress_quick: false,
            preview_show: None,
            preview_hide: None,
            gallery_show: None,
            gallery_hide: None,
            gallery_key: None,
            menu: None,
            closed: HashSet::new(),
            viewport: [584.0, 328.0],
            launch_outcome: LaunchOutcome::Success,
            ticker_overflow: 0.0,
            ticker_start: None,
            ticker_engaged: false,
            reduced: false,
            visible: true,
            rects: HashMap::new(),
            theme: 0,
            scenario: 0,
            task_focus: 0,
            restore_focus: None,
            origin_focus: None,
            menu_origin_focus: None,
            drag_slot: 100.0,
            measured_group_centers: Vec::new(),
            hovered_task: None,
            hovered_group: None,
            hovered_preview: false,
            hovered_gallery: false,
            last_gallery_selected: None,
            gallery_activation_focus_pending: false,
            menu_had_focus: false,
            focus_task_next: false,
            draw_tickers: HashMap::new(),
            draw_frame: 0,
            draw_animation_pending: false,
        }
    }
}

impl FixtureApp {
    pub fn showcase() -> Self {
        let mut app = Self {
            showcase_enabled: true,
            scenario: 6,
            ..Self::default()
        };
        app.input(Input::Snapshot {
            sequence: 1,
            windows: [
                ("demo-browser", "Browser", "Browser — Downloads", 101),
                ("demo-terminal", "Terminal", "Terminal — Build output", 102),
                ("demo-editor-1", "Editor", "Editor — main.rs", 103),
                ("demo-editor-2", "Editor", "Editor — theme.css", 103),
                ("demo-editor-3", "Editor", "Editor — Notes", 103),
            ]
            .into_iter()
            .map(|(id, process, title, pid)| TaskWindow {
                id: id.into(),
                identity: 1,
                title: title.into(),
                process: process.into(),
                pid: Some(pid),
                active: false,
                minimized: false,
                attention: false,
                busy: false,
                toasts: 0,
            })
            .collect(),
        });
        app.input(Input::Pins(
            [
                ("pin-browser", "Browser"),
                ("pin-editor", "Editor"),
                ("pin-terminal", "Terminal"),
            ]
            .into_iter()
            .map(|(id, name)| Pin {
                id: id.into(),
                name: name.into(),
            })
            .collect(),
        ));
        app.demo_rows = [
            ("fake-browser", "Browser", 12.4, 384, true),
            ("fake-editor", "Editor", 4.8, 256, true),
            ("fake-terminal", "Terminal", 0.7, 48, true),
            ("fake-shell", "Fixture shell (protected)", 0.2, 64, false),
        ]
        .into_iter()
        .map(|(id, name, cpu, memory, killable)| DemoProcess {
            id: id.into(),
            identity: 1,
            name: name.into(),
            cpu_percent: cpu,
            memory_mib: memory,
            killable,
        })
        .collect();
        app
    }
    fn sync_demo_rows(&mut self) {
        if let Some(pm) = &mut self.view.process_manager {
            let filter = self.demo_filter.to_lowercase();
            pm.rows = self
                .demo_rows
                .iter()
                .filter(|r| r.name.to_lowercase().contains(&filter))
                .cloned()
                .collect();
            pm.rows.sort_by(|a, b| {
                b.cpu_percent
                    .total_cmp(&a.cpu_percent)
                    .then_with(|| a.id.cmp(&b.id))
            });
            if !self
                .demo_rows
                .iter()
                .any(|r| Some(&r.id) == pm.selected.as_ref())
            {
                pm.selected = None;
            }
        }
    }
    fn open_demo(&mut self, target: Option<String>, origin: Option<egui::Id>) {
        // The invoking route transfers its origin; an old menu owns no later
        // direct Processes invocation. Popup close consumes this one-shot origin.
        self.demo_origin_focus = origin;
        self.menu_origin_focus = None;
        self.dismiss();
        self.restore_focus = None;
        self.demo_focus_pending = true;
        self.demo_filter.clear();
        self.demo_confirmation = None;
        self.view.process_manager = Some(DemoProcessView {
            rows: vec![],
            selected: target.clone(),
            target,
            confirmation: None,
            revision: 0,
        });
        self.sync_demo_rows();
    }
    fn open_target_demo(&mut self, members: &[(String, u64)]) {
        let target = members.iter().find_map(|(id, identity)| {
            self.windows
                .iter()
                .find(|w| &w.id == id && w.identity == *identity && w.pid.is_some())
                .and_then(|w| match w.id.as_str() {
                    "demo-browser" => Some("fake-browser"),
                    "demo-terminal" => Some("fake-terminal"),
                    "demo-editor-1" | "demo-editor-2" | "demo-editor-3" => Some("fake-editor"),
                    _ => None,
                })
        });
        if let Some(target) = target
            && self.demo_rows.iter().any(|r| r.id == target)
        {
            let origin = self.menu_origin_focus.take().or(self.origin_focus.take());
            self.open_demo(Some(target.into()), origin);
        }
    }
    pub fn view(&self) -> &View {
        &self.view
    }
    pub fn ledger(&self) -> &[LedgerEntry] {
        &self.ledger
    }
    pub fn widget_rect(&self, name: &str) -> Option<egui::Rect> {
        self.rects.get(name).copied()
    }
    pub fn draw(&mut self, ctx: &egui::Context) {
        self.draw_ui(ctx);
    }
    /// Frozen theme IDs for reproducible fixture evidence.
    pub fn theme_names() -> impl Iterator<Item = &'static str> {
        crate::theme::names()
    }
    /// In-memory theme selection; never reads or saves user preferences.
    pub fn set_demo_theme(&mut self, name: &str) -> bool {
        if let Some(index) = crate::theme::names().position(|id| id == name) {
            self.theme = index;
            true
        } else {
            false
        }
    }
    pub fn queue_snapshot(&mut self, sequence: u64, windows: Vec<TaskWindow>) {
        if !self.view.disposed
            && sequence > self.view.sequence
            && self.pending.as_ref().is_none_or(|p| sequence > p.0)
        {
            self.pending = Some((sequence, windows));
            self.view.pending_snapshots = 1;
        }
    }
    fn record(&mut self, action: Action, target: Option<String>, elevated: Option<bool>) {
        if self.ledger.len() == LEDGER_LIMIT {
            self.ledger.remove(0);
        }
        self.ledger.push(LedgerEntry {
            action,
            target,
            status: Outcome::Simulated,
            elevated,
        });
        self.view.diagnostic = Some(format!("Simulated {action:?}; no external action"));
    }
    pub(crate) fn members(&self) -> Vec<TaskWindow> {
        self.gallery_key
            .as_ref()
            .and_then(|k| self.view.groups.iter().find(|g| &g.key == k))
            .map(|g| {
                g.members
                    .iter()
                    .filter_map(|id| self.windows.iter().find(|w| &w.id == id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }
    fn selected(&self) -> Option<TaskWindow> {
        self.view
            .gallery
            .as_ref()
            .and_then(|g| self.members().get(g.selected).cloned())
    }
    fn snapshot(&mut self, sequence: u64, windows: Vec<TaskWindow>) {
        if sequence <= self.view.sequence {
            return;
        }
        let selected = self.selected().map(|w| (w.id, w.identity));
        let old_order: Vec<_> = self.view.groups.iter().map(|g| g.key.clone()).collect();
        self.windows = windows;
        if let Some(scenario) = [0, 1, 2, 3, 7, 1000]
            .iter()
            .position(|count| *count == self.windows.len())
        {
            self.scenario = scenario;
        }
        self.view.sequence = sequence;
        self.closed.retain(|(id, identity)| {
            self.windows
                .iter()
                .any(|w| &w.id == id && w.identity == *identity)
        });
        let mut groups: Vec<GroupView> = vec![];
        for w in &self.windows {
            let process = w.process.trim().to_lowercase();
            let key = if process.is_empty() {
                format!("window:{}", w.id)
            } else {
                process
            };
            let i = groups.iter().position(|g| g.key == key).unwrap_or_else(|| {
                groups.push(GroupView {
                    key: key.clone(),
                    members: vec![],
                    toasts: 0,
                    active: false,
                    minimized: true,
                    attention: false,
                    busy: false,
                    capsule: false,
                });
                groups.len() - 1
            });
            let g = &mut groups[i];
            g.members.push(w.id.clone());
            g.toasts = g.toasts.max(w.toasts);
            g.active |= w.active;
            g.minimized &= w.minimized;
            g.attention |= w.attention;
            g.busy |= w.busy && busy_eligible(w);
            g.capsule = g.members.len() >= 2;
        }
        groups.sort_by_key(|g| {
            old_order
                .iter()
                .position(|k| k == &g.key)
                .unwrap_or(old_order.len())
        });
        self.view.groups = groups;
        if let Some(key) = self.gallery_key.clone() {
            self.layout_gallery(&key);
            let members = self.members();
            if let Some(g) = &mut self.view.gallery {
                g.selected = selected
                    .and_then(|s| {
                        members
                            .iter()
                            .position(|w| w.id == s.0 && w.identity == s.1)
                    })
                    .unwrap_or(0);
            }
        }
        if self
            .view
            .preview
            .as_ref()
            .is_some_and(|id| !self.windows.iter().any(|w| &w.id == id))
        {
            self.view.preview = None;
        }
    }
    fn layout_gallery(&mut self, key: &str) {
        let count = self
            .view
            .groups
            .iter()
            .find(|g| g.key == key)
            .map_or(0, |g| g.members.len());
        if count == 0 {
            self.close_gallery();
            return;
        }
        let ribbon = (2..=3).contains(&count)
            && (16 + count * 180 + (count - 1) * 8) as f32 <= self.viewport[0];
        let columns = if ribbon {
            count
        } else {
            (((self.viewport[0] - 20.0) / 188.0).floor().max(1.0) as usize)
                .min(3)
                .min(count)
        };
        let selected = self
            .view
            .gallery
            .as_ref()
            .map_or(0, |g| g.selected.min(count - 1));
        // Opening owns activation focus; layout/authoritative snapshot revisions
        // do not. Surviving identity/action widget IDs retain their existing role.
        if self.view.gallery.is_none() {
            self.gallery_activation_focus_pending = true;
        }
        self.view.gallery = Some(GalleryView {
            mode: if ribbon {
                LayoutMode::Ribbon
            } else {
                LayoutMode::Matrix
            },
            columns,
            visible_rows: count.div_ceil(columns).min(2),
            close_size: 24.0,
            selected,
        });
        self.gallery_key = Some(key.into());
        // No DWM destinations exist in M1: unavailable placeholders are painted directly.
        self.view.thumbnail_destinations = 0;
    }
    fn close_gallery(&mut self) {
        self.view.gallery = None;
        self.gallery_activation_focus_pending = false;
        self.gallery_key = None;
        self.gallery_show = None;
        self.gallery_hide = None;
        self.view.thumbnail_destinations = 0;
    }
    fn dismiss(&mut self) {
        self.view.quick_launch = None;
        self.view.preview = None;
        self.preview_show = None;
        self.preview_hide = None;
        self.close_gallery();
        self.menu = None;
        self.view.menu_open = false;
        self.restore_focus = self.origin_focus.take();
    }
    fn open_quick(&mut self) {
        if self.view.quick_launch.is_some() {
            self.view.quick_launch = None;
            self.restore_focus = self.origin_focus.take();
        } else {
            self.view.quick_launch = Some(QuickLaunchView {
                pins: self.pins.iter().map(|p| p.id.clone()).collect(),
                selected: if self.pins.is_empty() { None } else { Some(0) },
            });
        }
    }
    fn launch(&mut self, action: Action, pin: String) {
        if !self.pins.iter().any(|p| p.id == pin) {
            return;
        }
        let elevated =
            (action == Action::RunAs).then_some(self.launch_outcome == LaunchOutcome::Success);
        self.record(action, Some(pin), elevated);
        self.menu = None;
        self.view.menu_open = false;
        match self.launch_outcome {
            LaunchOutcome::Success => {
                self.view.quick_launch = None;
                self.restore_focus = self.origin_focus.take();
            }
            LaunchOutcome::UnelevatedFallback => {
                self.view.quick_launch = None;
                self.view.diagnostic =
                    Some("Simulated unelevated fallback — not administrator".into());
                self.restore_focus = self.origin_focus.take();
            }
            LaunchOutcome::Failed => {
                self.view.diagnostic = Some("Simulated launch failed — retry available".into())
            }
            LaunchOutcome::Canceled => {
                self.view.diagnostic = Some("Simulated launch canceled — retry available".into())
            }
        }
    }
    fn task_action(&mut self, action: Action, id: String, identity: u64) {
        let Some(w) = self
            .windows
            .iter()
            .find(|w| w.id == id && w.identity == identity)
        else {
            return;
        };
        if action == Action::Minimize && w.minimized {
            return;
        }
        if action == Action::Process {
            if w.pid.is_some() {
                self.input(Input::Processes);
            }
            return;
        }
        if !matches!(
            action,
            Action::Focus
                | Action::Minimize
                | Action::Maximize
                | Action::Close
                | Action::RequestClose
        ) {
            return;
        }
        if action == Action::Close && !self.closed.insert((id.clone(), identity)) {
            return;
        }
        self.record(action, Some(id), None);
    }
    fn key(&mut self, key: Key) {
        if self.view.process_manager.is_some() {
            if key == Key::Escape {
                if self.demo_confirmation.is_some() {
                    self.input(Input::DemoProcessConfirm(false));
                } else {
                    self.input(Input::DemoProcessClose);
                }
            }
            return;
        }
        if key == Key::Escape {
            if self.drag.is_some() {
                self.input(Input::DragCancel);
            } else if self.resize.is_some() {
                self.input(Input::ResizeCancel);
            } else if self.menu.is_some() {
                self.menu = None;
                self.view.menu_open = false;
                self.restore_focus = self.menu_origin_focus.take();
            } else {
                self.dismiss();
            }
            return;
        }
        if self.menu.is_some() {
            return;
        }
        if let Some(q) = &mut self.view.quick_launch {
            let n = q.pins.len();
            if n == 0 {
                return;
            }
            let i = q.selected.unwrap_or(0);
            q.selected = Some(match key {
                Key::Up | Key::Left => (i + n - 1) % n,
                Key::Down | Key::Right => (i + 1) % n,
                Key::Home => 0,
                Key::End => n - 1,
                _ => i,
            });
            let id = q.pins[q.selected.unwrap_or(0)].clone();
            match key {
                Key::Enter => self.launch(Action::Launch, id),
                Key::ContextMenu => {
                    self.menu = Some(Menu::Pin(id));
                    self.view.menu_open = true;
                }
                _ => {}
            }
        } else if let Some(g) = &mut self.view.gallery {
            let n = self
                .windows
                .iter()
                .filter(|w| {
                    self.gallery_key
                        .as_ref()
                        .is_some_and(|k| group_key(w) == *k)
                })
                .count();
            if n == 0 {
                return;
            }
            let i = g.selected;
            let row = i / g.columns * g.columns;
            g.selected = match key {
                Key::Home => 0,
                Key::End => n - 1,
                Key::Left => i.saturating_sub(1).max(row),
                Key::Right => (i + 1).min((row + g.columns - 1).min(n - 1)),
                Key::Up => {
                    if row == 0 {
                        i
                    } else {
                        i - g.columns
                    }
                }
                Key::Down => {
                    if row + g.columns >= n {
                        i
                    } else {
                        (i + g.columns).min(n - 1)
                    }
                }
                _ => i,
            };
            if g.selected != i {
                self.gallery_activation_focus_pending = true;
            }
            match key {
                Key::Enter => self.input(Input::GalleryAction(Action::Focus)),
                Key::ContextMenu => {
                    if let Some(w) = self.selected() {
                        self.input(Input::TaskMenu(w.id));
                    }
                }
                _ => {}
            }
        }
    }
    pub fn input(&mut self, input: Input) {
        if self.view.disposed {
            return;
        }
        match input {
            Input::DemoProcessFilter(filter) => {
                if self.view.process_manager.is_some() {
                    self.demo_filter = filter.chars().take(256).collect();
                    self.sync_demo_rows();
                }
            }
            Input::DemoProcessSort(_) => self.sync_demo_rows(),
            Input::DemoProcessSelect(id) => {
                if let Some(pm) = &mut self.view.process_manager
                    && pm.rows.iter().any(|r| r.id == id)
                {
                    pm.selected = Some(id);
                }
            }
            Input::DemoProcessRefresh => {
                if let Some(pm) = &mut self.view.process_manager {
                    pm.revision = pm.revision.saturating_add(1);
                    for (i, r) in self.demo_rows.iter_mut().enumerate() {
                        r.cpu_percent = ((pm.revision as usize + i * 7) % 40) as f32;
                    }
                    self.sync_demo_rows();
                }
            }
            Input::DemoProcessEnd => {
                if let Some(pm) = &mut self.view.process_manager
                    && let Some(row) = self
                        .demo_rows
                        .iter()
                        .find(|r| Some(&r.id) == pm.selected.as_ref() && r.killable)
                {
                    self.demo_confirmation = Some((row.id.clone(), row.identity));
                    pm.confirmation = Some(row.id.clone());
                }
            }
            Input::DemoProcessConfirm(confirm) => {
                if let Some((id, identity)) = self.demo_confirmation.take()
                    && confirm
                    && self.view.process_manager.is_some()
                    && self
                        .demo_rows
                        .iter()
                        .any(|r| r.id == id && r.identity == identity && r.killable)
                {
                    self.demo_rows.retain(|r| r.id != id);
                    self.record(Action::EndDemoProcess, Some(id), None);
                }
                if let Some(pm) = &mut self.view.process_manager {
                    pm.confirmation = None;
                }
                self.sync_demo_rows();
            }
            Input::DemoProcessClose => {
                self.view.process_manager = None;
                self.demo_confirmation = None;
                self.demo_filter.clear();
                self.demo_focus_pending = false;
                self.restore_focus = self.demo_origin_focus.take();
            }
            Input::DemoProcesses(rows) => {
                if self.showcase_enabled {
                    let mut ids = HashSet::new();
                    self.demo_rows = rows
                        .into_iter()
                        .filter(|r| r.id.len() <= 256 && ids.insert(r.id.clone()))
                        .take(256)
                        .map(|mut r| {
                            r.cpu_percent = if r.cpu_percent.is_finite() {
                                r.cpu_percent.clamp(0.0, 100.0)
                            } else {
                                0.0
                            };
                            r.name = r.name.chars().take(256).collect();
                            r
                        })
                        .collect();
                    self.sync_demo_rows();
                }
            }
            Input::Snapshot { sequence, windows } => self.snapshot(sequence, windows),
            Input::Pins(mut pins) => {
                pins.sort_by_key(|p| p.name.to_lowercase());
                self.pins = pins;
                if let Some(q) = &mut self.view.quick_launch {
                    q.pins = self.pins.iter().map(|p| p.id.clone()).collect();
                    q.selected = if q.pins.is_empty() {
                        None
                    } else {
                        Some(q.selected.unwrap_or(0).min(q.pins.len() - 1))
                    };
                }
            }
            Input::StripMetrics {
                available,
                required,
            } => {
                if available.is_finite() && required.is_finite() {
                    if !self.view.pressure && required - available > 24.0 {
                        self.view.pressure = true;
                    } else if self.view.pressure && available - required >= 48.0 {
                        self.view.pressure = false;
                    }
                }
            }
            Input::HeightLocked(locked) => {
                if locked {
                    self.input(Input::ResizeCancel);
                }
                self.locked = locked;
            }
            Input::ResizeStart { y } => {
                if !self.locked && y.is_finite() {
                    self.resize = Some(Resize {
                        y,
                        height: self.view.height,
                        dirty: false,
                    });
                }
            }
            Input::ResizeMove { y } => {
                if let Some(r) = &mut self.resize
                    && y.is_finite()
                {
                    self.view.height = (r.height + r.y - y).clamp(24.0, 120.0);
                    r.dirty = true;
                }
            }
            Input::ResizeEnd => {
                self.flush_resize();
                self.resize = None;
            }
            Input::ResizeCancel => {
                if let Some(r) = self.resize.take() {
                    self.view.height = r.height;
                }
            }
            Input::TileActivate {
                id,
                pre_click_active,
            } => {
                if let Some(w) = self.windows.iter().find(|w| w.id == id) {
                    self.task_action(
                        if pre_click_active {
                            Action::Minimize
                        } else {
                            Action::Focus
                        },
                        id,
                        w.identity,
                    );
                }
            }
            Input::HoverTask(id) => {
                self.preview_show = if self.menu.is_some() || self.view.process_manager.is_some() {
                    None
                } else {
                    id.map(|id| (self.now.saturating_add(180), id))
                };
                self.preview_hide = if self.preview_show.is_none() {
                    Some(self.now.saturating_add(140))
                } else {
                    None
                };
            }
            Input::HoverPreview(hover) => {
                self.preview_hide = if hover {
                    None
                } else {
                    Some(self.now.saturating_add(140))
                }
            }
            Input::PreviewAction(action) => {
                if let Some(w) = self
                    .view
                    .preview
                    .as_ref()
                    .and_then(|id| self.windows.iter().find(|w| &w.id == id))
                    .cloned()
                {
                    self.task_action(action, w.id, w.identity);
                }
            }
            Input::Viewport { width, height } => {
                if width.is_finite() && height.is_finite() {
                    self.viewport = [width.max(29.0), height.max(49.0)];
                    if let Some(k) = self.gallery_key.clone() {
                        self.layout_gallery(&k);
                    }
                }
            }
            Input::OpenGallery(k) => {
                if self.gallery_key.as_ref() == Some(&k) {
                    self.close_gallery();
                } else {
                    self.closed.clear();
                    self.layout_gallery(&k);
                    self.gallery_activation_focus_pending = self.view.gallery.is_some();
                    self.gallery_hide = None;
                    self.gallery_show = None;
                }
            }
            Input::HoverGroup(k) => {
                self.gallery_show = if self.menu.is_some() || self.view.process_manager.is_some() {
                    None
                } else {
                    k.map(|k| (self.now.saturating_add(300), k))
                };
                self.gallery_hide = if self.gallery_show.is_none() {
                    Some(self.now.saturating_add(140))
                } else {
                    None
                };
            }
            Input::HoverGallery(hover) => {
                self.gallery_hide = if hover {
                    None
                } else {
                    Some(self.now.saturating_add(140))
                }
            }
            Input::Key(key) => self.key(key),
            Input::GalleryAction(action) => {
                if let Some(w) = self.selected() {
                    self.task_action(action, w.id, w.identity);
                }
            }
            Input::GroupMenu(k) => {
                if let Some(g) = self.view.groups.iter().find(|g| g.key == k) {
                    self.menu = Some(Menu::Group(
                        g.members
                            .iter()
                            .filter_map(|id| {
                                self.windows
                                    .iter()
                                    .find(|w| &w.id == id)
                                    .map(|w| (w.id.clone(), w.identity))
                            })
                            .collect(),
                    ));
                    self.view.menu_open = true;
                    self.gallery_show = None;
                    self.preview_show = None;
                }
            }
            Input::TaskMenu(id) => {
                if let Some(w) = self.windows.iter().find(|w| w.id == id) {
                    self.menu = Some(Menu::Task(id, w.identity));
                    self.view.menu_open = true;
                    self.preview_show = None;
                    self.gallery_show = None;
                }
            }
            Input::MenuAction(action) => {
                if let Some(menu) = self.menu.clone() {
                    match menu {
                        Menu::Pin(id) => {
                            if action == Action::RunAs {
                                self.launch(action, id);
                            }
                        }
                        Menu::Task(id, identity) => {
                            if action == Action::Process && self.showcase_enabled {
                                self.open_target_demo(&[(id, identity)]);
                            } else {
                                self.task_action(action, id, identity);
                            }
                        }
                        Menu::Group(members) => {
                            if action == Action::CloseAll {
                                for (id, identity) in members {
                                    self.task_action(Action::RequestClose, id, identity);
                                }
                            } else if action == Action::Process {
                                if self.showcase_enabled {
                                    self.open_target_demo(&members);
                                } else {
                                    self.input(Input::Processes);
                                }
                            }
                        }
                    }
                    self.menu = None;
                    self.view.menu_open = false;
                }
            }
            Input::ToggleQuickLaunch => self.open_quick(),
            Input::QuickLaunchButtonDown => {
                self.suppress_quick = self.view.quick_launch.is_some();
                if self.suppress_quick {
                    self.view.quick_launch = None;
                }
            }
            Input::QuickLaunchButtonClick => {
                if !self.suppress_quick {
                    self.open_quick();
                }
                self.suppress_quick = false;
            }
            Input::LaunchOutcome(outcome) => self.launch_outcome = outcome,
            Input::Blur => {
                self.input(Input::DemoProcessClose);
                self.input(Input::DragCancel);
                self.input(Input::ResizeCancel);
                self.dismiss();
            }
            Input::DragStart { group, x } => self.start_drag(group, x),
            Input::DragMove { x } => self.move_drag(x),
            Input::DragEnd => {
                if let Some(d) = self.drag.take() {
                    self.suppress_click = Some(d.key.clone());
                    if !d.crossed {
                        self.activate_group(&d.key);
                    }
                }
            }
            Input::DragCancel => {
                if let Some(d) = self.drag.take() {
                    self.view.groups.sort_by_key(|g| {
                        d.order
                            .iter()
                            .position(|k| k == &g.key)
                            .unwrap_or(d.order.len())
                    });
                    self.suppress_click = Some(d.key);
                }
            }
            Input::TrailingClick(k) => {
                if self.suppress_click.as_ref() == Some(&k) {
                    self.suppress_click = None;
                } else {
                    self.activate_group(&k);
                }
            }
            Input::TickerMeasure {
                text_width,
                viewport_width,
            } => {
                let overflow = if text_width.is_finite() && viewport_width.is_finite() {
                    (text_width - viewport_width).max(0.0)
                } else {
                    0.0
                };
                if overflow != self.ticker_overflow {
                    self.ticker_overflow = overflow;
                    self.ticker_start = self.ticker_engaged.then_some(self.now);
                }
            }
            Input::TickerEngage { hover, focus } => {
                let engaged = hover || focus;
                if engaged != self.ticker_engaged {
                    self.ticker_engaged = engaged;
                    self.ticker_start = engaged.then_some(self.now);
                }
            }
            Input::ReducedMotion(reduced) => self.reduced = reduced,
            Input::Visible(visible) => self.visible = visible,
            Input::Processes => {
                if self.showcase_enabled {
                    let origin = self.origin_focus.take();
                    self.open_demo(None, origin);
                } else {
                    self.view.diagnostic = Some("Process Manager unavailable in Simulated fixture (integration track not connected)".into());
                }
            }
        }
        self.update_ticker();
    }
    pub(crate) fn activate_group(&mut self, key: &str) {
        if let Some(g) = self.view.groups.iter().find(|g| g.key == key) {
            if g.capsule {
                self.input(Input::OpenGallery(key.into()));
            } else if let Some(w) = g
                .members
                .first()
                .and_then(|id| self.windows.iter().find(|w| &w.id == id))
                .cloned()
            {
                self.input(Input::TileActivate {
                    id: w.id,
                    pre_click_active: w.active,
                });
            }
        }
    }
    fn start_drag(&mut self, group: String, x: f32) {
        if !x.is_finite() || !self.view.groups.iter().any(|g| g.key == group) {
            return;
        }
        let order: Vec<_> = self.view.groups.iter().map(|g| g.key.clone()).collect();
        let measured: Option<Vec<_>> = order
            .iter()
            .map(|key| {
                self.measured_group_centers
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, center)| *center)
            })
            .collect();
        // Headless model input has explicit 100px default slots; drawable input
        // captures actual Response centers, including the UI's inter-tile gap.
        let centers = measured.unwrap_or_else(|| {
            (0..order.len())
                .map(|i| (i as f32 + 0.5) * self.drag_slot)
                .collect()
        });
        self.drag = Some(Drag {
            key: group,
            x,
            delta: 0.0,
            crossed: false,
            order,
            centers,
        });
    }
    /// Keyboard placement shares captured-center math but owns no pointer click.
    pub(crate) fn reorder_from_keyboard(&mut self, group: String, delta: f32) {
        if self.drag.is_some() {
            return;
        }
        self.start_drag(group, 0.0);
        self.move_drag(delta);
        self.drag = None;
    }
    fn move_drag(&mut self, x: f32) {
        let Some(d) = &mut self.drag else {
            return;
        };
        if !x.is_finite() {
            return;
        }
        d.delta = x - d.x;
        d.crossed |= d.delta.abs() >= 6.0;
        if !d.crossed {
            return;
        }
        let Some(source) = d.order.iter().position(|key| key == &d.key) else {
            return;
        };
        let moved_center = d.centers[source] + d.delta;
        let mut target = source;
        if d.delta > 0.0 {
            for index in source + 1..d.order.len() {
                if moved_center > d.centers[index] {
                    target = index;
                }
            }
        } else if d.delta < 0.0 {
            for index in (0..source).rev() {
                if moved_center < d.centers[index] {
                    target = index;
                }
            }
        }
        let mut order = d.order.clone();
        let key = order.remove(source);
        order.insert(target, key);
        self.view.groups.sort_by_key(|g| {
            order
                .iter()
                .position(|key| key == &g.key)
                .unwrap_or(order.len())
        });
    }
    fn flush_resize(&mut self) {
        if let Some(r) = &mut self.resize
            && r.dirty
        {
            r.dirty = false;
            self.record(Action::Resize, None, None);
        }
    }
    fn update_ticker(&mut self) {
        self.view.ticker_offset = 0.0;
        self.view.animation_pending = false;
        if !self.visible || self.reduced || !self.ticker_engaged || self.ticker_overflow <= 0.0 {
            return;
        }
        let Some(start) = self.ticker_start else {
            return;
        };
        self.view.animation_pending = true;
        let elapsed = self.now.saturating_sub(start) as f32;
        if elapsed < 600.0 {
            return;
        }
        let travel = self.ticker_overflow / 0.04;
        let phase = (elapsed - 600.0) % (travel * 2.0 + 5000.0);
        self.view.ticker_offset = if phase < travel {
            -phase * 0.04
        } else if phase < travel + 2000.0 {
            -self.ticker_overflow
        } else if phase < 2.0 * travel + 2000.0 {
            -self.ticker_overflow + (phase - travel - 2000.0) * 0.04
        } else {
            0.0
        };
    }
    pub fn tick(&mut self, now_ms: u64) {
        if self.view.disposed {
            return;
        }
        self.now = self.now.max(now_ms);
        if let Some((sequence, windows)) = self.pending.take() {
            self.view.pending_snapshots = 0;
            self.snapshot(sequence, windows);
        }
        self.flush_resize();
        if self
            .preview_show
            .as_ref()
            .is_some_and(|(deadline, _)| self.now >= *deadline)
            && let Some((_, id)) = self.preview_show.take()
            && self.windows.iter().any(|w| w.id == id)
        {
            self.view.preview = Some(id);
        }
        if self
            .preview_hide
            .is_some_and(|deadline| self.now >= deadline)
        {
            self.preview_hide = None;
            self.view.preview = None;
        }
        if self
            .gallery_show
            .as_ref()
            .is_some_and(|(deadline, _)| self.now >= *deadline)
            && let Some((_, k)) = self.gallery_show.take()
        {
            self.layout_gallery(&k);
        }
        if self
            .gallery_hide
            .is_some_and(|deadline| self.now >= deadline)
        {
            self.close_gallery();
        }
        self.update_ticker();
    }
    pub(crate) fn next_deadline(&self) -> Option<u64> {
        [
            self.preview_show.as_ref().map(|p| p.0),
            self.preview_hide,
            self.gallery_show.as_ref().map(|p| p.0),
            self.gallery_hide,
        ]
        .into_iter()
        .flatten()
        .min()
    }
    pub fn dispose(&mut self) {
        self.demo_focus_pending = false;
        self.demo_origin_focus = None;
        self.view.process_manager = None;
        self.demo_confirmation = None;
        self.demo_rows.clear();
        self.demo_filter.clear();
        self.dismiss();
        self.windows.clear();
        self.measured_group_centers.clear();
        self.draw_tickers.clear();
        self.draw_animation_pending = false;
        self.pins.clear();
        self.closed.clear();
        self.view.groups.clear();
        self.pending = None;
        self.drag = None;
        self.resize = None;
        self.ticker_start = None;
        self.view.animation_pending = false;
        self.view.ticker_offset = 0.0;
        self.view.pending_snapshots = 0;
        self.view.cache_bytes = 0;
        self.rects.clear();
        self.view.disposed = true;
    }
    pub(crate) fn load_scenario(&mut self, count: usize) {
        self.dismiss();
        self.input(Input::Pins(if count == 0 {
            vec![]
        } else {
            vec![
                Pin {
                    id: "demo-editor".into(),
                    name: "Editor".into(),
                },
                Pin {
                    id: "demo-browser".into(),
                    name: "Browser".into(),
                },
            ]
        }));
        let windows = (0..count)
            .map(|i| TaskWindow {
                id: format!("demo-{i}"),
                identity: 1,
                title: format!(
                    "Document {i} — a long simulated document title for overflow inspection"
                ),
                process: "Editor".into(),
                pid: Some(100),
                active: i == 0,
                minimized: i == 1,
                attention: i == 2,
                busy: false,
                toasts: if i == 2 { 7 } else { 0 },
            })
            .collect();
        self.input(Input::Snapshot {
            sequence: self.view.sequence.saturating_add(1),
            windows,
        });
    }
}
fn group_key(w: &TaskWindow) -> String {
    let p = w.process.trim().to_lowercase();
    if p.is_empty() {
        format!("window:{}", w.id)
    } else {
        p
    }
}
fn busy_eligible(w: &TaskWindow) -> bool {
    let p = w.process.trim().to_lowercase();
    let t = w.title.to_lowercase();
    [
        "terminal",
        "windowsterminal",
        "windows terminal",
        "wt",
        "cmd",
        "command prompt",
        "powershell",
        "pwsh",
        "conhost",
        "console",
        "opencode",
        "open code",
        "claude",
        "copilot",
        "cursor",
        "aider",
        "continue",
        "llm",
        "chatgpt",
        "codex",
    ]
    .iter()
    .any(|s| p.contains(s))
        || ([
            "firefox", "chrome", "msedge", "edge", "brave", "opera", "vivaldi",
        ]
        .iter()
        .any(|s| p.contains(s))
            && t.contains("download"))
}
