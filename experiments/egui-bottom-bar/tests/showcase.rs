//! Populated, visibly simulated showcase; no real process/pin/launch authority.
use egui::{Context, Event, Key, Modifiers, PointerButton, RawInput, Rect, pos2, vec2};
use egui_bottom_bar::fixture::*;

fn frame(ctx: &Context, a: &mut FixtureApp, time: f64, events: Vec<Event>) -> egui::FullOutput {
    let modifiers = events
        .iter()
        .find_map(|e| match e {
            Event::Key { modifiers, .. } => Some(*modifiers),
            _ => None,
        })
        .unwrap_or(Modifiers::NONE);
    let output = ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0))),
            time: Some(time),
            events,
            modifiers,
            ..Default::default()
        },
        |ctx| a.draw(ctx),
    );
    assert!(!output.shapes.is_empty());
    output
}

fn click(ctx: &Context, a: &mut FixtureApp, label: &str, time: f64, button: PointerButton) {
    let p = a
        .widget_rect(label)
        .expect("actual semantic widget response")
        .center();
    for (pressed, time) in [(true, time), (false, time + 0.01)] {
        frame(
            ctx,
            a,
            time,
            vec![
                Event::PointerMoved(p),
                Event::PointerButton {
                    pos: p,
                    button,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
}

fn key(ctx: &Context, a: &mut FixtureApp, key: Key, time: f64) {
    for (pressed, time) in [(true, time), (false, time + 0.01)] {
        frame(
            ctx,
            a,
            time,
            vec![Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
    }
}

fn focus_by_tab(ctx: &Context, a: &mut FixtureApp, label: &str, time: f64) {
    for i in 0..40 {
        key(ctx, a, Key::Tab, time + i as f64 * 0.02);
        if ctx
            .memory(|m| m.focused())
            .and_then(|id| ctx.read_response(id))
            .is_some_and(|r| Some(r.rect) == a.widget_rect(label))
        {
            return;
        }
    }
    panic!("keyboard must reach {label}");
}

fn painted_text(output: &egui::FullOutput, fragment: &str) -> bool {
    output.shapes.iter().any(|shape| {
        matches!(&shape.shape,
        egui::epaint::Shape::Text(t) if t.galley.job.text.contains(fragment))
    })
}

fn open_processes() -> FixtureApp {
    let mut a = FixtureApp::showcase();
    a.input(Input::Processes);
    assert!(a.view().process_manager.is_some());
    a
}

#[test]
fn showcase_is_populated_mixed_while_default_remains_empty_and_safe() {
    let a = FixtureApp::showcase();
    assert!(!a.view().live_capabilities);
    assert!(
        a.view()
            .groups
            .iter()
            .any(|g| !g.capsule && g.members == ["demo-browser"])
    );
    assert!(
        a.view()
            .groups
            .iter()
            .any(|g| !g.capsule && g.members == ["demo-terminal"])
    );
    let editor = a.view().groups.iter().find(|g| g.key == "editor").unwrap();
    assert!(editor.capsule && editor.members.len() == 3);
    assert!(a.ledger().is_empty());
    let mut empty = FixtureApp::default();
    assert!(empty.view().groups.is_empty());
    empty.input(Input::Processes);
    assert!(empty.view().process_manager.is_none());
    assert!(
        empty
            .view()
            .diagnostic
            .as_deref()
            .unwrap()
            .contains("unavailable")
    );
}

#[test]
fn dummy_process_rows_are_finite_and_filter_sort_refresh_are_local() {
    let mut a = open_processes();
    let pm = a.view().process_manager.as_ref().unwrap();
    assert!(pm.rows.len() >= 3);
    assert!(
        pm.rows
            .iter()
            .all(|r| r.cpu_percent.is_finite() && (0.0..=100.0).contains(&r.cpu_percent))
    );
    assert!(pm.rows.iter().any(|r| !r.killable));
    let revision = pm.revision;
    a.input(Input::DemoProcessFilter("bRoWsEr".into()));
    let pm = a.view().process_manager.as_ref().unwrap();
    assert!(!pm.rows.is_empty());
    assert!(
        pm.rows
            .iter()
            .all(|r| r.name.to_lowercase().contains("browser"))
    );
    a.input(Input::DemoProcessFilter(
        "no fixture name matches this".into(),
    ));
    assert!(a.view().process_manager.as_ref().unwrap().rows.is_empty());
    a.input(Input::DemoProcessFilter(String::new()));
    a.input(Input::DemoProcessSort(DemoProcessSort::CpuDescending));
    assert!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .rows
            .windows(2)
            .all(|rows| rows[0].cpu_percent >= rows[1].cpu_percent)
    );
    a.input(Input::DemoProcessRefresh);
    assert!(a.view().process_manager.as_ref().unwrap().revision > revision);
    assert!(a.ledger().is_empty());
    assert!(!a.view().live_capabilities);
}

#[test]
fn task_and_group_process_entries_target_fake_rows_only() {
    let mut a = FixtureApp::showcase();
    for (group, target, expected) in [
        (false, "demo-browser", "fake-browser"),
        (true, "editor", "fake-editor"),
    ] {
        if group {
            a.input(Input::GroupMenu(target.into()));
        } else {
            a.input(Input::TaskMenu(target.into()));
        }
        a.input(Input::MenuAction(Action::Process));
        let pm = a.view().process_manager.as_ref().unwrap();
        assert_eq!(pm.target.as_deref(), Some(expected));
        assert_eq!(pm.selected.as_deref(), Some(expected));
        assert!(pm.rows.iter().any(|r| r.id == expected));
        a.input(Input::DemoProcessClose);
    }
    assert!(a.ledger().is_empty());
}

#[test]
fn missing_pid_and_stale_task_menu_cannot_open_targeted_dummy() {
    for reused in [false, true] {
        let mut a = FixtureApp::showcase();
        let w = TaskWindow {
            id: "isolated".into(),
            identity: 1,
            title: "No PID".into(),
            process: "Editor".into(),
            pid: None,
            active: false,
            minimized: false,
            attention: false,
            busy: false,
            toasts: 0,
        };
        a.input(Input::Snapshot {
            sequence: 50,
            windows: vec![w.clone()],
        });
        a.input(Input::TaskMenu("isolated".into()));
        a.input(Input::MenuAction(Action::Process));
        assert!(a.view().process_manager.is_none());
        let mut valid = w;
        valid.pid = Some(100);
        a.input(Input::Snapshot {
            sequence: 51,
            windows: vec![valid.clone()],
        });
        a.input(Input::TaskMenu("isolated".into()));
        valid.identity = 2;
        a.input(Input::Snapshot {
            sequence: 52,
            windows: if reused { vec![valid] } else { vec![] },
        });
        a.input(Input::MenuAction(Action::Process));
        assert!(a.view().process_manager.is_none());
        assert!(a.ledger().is_empty());
    }
}

#[test]
fn fake_end_requires_selection_confirmation_and_cancel_is_inert() {
    let mut a = open_processes();
    assert!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .selected
            .is_none()
    );
    a.input(Input::DemoProcessEnd);
    assert!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .confirmation
            .is_none()
    );
    assert!(a.ledger().is_empty());
    a.input(Input::DemoProcessSelect("fake-browser".into()));
    a.input(Input::DemoProcessEnd);
    assert_eq!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .confirmation
            .as_deref(),
        Some("fake-browser")
    );
    assert!(a.ledger().is_empty());
    a.input(Input::DemoProcessConfirm(false));
    assert!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .confirmation
            .is_none()
    );
    assert!(a.ledger().is_empty());
    a.input(Input::DemoProcessEnd);
    a.input(Input::DemoProcessConfirm(true));
    a.input(Input::DemoProcessConfirm(true));
    assert_eq!(a.ledger().len(), 1);
    assert_eq!(a.ledger()[0].action, Action::EndDemoProcess);
    assert_eq!(a.ledger()[0].target.as_deref(), Some("fake-browser"));
    assert_eq!(a.ledger()[0].status, Outcome::Simulated);
    assert!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .all(|r| r.id != "fake-browser")
    );
    assert!(
        a.view()
            .groups
            .iter()
            .any(|g| g.members.contains(&"demo-browser".into())),
        "fake end cannot mutate authoritative task snapshot"
    );
}

