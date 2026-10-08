//! Safe M1 acceptance. No native handles, paths, services, or external actions.
use egui_bottom_bar::fixture::*;

// Capacity must be positive at compile time, not a constant-valued runtime test.
const _: () = assert!(CACHE_LIMIT_BYTES > 0);

fn window(id: &str, process: &str) -> TaskWindow {
    TaskWindow {
        id: id.into(),
        identity: 1,
        title: format!("{process} {id}"),
        process: process.into(),
        pid: Some(100),
        active: false,
        minimized: false,
        attention: false,
        busy: false,
        toasts: 0,
    }
}

fn app(windows: Vec<TaskWindow>) -> FixtureApp {
    let mut app = FixtureApp::default();
    app.input(Input::Snapshot {
        sequence: 12,
        windows,
    });
    app
}

fn group_app(count: usize) -> FixtureApp {
    app((0..count)
        .map(|i| window(&format!("w{i}"), "Editor"))
        .collect())
}

#[test]
fn sequenced_snapshots_reject_stale_equal_and_zero() {
    let mut a = app(vec![window("a", "Editor")]);
    for sequence in [0, 11, 12] {
        a.input(Input::Snapshot {
            sequence,
            windows: vec![window("bad", "Other")],
        });
        assert_eq!(a.view().sequence, 12);
        assert_eq!(a.view().groups[0].members, ["a"]);
    }
    a.input(Input::Snapshot {
        sequence: 13,
        windows: vec![window("b", "Other")],
    });
    assert_eq!(a.view().sequence, 13);
    assert_eq!(a.view().groups[0].members, ["b"]);
}

#[test]
fn grouping_normalization_fallback_and_aggregate_golden() {
    let mut first = window("a", " Editor ");
    first.toasts = 2;
    first.minimized = true;
    first.attention = true;
    first.busy = true; // Generic editor is NOT eligible.
    let mut second = window("b", "editor");
    second.toasts = 7;
    second.active = true;
    let a = app(vec![first, second, window("c", ""), window("d", "")]);
    let v = a.view();
    assert_eq!(
        v.groups.iter().map(|g| g.key.as_str()).collect::<Vec<_>>(),
        ["editor", "window:c", "window:d"]
    );
    let g = &v.groups[0];
    assert_eq!(g.members, ["a", "b"]);
    assert_eq!(g.toasts, 7); // max, not sum
    assert!(g.active && g.attention && g.capsule);
    assert!(!g.minimized && !g.busy);
    assert!(!v.groups[1].capsule);
}

#[test]
fn busy_eligibility_matches_current_process_title_contract() {
    for (process, title, eligible) in [
        ("pwsh", "cargo test", true),
        ("OpenCode", "Generating", true),
        ("chrome", "Downloading file", true),
        ("chrome", "Docs", false),
        ("notepad", "Chrome download OpenCode notes", false),
    ] {
        let mut w = window("a", process);
        w.title = title.into();
        w.busy = true;
        let a = app(vec![w]);
        assert_eq!(a.view().groups[0].busy, eligible, "{process}: {title}");
    }
}

#[test]
fn pressure_hysteresis_strict_boundaries() {
    let mut a = FixtureApp::default();
    for (available, required, expected) in [
        (100.0, 124.0, false),
        (100.0, 124.1, true),
        (147.9, 100.0, true),
        (148.0, 100.0, false),
    ] {
        a.input(Input::StripMetrics {
            available,
            required,
        });
        assert_eq!(a.view().pressure, expected);
    }
}

#[test]
fn height_locked_clamped_coalesced_and_canceled() {
    let mut a = FixtureApp::default();
    a.input(Input::HeightLocked(true));
    a.input(Input::ResizeStart { y: 200.0 });
    a.input(Input::ResizeMove { y: 0.0 });
    a.input(Input::ResizeEnd);
    assert_eq!(a.view().height, 32.4);
    assert!(a.ledger().is_empty());
    a.input(Input::HeightLocked(false));
    a.input(Input::ResizeStart { y: 200.0 });
    a.input(Input::ResizeMove { y: 190.0 });
    a.input(Input::ResizeMove { y: 0.0 });
    assert_eq!(a.view().height, 120.0);
    a.tick(16);
    assert_eq!(
        a.ledger()
            .iter()
            .filter(|e| e.action == Action::Resize)
            .count(),
        1
    );
    a.input(Input::ResizeCancel);
    assert_eq!(a.view().height, 32.4);
    a.input(Input::ResizeStart { y: 200.0 });
    a.input(Input::ResizeMove { y: 400.0 });
    a.input(Input::ResizeEnd);
    assert_eq!(a.view().height, 24.0);
}

