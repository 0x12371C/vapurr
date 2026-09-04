//! Brand system drawn from the board. The PNG is a spec, not a texture.

mod card;
mod cat;
pub mod icons;
mod orb;

use std::sync::Arc;

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Style, Visuals};

pub use card::vapurr_card;
pub use cat::{cat_mark, paint_cat};
pub use orb::thinking_orb;

pub const LIME: Color32 = Color32::from_rgb(0x00, 0xF0, 0x5A);
pub const FOREST: Color32 = Color32::from_rgb(0x0A, 0x2E, 0x1B);
pub const VOID: Color32 = Color32::from_rgb(0x0E, 0x0E, 0x0E);
pub const STEEL: Color32 = Color32::from_rgb(0x1F, 0x23, 0x27);
pub const SNOW: Color32 = Color32::from_rgb(0xF2, 0xF3, 0xF4);
pub const MUTED: Color32 = Color32::from_rgb(0x8A, 0xA0, 0x90);

pub const WORDMARK: &str = "VAPURR";
pub const TAGLINE: &str = "BROWSE. OWN. SPEND.";

pub fn apply_once(ctx: &egui::Context) {
    install_fonts(ctx);
    apply_frame(ctx);
}

pub fn apply_frame(ctx: &egui::Context) {
    let mut style = Style {
        visuals: dark_visuals(),
        ..Style::default()
    };
    style.spacing.item_spacing = egui::vec2(12.0, 10.0);
    style.spacing.button_padding = egui::vec2(16.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(14);
    ctx.set_style(style);
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "sora".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../../assets/fonts/Sora-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "sora-semibold".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../../assets/fonts/Sora-SemiBold.ttf"
        ))),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "sora".to_owned());
    fonts.families.insert(
        FontFamily::Name("sora-semibold".into()),
        vec!["sora-semibold".to_owned(), "sora".to_owned()],
    );
    ctx.set_fonts(fonts);
}

pub fn semibold() -> FontFamily {
    FontFamily::Name("sora-semibold".into())
}

pub fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.dark_mode = true;
    v.override_text_color = Some(SNOW);
    v.panel_fill = VOID;
    v.window_fill = STEEL;
    v.faint_bg_color = FOREST;
    v.extreme_bg_color = STEEL;
    v.code_bg_color = STEEL;
    v.hyperlink_color = LIME;
    v.warn_fg_color = LIME;
    v.error_fg_color = LIME;
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0x00, 0xF0, 0x5A, 40);
    v.selection.stroke = Stroke::new(1.0_f32, LIME);
    v.widgets.inactive.bg_fill = STEEL;
    v.widgets.inactive.weak_bg_fill = STEEL;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, SNOW);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, FOREST);
    v.widgets.hovered.bg_fill = FOREST;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, LIME);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, SNOW);
    v.widgets.active.bg_fill = LIME;
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, VOID);
    v.widgets.open.bg_fill = STEEL;
    v.window_stroke = Stroke::new(1.0_f32, FOREST);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(8);
    v.widgets.inactive.corner_radius = CornerRadius::same(8);
    v.widgets.hovered.corner_radius = CornerRadius::same(8);
    v.widgets.active.corner_radius = CornerRadius::same(8);
    v
}

pub fn wordmark(ui: &mut egui::Ui, size: f32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        cat_mark(ui, size * 1.25);
        ui.label(
            egui::RichText::new(WORDMARK)
                .family(semibold())
                .size(size)
                .color(SNOW),
        );
    });
}

pub fn purr_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(label)
                .color(VOID)
                .family(semibold())
                .strong(),
        )
        .fill(LIME)
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(8)),
    )
}

pub fn ghost_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(label).color(SNOW))
            .fill(STEEL)
            .stroke(Stroke::new(1.0_f32, FOREST))
            .corner_radius(CornerRadius::same(8)),
    )
}

pub fn tile(ui: &mut egui::Ui, icon: impl FnOnce(&mut egui::Ui, f32) -> egui::Response, label: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(112.0, 96.0), egui::Sense::click());
    let fill = if resp.hovered() { FOREST } else { STEEL };
    ui.painter().rect_filled(rect, 12.0, fill);
    ui.painter()
        .rect_stroke(rect, 12.0, Stroke::new(1.0_f32, FOREST), egui::StrokeKind::Outside);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    child.add_space(16.0);
    icon(&mut child, 28.0);
    child.add_space(8.0);
    child.label(egui::RichText::new(label).color(SNOW).size(13.0));
    resp
}
