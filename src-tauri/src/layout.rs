#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
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

#[cfg(test)]
mod tests {
    use super::build_shell_preview_rects;

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
}