#[test]
fn protected_and_reused_fake_process_confirmation_cannot_dispatch() {
    let mut a = open_processes();
    let protected = a
        .view()
        .process_manager
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .find(|r| !r.killable)
        .unwrap()
        .id
        .clone();
    a.input(Input::DemoProcessSelect(protected));
    a.input(Input::DemoProcessEnd);
    assert!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .confirmation
            .is_none()
    );
    assert!(a.ledger().is_empty());
    a.input(Input::DemoProcessSelect("fake-browser".into()));
    a.input(Input::DemoProcessEnd);
    let mut replacement = a.view().process_manager.as_ref().unwrap().rows.clone();
    replacement
        .iter_mut()
        .find(|r| r.id == "fake-browser")
        .unwrap()
        .identity += 1;
    a.input(Input::DemoProcesses(replacement));
    a.input(Input::DemoProcessConfirm(true));
    assert!(a.ledger().is_empty());
}

#[test]
fn popup_escape_closes_confirmation_first_then_popup_and_disposal_rejects_work() {
    let mut a = open_processes();
    a.input(Input::DemoProcessSelect("fake-browser".into()));
    a.input(Input::DemoProcessEnd);
    a.input(Input::Key(egui_bottom_bar::fixture::Key::Escape));
    assert!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .confirmation
            .is_none()
    );
    a.input(Input::Key(egui_bottom_bar::fixture::Key::Escape));
    assert!(a.view().process_manager.is_none());
    a.input(Input::Processes);
    a.dispose();
    a.input(Input::DemoProcessRefresh);
    a.input(Input::DemoProcessConfirm(true));
    assert!(a.view().process_manager.is_none());
    assert!(a.ledger().is_empty());
}

