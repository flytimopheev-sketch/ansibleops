use egui::{Color32, Context, FontId, Rounding, Stroke, TextStyle, Vec2};
use std::sync::RwLock;

pub const APP_NAME: &str = "AnsibleOps";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_REPO: &str = "https://github.com/flytimopheev-sketch/ansibleops";
pub const AUTHOR_EMAIL: &str = "flytimopheev-sketch@users.noreply.github.com";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub bg: Color32,
    pub bg_deep: Color32,
    pub surface: Color32,
    pub surface_hi: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub blue: Color32,
    pub purple: Color32,
    pub green: Color32,
    pub red: Color32,
    pub orange: Color32,
    pub cyan: Color32,
    pub light: bool,
}

impl Palette {
    pub const fn cosmic() -> Self {
        Self {
            bg: Color32::from_rgb(35, 33, 44),
            bg_deep: Color32::from_rgb(28, 26, 37),
            surface: Color32::from_rgb(48, 46, 60),
            surface_hi: Color32::from_rgb(64, 62, 80),
            text: Color32::from_rgb(222, 220, 235),
            text_muted: Color32::from_rgb(150, 148, 170),
            blue: Color32::from_rgb(122, 162, 247),
            purple: Color32::from_rgb(187, 154, 247),
            green: Color32::from_rgb(158, 206, 106),
            red: Color32::from_rgb(247, 118, 142),
            orange: Color32::from_rgb(255, 158, 100),
            cyan: Color32::from_rgb(125, 207, 255),
            light: false,
        }
    }
    pub const fn violet() -> Self {
        Self {
            bg: Color32::from_rgb(54, 37, 92),
            bg_deep: Color32::from_rgb(44, 29, 78),
            surface: Color32::from_rgb(70, 52, 110),
            surface_hi: Color32::from_rgb(88, 68, 132),
            text: Color32::from_rgb(232, 225, 250),
            text_muted: Color32::from_rgb(170, 158, 205),
            blue: Color32::from_rgb(140, 170, 255),
            purple: Color32::from_rgb(200, 160, 255),
            green: Color32::from_rgb(160, 220, 140),
            red: Color32::from_rgb(255, 130, 160),
            orange: Color32::from_rgb(255, 170, 120),
            cyan: Color32::from_rgb(140, 215, 255),
            light: false,
        }
    }
    pub const fn lavender() -> Self {
        Self {
            bg: Color32::from_rgb(210, 195, 246),
            bg_deep: Color32::from_rgb(222, 210, 250),
            surface: Color32::from_rgb(196, 180, 235),
            surface_hi: Color32::from_rgb(180, 162, 222),
            text: Color32::from_rgb(40, 32, 66),
            text_muted: Color32::from_rgb(90, 80, 130),
            blue: Color32::from_rgb(70, 110, 200),
            purple: Color32::from_rgb(110, 70, 190),
            green: Color32::from_rgb(70, 140, 70),
            red: Color32::from_rgb(200, 60, 90),
            orange: Color32::from_rgb(200, 110, 40),
            cyan: Color32::from_rgb(40, 130, 190),
            light: true,
        }
    }
    pub const fn vanilla() -> Self {
        Self {
            bg: Color32::from_rgb(241, 254, 200),
            bg_deep: Color32::from_rgb(248, 255, 220),
            surface: Color32::from_rgb(228, 242, 180),
            surface_hi: Color32::from_rgb(214, 230, 160),
            text: Color32::from_rgb(50, 58, 30),
            text_muted: Color32::from_rgb(105, 115, 75),
            blue: Color32::from_rgb(60, 110, 190),
            purple: Color32::from_rgb(120, 80, 180),
            green: Color32::from_rgb(80, 140, 60),
            red: Color32::from_rgb(200, 70, 80),
            orange: Color32::from_rgb(190, 120, 40),
            cyan: Color32::from_rgb(40, 140, 170),
            light: true,
        }
    }
}

pub static PALETTE: RwLock<Palette> = RwLock::new(Palette::cosmic());

#[allow(dead_code)]
pub fn c_bg() -> Color32 {
    PALETTE.read().unwrap().bg
}
pub fn c_bgd() -> Color32 {
    PALETTE.read().unwrap().bg_deep
}
pub fn c_surf() -> Color32 {
    PALETTE.read().unwrap().surface
}
pub fn c_surfh() -> Color32 {
    PALETTE.read().unwrap().surface_hi
}
pub fn c_tx() -> Color32 {
    PALETTE.read().unwrap().text
}
pub fn c_tm() -> Color32 {
    PALETTE.read().unwrap().text_muted
}
pub fn c_blue() -> Color32 {
    PALETTE.read().unwrap().blue
}
pub fn c_purple() -> Color32 {
    PALETTE.read().unwrap().purple
}
pub fn c_green() -> Color32 {
    PALETTE.read().unwrap().green
}
pub fn c_red() -> Color32 {
    PALETTE.read().unwrap().red
}
pub fn c_orange() -> Color32 {
    PALETTE.read().unwrap().orange
}
pub fn c_cyan() -> Color32 {
    PALETTE.read().unwrap().cyan
}

pub const THEME_NAMES: [&str; 4] = ["Cosmic", "Violet", "Lavender", "Vanilla"];

pub fn apply_theme(ctx: &Context, idx: usize) {
    let p = match idx % 4 {
        0 => Palette::cosmic(),
        1 => Palette::violet(),
        2 => Palette::lavender(),
        _ => Palette::vanilla(),
    };
    *PALETTE.write().unwrap() = p;
    let mut v = if p.light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    v.panel_fill = p.bg;
    v.window_fill = p.bg_deep;
    v.extreme_bg_color = p.bg_deep;
    v.faint_bg_color = p.surface;
    v.widgets.noninteractive.bg_fill = p.surface;
    v.widgets.inactive.bg_fill = p.surface;
    v.widgets.hovered.bg_fill = p.surface_hi;
    v.widgets.active.bg_fill = p.surface_hi;
    v.widgets.noninteractive.fg_stroke.color = p.text;
    v.widgets.inactive.fg_stroke.color = p.text;
    v.widgets.hovered.fg_stroke.color = p.text;
    v.widgets.active.fg_stroke.color = p.text;
    v.selection.bg_fill = p.blue.gamma_multiply(0.35);
    v.selection.stroke.color = p.blue;
    v.window_rounding = Rounding::same(12.0);
    v.window_shadow = Default::default();
    v.window_stroke = Stroke::new(1.0_f32, p.surface_hi);
    v.widgets.noninteractive.rounding = Rounding::same(8.0);
    v.widgets.inactive.rounding = Rounding::same(8.0);
    v.widgets.hovered.rounding = Rounding::same(8.0);
    v.widgets.active.rounding = Rounding::same(8.0);
    ctx.set_visuals(v);
    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(14.0));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(14.0));
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(20.0));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(12.0));
    style
        .text_styles
        .insert(TextStyle::Monospace, FontId::monospace(13.0));
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    ctx.set_style(style);
}
