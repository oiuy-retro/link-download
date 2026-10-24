//! Windows 98 message boxes.
//!
//! `MessageBoxW` is drawn by Windows itself. On Windows 10 and 11 it always gets the
//! modern look (white text area, Segoe UI, new icons, the system title bar), whatever
//! the app does. So the app draws its own box here, the same way the main window gets
//! its look: no system frame, the classic 3D frame and gradient caption from `classic`,
//! the gray face, MS Sans Serif, and classic push buttons with visual styles off.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DrawTextW, EndPaint, GetDC, GetDeviceCaps, GetMonitorInfoW, InvalidateRect,
    MonitorFromWindow, ReleaseDC, ScreenToClient, SelectObject, SetBkMode, SetTextColor, DT_CALCRECT,
    DT_EXPANDTABS, DT_LEFT, DT_NOPREFIX, DT_WORDBREAK, HBRUSH, HFONT, LOGPIXELSX, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::Graphics::Gdi::DeleteObject;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::SetWindowTheme;
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, IsWindowEnabled, ReleaseCapture, SetCapture, SetFocus};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetCursorPos, GetMessageW,
    GetWindowRect, IsDialogMessageW, LoadCursorW, PostQuitMessage, RegisterClassW, SendMessageW,
    SetForegroundWindow, ShowWindow, TranslateMessage, HMENU, HTCAPTION, HTCLIENT, IDC_ARROW, MSG, SW_SHOW,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_NCACTIVATE, WM_NCHITTEST, WM_PAINT, WM_SETFONT, WNDCLASSW,
};

use crate::classic::{self, CapBtn};

#[link(name = "user32")]
extern "system" {
    fn MessageBeep(kind: u32) -> i32;
}

/// The picture on the left of the box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxIcon {
    Warning,
    Error,
    Info,
}

impl BoxIcon {
    /// The Windows sound for this kind of box, the same one MessageBox plays.
    fn beep(self) -> u32 {
        match self {
            BoxIcon::Warning => 0x30,
            BoxIcon::Error => 0x10,
            BoxIcon::Info => 0x40,
        }
    }
}

/// Button ids. Enter presses the first button; Esc and the caption's X give `cancel`.
pub const ID_OK: i32 = 1;
pub const ID_CANCEL: i32 = 2;

struct BoxState {
    title: String,
    text: String,
    icon: BoxIcon,
    font: HFONT,
    caption_font: HFONT,
    face: HBRUSH,
    border: i32,
    caption_h: i32,
    icon_rect: RECT,
    text_rect: RECT,
    close_rect: RECT,
    default_id: i32,
    cancel_id: i32,
    active: bool,
    close_hot: bool,
    close_press: bool,
    result: Option<i32>,
}

thread_local! {
    static STATE: RefCell<Option<BoxState>> = const { RefCell::new(None) };
    static REGISTERED: Cell<bool> = const { Cell::new(false) };
    static WAITING: RefCell<VecDeque<(HWND, String, String, BoxIcon)>> = const { RefCell::new(VecDeque::new()) };
}

/// Shows a box with one OK button. When a box is already up, this one waits and opens
/// right after it closes, so two messages never stack on top of each other.
pub fn message(owner: HWND, title: &str, text: &str, icon: BoxIcon) {
    if STATE.with(|state| state.borrow().is_some()) {
        WAITING.with(|queue| queue.borrow_mut().push_back((owner, title.into(), text.into(), icon)));
        return;
    }
    show(owner, title, text, icon, &[("OK", ID_OK)]);
    while let Some((owner, title, text, icon)) = WAITING.with(|queue| queue.borrow_mut().pop_front()) {
        show(owner, &title, &text, icon, &[("OK", ID_OK)]);
    }
}