#[test]
fn raw_showcase_hover_artwork_then_process_popup_filter_refresh_and_keyboard_dismiss() {
    let ctx = Context::default();
    let mut a = FixtureApp::showcase();
    frame(&ctx, &mut a, 0.0, vec![]);
    assert!(a.widget_rect("Editor, 3 windows").is_some());
    let p = a.widget_rect("Browser — Downloads").unwrap().center();
    frame(&ctx, &mut a, 0.1, vec![Event::PointerMoved(p)]);
    frame(&ctx, &mut a, 0.279, vec![]);
    assert!(a.view().preview.is_none());
    let preview = frame(&ctx, &mut a, 0.280, vec![]);
    assert_eq!(a.view().preview.as_deref(), Some("demo-browser"));
    assert!(painted_text(&preview, "Simulated preview"));
    let artwork = a
        .widget_rect("Simulated preview artwork")
        .expect("measured generated illustration canvas");
    assert!(artwork.width() > 100.0 && artwork.height() > 80.0);
    assert!(
        preview
            .shapes
            .iter()
            .filter(|s| match &s.shape {
                egui::epaint::Shape::Rect(rect) => artwork.contains_rect(rect.rect),
                _ => false,
            })
            .count()
            >= 3,
        "generated illustration has actual canvas-local graphic blocks, not just placeholder text"
    );
    click(&ctx, &mut a, "Processes", 0.4, PointerButton::Primary);
    let popup = frame(&ctx, &mut a, 0.42, vec![]);
    assert!(painted_text(&popup, "Simulated Process Manager"));
    assert!(a.view().process_manager.is_some());
    click(
        &ctx,
        &mut a,
        "Filter simulated processes",
        0.5,
        PointerButton::Primary,
    );
    frame(&ctx, &mut a, 0.52, vec![Event::Text("browser".into())]);
    assert!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .all(|r| r.name.to_lowercase().contains("browser"))
    );
    assert!(!a.view().process_manager.as_ref().unwrap().rows.is_empty());
    let revision = a.view().process_manager.as_ref().unwrap().revision;
    click(
        &ctx,
        &mut a,
        "Refresh simulated processes",
        0.6,
        PointerButton::Primary,
    );
    assert!(a.view().process_manager.as_ref().unwrap().revision > revision);
    focus_by_tab(&ctx, &mut a, "Dismiss Simulated Process Manager", 0.7);
    key(&ctx, &mut a, Key::Enter, 1.6);
    assert!(a.view().process_manager.is_none());
    assert!(a.ledger().is_empty());
    assert!(!a.view().live_capabilities);
}

