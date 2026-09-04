//! Geometric cat mark, traced from the brand board. Not a fox. Not a bitmap.

use egui::epaint::{PathShape, PathStroke};
use egui::{pos2, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use crate::LIME;

/// Design-space cat, 0..1. Ears up, almond eyes, short muzzle, whisker pads.
fn p(rect: Rect, x: f32, y: f32) -> Pos2 {
    pos2(
        rect.left() + rect.width() * x,
        rect.top() + rect.height() * y,
    )
}

pub fn cat_mark(ui: &mut Ui, size: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    paint_cat(ui.painter(), rect, LIME);
    resp
}

pub fn paint_cat(painter: &egui::Painter, rect: Rect, lime: Color32) {
    let w = rect.width().max(1.0);
    let stroke = PathStroke::new((w * 0.048).clamp(1.4, 4.2), lime);

    // Outer skull + ears. Cat: ears more vertical than a fox, chin shorter.
    let outer = [
        (0.30, 0.05),
        (0.17, 0.34),
        (0.10, 0.50),
        (0.14, 0.68),
        (0.31, 0.88),
        (0.50, 0.96),
        (0.69, 0.88),
        (0.86, 0.68),
        (0.90, 0.50),
        (0.83, 0.34),
        (0.70, 0.05),
        (0.60, 0.30),
        (0.50, 0.22),
        (0.40, 0.30),
    ]
    .map(|(x, y)| p(rect, x, y));
    painter.add(PathShape::closed_line(outer.to_vec(), stroke.clone()));

    // Inner ears
    painter.add(PathShape::line(
        vec![p(rect, 0.30, 0.34), p(rect, 0.33, 0.14), p(rect, 0.41, 0.32)],
        stroke.clone(),
    ));
    painter.add(PathShape::line(
        vec![p(rect, 0.59, 0.32), p(rect, 0.67, 0.14), p(rect, 0.70, 0.34)],
        stroke.clone(),
    ));

    // Feline eyes — filled almonds, inner corner down (the board's glare)
    painter.add(PathShape::convex_polygon(
        vec![
            p(rect, 0.29, 0.50),
            p(rect, 0.40, 0.43),
            p(rect, 0.47, 0.52),
            p(rect, 0.36, 0.58),
        ],
        lime,
        Stroke::NONE,
    ));
    painter.add(PathShape::convex_polygon(
        vec![
            p(rect, 0.53, 0.52),
            p(rect, 0.60, 0.43),
            p(rect, 0.71, 0.50),
            p(rect, 0.64, 0.58),
        ],
        lime,
        Stroke::NONE,
    ));

    // Nose bridge / muzzle V — cat, not a snout
    painter.add(PathShape::line(
        vec![p(rect, 0.42, 0.62), p(rect, 0.50, 0.78), p(rect, 0.58, 0.62)],
        stroke.clone(),
    ));

    // Whisker pads
    painter.add(PathShape::line(
        vec![p(rect, 0.18, 0.58), p(rect, 0.30, 0.70)],
        stroke.clone(),
    ));
    painter.add(PathShape::line(
        vec![p(rect, 0.82, 0.58), p(rect, 0.70, 0.70)],
        stroke.clone(),
    ));
    // Whiskers
    let whisk = PathStroke::new((w * 0.028).clamp(1.0, 2.4), lime);
    for (y, dx) in [(0.56, 0.16), (0.62, 0.17), (0.68, 0.14)] {
        painter.add(PathShape::line(
            vec![p(rect, 0.20, y), p(rect, 0.20 - dx, y - 0.02)],
            whisk.clone(),
        ));
        painter.add(PathShape::line(
            vec![p(rect, 0.80, y), p(rect, 0.80 + dx, y - 0.02)],
            whisk.clone(),
        ));
    }
}