/// Shows a modal box and returns the id of the button that closed it. The first button is
/// the default (Enter). Esc, Alt+F4 and the X give `ID_CANCEL` when there is such a button,
/// else the last button. `&` in a label makes an Alt shortcut, as on any button.
pub fn show(owner: HWND, title: &str, text: &str, icon: BoxIcon, buttons: &[(&str, i32)]) -> i32 {
    let buttons: Vec<(&str, i32)> = if buttons.is_empty() { vec![("OK", ID_OK)] } else { buttons.to_vec() };
    let default_id = buttons[0].1;
    let cancel_id = buttons.iter().find(|(_, id)| *id == ID_CANCEL).map(|(_, id)| *id).unwrap_or(buttons[buttons.len() - 1].1);
    if STATE.with(|state| state.borrow().is_some()) {
        // Never nest. `message` queues; a direct nested call gives the cancel answer.
        return cancel_id;
    }
    unsafe {
        let instance = GetModuleHandleW(None).unwrap_or_default();
        register(instance.into());
        let dpi = screen_dpi();
        let font = crate::ui::make_font(8, dpi, false);
        let caption_font = classic::caption_font(dpi);
        let s = |px: i32| classic::scale(px, dpi);
        let border = s(3);
        let caption_h = s(18);
        let margin = s(12);
        let icon_px = s(32);
        let button_w = s(75);
        let button_h = s(23);
        let button_gap = s(6);

        // Measure the text the way it will be drawn: wrapped at a comfortable width.
        let (text_w, text_h) = measure(font, text, s(360));
        let (title_w, _) = measure(caption_font, title, s(2000));
        let text_x = margin + icon_px + s(14);
        let content_h = text_h.max(icon_px);
        let buttons_w = buttons.len() as i32 * button_w + (buttons.len() as i32 - 1) * button_gap;
        let client_w = (text_x + text_w + margin)
            .max(buttons_w + margin * 2)
            .max(title_w + s(40))
            .max(s(120));
        let buttons_y = margin + content_h + s(14);
        let client_h = buttons_y + button_h + margin;
        let win_w = client_w + border * 2;
        let win_h = client_h + border * 2 + caption_h;
        let top = border + caption_h;
        let text_top = top + margin + if text_h < icon_px { (icon_px - text_h) / 2 } else { 0 };
        let close_w = s(16);
        let close_h = s(14);
        let close_y = border + (caption_h - close_h) / 2;
        let state = BoxState {
            title: title.into(),
            text: text.into(),
            icon,
            font,
            caption_font,
            face: CreateSolidBrush(classic::rgb(classic::FACE)),
            border,
            caption_h,
            icon_rect: RECT { left: border + margin, top: top + margin, right: border + margin + icon_px, bottom: top + margin + icon_px },
            text_rect: RECT { left: border + text_x, top: text_top, right: border + text_x + text_w, bottom: text_top + text_h },
            close_rect: RECT { left: win_w - border - s(2) - close_w, top: close_y, right: win_w - border - s(2), bottom: close_y + close_h },
            default_id,
            cancel_id,
            active: true,
            close_hot: false,
            close_press: false,
            result: None,
        };
        STATE.with(|slot| *slot.borrow_mut() = Some(state));

        let (x, y) = place(owner, win_w, win_h);
        let wide_title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        // WS_POPUP | WS_CLIPCHILDREN | WS_SYSMENU. No system frame or caption, so Windows 10
        // and 11 draw nothing of their own. WS_EX_DLGMODALFRAME is left off for the same reason.
        let dlg = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("LinkDownloadBox"),
            PCWSTR(wide_title.as_ptr()),
            WINDOW_STYLE(0x80000000 | 0x02000000 | 0x00080000),
            x,
            y,
            win_w,
            win_h,
            owner,
            HMENU(std::ptr::null_mut()),
            instance,
            None,
        )
        .unwrap_or_default();
        if dlg.0.is_null() {
            STATE.with(|slot| *slot.borrow_mut() = None);
            let _ = DeleteObject(font);
            let _ = DeleteObject(caption_font);
            return cancel_id;
        }
        let mut first = HWND(std::ptr::null_mut());
        let mut bx = border + (client_w - buttons_w) / 2;
        for (index, (label, id)) in buttons.iter().enumerate() {
            let wide_label: Vec<u16> = label.encode_utf16().chain(Some(0)).collect();
            // Child, visible, tab stop, BS_DEFPUSHBUTTON on the first one.
            let style = 0x40000000 | 0x10000000 | 0x00010000 | if index == 0 { 1 } else { 0 };
            let button = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("Button"),
                PCWSTR(wide_label.as_ptr()),
                WINDOW_STYLE(style),
                bx,
                top + buttons_y,
                button_w,
                button_h,
                dlg,
                HMENU(*id as isize as *mut _),
                instance,
                None,
            )
            .unwrap_or_default();
            SendMessageW(button, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
            let _ = SetWindowTheme(button, w!(""), w!(""));
            if index == 0 {
                first = button;
            }
            bx += button_w + button_gap;
        }

        let owner_was_enabled = !owner.0.is_null() && IsWindowEnabled(owner).as_bool();
        if owner_was_enabled {
            let _ = EnableWindow(owner, false);
        }
        let _ = ShowWindow(dlg, SW_SHOW);
        let _ = SetForegroundWindow(dlg);
        let _ = SetFocus(first);
        MessageBeep(icon.beep());

        let mut msg = MSG::default();
        let mut quit = None;
        while STATE.with(|slot| slot.borrow().as_ref().map(|state| state.result.is_none()).unwrap_or(false)) {
            let got = GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0);
            if got.0 <= 0 {
                quit = Some(msg.wParam.0 as i32);
                break;
            }
            // Enter, Esc, Tab, arrows and Alt shortcuts, the same as in a dialog box.
            if IsDialogMessageW(dlg, &msg).as_bool() {
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        // Turn the owner back on before the box goes, so Windows activates the owner
        // and not some other program.
        if owner_was_enabled {
            let _ = EnableWindow(owner, true);
            let _ = SetForegroundWindow(owner);
        }
        let _ = DestroyWindow(dlg);
        let state = STATE.with(|slot| slot.borrow_mut().take());
        if let Some(code) = quit {
            PostQuitMessage(code);
        }
        let _ = DeleteObject(font);
        let _ = DeleteObject(caption_font);
        match state {
            Some(state) => {
                let _ = DeleteObject(state.face);
                state.result.unwrap_or(cancel_id)
            }
            None => cancel_id,
        }
    }
}