#[test]
fn raw_task_context_menu_opens_targeted_dummy_not_primary_action() {
    let ctx = Context::default();
    let mut a = FixtureApp::showcase();
    frame(&ctx, &mut a, 0.0, vec![]);
    click(
        &ctx,
        &mut a,
        "Browser — Downloads",
        0.1,
        PointerButton::Secondary,
    );
    assert!(a.view().menu_open);
    assert!(a.ledger().is_empty());
    frame(&ctx, &mut a, 0.12, vec![]);
    focus_by_tab(&ctx, &mut a, "Open Simulated Process Manager", 0.2);
    key(&ctx, &mut a, Key::Enter, 1.1);
    let pm = a.view().process_manager.as_ref().unwrap();
    assert_eq!(pm.selected.as_deref(), Some("fake-browser"));
    assert_eq!(pm.target.as_deref(), Some("fake-browser"));
    assert!(a.ledger().is_empty());
    key(&ctx, &mut a, Key::Escape, 1.2);
    assert!(a.view().process_manager.is_none());
}

#[test]
fn raw_group_menu_target_fake_end_cancel_then_confirm_is_simulated_only() {
    let ctx = Context::default();
    let mut a = FixtureApp::showcase();
    frame(&ctx, &mut a, 0.0, vec![]);
    click(
        &ctx,
        &mut a,
        "Editor, 3 windows",
        0.1,
        PointerButton::Secondary,
    );
    assert!(a.view().menu_open && a.view().gallery.is_none());
    assert!(a.ledger().is_empty());
    frame(&ctx, &mut a, 0.12, vec![]);
    focus_by_tab(&ctx, &mut a, "Open Simulated Process Manager", 0.2);
    key(&ctx, &mut a, Key::Enter, 1.1);
    assert_eq!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .selected
            .as_deref(),
        Some("fake-editor")
    );
    click(
        &ctx,
        &mut a,
        "End simulated process",
        1.2,
        PointerButton::Primary,
    );
    assert_eq!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .confirmation
            .as_deref(),
        Some("fake-editor")
    );
    assert!(a.ledger().is_empty());
    key(&ctx, &mut a, Key::Escape, 1.3);
    assert!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .confirmation
            .is_none()
    );
    assert!(a.ledger().is_empty());
    click(
        &ctx,
        &mut a,
        "End simulated process",
        1.4,
        PointerButton::Primary,
    );
    frame(&ctx, &mut a, 1.42, vec![]);
    click(
        &ctx,
        &mut a,
        "Confirm simulated end",
        1.5,
        PointerButton::Primary,
    );
    assert_eq!(a.ledger().len(), 1);
    assert_eq!(a.ledger()[0].action, Action::EndDemoProcess);
    assert_eq!(a.ledger()[0].status, Outcome::Simulated);
    assert_eq!(a.ledger()[0].target.as_deref(), Some("fake-editor"));
    assert_eq!(
        a.view()
            .groups
            .iter()
            .find(|g| g.key == "editor")
            .unwrap()
            .members
            .len(),
        3
    );
    assert!(!a.view().live_capabilities);
}

#[test]
fn raw_showcase_gallery_contains_illustrative_simulated_cards() {
    let ctx = Context::default();
    let mut a = FixtureApp::showcase();
    frame(&ctx, &mut a, 0.0, vec![]);
    click(
        &ctx,
        &mut a,
        "Editor, 3 windows",
        0.1,
        PointerButton::Primary,
    );
    let output = frame(&ctx, &mut a, 0.12, vec![]);
    assert_eq!(a.view().gallery.as_ref().unwrap().mode, LayoutMode::Ribbon);
    assert!(painted_text(&output, "Simulated preview"));
    for id in ["demo-editor-1", "demo-editor-2", "demo-editor-3"] {
        let rect = a
            .widget_rect(&format!("Simulated gallery artwork {id}"))
            .expect("actual gallery illustration canvas");
        assert!(rect.width() > 100.0 && rect.height() > 80.0);
        assert!(
            output
                .shapes
                .iter()
                .filter(|s| match &s.shape {
                    egui::epaint::Shape::Rect(block) => rect.contains_rect(block.rect),
                    _ => false,
                })
                .count()
                >= 3
        );
    }
    assert_eq!(a.view().thumbnail_destinations, 0);
    assert!(!a.view().live_capabilities);
    assert!(a.ledger().is_empty());
}

