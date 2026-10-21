//! Windows 98 classic window chrome: raised 3D frame, gradient caption,
//! and the small 3D minimize / maximize / close buttons.

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, FillRect, SelectObject, SetBkMode,
    SetTextColor, DEFAULT_QUALITY, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
    HDC, HFONT, TRANSPARENT,
};

pub const FACE: (u8, u8, u8) = (192, 192, 192);
pub const WHITE: (u8, u8, u8) = (255, 255, 255);
pub const BLACK: (u8, u8, u8) = (0, 0, 0);
pub const DARK: (u8, u8, u8) = (128, 128, 128);
pub const SHADOW: (u8, u8, u8) = (64, 64, 64);


pub fn rgb(c: (u8, u8, u8)) -> COLORREF {
    COLORREF(c.0 as u32 | ((c.1 as u32) << 8) | ((c.2 as u32) << 16))
}

pub fn lerp(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    (
        (a.0 as f32 + (b.0 as f32 - a.0 as f32) * t).round() as u8,
        (a.1 as f32 + (b.1 as f32 - a.1 as f32) * t).round() as u8,
        (a.2 as f32 + (b.2 as f32 - a.2 as f32) * t).round() as u8,
    )
}

pub fn fill(hdc: HDC, r: RECT, c: (u8, u8, u8)) {
    unsafe {
        let br = CreateSolidBrush(rgb(c));
        let _ = FillRect(hdc, &r, br);
        let _ = DeleteObject(br);
    }
}

pub fn vline(hdc: HDC, x: i32, y0: i32, y1: i32, c: (u8, u8, u8)) {
    fill(
        hdc,
        RECT {
            left: x,
            top: y0,
            right: x + 1,
            bottom: y1,
        },
        c,
    );
}

pub fn hline(hdc: HDC, x0: i32, x1: i32, y: i32, c: (u8, u8, u8)) {
    fill(
        hdc,
        RECT {
            left: x0,
            top: y,
            right: x1,
            bottom: y + 1,
        },
        c,
    );
}

pub fn frame_rect(hdc: HDC, r: RECT, c: (u8, u8, u8)) {
    hline(hdc, r.left, r.right, r.top, c);
    hline(hdc, r.left, r.right, r.bottom - 1, c);
    vline(hdc, r.left, r.top, r.bottom, c);
    vline(hdc, r.right - 1, r.top, r.bottom, c);
}

pub fn hit(r: RECT, p: POINT) -> bool {
    p.x >= r.left && p.x < r.right && p.y >= r.top && p.y < r.bottom
}

pub fn inset(r: RECT, n: i32) -> RECT {
    RECT {
        left: r.left + n,
        top: r.top + n,
        right: r.right - n,
        bottom: r.bottom - n,
    }
}

pub fn scale(px: i32, dpi: i32) -> i32 {
    px * dpi / 96
}

