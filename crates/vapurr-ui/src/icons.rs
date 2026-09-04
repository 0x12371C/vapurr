//! Line icons from the brand board. Drawn. Not a sprite sheet.

use egui::epaint::{PathShape, PathStroke};
use egui::{pos2, Rect, Sense, Stroke, Ui, Vec2};

use crate::LIME;

fn stroke(w: f32) -> PathStroke {
    PathStroke::new((w * 0.08).clamp(1.2, 2.6), LIME)
}

fn box_rect(ui: &mut Ui, size: f32) -> (Rect, egui::Response) {
    ui.allocate_exact_size(Vec2::splat(size), Sense::click())
}

pub fn bolt(ui: &mut Ui, size: f32) -> egui::Response {
    let (r, resp) = box_rect(ui, size);
    let p = |x, y| pos2(r.left() + r.width() * x, r.top() + r.height() * y);
    ui.painter().add(PathShape::line(
        vec![p(0.58, 0.08), p(0.32, 0.52), p(0.52, 0.52), p(0.40, 0.92), p(0.72, 0.42), p(0.50, 0.42), p(0.58, 0.08)],
        stroke(size),
    ));
    resp
}

pub fn wallet(ui: &mut Ui, size: f32) -> egui::Response {
    let (r, resp) = box_rect(ui, size);
    let painter = ui.painter();
    let pad = r.width() * 0.16;
    let body = Rect::from_min_max(
        pos2(r.left() + pad, r.top() + pad * 1.2),
        pos2(r.right() - pad, r.bottom() - pad),
    );
    painter.rect_stroke(body, 3.0, Stroke::new(size * 0.08, LIME), egui::StrokeKind::Outside);
    let flap = Rect::from_min_max(
        pos2(body.center().x + pad * 0.2, body.top() + pad * 0.7),
        pos2(body.right() - pad * 0.15, body.top() + pad * 1.7),
    );
    painter.rect_stroke(flap, 2.0, Stroke::new(size * 0.07, LIME), egui::StrokeKind::Outside);
    resp
}

pub fn pay_404(ui: &mut Ui, size: f32) -> egui::Response {
    let (r, resp) = box_rect(ui, size);
    ui.painter().rect_stroke(
        r.shrink(size * 0.08),
        4.0,
        Stroke::new(size * 0.06, LIME),
        egui::StrokeKind::Outside,
    );
    ui.painter().text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        "404",
        egui::FontId::proportional((size * 0.32).clamp(8.0, 16.0)),
        LIME,
    );
    resp
}

pub fn card(ui: &mut Ui, size: f32) -> egui::Response {
    let (r, resp) = box_rect(ui, size);
    let body = Rect::from_center_size(r.center(), Vec2::new(r.width() * 0.72, r.height() * 0.50));
    ui.painter().rect_stroke(body, 3.0, Stroke::new(size * 0.07, LIME), egui::StrokeKind::Outside);
    ui.painter().line_segment(
        [pos2(body.left() + 4.0, body.center().y - 2.0), pos2(body.right() - 4.0, body.center().y - 2.0)],
        Stroke::new(size * 0.05, LIME),
    );
    resp
}

pub fn shield(ui: &mut Ui, size: f32) -> egui::Response {
    let (r, resp) = box_rect(ui, size);
    let p = |x, y| pos2(r.left() + r.width() * x, r.top() + r.height() * y);
    ui.painter().add(PathShape::closed_line(
        vec![p(0.50, 0.12), p(0.82, 0.28), p(0.78, 0.62), p(0.50, 0.90), p(0.22, 0.62), p(0.18, 0.28)],
        stroke(size),
    ));
    resp
}