#[test]
fn tile_toggle_uses_preclick_intent_and_all_actions_are_simulated() {
    for (active, minimized, expected) in [
        (true, false, Action::Minimize),
        (false, false, Action::Focus),
        (false, true, Action::Focus),
    ] {
        let mut w = window("a", "Editor");
        w.active = active;
        w.minimized = minimized;
        let mut a = app(vec![w]);
        a.input(Input::TileActivate {
            id: "a".into(),
            pre_click_active: active,
        });
        assert_eq!(a.ledger().len(), 1);
        assert_eq!(a.ledger()[0].action, expected);
        assert_eq!(a.ledger()[0].status, Outcome::Simulated);
    }
}

#[test]
fn preview_fake_clock_crossing_and_unavailable_actions() {
    let mut a = app(vec![window("a", "Editor")]);
    a.input(Input::HoverTask(Some("a".into())));
    a.tick(179);
    assert!(a.view().preview.is_none());
    a.tick(180);
    assert_eq!(a.view().preview.as_deref(), Some("a"));
    a.input(Input::HoverTask(None));
    a.tick(319);
    assert!(a.view().preview.is_some());
    a.input(Input::HoverPreview(true));
    a.tick(320);
    assert!(a.view().preview.is_some());
    a.input(Input::PreviewAction(Action::Maximize));
    a.input(Input::PreviewAction(Action::Close));
    assert_eq!(
        a.ledger().iter().map(|e| e.action).collect::<Vec<_>>(),
        [Action::Maximize, Action::Close]
    );
    assert!(a.ledger().iter().all(|e| e.status == Outcome::Simulated));
    a.input(Input::HoverPreview(false));
    a.tick(460);
    assert!(a.view().preview.is_none());
}

#[test]
fn newer_hover_cancels_obsolete_preview_timer() {
    let mut a = app(vec![window("a", "Editor"), window("b", "Browser")]);
    a.input(Input::HoverTask(Some("a".into())));
    a.tick(100);
    a.input(Input::HoverTask(Some("b".into())));
    a.tick(180);
    assert!(a.view().preview.is_none());
    a.tick(280);
    assert_eq!(a.view().preview.as_deref(), Some("b"));
}

#[test]
fn gallery_layout_counts_and_narrow_width() {
    for (count, width, mode, columns) in [
        (2, 584.0, LayoutMode::Ribbon, 2),
        (3, 584.0, LayoutMode::Ribbon, 3),
        (4, 584.0, LayoutMode::Matrix, 3),
        (7, 584.0, LayoutMode::Matrix, 3),
        (3, 320.0, LayoutMode::Matrix, 1),
    ] {
        let mut a = group_app(count);
        a.input(Input::Viewport {
            width,
            height: 328.0,
        });
        a.input(Input::OpenGallery("editor".into()));
        let v = a.view();
        let g = v.gallery.as_ref().unwrap();
        assert_eq!(g.mode, mode);
        assert_eq!(g.columns, columns);
        assert!(g.visible_rows <= 2);
        assert!(g.close_size >= 24.0);
    }
}

#[test]
fn gallery_hover_and_crossing_use_300_and_140_ms() {
    let mut a = group_app(3);
    a.input(Input::HoverGroup(Some("editor".into())));
    a.tick(299);
    assert!(a.view().gallery.is_none());
    a.tick(300);
    assert!(a.view().gallery.is_some());
    a.input(Input::HoverGroup(None));
    a.tick(439);
    assert!(a.view().gallery.is_some());
    a.input(Input::HoverGallery(true));
    a.tick(440);
    assert!(a.view().gallery.is_some());
    a.input(Input::HoverGallery(false));
    a.tick(580);
    assert!(a.view().gallery.is_none());
}