pub fn caption_font(dpi: i32) -> HFONT {
    unsafe {
        CreateFontW(
            -((8 * dpi + 36) / 72),
            0,
            0,
            0,
            700,
            0,
            0,
            0,
            1,
            0,
            0,
            DEFAULT_QUALITY.0 as u32,
            0,
            w!("MS Sans Serif"),
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CapHot {
    None,
    Min,
    Max,
    Close,
    Icon,
    Caption,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CapBtn {
    Min,
    Max,
    Restore,
    Close,
}

pub struct Layout {
    pub window: RECT,
    pub caption: RECT,
    pub btn_min: RECT,
    pub btn_max: RECT,
    pub btn_close: RECT,
    pub icon: RECT,
    pub title: RECT,
    pub inner: RECT,
    pub border: i32,
    pub caption_h: i32,
}

pub fn layout(window: RECT, dpi: i32, maximized: bool) -> Layout {
    let border = if maximized { 0 } else { scale(3, dpi) };
    let caption_h = scale(18, dpi);
    let btn_w = scale(16, dpi);
    let btn_h = scale(14, dpi);
    let gap = scale(2, dpi);
    let inner = RECT {
        left: window.left + border,
        top: window.top + border,
        right: window.right - border,
        bottom: window.bottom - border,
    };
    let caption = RECT {
        left: inner.left,
        top: inner.top,
        right: inner.right,
        bottom: inner.top + caption_h,
    };
    let by = caption.top + (caption_h - btn_h) / 2;
    let close = RECT {
        left: caption.right - gap - btn_w,
        top: by,
        right: caption.right - gap,
        bottom: by + btn_h,
    };
    let max = RECT {
        left: close.left - gap - btn_w,
        top: by,
        right: close.left - gap,
        bottom: by + btn_h,
    };
    let min = RECT {
        left: max.left - btn_w,
        top: by,
        right: max.left,
        bottom: by + btn_h,
    };
    let icon_px = scale(16, dpi);
    let iy = caption.top + (caption_h - icon_px) / 2;
    let icon = RECT {
        left: caption.left + scale(2, dpi),
        top: iy,
        right: caption.left + scale(2, dpi) + icon_px,
        bottom: iy + icon_px,
    };
    let title = RECT {
        left: icon.right + scale(3, dpi),
        top: caption.top,
        right: min.left - scale(4, dpi),
        bottom: caption.bottom,
    };
    Layout {
        window,
        caption,
        btn_min: min,
        btn_max: max,
        btn_close: close,
        icon,
        title,
        inner,
        border,
        caption_h,
    }
}

pub fn raised(hdc: HDC, r: RECT) {
    hline(hdc, r.left, r.right, r.top, WHITE);
    vline(hdc, r.left, r.top, r.bottom, WHITE);
    hline(hdc, r.left, r.right, r.bottom - 1, SHADOW);
    vline(hdc, r.right - 1, r.top, r.bottom, SHADOW);
    hline(hdc, r.left + 1, r.right - 1, r.bottom - 2, DARK);
    vline(hdc, r.right - 2, r.top + 1, r.bottom - 1, DARK);
}

pub fn sunken(hdc: HDC, r: RECT) {
    hline(hdc, r.left, r.right, r.top, DARK);
    vline(hdc, r.left, r.top, r.bottom, DARK);
    hline(hdc, r.left, r.right, r.bottom - 1, WHITE);
    vline(hdc, r.right - 1, r.top, r.bottom, WHITE);
    hline(hdc, r.left + 1, r.right - 1, r.top + 1, SHADOW);
    vline(hdc, r.left + 1, r.top + 1, r.bottom - 1, SHADOW);
}

pub fn window_frame(hdc: HDC, r: RECT, border: i32) {
    if border > 0 {
        fill(hdc, RECT { left: r.left, top: r.top, right: r.right, bottom: r.top + border }, FACE);
        fill(hdc, RECT { left: r.left, top: r.bottom - border, right: r.right, bottom: r.bottom }, FACE);
        fill(hdc, RECT { left: r.left, top: r.top, right: r.left + border, bottom: r.bottom }, FACE);
        fill(hdc, RECT { left: r.right - border, top: r.top, right: r.right, bottom: r.bottom }, FACE);
        raised(hdc, r);
    }
}

pub fn is_button(hot: CapHot) -> bool {
    matches!(hot, CapHot::Min | CapHot::Max | CapHot::Close)
}

fn caption_fill(hdc: HDC, r: RECT, active: bool) {
    let (left, right) = if active {
        ((0x0A, 0x24, 0x6A), (0xA6, 0xCA, 0xF0))
    } else {
        ((0x80, 0x80, 0x80), (0xC0, 0xC0, 0xC0))
    };
    let w = (r.right - r.left).max(1);
    for x in r.left..r.right {
        let t = (x - r.left) as f32 / w as f32;
        vline(hdc, x, r.top, r.bottom, lerp(left, right, t));
    }
}

pub fn text_left(hdc: HDC, font: HFONT, r: RECT, s: &str, color: (u8, u8, u8)) {
    if s.is_empty() {
        return;
    }
    let mut w: Vec<u16> = s.encode_utf16().collect();
    unsafe {
        let old = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, rgb(color));
        let _ = DrawTextW(
            hdc,
            &mut w,
            &mut { r },
            windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT(
                DT_LEFT.0 | DT_VCENTER.0 | DT_SINGLELINE.0 | DT_NOPREFIX.0 | DT_END_ELLIPSIS.0,
            ),
        );
        SelectObject(hdc, old);
    }
}

pub fn caption_button(hdc: HDC, r: RECT, kind: CapBtn, hover: bool, press: bool, _active: bool) {
    fill(hdc, r, FACE);
    if press {
        sunken(hdc, r);
    } else {
        raised(hdc, r);
        if hover {
            frame_rect(hdc, inset(r, 1), WHITE);
        }
    }
    let mut cx = (r.left + r.right) / 2;
    let mut cy = (r.top + r.bottom) / 2;
    if press {
        cx += 1;
        cy += 1;
    }
    let ink = BLACK;
    match kind {
        CapBtn::Min => {
            hline(hdc, cx - 4, cx + 5, cy + 3, ink);
            hline(hdc, cx - 4, cx + 5, cy + 4, ink);
        }
        CapBtn::Max => {
            frame_rect(
                hdc,
                RECT {
                    left: cx - 4,
                    top: cy - 4,
                    right: cx + 5,
                    bottom: cy + 4,
                },
                ink,
            );
            hline(hdc, cx - 4, cx + 5, cy - 3, ink);
        }
        CapBtn::Restore => {
            frame_rect(
                hdc,
                RECT {
                    left: cx - 2,
                    top: cy - 5,
                    right: cx + 5,
                    bottom: cy + 1,
                },
                ink,
            );
            hline(hdc, cx - 2, cx + 5, cy - 4, ink);
            frame_rect(
                hdc,
                RECT {
                    left: cx - 5,
                    top: cy - 2,
                    right: cx + 2,
                    bottom: cy + 5,
                },
                ink,
            );
            hline(hdc, cx - 5, cx + 2, cy - 1, ink);
            fill(
                hdc,
                RECT {
                    left: cx - 4,
                    top: cy - 1,
                    right: cx + 1,
                    bottom: cy + 4,
                },
                FACE,
            );
        }
        CapBtn::Close => {
            for i in -3..=3 {
                fill(
                    hdc,
                    RECT {
                        left: cx + i,
                        top: cy + i,
                        right: cx + i + 1,
                        bottom: cy + i + 1,
                    },
                    ink,
                );
                fill(
                    hdc,
                    RECT {
                        left: cx + i,
                        top: cy - i,
                        right: cx + i + 1,
                        bottom: cy - i + 1,
                    },
                    ink,
                );
                fill(
                    hdc,
                    RECT {
                        left: cx + i - 1,
                        top: cy + i,
                        right: cx + i,
                        bottom: cy + i + 1,
                    },
                    ink,
                );
                fill(
                    hdc,
                    RECT {
                        left: cx + i - 1,
                        top: cy - i,
                        right: cx + i,
                        bottom: cy - i + 1,
                    },
                    ink,
                );
            }
        }
    }
}

pub fn paint_caption(
    hdc: HDC,
    l: &Layout,
    title: &str,
    font: HFONT,
    active: bool,
    maximized: bool,
    hot: CapHot,
    press: CapHot,
) {
    caption_fill(hdc, l.caption, active);
    let color = if active { WHITE } else { (212, 208, 200) };
    text_left(hdc, font, l.title, title, color);
    let max_kind = if maximized { CapBtn::Restore } else { CapBtn::Max };
    caption_button(
        hdc,
        l.btn_min,
        CapBtn::Min,
        hot == CapHot::Min,
        press == CapHot::Min,
        active,
    );
    caption_button(
        hdc,
        l.btn_max,
        max_kind,
        hot == CapHot::Max,
        press == CapHot::Max,
        active,
    );
    caption_button(
        hdc,
        l.btn_close,
        CapBtn::Close,
        hot == CapHot::Close,
        press == CapHot::Close,
        active,
    );
}

pub fn gripper(hdc: HDC, r: RECT) {
    let x = r.right - 12;
    let y = r.bottom - 12;
    for row in 0..3 {
        for col in 0..3 {
            if col + row < 2 {
                continue;
            }
            let px = x + col * 3;
            let py = y + row * 3;
            fill(
                hdc,
                RECT {
                    left: px + 1,
                    top: py + 1,
                    right: px + 3,
                    bottom: py + 3,
                },
                WHITE,
            );
            fill(
                hdc,
                RECT {
                    left: px,
                    top: py,
                    right: px + 2,
                    bottom: py + 2,
                },
                DARK,
            );
        }
    }
}

pub fn hot_at(l: &Layout, p: POINT) -> CapHot {
    if hit(l.btn_close, p) {
        CapHot::Close
    } else if hit(l.btn_max, p) {
        CapHot::Max
    } else if hit(l.btn_min, p) {
        CapHot::Min
    } else if hit(l.icon, p) {
        CapHot::Icon
    } else if hit(l.caption, p) {
        CapHot::Caption
    } else {
        CapHot::None
    }
}