fn register(instance: windows::Win32::Foundation::HINSTANCE) {
    if REGISTERED.get() {
        return;
    }
    unsafe {
        let class = WNDCLASSW {
            lpfnWndProc: Some(box_proc),
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: w!("LinkDownloadBox"),
            ..Default::default()
        };
        let _ = RegisterClassW(&class);
    }
    REGISTERED.set(true);
}

fn screen_dpi() -> i32 {
    unsafe {
        let dc = GetDC(None);
        let dpi = GetDeviceCaps(dc, LOGPIXELSX);
        let _ = ReleaseDC(None, dc);
        if dpi <= 0 { 96 } else { dpi }
    }
}

/// Width and height of `text` in `font`, wrapped at `max_w`.
fn measure(font: HFONT, text: &str, max_w: i32) -> (i32, i32) {
    if text.is_empty() {
        return (0, 0);
    }
    unsafe {
        let dc = GetDC(None);
        let old = SelectObject(dc, font);
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        let mut rect = RECT { left: 0, top: 0, right: max_w, bottom: 0 };
        DrawTextW(dc, &mut wide, &mut rect, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX | DT_EXPANDTABS | DT_LEFT);
        SelectObject(dc, old);
        let _ = ReleaseDC(None, dc);
        (rect.right - rect.left, rect.bottom - rect.top)
    }
}

/// Centered on the owner, kept inside the owner's screen.
fn place(owner: HWND, w: i32, h: i32) -> (i32, i32) {
    unsafe {
        let mut area = RECT { left: 0, top: 0, right: 800, bottom: 600 };
        let monitor = MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            area = info.rcWork;
        }
        let mut around = area;
        if !owner.0.is_null() {
            let _ = GetWindowRect(owner, &mut around);
        }
        centered(around, area, w, h)
    }
}

/// Where a `w` by `h` box goes: centered on `around`, then moved inside `area`.
fn centered(around: RECT, area: RECT, w: i32, h: i32) -> (i32, i32) {
    let x = around.left + (around.right - around.left - w) / 2;
    let y = around.top + (around.bottom - around.top - h) / 2;
    let x = x.min(area.right - w).max(area.left);
    let y = y.min(area.bottom - h).max(area.top);
    (x, y)
}

fn finish(id: i32) {
    STATE.with(|slot| {
        if let Some(state) = slot.borrow_mut().as_mut() {
            if state.result.is_none() {
                state.result = Some(id);
            }
        }
    });
}

