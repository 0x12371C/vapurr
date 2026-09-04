//! VAPURR card, drawn from the board. Black plate, lime cat, Visa lockup.

use egui::{pos2, Align2, FontId, Rect, Sense, Stroke, Ui, Vec2};

use crate::{cat::paint_cat, FOREST, LIME, SNOW, STEEL, VOID};

pub fn vapurr_card(ui: &mut Ui, width: f32) {
    let height = width * 0.63;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 14.0, VOID);
    painter.rect_stroke(rect, 14.0, Stroke::new(1.0_f32, FOREST), egui::StrokeKind::Outside);

    // diagonal sheen
    for i in 0..6 {
        let t = i as f32 / 6.0;
        let x = rect.left() + rect.width() * (0.35 + t * 0.7);
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x - rect.width() * 0.22, rect.bottom())],
            Stroke::new(8.0_f32, with_alpha(STEEL, 18)),
        );
    }

    let chip = Rect::from_min_size(
        pos2(rect.left() + 22.0, rect.top() + 28.0),
        Vec2::new(36.0, 28.0),
    );
    painter.rect_stroke(chip, 4.0, Stroke::new(1.2_f32, LIME), egui::StrokeKind::Outside);

    let cat_box = Rect::from_center_size(rect.center(), Vec2::splat(rect.height() * 0.42));
    paint_cat(painter, cat_box, LIME);

    painter.text(
        pos2(rect.left() + 22.0, rect.bottom() - 38.0),
        Align2::LEFT_BOTTOM,
        "VAPURR",
        FontId::proportional(16.0),
        SNOW,
    );
    painter.text(
        pos2(rect.left() + 22.0, rect.bottom() - 20.0),
        Align2::LEFT_BOTTOM,
        "BY AVALANCHE",
        FontId::proportional(10.0),
        LIME,
    );
    painter.text(
        pos2(rect.right() - 22.0, rect.bottom() - 22.0),
        Align2::RIGHT_BOTTOM,
        "VISA",
        FontId::proportional(14.0),
        SNOW,
    );
}

fn with_alpha(c: egui::Color32, a: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}
