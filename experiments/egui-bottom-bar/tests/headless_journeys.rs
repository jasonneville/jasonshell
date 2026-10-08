//! Native egui RawInput frames, not live desktop focus, Narrator, GPU or shell E2E.
use egui::{Context, Event, Key, Modifiers, PointerButton, RawInput, Rect, pos2, vec2};
use egui_bottom_bar::fixture::{
    Action, FixtureApp, Input, LaunchOutcome, Outcome, Pin, TaskWindow,
};

fn frame(ctx: &Context, app: &mut FixtureApp, time: f64, events: Vec<Event>) -> egui::FullOutput {
    let modifiers = events
        .iter()
        .find_map(|event| match event {
            Event::Key { modifiers, .. } => Some(*modifiers),
            _ => None,
        })
        .unwrap_or(Modifiers::NONE);
    let output = ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0))),
            time: Some(time),
            modifiers,
            events,
            ..Default::default()
        },
        |ctx| app.draw(ctx),
    );
    assert!(
        !output.shapes.is_empty(),
        "must paint fixture UI, not only reducer"
    );
    output
}

fn click(ctx: &Context, app: &mut FixtureApp, label: &str, time: f64) {
    // Rects must come from actual widget Responses; labels are semantic names.
    let p = app
        .widget_rect(label)
        .expect("visible semantic widget")
        .center();
    frame(
        ctx,
        app,
        time,
        vec![
            Event::PointerMoved(p),
            Event::PointerButton {
                pos: p,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    frame(
        ctx,
        app,
        time + 0.01,
        vec![Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
}

fn key(ctx: &Context, app: &mut FixtureApp, key: Key, time: f64) {
    frame(
        ctx,
        app,
        time,
        vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    frame(
        ctx,
        app,
        time + 0.01,
        vec![Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
}

#[test]
fn pointer_open_keyboard_select_failure_retry_success_and_dismiss() {
    let ctx = Context::default();
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
    a.input(Input::LaunchOutcome(LaunchOutcome::Failed));
    frame(&ctx, &mut a, 0.0, vec![]);
    click(&ctx, &mut a, "Quick Launch", 0.1);
    assert!(a.view().quick_launch.is_some());
    key(&ctx, &mut a, Key::End, 0.2);
    assert_eq!(a.view().quick_launch.as_ref().unwrap().selected, Some(1));
    key(&ctx, &mut a, Key::Enter, 0.3);
    assert!(a.view().quick_launch.is_some());
    assert!(a.view().diagnostic.is_some());
    assert_eq!(a.ledger()[0].target.as_deref(), Some("z"));
    a.input(Input::LaunchOutcome(LaunchOutcome::Success));
    key(&ctx, &mut a, Key::Enter, 0.4);
    assert!(a.view().quick_launch.is_none());
    assert_eq!(a.ledger().len(), 2);
    assert!(a.ledger().iter().all(|e| e.status == Outcome::Simulated));
    click(&ctx, &mut a, "Quick Launch", 0.5);
    key(&ctx, &mut a, Key::Escape, 0.6);
    assert!(a.view().quick_launch.is_none());
    assert_eq!(a.ledger().len(), 2);
}

#[test]
fn capsule_open_spatial_keys_separate_close_escape_then_processes() {
    let ctx = Context::default();
    let mut a = FixtureApp::default();
    a.input(Input::Snapshot {
        sequence: 1,
        windows: (0..7)
            .map(|i| TaskWindow {
                id: format!("w{i}"),
                identity: 1,
                title: format!("Document {i}"),
                process: "Editor".into(),
                pid: Some(100),
                active: false,
                minimized: false,
                attention: false,
                busy: false,
                toasts: 0,
            })
            .collect(),
    });
    frame(&ctx, &mut a, 0.0, vec![]);
    click(&ctx, &mut a, "Editor, 7 windows", 0.1);
    assert!(a.view().gallery.is_some());
    key(&ctx, &mut a, Key::End, 0.2);
    assert_eq!(a.view().gallery.as_ref().unwrap().selected, 6);
    click(&ctx, &mut a, "Close Document 6", 0.3);
    assert_eq!(a.ledger().len(), 1);
    assert_eq!(a.ledger()[0].action, Action::Close);
    assert_eq!(a.ledger()[0].status, Outcome::Simulated);
    key(&ctx, &mut a, Key::Escape, 0.4);
    assert!(a.view().gallery.is_none());
    click(&ctx, &mut a, "Processes", 0.5);
    assert!(
        a.view()
            .diagnostic
            .as_deref()
            .unwrap()
            .to_lowercase()
            .contains("unavailable")
    );
    assert_eq!(a.ledger().len(), 1);
}

#[test]
fn keyboard_admin_menu_cancel_then_truthful_unelevated_fallback() {
    let ctx = Context::default();
    let mut a = FixtureApp::default();
    a.input(Input::Pins(vec![Pin {
        id: "a".into(),
        name: "Alpha".into(),
    }]));
    a.input(Input::LaunchOutcome(LaunchOutcome::Canceled));
    frame(&ctx, &mut a, 0.0, vec![]);
    click(&ctx, &mut a, "Quick Launch", 0.1);
    for (time, outcome) in [
        (0.2, LaunchOutcome::Canceled),
        (0.4, LaunchOutcome::UnelevatedFallback),
    ] {
        a.input(Input::LaunchOutcome(outcome));
        frame(
            &ctx,
            &mut a,
            time,
            vec![Event::Key {
                key: Key::F10,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::SHIFT,
            }],
        );
        assert!(a.view().menu_open);
        key(&ctx, &mut a, Key::Enter, time + 0.05);
        assert!(!a.view().menu_open);
        assert_eq!(a.ledger().last().unwrap().action, Action::RunAs);
        assert_eq!(a.ledger().last().unwrap().status, Outcome::Simulated);
        assert_eq!(a.ledger().last().unwrap().elevated, Some(false));
        assert_eq!(
            a.view().quick_launch.is_some(),
            outcome == LaunchOutcome::Canceled
        );
    }
    assert_eq!(a.ledger().len(), 2);
}

fn task(id: &str, process: &str, title: &str) -> TaskWindow {
    TaskWindow {
        id: id.into(),
        identity: 1,
        title: title.into(),
        process: process.into(),
        pid: Some(100),
        active: false,
        minimized: false,
        attention: false,
        busy: false,
        toasts: 0,
    }
}

fn with_tasks(windows: Vec<TaskWindow>) -> FixtureApp {
    let mut a = FixtureApp::default();
    a.input(Input::Snapshot {
        sequence: 1,
        windows,
    });
    a
}

fn modified_key(ctx: &Context, a: &mut FixtureApp, native: Key, modifiers: Modifiers, time: f64) {
    for (pressed, time) in [(true, time), (false, time + 0.01)] {
        frame(
            ctx,
            a,
            time,
            vec![Event::Key {
                key: native,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            }],
        );
    }
}

fn focus_by_tab(ctx: &Context, a: &mut FixtureApp, label: &str, time: f64) {
    for step in 0..30 {
        key(ctx, a, Key::Tab, time + step as f64 * 0.02);
        if ctx
            .memory(|m| m.focused())
            .and_then(|id| ctx.read_response(id))
            .is_some_and(|response| Some(response.rect) == a.widget_rect(label))
        {
            return;
        }
    }
    panic!("Tab must reach visible widget {label}");
}

fn secondary_menu(mut a: FixtureApp, label: &str) {
    let ctx = Context::default();
    frame(&ctx, &mut a, 0.0, vec![]);
    let p = a.widget_rect(label).unwrap().center();
    for (pressed, time) in [(true, 0.1), (false, 0.11)] {
        frame(
            &ctx,
            &mut a,
            time,
            vec![
                Event::PointerMoved(p),
                Event::PointerButton {
                    pos: p,
                    button: PointerButton::Secondary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    assert!(a.view().menu_open, "secondary gesture opens menu");
    assert!(
        a.ledger().is_empty(),
        "secondary gesture must not activate/minimize"
    );
    assert!(
        a.view().gallery.is_none(),
        "secondary gesture must not open gallery"
    );
}

#[test]
fn secondary_single_opens_only_menu_without_primary_action() {
    secondary_menu(
        with_tasks(vec![task("a", "Editor", "Document")]),
        "Document",
    );
}

#[test]
fn secondary_capsule_opens_only_menu_without_gallery() {
    secondary_menu(
        with_tasks(vec![
            task("a", "Editor", "First"),
            task("b", "Editor", "Second"),
        ]),
        "Editor, 2 windows",
    );
}

#[test]
fn duplicate_gallery_titles_enter_dispatches_only_selected_identity() {
    let control_ctx = Context::default();
    let mut control = with_tasks(vec![
        task("a", "Editor", "First"),
        task("b", "Editor", "Second"),
    ]);
    frame(&control_ctx, &mut control, 0.0, vec![]);
    click(&control_ctx, &mut control, "Editor, 2 windows", 0.1);
    key(&control_ctx, &mut control, Key::Enter, 0.2);
    assert_eq!(
        control.ledger().len(),
        1,
        "positive control: initial selected activation receives Enter"
    );
    assert_eq!(control.ledger()[0].target.as_deref(), Some("a"));
    assert_eq!(control.ledger()[0].action, Action::Focus);
    let ctx = Context::default();
    let mut a = with_tasks(vec![
        task("a", "Editor", "Duplicate"),
        task("b", "Editor", "Duplicate"),
    ]);
    frame(&ctx, &mut a, 0.0, vec![]);
    click(&ctx, &mut a, "Editor, 2 windows", 0.1);
    assert_eq!(a.view().gallery.as_ref().unwrap().selected, 0);
    key(&ctx, &mut a, Key::Enter, 0.2);
    assert_eq!(
        a.ledger().len(),
        1,
        "duplicate labels cannot share interaction identity"
    );
    assert_eq!(a.ledger()[0].target.as_deref(), Some("a"));
    assert_eq!(a.ledger()[0].action, Action::Focus);
}

#[test]
fn gallery_focused_close_survives_title_refresh_for_same_identity() {
    let ctx = Context::default();
    let mut a = with_tasks(vec![
        task("a", "Editor", "First"),
        task("b", "Editor", "Second"),
    ]);
    frame(&ctx, &mut a, 0.0, vec![]);
    click(&ctx, &mut a, "Editor, 2 windows", 0.1);
    key(&ctx, &mut a, Key::Tab, 0.2);
    let close_focus = ctx
        .memory(|m| m.focused())
        .expect("selected Close focus after Tab");
    // Positive control on isolated app proves Tab selected Close, not another control.
    let mut control = with_tasks(vec![
        task("a", "Editor", "First"),
        task("b", "Editor", "Second"),
    ]);
    let control_ctx = Context::default();
    frame(&control_ctx, &mut control, 0.0, vec![]);
    click(&control_ctx, &mut control, "Editor, 2 windows", 0.1);
    key(&control_ctx, &mut control, Key::Tab, 0.2);
    key(&control_ctx, &mut control, Key::Enter, 0.25);
    assert_eq!(control.ledger().len(), 1);
    assert_eq!(control.ledger()[0].action, Action::Close);
    assert_eq!(control.ledger()[0].target.as_deref(), Some("a"));
    a.input(Input::Snapshot {
        sequence: 2,
        windows: vec![
            task("a", "Editor", "Renamed"),
            task("b", "Editor", "Second"),
        ],
    });
    frame(&ctx, &mut a, 0.3, vec![]);
    assert_eq!(
        ctx.memory(|m| m.focused()),
        Some(close_focus),
        "identity-stable Close retains focus despite caption change"
    );
    key(&ctx, &mut a, Key::Enter, 0.4);
    assert_eq!(a.ledger().len(), 1);
    assert_eq!(a.ledger()[0].action, Action::Close);
    assert_eq!(a.ledger()[0].target.as_deref(), Some("a"));
}

#[test]
fn keyboard_reorder_then_enter_activates_once_without_pointer_suppression() {
    let ctx = Context::default();
    let mut a = with_tasks(vec![
        task("a", "First", "First document"),
        task("b", "Second", "Second document"),
    ]);
    frame(&ctx, &mut a, 0.0, vec![]);
    click(&ctx, &mut a, "First document", 0.1);
    assert_eq!(a.ledger().len(), 1);
    frame(
        &ctx,
        &mut a,
        0.12,
        vec![Event::PointerMoved(pos2(1000.0, 200.0))],
    );
    focus_by_tab(&ctx, &mut a, "First document", 0.2);
    modified_key(&ctx, &mut a, Key::ArrowRight, Modifiers::ALT, 1.0);
    assert_eq!(
        a.view().groups[0].key,
        "second",
        "keyboard reorder actually occurred"
    );
    focus_by_tab(&ctx, &mut a, "First document", 1.1);
    key(&ctx, &mut a, Key::Enter, 2.0);
    assert_eq!(
        a.ledger().len(),
        2,
        "legitimate keyboard activation cannot be suppressed as trailing pointer click"
    );
    assert_eq!(a.ledger()[1].target.as_deref(), Some("a"));
    assert_eq!(a.ledger()[1].action, Action::Focus);
}

fn painted_text(output: &egui::FullOutput, text: &str) -> (f32, Rect) {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(t) if t.galley.job.text == text => {
                Some((t.pos.x, shape.clip_rect))
            }
            _ => None,
        })
        .expect("real painted title and its local clip")
}

#[test]
fn simultaneous_hover_and_focus_tickers_animate_independently_and_stop_hidden() {
    let first = "First document with a very long overflowing caption that must keep scrolling while focused";
    let second = "Second much longer document caption with different overflow width and independently timed motion through actual frames";
    let ctx = Context::default();
    let mut a = with_tasks(vec![task("a", "First", first), task("b", "Second", second)]);
    frame(&ctx, &mut a, 0.0, vec![]);
    click(&ctx, &mut a, first, 0.1);
    focus_by_tab(&ctx, &mut a, first, 0.2);
    let focus = ctx
        .memory(|m| m.focused())
        .expect("first task focused by input");
    let p = a.widget_rect(second).unwrap().center();
    let initial = frame(&ctx, &mut a, 1.0, vec![Event::PointerMoved(p)]);
    assert_eq!(
        ctx.memory(|m| m.focused()),
        Some(focus),
        "hover never steals focus"
    );
    let (first_x, first_clip) = painted_text(&initial, first);
    let (second_x, second_clip) = painted_text(&initial, second);
    let later = frame(&ctx, &mut a, 1.8, vec![]);
    let (first_later, first_later_clip) = painted_text(&later, first);
    let (second_later, second_later_clip) = painted_text(&later, second);
    assert!(
        first_later < first_x - 1.0,
        "focused ticker must advance despite differently sized hovered ticker"
    );
    assert!(
        second_later < second_x - 1.0,
        "hovered ticker must independently advance"
    );
    assert_eq!(first_clip, first_later_clip);
    assert_eq!(second_clip, second_later_clip);
    assert!(
        first_clip.right() < second_clip.left(),
        "titles remain in disjoint local clip regions"
    );
    a.input(Input::Visible(false));
    frame(&ctx, &mut a, 1.9, vec![]);
    assert!(
        !a.view().animation_pending,
        "hidden labels must not schedule animation work"
    );
}

#[test]
fn hidden_engaged_tickers_leave_no_animation_pending() {
    let first = "First long overflowing title that must stop while hidden despite engagement";
    let second = "Second differently sized long overflowing title with another lifetime that must also stop while hidden";
    let ctx = Context::default();
    let mut a = with_tasks(vec![task("a", "First", first), task("b", "Second", second)]);
    frame(&ctx, &mut a, 0.0, vec![]);
    focus_by_tab(&ctx, &mut a, first, 0.1);
    let p = a.widget_rect(second).unwrap().center();
    frame(&ctx, &mut a, 1.0, vec![Event::PointerMoved(p)]);
    a.input(Input::Visible(false));
    for time in [1.1, 2.0, 3.0] {
        frame(&ctx, &mut a, time, vec![]);
        assert!(!a.view().animation_pending, "hidden at {time}");
    }
}

fn surviving_close_after_membership_change(selected: &str, refreshed: Vec<TaskWindow>) {
    let ctx = Context::default();
    let mut a = with_tasks(vec![
        task("a", "Editor", "First"),
        task("b", "Editor", "Second"),
    ]);
    frame(&ctx, &mut a, 0.0, vec![]);
    click(&ctx, &mut a, "Editor, 2 windows", 0.1);
    if selected == "b" {
        key(&ctx, &mut a, Key::End, 0.2);
    }
    let name = if selected == "a" {
        "Close First"
    } else {
        "Close Second"
    };
    focus_by_tab(&ctx, &mut a, name, 0.3);
    let close_focus = ctx
        .memory(|m| m.focused())
        .expect("measured selected Close focused");
    assert!(
        a.ledger().is_empty(),
        "focus setup dispatches no window action"
    );
    a.input(Input::Snapshot {
        sequence: 2,
        windows: refreshed,
    });
    frame(&ctx, &mut a, 1.0, vec![]);
    // Press Enter even if focus regressed, so RED proves resulting wrong action.
    let retained_focus = ctx.memory(|m| m.focused()) == Some(close_focus);
    key(&ctx, &mut a, Key::Enter, 1.1);
    assert_eq!(
        a.ledger().len(),
        1,
        "surviving selected Close dispatches exactly once"
    );
    assert_eq!(a.ledger()[0].target.as_deref(), Some(selected));
    assert_eq!(
        a.ledger()[0].action,
        Action::Close,
        "snapshot index change cannot replace Close focus with activation"
    );
    assert!(a.ledger().iter().all(|entry| entry.action != Action::Focus));
    assert!(
        retained_focus,
        "same identity/control role retains actual egui focus"
    );
}

#[test]
fn reordered_gallery_survivor_retains_close_focus_not_activation() {
    surviving_close_after_membership_change(
        "a",
        vec![task("b", "Editor", "Second"), task("a", "Editor", "First")],
    );
}

#[test]
fn removal_before_gallery_survivor_retains_close_focus_not_activation() {
    surviving_close_after_membership_change("b", vec![task("b", "Editor", "Second")]);
}
