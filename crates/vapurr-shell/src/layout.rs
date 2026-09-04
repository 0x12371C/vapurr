use tao::dpi::{LogicalPosition, LogicalSize};
use wry::Rect;

pub(crate) const ZOOM_STEPS: &[f64] = &[
    0.25, 0.33, 0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0, 5.0,
];

pub(crate) fn clamp_zoom(z: f64) -> f64 {
    if !z.is_finite() {
        1.0
    } else {
        (z * 100.0).round() / 100.0
    }
    .clamp(0.25, 5.0)
}

pub(crate) fn zoom_in(cur: f64) -> f64 {
    ZOOM_STEPS
        .iter()
        .copied()
        .find(|&s| s > cur + 0.001)
        .unwrap_or(5.0)
}

pub(crate) fn zoom_out(cur: f64) -> f64 {
    ZOOM_STEPS
        .iter()
        .rev()
        .copied()
        .find(|&s| s < cur - 0.001)
        .unwrap_or(0.25)
}

#[derive(Clone, Copy)]
pub(crate) struct RadioUi {
    pub(crate) float: bool,
    pub(crate) collapsed: bool,
    /// 0 br, 1 bl, 2 tl, 3 tr
    pub(crate) corner: u8,
}

impl Default for RadioUi {
    fn default() -> Self {
        Self {
            float: false,
            collapsed: false,
            corner: 0,
        }
    }
}

pub(crate) fn radio_strip_h(ui: &RadioUi) -> f64 {
    if ui.collapsed {
        52.0
    } else {
        78.0
    }
}

pub(crate) fn parse_radio_corner(s: &str) -> u8 {
    match s {
        "bl" => 1,
        "tl" => 2,
        "tr" => 3,
        _ => 0,
    }
}

/// Tab strip 36 + omnibox row 48. Must match `frontend/toolbar.html`.
pub(crate) const BAR_H: f64 = 84.0;
pub(crate) const RAIL_W: f64 = 64.0;

pub(crate) fn layout(width: f64, height: f64, ui: &RadioUi) -> (Rect, Rect, Rect, Rect) {
    let rail = RAIL_W;
    let top = BAR_H;
    let side = Rect {
        position: LogicalPosition::new(0.0, 0.0).into(),
        size: LogicalSize::new(rail, height).into(),
    };
    let bar = Rect {
        position: LogicalPosition::new(rail, 0.0).into(),
        size: LogicalSize::new((width - rail).max(200.0), top).into(),
    };
    let rh = radio_strip_h(ui);
    let (radio, page_cut) = if ui.float {
        let rw = 360.0_f64.min((width - rail - 28.0).max(240.0));
        let pad = 14.0;
        let (x, y) = match ui.corner {
            1 => (rail + pad, (height - rh - pad).max(top + pad)),
            2 => (rail + pad, top + pad),
            3 => ((width - rw - pad).max(rail + pad), top + pad),
            _ => (
                (width - rw - pad).max(rail + pad),
                (height - rh - pad).max(top + pad),
            ),
        };
        (
            Rect {
                position: LogicalPosition::new(x, y).into(),
                size: LogicalSize::new(rw, rh).into(),
            },
            0.0,
        )
    } else {
        (
            Rect {
                position: LogicalPosition::new(rail, (height - rh).max(top)).into(),
                size: LogicalSize::new((width - rail).max(200.0), rh).into(),
            },
            rh,
        )
    };
    let page = Rect {
        position: LogicalPosition::new(rail, top).into(),
        size: LogicalSize::new(
            (width - rail).max(200.0),
            (height - top - page_cut).max(200.0),
        )
        .into(),
    };
    (side, bar, page, radio)
}