#[test]
fn gallery_spatial_navigation_menu_escape_and_duplicate_close() {
    let mut a = group_app(7);
    a.input(Input::OpenGallery("editor".into()));
    for (key, selected) in [
        (Key::End, 6),
        (Key::Up, 3),
        (Key::Right, 4),
        (Key::Down, 6),
        (Key::Right, 6),
        (Key::Home, 0),
    ] {
        a.input(Input::Key(key));
        assert_eq!(a.view().gallery.as_ref().unwrap().selected, selected);
    }
    a.input(Input::Key(Key::ContextMenu));
    assert!(a.view().menu_open);
    a.input(Input::Key(Key::Escape));
    assert!(!a.view().menu_open);
    assert!(a.view().gallery.is_some());
    a.input(Input::GalleryAction(Action::Close));
    a.input(Input::GalleryAction(Action::Close));
    assert_eq!(a.ledger().len(), 1);
    assert_eq!(a.ledger()[0].target.as_deref(), Some("w0"));
    a.input(Input::Key(Key::Escape));
    assert!(a.view().gallery.is_none());
}

#[test]
fn gallery_refresh_preserves_selected_survivor_not_reused_identity() {
    let mut a = group_app(7);
    a.input(Input::OpenGallery("editor".into()));
    a.input(Input::Key(Key::End));
    a.input(Input::Snapshot {
        sequence: 13,
        windows: vec![window("w6", "Editor"), window("w0", "Editor")],
    });
    assert_eq!(a.view().gallery.as_ref().unwrap().selected, 0);
    let mut reused = window("w6", "Editor");
    reused.identity = 2;
    a.input(Input::Snapshot {
        sequence: 14,
        windows: vec![window("w0", "Editor"), reused],
    });
    assert_eq!(a.view().gallery.as_ref().unwrap().selected, 0);
}

#[test]
fn group_menu_captures_members_excludes_newcomers_and_reused_identity() {
    let mut a = group_app(3);
    a.input(Input::GroupMenu("editor".into()));
    assert!(a.view().gallery.is_none());
    let mut reused = window("w1", "Editor");
    reused.identity = 2;
    a.input(Input::Snapshot {
        sequence: 13,
        windows: vec![window("w0", "Editor"), reused, window("new", "Editor")],
    });
    a.input(Input::MenuAction(Action::CloseAll));
    assert_eq!(a.ledger().len(), 1);
    assert_eq!(a.ledger()[0].action, Action::RequestClose);
    assert_eq!(a.ledger()[0].target.as_deref(), Some("w0"));
    assert_eq!(a.ledger()[0].status, Outcome::Simulated);
}

#[test]
fn standalone_disabled_actions_and_stale_target_never_dispatch() {
    let mut w = window("a", "Editor");
    w.pid = None;
    w.minimized = true;
    let mut a = app(vec![w]);
    a.input(Input::TaskMenu("a".into()));
    a.input(Input::MenuAction(Action::Minimize));
    assert!(a.ledger().is_empty());
    assert!(
        a.view().diagnostic.is_none(),
        "disabled action cannot report success"
    );
    a.input(Input::TaskMenu("a".into()));
    assert!(a.view().menu_open);
    a.input(Input::MenuAction(Action::Process));
    assert!(a.ledger().is_empty());
    assert!(
        a.view().diagnostic.is_none(),
        "missing PID cannot report process handoff"
    );
    a.input(Input::TaskMenu("a".into()));
    assert!(a.view().menu_open);
    a.input(Input::Snapshot {
        sequence: 13,
        windows: vec![],
    });
    a.input(Input::MenuAction(Action::Close));
    assert!(a.ledger().is_empty());
    assert!(
        a.view().diagnostic.is_none(),
        "stale action cannot report simulated success"
    );

    let mut reused = app(vec![window("a", "Editor")]);
    reused.input(Input::TaskMenu("a".into()));
    let mut replacement = window("a", "Editor");
    replacement.identity = 2;
    reused.input(Input::Snapshot {
        sequence: 13,
        windows: vec![replacement],
    });
    reused.input(Input::MenuAction(Action::Close));
    assert!(reused.ledger().is_empty());
    assert!(reused.view().diagnostic.is_none());

    let mut valid = app(vec![window("a", "Editor")]);
    valid.input(Input::TaskMenu("a".into()));
    valid.input(Input::MenuAction(Action::Minimize));
    assert_eq!(valid.ledger().len(), 1);
    assert_eq!(valid.ledger()[0].action, Action::Minimize);
    assert!(
        valid
            .view()
            .diagnostic
            .as_deref()
            .unwrap()
            .contains("Simulated")
    );
    valid.input(Input::TaskMenu("a".into()));
    valid.input(Input::MenuAction(Action::Process));
    assert_eq!(valid.ledger().len(), 1);
    assert!(
        valid
            .view()
            .diagnostic
            .as_deref()
            .unwrap()
            .contains("unavailable")
    );
    valid.input(Input::TaskMenu("a".into()));
    valid.input(Input::MenuAction(Action::Close));
    assert_eq!(valid.ledger().len(), 2);
    assert_eq!(valid.ledger()[1].action, Action::Close);
    assert!(
        valid
            .view()
            .diagnostic
            .as_deref()
            .unwrap()
            .contains("Simulated")
    );
}