fn point_of(lparam: LPARAM) -> POINT {
    POINT { x: (lparam.0 & 0xFFFF) as i16 as i32, y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32 }
}

unsafe extern "system" fn box_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        0x0135 => {
            // WM_CTLCOLORBTN: the corners around each button stay gray.
            let face = STATE.with(|slot| slot.borrow().as_ref().map(|state| state.face).unwrap_or_default());
            LRESULT(face.0 as isize)
        }
        WM_NCHITTEST => {
            let mut pt = point_of(lparam);
            let _ = ScreenToClient(hwnd, &mut pt);
            let caption = STATE.with(|slot| {
                slot.borrow().as_ref().map(|state| {
                    let in_bar = pt.y >= state.border && pt.y < state.border + state.caption_h;
                    in_bar && !classic::hit(state.close_rect, pt)
                })
            });
            // Dragging the caption moves the box, as on any window.
            LRESULT(if caption == Some(true) { HTCAPTION as isize } else { HTCLIENT as isize })
        }
        WM_NCACTIVATE => {
            let on = wparam.0 != 0;
            STATE.with(|slot| {
                if let Some(state) = slot.borrow_mut().as_mut() {
                    state.active = on;
                }
            });
            invalidate_caption(hwnd);
            // TRUE: there is no system frame to repaint.
            LRESULT(1)
        }
        WM_LBUTTONDOWN => {
            let pt = point_of(lparam);
            let on_close = STATE.with(|slot| slot.borrow().as_ref().map(|state| classic::hit(state.close_rect, pt)).unwrap_or(false));
            if on_close {
                STATE.with(|slot| {
                    if let Some(state) = slot.borrow_mut().as_mut() {
                        state.close_press = true;
                        state.close_hot = true;
                    }
                });
                SetCapture(hwnd);
                invalidate_caption(hwnd);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let pt = point_of(lparam);
            let changed = STATE.with(|slot| {
                let mut borrow = slot.borrow_mut();
                let Some(state) = borrow.as_mut() else { return false };
                let hot = classic::hit(state.close_rect, pt);
                let changed = hot != state.close_hot;
                state.close_hot = hot;
                changed && state.close_press
            });
            if changed {
                invalidate_caption(hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let pt = point_of(lparam);
            let close = STATE.with(|slot| {
                let mut borrow = slot.borrow_mut();
                let Some(state) = borrow.as_mut() else { return None };
                let pressed = state.close_press;
                state.close_press = false;
                state.close_hot = false;
                (pressed && classic::hit(state.close_rect, pt)).then_some(state.cancel_id)
            });
            let _ = ReleaseCapture();
            invalidate_caption(hwnd);
            if let Some(id) = close {
                finish(id);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            let (default_id, cancel_id) = STATE.with(|slot| slot.borrow().as_ref().map(|state| (state.default_id, state.cancel_id)).unwrap_or((ID_OK, ID_CANCEL)));
            if lparam.0 != 0 {
                // A button was pressed: by click, Space, Enter on it, or its Alt shortcut.
                finish(id);
            } else if id == ID_CANCEL {
                // Esc from the dialog keys.
                finish(cancel_id);
            } else if id == ID_OK {
                finish(default_id);
            }
            LRESULT(0)
        }
        0x0400 => {
            // DM_GETDEFID, asked by the dialog keys to know which button Enter presses.
            let default_id = STATE.with(|slot| slot.borrow().as_ref().map(|state| state.default_id).unwrap_or(ID_OK));
            LRESULT(((0x534B_isize) << 16) | (default_id as isize & 0xFFFF))
        }
        WM_CLOSE => {
            let cancel_id = STATE.with(|slot| slot.borrow().as_ref().map(|state| state.cancel_id).unwrap_or(ID_CANCEL));
            finish(cancel_id);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn invalidate_caption(hwnd: HWND) {
    unsafe {
        let mut rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut rect);
        let caption = STATE.with(|slot| slot.borrow().as_ref().map(|state| (state.border, state.caption_h)));
        if let Some((border, caption_h)) = caption {
            rect.bottom = border + caption_h;
        }
        let _ = InvalidateRect(hwnd, Some(&rect), false);
    }
}

fn paint(hwnd: HWND) {
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mut whole = RECT::default();
        let _ = GetClientRect(hwnd, &mut whole);
        STATE.with(|slot| {
            let borrow = slot.borrow();
            let Some(state) = borrow.as_ref() else { return };
            classic::fill(hdc, whole, classic::FACE);
            classic::window_frame(hdc, whole, state.border);
            let caption = RECT {
                left: state.border,
                top: state.border,
                right: whole.right - state.border,
                bottom: state.border + state.caption_h,
            };
            classic::caption_fill(hdc, caption, state.active);
            // A message box caption has no icon. The title starts at the left edge.
            let title = RECT { left: caption.left + classic::scale(3, crate::popup::dpi_of(state)), right: state.close_rect.left - 2, ..caption };
            let color = if state.active { classic::WHITE } else { (212, 208, 200) };
            classic::text_left(hdc, state.caption_font, title, &state.title, color);
            let mut cursor = POINT::default();
            let _ = GetCursorPos(&mut cursor);
            let _ = ScreenToClient(hwnd, &mut cursor);
            let pressed = state.close_press && classic::hit(state.close_rect, cursor);
            classic::caption_button(hdc, state.close_rect, CapBtn::Close, false, pressed, state.active);
            draw_icon(hdc, state.icon, state.icon_rect);
            let old = SelectObject(hdc, state.font);
            let _ = SetBkMode(hdc, TRANSPARENT);
            let _ = SetTextColor(hdc, classic::rgb(classic::BLACK));
            let mut wide: Vec<u16> = state.text.encode_utf16().collect();
            let mut area = state.text_rect;
            let _ = DrawTextW(hdc, &mut wide, &mut area, DT_WORDBREAK | DT_NOPREFIX | DT_EXPANDTABS | DT_LEFT);
            SelectObject(hdc, old);
        });
        let _ = EndPaint(hwnd, &ps);
    }
}

fn dpi_of(state: &BoxState) -> i32 {
    // The border is 3 pixels at 96 DPI.
    (state.border * 96 / 3).max(96)
}

/// Draws the 32 by 32 classic icon scaled into `rect`, one block per icon pixel.
fn draw_icon(hdc: windows::Win32::Graphics::Gdi::HDC, icon: BoxIcon, rect: RECT) {
    let size = rect.right - rect.left;
    let edge = |n: i32| n * size / 32;
    for y in 0..32 {
        let mut x = 0;
        while x < 32 {
            let color = icon_pixel(icon, x, y);
            let mut end = x + 1;
            while end < 32 && icon_pixel(icon, end, y) == color {
                end += 1;
            }
            if let Some(color) = color {
                let block = RECT {
                    left: rect.left + edge(x),
                    top: rect.top + edge(y),
                    right: rect.left + edge(end),
                    bottom: rect.top + edge(y + 1),
                };
                classic::fill(hdc, block, color);
            }
            x = end;
        }
    }
}

const YELLOW: (u8, u8, u8) = (255, 255, 0);
const RED: (u8, u8, u8) = (255, 0, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);

/// One pixel of the classic 32 by 32 icons: warning (yellow triangle, black "!"),
/// error (red disc, white X) and information (white disc, blue "i"). Each has a black
/// outline and a gray shadow down and to the right. `None` is see-through.
fn icon_pixel(icon: BoxIcon, x: i32, y: i32) -> Option<(u8, u8, u8)> {
    let px = x as f32 + 0.5;
    let py = y as f32 + 0.5;
    match icon {
        BoxIcon::Warning => {
            let inside = |dx: f32, dy: f32| triangle_depth(px - dx, py - dy);
            let depth = inside(0.0, 0.0);
            if depth > 0.0 {
                if depth < 1.3 {
                    return Some(classic::BLACK);
                }
                let bar = (14..18).contains(&x) && (9..17).contains(&y) || (15..17).contains(&x) && (17..20).contains(&y);
                let dot = (14..18).contains(&x) && (22..26).contains(&y);
                return Some(if bar || dot { classic::BLACK } else { YELLOW });
            }
            (inside(2.0, 2.0) > 0.0).then_some(classic::DARK)
        }
        BoxIcon::Error | BoxIcon::Info => {
            let (cx, cy, r) = (15.0_f32, 15.0_f32, 13.5_f32);
            let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
            if d <= r {
                if d > r - 1.3 {
                    return Some(classic::BLACK);
                }
                let (dx, dy) = (px - cx, py - cy);
                if icon == BoxIcon::Error {
                    let cross = d < 8.5 && ((dx - dy).abs() < 2.3 || (dx + dy).abs() < 2.3);
                    return Some(if cross { classic::WHITE } else { RED });
                }
                let dot = ((px - 15.5).powi(2) + (py - 8.5).powi(2)).sqrt() < 2.2;
                let stem = (14..18).contains(&x) && (12..22).contains(&y);
                let foot = (12..20).contains(&x) && (21..23).contains(&y);
                let cap = (13..18).contains(&x) && (12..14).contains(&y);
                return Some(if dot || stem || foot || cap { BLUE } else { classic::WHITE });
            }
            let shadow = ((px - cx - 2.0).powi(2) + (py - cy - 2.0).powi(2)).sqrt() <= r;
            shadow.then_some(classic::DARK)
        }
    }
}

/// How far inside the warning triangle a point is, in pixels. Negative is outside.
fn triangle_depth(x: f32, y: f32) -> f32 {
    let corners = [(15.5_f32, 1.5_f32), (29.0, 27.5), (2.0, 27.5)];
    let mut depth = f32::MAX;
    for i in 0..3 {
        let (ax, ay) = corners[i];
        let (bx, by) = corners[(i + 1) % 3];
        let (ex, ey) = (bx - ax, by - ay);
        let len = (ex * ex + ey * ey).sqrt();
        // Clockwise on screen, so the inside is to the right of each edge.
        let side = (ex * (y - ay) - ey * (x - ax)) / len;
        depth = depth.min(side);
    }
    depth
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_warning_icon_is_a_yellow_triangle_with_a_black_mark() {
        assert_eq!(icon_pixel(BoxIcon::Warning, 0, 0), None);
        assert_eq!(icon_pixel(BoxIcon::Warning, 31, 0), None);
        assert_eq!(icon_pixel(BoxIcon::Warning, 15, 12), Some(classic::BLACK));
        assert_eq!(icon_pixel(BoxIcon::Warning, 15, 24), Some(classic::BLACK));
        assert_eq!(icon_pixel(BoxIcon::Warning, 8, 24), Some(YELLOW));
        assert_eq!(icon_pixel(BoxIcon::Warning, 15, 21), Some(YELLOW));
        // The shadow peeks out below the base.
        assert_eq!(icon_pixel(BoxIcon::Warning, 20, 28), Some(classic::DARK));
    }

    #[test]
    fn the_error_icon_is_a_red_disc_with_a_white_cross() {
        assert_eq!(icon_pixel(BoxIcon::Error, 15, 15), Some(classic::WHITE));
        assert_eq!(icon_pixel(BoxIcon::Error, 15, 5), Some(RED));
        assert_eq!(icon_pixel(BoxIcon::Error, 15, 2), Some(classic::BLACK));
        assert_eq!(icon_pixel(BoxIcon::Error, 0, 0), None);
    }

    #[test]
    fn the_info_icon_is_a_white_disc_with_a_blue_i() {
        assert_eq!(icon_pixel(BoxIcon::Info, 15, 8), Some(BLUE));
        assert_eq!(icon_pixel(BoxIcon::Info, 15, 17), Some(BLUE));
        assert_eq!(icon_pixel(BoxIcon::Info, 7, 15), Some(classic::WHITE));
        assert_eq!(icon_pixel(BoxIcon::Info, 31, 31), None);
    }

    #[test]
    fn a_box_is_centered_on_its_owner_and_kept_on_screen() {
        let screen = RECT { left: 0, top: 0, right: 1000, bottom: 800 };
        let owner = RECT { left: 100, top: 100, right: 500, bottom: 400 };
        assert_eq!(centered(owner, screen, 200, 100), (200, 200));
        let corner = RECT { left: -300, top: 700, right: 0, bottom: 900 };
        assert_eq!(centered(corner, screen, 200, 100), (0, 700));
    }

    #[test]
    fn beeps_match_message_box_sounds() {
        assert_eq!(BoxIcon::Warning.beep(), 0x30);
        assert_eq!(BoxIcon::Error.beep(), 0x10);
        assert_eq!(BoxIcon::Info.beep(), 0x40);
    }
}
