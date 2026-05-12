#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Rect {
    pub fn width(self) -> i32 {
        self.right - self.left
    }

    pub fn height(self) -> i32 {
        self.bottom - self.top
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShellPreviewRects {
    pub top: Rect,
    pub bottom: Rect,
}

pub fn build_shell_preview_rects(
    origin_x: i32,
    origin_y: i32,
    width: i32,
    height: i32,
    top_height: i32,
    bottom_height: i32,
) -> ShellPreviewRects {
    let top = Rect {
        left: origin_x,
        top: origin_y,
        right: origin_x + width,
        bottom: origin_y + top_height,
    };

    let bottom = Rect {
        left: origin_x,
        top: origin_y + height - bottom_height,
        right: origin_x + width,
        bottom: origin_y + height,
    };

    ShellPreviewRects { top, bottom }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MonitorId(pub u32);

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonitorDescriptor {
    pub id: MonitorId,
    pub bounds: Rect,
    pub scale_factor: f64,
    pub is_primary: bool,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonitorShellOwnership {
    PrimaryShell,
    SecondaryTaskStrip,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorShellPlan {
    pub monitor_id: MonitorId,
    pub ownership: MonitorShellOwnership,
    pub top_bar: Option<Rect>,
    pub bottom_strip: Rect,
    pub work_area: Rect,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupAnchorEdge {
    Top,
    Bottom,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopupAnchorPlan {
    pub monitor_id: MonitorId,
    pub rect: Rect,
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn plan_monitor_shell_layout(
    monitors: &[MonitorDescriptor],
    top_bar_height_logical: f64,
    bottom_strip_height_logical: f64,
) -> Vec<MonitorShellPlan> {
    let primary_id = primary_monitor_id(monitors);

    monitors
        .iter()
        .map(|monitor| {
            let top_height = physical_length(top_bar_height_logical, monitor.scale_factor);
            let bottom_height = physical_length(bottom_strip_height_logical, monitor.scale_factor);
            let is_primary_shell = Some(monitor.id) == primary_id;
            let top_bar = is_primary_shell.then(|| top_edge_rect(monitor.bounds, top_height));
            let bottom_strip = bottom_edge_rect(monitor.bounds, bottom_height);
            let work_area = Rect {
                left: monitor.bounds.left,
                top: top_bar.map_or(monitor.bounds.top, |rect| rect.bottom),
                right: monitor.bounds.right,
                bottom: bottom_strip.top,
            };

            MonitorShellPlan {
                monitor_id: monitor.id,
                ownership: if is_primary_shell {
                    MonitorShellOwnership::PrimaryShell
                } else {
                    MonitorShellOwnership::SecondaryTaskStrip
                },
                top_bar,
                bottom_strip,
                work_area,
            }
        })
        .collect()
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn plan_popup_anchor(
    monitor: MonitorDescriptor,
    owning_bar: Rect,
    edge: PopupAnchorEdge,
    anchor_left_logical: f64,
    anchor_width_logical: f64,
    popup_width_logical: f64,
    popup_height_logical: f64,
    edge_padding_physical: i32,
) -> PopupAnchorPlan {
    let popup_width = physical_length(popup_width_logical, monitor.scale_factor)
        .min((monitor.bounds.width() - (edge_padding_physical * 2)).max(1));
    let popup_height = physical_length(popup_height_logical, monitor.scale_factor)
        .min((monitor.bounds.height() - (edge_padding_physical * 2)).max(1));
    let anchor_midpoint = owning_bar.left
        + physical_length(
            anchor_left_logical + (anchor_width_logical / 2.0),
            monitor.scale_factor,
        );
    let min_x = monitor.bounds.left + edge_padding_physical;
    let max_x = monitor.bounds.right - popup_width - edge_padding_physical;
    let left = (anchor_midpoint - (popup_width / 2)).clamp(min_x, max_x.max(min_x));
    let top = match edge {
        PopupAnchorEdge::Top => owning_bar.bottom,
        PopupAnchorEdge::Bottom => owning_bar.top - popup_height - edge_padding_physical,
    };
    let top = top.clamp(
        monitor.bounds.top + edge_padding_physical,
        (monitor.bounds.bottom - popup_height - edge_padding_physical)
            .max(monitor.bounds.top + edge_padding_physical),
    );

    PopupAnchorPlan {
        monitor_id: monitor.id,
        rect: Rect {
            left,
            top,
            right: left + popup_width,
            bottom: top + popup_height,
        },
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn assign_task_strip_monitor(
    plans: &[MonitorShellPlan],
    window_monitor_id: Option<MonitorId>,
    previous_monitor_id: Option<MonitorId>,
) -> Option<MonitorId> {
    if let Some(monitor_id) =
        window_monitor_id.filter(|id| plans.iter().any(|plan| plan.monitor_id == *id))
    {
        return Some(monitor_id);
    }

    if let Some(monitor_id) =
        previous_monitor_id.filter(|id| plans.iter().any(|plan| plan.monitor_id == *id))
    {
        return Some(monitor_id);
    }

    plans
        .iter()
        .find(|plan| plan.ownership == MonitorShellOwnership::PrimaryShell)
        .or_else(|| plans.first())
        .map(|plan| plan.monitor_id)
}

#[cfg_attr(not(test), allow(dead_code))]
fn primary_monitor_id(monitors: &[MonitorDescriptor]) -> Option<MonitorId> {
    monitors
        .iter()
        .find(|monitor| monitor.is_primary)
        .or_else(|| monitors.first())
        .map(|monitor| monitor.id)
}

#[cfg_attr(not(test), allow(dead_code))]
fn physical_length(logical: f64, scale_factor: f64) -> i32 {
    (logical * scale_factor).round().max(1.0) as i32
}

#[cfg_attr(not(test), allow(dead_code))]
fn top_edge_rect(bounds: Rect, height: i32) -> Rect {
    Rect {
        left: bounds.left,
        top: bounds.top,
        right: bounds.right,
        bottom: bounds.top + height,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn bottom_edge_rect(bounds: Rect, height: i32) -> Rect {
    Rect {
        left: bounds.left,
        top: bounds.bottom - height,
        right: bounds.right,
        bottom: bounds.bottom,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        assign_task_strip_monitor, build_shell_preview_rects, plan_monitor_shell_layout,
        plan_popup_anchor, MonitorDescriptor, MonitorId, MonitorShellOwnership, PopupAnchorEdge,
        Rect,
    };

    #[test]
    fn preview_rects_span_full_width() {
        let rects = build_shell_preview_rects(0, 0, 1920, 1080, 28, 48);

        assert_eq!(rects.top.left, 0);
        assert_eq!(rects.top.right, 1920);
        assert_eq!(rects.bottom.left, 0);
        assert_eq!(rects.bottom.right, 1920);
    }

    #[test]
    fn preview_rects_leave_center_workspace_gap() {
        let rects = build_shell_preview_rects(0, 0, 1920, 1080, 28, 48);

        assert_eq!(rects.top.height(), 28);
        assert_eq!(rects.bottom.height(), 48);
        assert_eq!(rects.top.bottom, 28);
        assert_eq!(rects.bottom.top, 1032);
        assert!(rects.top.bottom < rects.bottom.top);
    }

    #[test]
    fn monitor_shell_layout_uses_per_monitor_dpi_for_mixed_dpi_strips() {
        let monitors = mixed_dpi_monitors();

        let plans = plan_monitor_shell_layout(&monitors, 23.4, 32.4);

        assert_eq!(plans[0].top_bar.unwrap().height(), 23);
        assert_eq!(plans[0].bottom_strip.height(), 32);
        assert_eq!(plans[1].top_bar, None);
        assert_eq!(plans[1].bottom_strip.height(), 49);
        assert_eq!(plans[1].work_area.bottom, 1440 - 49);
    }

    #[test]
    fn monitor_shell_layout_assigns_primary_shell_and_secondary_task_strip_ownership() {
        let monitors = mixed_dpi_monitors();

        let plans = plan_monitor_shell_layout(&monitors, 23.4, 32.4);

        assert_eq!(plans[0].ownership, MonitorShellOwnership::PrimaryShell);
        assert!(plans[0].top_bar.is_some());
        assert_eq!(
            plans[1].ownership,
            MonitorShellOwnership::SecondaryTaskStrip
        );
        assert!(plans[1].top_bar.is_none());
        assert_eq!(plans[1].bottom_strip.left, 1920);
        assert_eq!(plans[1].bottom_strip.right, 4480);
    }

    #[test]
    fn popup_anchor_uses_source_monitor_scale_and_clamps_to_monitor_bounds() {
        let monitors = mixed_dpi_monitors();
        let plans = plan_monitor_shell_layout(&monitors, 23.4, 32.4);
        let secondary = monitors[1];
        let secondary_plan = plans[1];

        let popup = plan_popup_anchor(
            secondary,
            secondary_plan.bottom_strip,
            PopupAnchorEdge::Bottom,
            2_400.0,
            120.0,
            980.0,
            430.0,
            8,
        );

        assert_eq!(popup.monitor_id, MonitorId(2));
        assert_eq!(popup.rect.width(), 1470);
        assert!(popup.rect.left >= secondary.bounds.left + 8);
        assert!(popup.rect.right <= secondary.bounds.right - 8);
        assert!(popup.rect.bottom <= secondary_plan.bottom_strip.top - 8);
    }

    #[test]
    fn popup_anchor_places_primary_top_popups_below_the_primary_top_bar() {
        let monitors = mixed_dpi_monitors();
        let plans = plan_monitor_shell_layout(&monitors, 23.4, 32.4);
        let primary_top = plans[0]
            .top_bar
            .expect("primary shell should own the only top bar");

        let popup = plan_popup_anchor(
            monitors[0],
            primary_top,
            PopupAnchorEdge::Top,
            64.0,
            40.0,
            420.0,
            320.0,
            8,
        );

        assert_eq!(popup.monitor_id, MonitorId(1));
        assert_eq!(popup.rect.top, primary_top.bottom);
        assert!(popup.rect.left >= monitors[0].bounds.left + 8);
    }

    #[test]
    fn task_strip_assignment_prefers_current_monitor_then_stable_previous_then_primary() {
        let plans = plan_monitor_shell_layout(&mixed_dpi_monitors(), 23.4, 32.4);

        assert_eq!(
            assign_task_strip_monitor(&plans, Some(MonitorId(2)), Some(MonitorId(1))),
            Some(MonitorId(2))
        );
        assert_eq!(
            assign_task_strip_monitor(&plans, None, Some(MonitorId(2))),
            Some(MonitorId(2))
        );
        assert_eq!(
            assign_task_strip_monitor(&plans, Some(MonitorId(99)), Some(MonitorId(88))),
            Some(MonitorId(1))
        );
    }

    fn mixed_dpi_monitors() -> [MonitorDescriptor; 2] {
        [
            MonitorDescriptor {
                id: MonitorId(1),
                bounds: Rect {
                    left: 0,
                    top: 0,
                    right: 1920,
                    bottom: 1080,
                },
                scale_factor: 1.0,
                is_primary: true,
            },
            MonitorDescriptor {
                id: MonitorId(2),
                bounds: Rect {
                    left: 1920,
                    top: 0,
                    right: 4480,
                    bottom: 1440,
                },
                scale_factor: 1.5,
                is_primary: false,
            },
        ]
    }
}