#[test]
fn quick_launch_sort_wrap_failure_retry_and_admin_truth() {
    let mut a = FixtureApp::default();
    a.input(Input::Pins(vec![
        Pin {
            id: "z".into(),
            name: "Zulu".into(),
        },
        Pin {
            id: "a".into(),
            name: "Alpha".into(),
        },
    ]));
    a.input(Input::ToggleQuickLaunch);
    assert_eq!(a.view().quick_launch.as_ref().unwrap().pins, ["a", "z"]);
    a.input(Input::Key(Key::Up));
    assert_eq!(a.view().quick_launch.as_ref().unwrap().selected, Some(1));
    a.input(Input::LaunchOutcome(LaunchOutcome::Failed));
    a.input(Input::Key(Key::Enter));
    assert!(a.view().quick_launch.is_some());
    assert!(a.view().diagnostic.is_some());
    a.input(Input::LaunchOutcome(LaunchOutcome::Canceled));
    a.input(Input::Key(Key::ContextMenu));
    a.input(Input::MenuAction(Action::RunAs));
    assert!(a.view().quick_launch.is_some());
    a.input(Input::LaunchOutcome(LaunchOutcome::UnelevatedFallback));
    a.input(Input::Key(Key::ContextMenu));
    a.input(Input::MenuAction(Action::RunAs));
    assert!(a.view().quick_launch.is_none());
    assert_eq!(a.ledger().last().unwrap().elevated, Some(false));
    assert!(a.ledger().iter().all(|e| e.status == Outcome::Simulated));
}

#[test]
fn quick_launch_dismiss_empty_and_removed_pin_are_safe() {
    let mut a = FixtureApp::default();
    a.input(Input::ToggleQuickLaunch);
    a.input(Input::Key(Key::Enter));
    assert!(a.ledger().is_empty());
    a.input(Input::Key(Key::Escape));
    assert!(a.view().quick_launch.is_none());
    a.input(Input::Pins(vec![Pin {
        id: "a".into(),
        name: "Alpha".into(),
    }]));
    a.input(Input::ToggleQuickLaunch);
    a.input(Input::Pins(vec![]));
    a.input(Input::Key(Key::Enter));
    assert!(a.ledger().is_empty());
    a.input(Input::Blur);
    assert!(a.view().quick_launch.is_none());
}

#[test]
fn quick_launch_button_pointerdown_close_suppresses_trailing_click() {
    let mut a = FixtureApp::default();
    a.input(Input::ToggleQuickLaunch);
    a.input(Input::QuickLaunchButtonDown);
    assert!(a.view().quick_launch.is_none());
    a.input(Input::QuickLaunchButtonClick);
    assert!(a.view().quick_launch.is_none());
    a.input(Input::QuickLaunchButtonDown);
    a.input(Input::QuickLaunchButtonClick);
    assert!(a.view().quick_launch.is_some());
}

#[test]
fn repeated_gallery_cycles_do_not_retain_pending_resources() {
    let mut a = group_app(20);
    for _ in 0..100 {
        a.input(Input::OpenGallery("editor".into()));
        a.input(Input::Key(Key::End));
        a.input(Input::Key(Key::Escape));
        assert!(a.view().gallery.is_none());
        assert_eq!(a.view().thumbnail_destinations, 0);
        assert!(a.view().cache_bytes <= CACHE_LIMIT_BYTES);
    }
    assert!(a.ledger().is_empty());
}

#[test]
fn drag_threshold_cancel_and_click_suppression() {
    for (delta, cancel, actions) in [
        (5.9, false, 1),
        (6.0, false, 0),
        (120.0, false, 0),
        (120.0, true, 0),
    ] {
        let mut a = app(vec![window("a", "First"), window("b", "Second")]);
        a.input(Input::DragStart {
            group: "first".into(),
            x: 0.0,
        });
        a.input(Input::DragMove { x: delta });
        if cancel {
            a.input(Input::DragCancel);
        } else {
            a.input(Input::DragEnd);
        }
        a.input(Input::TrailingClick("first".into()));
        assert_eq!(a.ledger().len(), actions, "delta {delta}, cancel {cancel}");
        if cancel {
            assert_eq!(a.view().groups[0].key, "first");
        }
        if delta == 120.0 && !cancel {
            assert_eq!(a.view().groups[0].key, "second");
        }
    }
}

