//! Lime thinking-orb. MetalForge-class motion, vapurr palette, generated each frame.

use egui::{pos2, Color32, Pos2, Rect, Stroke, Ui};

use crate::{FOREST, LIME, VOID};

pub fn thinking_orb(ui: &mut Ui, rect: Rect, t: f64) {
    let painter = ui.painter_at(rect);
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.42;
    let t = t as f32;

    painter.circle_filled(c, r * 1.05, VOID);

    for i in 0..10 {
        let phase = t * 0.65 + i as f32 * 0.51;
        let pulse = (phase.sin() * 0.5) + 0.5;
        let rr = r * (0.38 + i as f32 * 0.07 + pulse * 0.025);
        let a = (22u8).saturating_sub(i as u8 * 2);
        painter.circle_stroke(
            c,
            rr,
            Stroke::new(1.25_f32, Color32::from_rgba_unmultiplied(0x00, 0xF0, 0x5A, a)),
        );
    }

    let core = r * (0.18 + 0.03 * (t * 1.1).sin());
    painter.circle_filled(
        c,
        core * 1.8,
        Color32::from_rgba_unmultiplied(0x0A, 0x2E, 0x1B, 180),
    );
    painter.circle_filled(
        c,
        core,
        Color32::from_rgba_unmultiplied(0x00, 0xF0, 0x5A, 55),
    );

    for k in 0..28 {
        let kf = k as f32;
        let ang = t * 0.35 + kf * 0.224;
        let rad = r * (0.28 + 0.34 * ((t * 0.4 + kf * 0.7).sin() * 0.5 + 0.5));
        let p = pos2(c.x + ang.cos() * rad, c.y + ang.sin() * rad * 0.92);
        let sz = 1.2 + ((t + kf).sin().abs()) * 1.8;
        painter.circle_filled(p, sz, LIME);
    }

    // glass ridge
    painter.circle_stroke(c, r * 0.72, Stroke::new(2.0_f32, FOREST));
}

pub fn orb_center_in(ui: &Ui, size: f32) -> Rect {
    let avail = ui.max_rect();
    Rect::from_center_size(avail.center(), egui::vec2(size, size))
}

#[allow(dead_code)]
fn _pt(c: Pos2, a: f32, r: f32) -> Pos2 {
    pos2(c.x + a.cos() * r, c.y + a.sin() * r)
}
