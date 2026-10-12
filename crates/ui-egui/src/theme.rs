//! Colours. Panels and the canvas read them from here, never as literals.

use egui::Color32;

#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub canvas: Color32,
    pub grid_dot: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub node_border: Color32,
    pub selection: Color32,
    pub branch: Color32,
    pub link: Color32,
    pub link_label_bg: Color32,
}

impl Tokens {
    pub const LIGHT: Self = Self {
        canvas: Color32::from_rgb(0xf7, 0xf7, 0xfa),
        grid_dot: Color32::from_rgb(0xd9, 0xdb, 0xe3),
        text: Color32::from_rgb(0x1f, 0x22, 0x2b),
        muted: Color32::from_rgb(0x6b, 0x70, 0x80),
        node_border: Color32::from_rgb(0x9a, 0xa0, 0xb0),
        selection: Color32::from_rgb(0x3d, 0x6b, 0xf5),
        branch: Color32::from_rgb(0x8a, 0x90, 0xa3),
        link: Color32::from_rgb(0xd9, 0x6c, 0x2b),
        link_label_bg: Color32::from_rgb(0xff, 0xff, 0xff),
    };

    pub const DARK: Self = Self {
        canvas: Color32::from_rgb(0x1c, 0x1e, 0x24),
        grid_dot: Color32::from_rgb(0x34, 0x37, 0x41),
        text: Color32::from_rgb(0x1f, 0x22, 0x2b),
        muted: Color32::from_rgb(0xa8, 0xad, 0xbb),
        node_border: Color32::from_rgb(0x5a, 0x5f, 0x6e),
        selection: Color32::from_rgb(0x7c, 0x9c, 0xff),
        branch: Color32::from_rgb(0x70, 0x76, 0x88),
        link: Color32::from_rgb(0xff, 0x9a, 0x5c),
        link_label_bg: Color32::from_rgb(0x2a, 0x2d, 0x35),
    };

    pub fn for_dark_mode(dark: bool) -> Self {
        if dark { Self::DARK } else { Self::LIGHT }
    }
}