#[test]
fn ticker_fractional_overflow_phases_and_engagement() {
    let mut a = FixtureApp::default();
    a.input(Input::TickerMeasure {
        text_width: 120.5,
        viewport_width: 100.0,
    });
    a.input(Input::TickerEngage {
        hover: true,
        focus: false,
    });
    for (ms, offset) in [
        (599, 0.0),
        (600, 0.0),
        (850, -10.0),
        (1113, -20.5),
        (3113, -20.48),
        (3625, 0.0),
        (6625, 0.0),
        (6875, -10.0),
    ] {
        a.tick(ms);
        assert!((a.view().ticker_offset - offset).abs() < 0.1, "at {ms}");
    }
    a.input(Input::TickerEngage {
        hover: false,
        focus: true,
    });
    a.tick(7000);
    assert!(a.view().ticker_offset < 0.0);
    a.input(Input::ReducedMotion(true));
    assert_eq!(a.view().ticker_offset, 0.0);
    a.input(Input::TickerEngage {
        hover: false,
        focus: false,
    });
    a.tick(9000);
    assert_eq!(a.view().ticker_offset, 0.0);
}

#[test]
fn drag_displacement_waits_until_source_center_passes_neighbor_center() {
    // Current TS golden: moved center must be strictly beyond neighbor center.
    // Default model slots are 100px: source center 50, neighbor center 150.
    // The 6px drag threshold suppresses click; it does NOT halve displacement.
    for (delta, first_group) in [(50.1, "first"), (100.0, "first"), (100.1, "second")] {
        let mut a = app(vec![window("a", "First"), window("b", "Second")]);
        a.input(Input::DragStart {
            group: "first".into(),
            x: 0.0,
        });
        a.input(Input::DragMove { x: delta });
        a.input(Input::DragEnd);
        assert_eq!(a.view().groups[0].key, first_group, "displacement {delta}");
        assert!(a.ledger().is_empty());
    }
}

#[test]
fn ticker_fitting_hidden_and_disengaged_are_static() {
    let mut a = FixtureApp::default();
    a.input(Input::TickerMeasure {
        text_width: 99.5,
        viewport_width: 100.0,
    });
    a.input(Input::TickerEngage {
        hover: true,
        focus: true,
    });
    a.tick(5000);
    assert_eq!(a.view().ticker_offset, 0.0);
    a.input(Input::TickerMeasure {
        text_width: 200.0,
        viewport_width: 100.0,
    });
    a.input(Input::Visible(false));
    a.tick(9000);
    assert_eq!(a.view().ticker_offset, 0.0);
    assert!(!a.view().animation_pending);
}

#[test]
fn processes_truthful_unavailable_and_no_live_capabilities() {
    let mut a = FixtureApp::default();
    assert!(!a.view().live_capabilities);
    a.input(Input::Processes);
    assert!(
        a.view()
            .diagnostic
            .as_deref()
            .unwrap()
            .to_lowercase()
            .contains("unavailable")
    );
    assert!(a.ledger().is_empty());
}

#[test]
fn snapshot_stress_coalesces_and_disposal_rejects_late_work() {
    let mut a = FixtureApp::default();
    for sequence in 1..=100 {
        a.queue_snapshot(
            sequence,
            (0..1000)
                .map(|i| window(&format!("w{i}"), "Editor"))
                .collect(),
        );
        assert!(a.view().pending_snapshots <= 1);
    }
    a.tick(16);
    assert_eq!(a.view().sequence, 100);
    assert_eq!(a.view().groups[0].members.len(), 1000);
    assert!(a.view().thumbnail_destinations <= 12);
    assert!(a.view().cache_bytes <= CACHE_LIMIT_BYTES);
    a.input(Input::HoverGroup(Some("editor".into())));
    a.dispose();
    a.tick(1000);
    a.queue_snapshot(101, vec![window("late", "Late")]);
    a.input(Input::Processes);
    let v = a.view();
    assert!(v.disposed && v.gallery.is_none() && v.preview.is_none());
    assert_eq!(v.pending_snapshots, 0);
    assert_eq!(v.thumbnail_destinations, 0);
    assert_eq!(v.cache_bytes, 0);
    assert!(!v.animation_pending);
    assert!(a.ledger().is_empty());
}