pub fn swap(ui: &mut Ui, size: f32) -> egui::Response {
    let (r, resp) = box_rect(ui, size);
    let p = |x, y| pos2(r.left() + r.width() * x, r.top() + r.height() * y);
    let s = stroke(size);
    ui.painter().add(PathShape::line(vec![p(0.22, 0.38), p(0.78, 0.38), p(0.66, 0.26)], s.clone()));
    ui.painter().add(PathShape::line(vec![p(0.78, 0.62), p(0.22, 0.62), p(0.34, 0.74)], s));
    resp
}

pub fn lock(ui: &mut Ui, size: f32) -> egui::Response {
    let (r, resp) = box_rect(ui, size);
    let body = Rect::from_center_size(
        pos2(r.center().x, r.center().y + r.height() * 0.12),
        Vec2::new(r.width() * 0.46, r.height() * 0.38),
    );
    ui.painter().rect_stroke(body, 2.0, Stroke::new(size * 0.07, LIME), egui::StrokeKind::Outside);
    let shackle = [
        pos2(body.left() + 4.0, body.top()),
        pos2(body.left() + 4.0, r.top() + r.height() * 0.22),
        pos2(body.right() - 4.0, r.top() + r.height() * 0.22),
        pos2(body.right() - 4.0, body.top()),
    ];
    ui.painter().add(PathShape::line(shackle.to_vec(), stroke(size)));
    resp
}

pub fn phones(ui: &mut Ui, size: f32) -> egui::Response {
    let (r, resp) = box_rect(ui, size);
    let p = |x, y| pos2(r.left() + r.width() * x, r.top() + r.height() * y);
    let s = stroke(size);
    ui.painter().add(PathShape::line(
        vec![p(0.22, 0.42), p(0.22, 0.22), p(0.50, 0.12), p(0.78, 0.22), p(0.78, 0.42)],
        s.clone(),
    ));
    ui.painter().rect_stroke(
        Rect::from_min_max(p(0.14, 0.42), p(0.32, 0.78)),
        3.0,
        Stroke::new(size * 0.07, LIME),
        egui::StrokeKind::Outside,
    );
    ui.painter().rect_stroke(
        Rect::from_min_max(p(0.68, 0.42), p(0.86, 0.78)),
        3.0,
        Stroke::new(size * 0.07, LIME),
        egui::StrokeKind::Outside,
    );
    resp
}

pub fn chart(ui: &mut Ui, size: f32) -> egui::Response {
    let (r, resp) = box_rect(ui, size);
    let p = |x, y| pos2(r.left() + r.width() * x, r.top() + r.height() * y);
    ui.painter().add(PathShape::line(
        vec![p(0.18, 0.78), p(0.38, 0.48), p(0.55, 0.58), p(0.82, 0.22)],
        stroke(size),
    ));
    resp
}

pub fn grid(ui: &mut Ui, size: f32) -> egui::Response {
    let (r, resp) = box_rect(ui, size);
    let painter = ui.painter();
    let s = Stroke::new(size * 0.07, LIME);
    let cell = r.width() * 0.22;
    let gap = r.width() * 0.08;
    let origin = pos2(r.left() + r.width() * 0.18, r.top() + r.height() * 0.18);
    for i in 0..2 {
        for j in 0..2 {
            let o = pos2(origin.x + (cell + gap) * i as f32, origin.y + (cell + gap) * j as f32);
            painter.rect_stroke(Rect::from_min_size(o, Vec2::splat(cell)), 2.0, s, egui::StrokeKind::Outside);
        }
    }
    resp
}

pub fn home(ui: &mut Ui, size: f32) -> egui::Response {
    let (r, resp) = box_rect(ui, size);
    let p = |x, y| pos2(r.left() + r.width() * x, r.top() + r.height() * y);
    let s = stroke(size);
    ui.painter().add(PathShape::line(vec![p(0.18, 0.48), p(0.50, 0.18), p(0.82, 0.48)], s.clone()));
    ui.painter().add(PathShape::closed_line(
        vec![p(0.28, 0.46), p(0.28, 0.82), p(0.72, 0.82), p(0.72, 0.46)],
        s,
    ));
    resp
}