#[test]
fn fake_rows_are_bounded_normalized_and_removed_confirmation_is_inert() {
    let mut a = open_processes();
    a.input(Input::DemoProcessSelect("fake-browser".into()));
    a.input(Input::DemoProcessEnd);
    assert!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .confirmation
            .is_some()
    );
    let survivors = a
        .view()
        .process_manager
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .filter(|r| r.id != "fake-browser")
        .cloned()
        .collect();
    a.input(Input::DemoProcesses(survivors));
    a.input(Input::DemoProcessConfirm(true));
    assert!(
        a.ledger().is_empty(),
        "removed fake target cannot be confirmed"
    );
    assert!(
        a.view()
            .process_manager
            .as_ref()
            .unwrap()
            .confirmation
            .is_none()
    );

    let rows = (0..1000)
        .map(|i| DemoProcess {
            id: format!("fake-stress-{i}"),
            identity: 1,
            name: "x".repeat(500),
            cpu_percent: if i % 2 == 0 { f32::NAN } else { 200.0 },
            memory_mib: 1,
            killable: false,
        })
        .collect();
    a.input(Input::DemoProcesses(rows));
    let rows = &a.view().process_manager.as_ref().unwrap().rows;
    assert_eq!(rows.len(), 256, "declared dummy process row cap");
    assert!(rows.iter().all(|r| r.name.chars().count() <= 256));
    assert!(
        rows.iter()
            .all(|r| r.cpu_percent.is_finite() && (0.0..=100.0).contains(&r.cpu_percent))
    );
    assert!(a.ledger().is_empty());
    assert!(!a.view().live_capabilities);
}

#[test]
fn direct_processes_dismiss_restores_current_origin_not_previous_task_menu() {
    let ctx = Context::default();
    let mut a = FixtureApp::showcase();
    frame(&ctx, &mut a, 0.0, vec![]);
    focus_by_tab(&ctx, &mut a, "Browser — Downloads", 0.1);
    let browser_id = ctx.memory(|m| m.focused()).expect("Browser focused by Tab");
    click(
        &ctx,
        &mut a,
        "Browser — Downloads",
        1.0,
        PointerButton::Secondary,
    );
    assert!(a.view().menu_open);
    frame(&ctx, &mut a, 1.02, vec![]);
    focus_by_tab(&ctx, &mut a, "Open Simulated Process Manager", 1.1);
    key(&ctx, &mut a, Key::Enter, 2.0);
    assert!(a.view().process_manager.is_some());
    key(&ctx, &mut a, Key::Escape, 2.1);
    assert!(a.view().process_manager.is_none());
    assert_eq!(
        ctx.memory(|m| m.focused()),
        Some(browser_id),
        "positive control: task-menu popup dismissal restores Browser origin"
    );

    focus_by_tab(&ctx, &mut a, "Processes", 2.2);
    let processes_id = ctx
        .memory(|m| m.focused())
        .expect("Processes focused by Tab");
    assert_ne!(processes_id, browser_id);
    key(&ctx, &mut a, Key::Enter, 3.1);
    assert!(a.view().process_manager.is_some());
    key(&ctx, &mut a, Key::Escape, 3.2);
    assert!(a.view().process_manager.is_none());
    assert_eq!(
        ctx.memory(|m| m.focused()),
        Some(processes_id),
        "direct Processes dismissal restores this invocation's origin, not stale Browser menu origin"
    );
    assert!(a.ledger().is_empty());
    assert!(!a.view().live_capabilities);
}
