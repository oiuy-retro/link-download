//! Windows 98 wizard. Classic controls, MS Sans Serif, no visual styles.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::mem::size_of;
use std::path::PathBuf;
use std::time::Instant;

use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    BOOL, COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW,
    CreateRectRgn, CreateSolidBrush, DeleteDC, DeleteObject, DrawEdge, DrawTextW, EndPaint, FillRect, GetDC,
    GetDeviceCaps, GetMonitorInfoW, GetSysColor, InflateRect, InvalidateRect, MapWindowPoints, MonitorFromWindow,
    ScreenToClient,
    RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE, RedrawWindow, ReleaseDC, SelectObject, SetBkColor,
    SetBkMode, SetTextColor, UpdateWindow, BF_RECT, COLOR_BTNFACE, COLOR_BTNTEXT, COLOR_GRAYTEXT, COLOR_WINDOW,
    COLOR_WINDOWTEXT, EDGE_ETCHED, EDGE_RAISED, EDGE_SUNKEN, HBRUSH, HDC, HFONT, HRGN, LOGPIXELSX,
    MONITORINFO, MONITOR_DEFAULTTONEAREST, SetWindowRgn, SRCCOPY, TRANSPARENT, DT_END_ELLIPSIS, DT_LEFT,
    DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, DT_EDITCONTROL,
    DT_WORDBREAK, DrawFocusRect, GetSysColorBrush, GetTextExtentPoint32W, GetTextMetricsW, SetTextAlign, TextOutW,
    SYS_COLOR_INDEX, TEXTMETRICW, TEXT_ALIGN_OPTIONS,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, SetScrollInfo, SetWindowTheme, ShowScrollBar, ICC_WIN95_CLASSES, INITCOMMONCONTROLSEX,
    LVM_GETITEMCOUNT, LVM_GETITEMTEXTW, LVM_INSERTCOLUMNW, LVM_INSERTITEMW,
    LVM_SETITEMTEXTW, LVM_SETEXTENDEDLISTVIEWSTYLE, LVN_ENDLABELEDITW, LVM_EDITLABELW,
    LVS_EX_FULLROWSELECT, NMITEMACTIVATE, NM_CLICK, NM_DBLCLK, PBM_SETPOS, PBM_SETRANGE32, TTM_ADDTOOLW,
    TTM_SETMAXTIPWIDTH, TTS_ALWAYSTIP, TTS_NOPREFIX, LVCF_FMT, LVCF_TEXT, LVCF_WIDTH,
    LVCFMT_LEFT, LVCOLUMNW, LVIF_TEXT, LVITEMW, NMHDR, NMLVDISPINFOW, TTF_IDISHWND, TTF_SUBCLASS,
    TTTOOLINFOW, DRAWITEMSTRUCT, NMLVCUSTOMDRAW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, GetKeyState, IsWindowEnabled, ReleaseCapture, SetFocus, VK_BACK, VK_CONTROL, VK_DELETE,
    VK_DOWN, VK_LEFT, VK_MENU, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_TAB, VK_UP,
};
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileDialog, IShellItem, SHCreateItemFromParsingName, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS,
    SIGDN_FILESYSPATH, DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, DrawIconEx, GetClientRect,
    GetAncestor, GetClassNameW, GetCursorPos, GetScrollInfo, GA_ROOT, GetDlgItem, GetMessageW, GetParent, GetSystemMetrics, GetWindow,
    PostMessageW, WM_APP, IsChild, IsWindow,
    GetWindowLongPtrW, GetWindowRect, IsWindowVisible, IsZoomed, KillTimer, LoadCursorW, LoadImageW,
    MoveWindow, PostQuitMessage, RegisterClassW, SendMessageW, SetTimer, SetWindowLongPtrW,
    SetWindowPos, SetWindowTextW, ShowWindow, TranslateMessage, WindowFromPoint, BM_GETCHECK, BM_SETCHECK,
    CB_ADDSTRING, CB_GETCURSEL, CB_RESETCONTENT, CB_SETCURSEL, CBN_SELCHANGE, CS_DBLCLKS, CS_HREDRAW,
    CS_VREDRAW, CW_USEDEFAULT, DI_NORMAL, EN_CHANGE, EN_KILLFOCUS, GW_CHILD, GW_HWNDNEXT, GWL_STYLE,
    GWLP_ID, GWLP_USERDATA, HMENU, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTCLIENT, HTLEFT, HTRIGHT, HTTOP,
    HTTOPLEFT, HTTOPRIGHT, HWND_TOP, IDC_ARROW, IMAGE_ICON, MINMAXINFO, MSG,
    NCCALCSIZE_PARAMS, SB_VERT, SCROLLINFO, SIF_PAGE, SIF_POS, SIF_RANGE, SM_CXFRAME, SM_CXPADDEDBORDER,
    SM_CXSCREEN, SM_CYFRAME, SM_CYSCREEN, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, WM_ACTIVATE, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_ERASEBKGND, WM_GETMINMAXINFO,
    WM_CHAR, WM_GETTEXT, WM_GETTEXTLENGTH, WM_KEYDOWN, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_NCCALCSIZE, WM_NCDESTROY, WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NOTIFY, WM_PAINT, WM_SETFONT, WM_SIZE,
    WM_TIMER, WM_VSCROLL, WNDCLASSW, WS_CLIPCHILDREN, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_OVERLAPPED,
    WS_SYSMENU, WS_THICKFRAME,
};

use crate::classic::{self, CapHot};
use crate::paths::Mark;

use crate::model::{
    can_advance, container_after_toggle, problems, AudioCodec, AudioQuality, Container, Options,
    Resolution, VideoCodec,
};
use crate::names::{self, InsertAt, TextTools};
use crate::readout;
use crate::run::{self, Live, RowKind, Shared, WorkRow};
use crate::store::{self, Memory, SavedProfile};
use crate::{model, paths, tools};

const ID_BACK: i32 = 1;
const ID_NEXT: i32 = 2;
const ID_PROFILE: i32 = 10;
const ID_SAVE: i32 = 11;
const ID_NAME: i32 = 12;
const ID_RES: i32 = 13;
const ID_VCODEC: i32 = 14;
const ID_AQUAL: i32 = 15;
const ID_ACODEC: i32 = 16;
const ID_CONTAINER: i32 = 17;
const ID_AUDIO: i32 = 18;
const ID_REENCODE: i32 = 19;
const ID_SPONSORBLOCK: i32 = 24;
const ID_STRIP_GENRE: i32 = 25;
const ID_OVERWRITE: i32 = 26;
const ID_MIN: i32 = 20;
const ID_MAX: i32 = 21;
const ID_LOCATION: i32 = 22;
const ID_BROWSE: i32 = 23;
const ID_LINKS: i32 = 30;
const ID_LINK_ERROR: i32 = 31;
const ID_STATUS: i32 = 50;
const ID_PROGRESS: i32 = 51;
const ID_PERCENT: i32 = 53;
const ID_READOUT: i32 = 60;
const ID_REMOVE: i32 = 1000;
const ID_INSERT: i32 = 2000;
const ID_MODE: i32 = 3000;
const ID_FIND: i32 = 4000;
const ID_STRIP: i32 = 5000;
const ID_APPLY: i32 = 6000;
const ID_LIST: i32 = 7000;
const ID_ALBUM: i32 = 8000;
const ID_ARTIST: i32 = 9000;
const ID_RESET: i32 = 16000;
const ID_META_RESET: i32 = 17000;
const ID_NAME_ERROR: i32 = 18000;
const ID_TITLE_SAME_ALL: i32 = 21000;
const ID_TITLE_FIELD: i32 = 22000;
const ID_TITLE_SAME: i32 = 43000;
const GROUP_LIMIT: usize = 1000;
/// Posted by a Names list while its in-place editor is being typed in. WPARAM is the list.
const WM_LIVE_NAME: u32 = WM_APP + 1;
/// LVM_GETEDITCONTROL
const LVM_GETEDITCONTROL: u32 = 0x1018;
/// SetWindowSubclass id for Windows edit shortcuts (Ctrl+A, Ctrl+Backspace, Ctrl+Delete).
const EDIT_KEYS_SUBCLASS: usize = 2;

const PAGE_NAMES: [&str; 6] = ["Options", "Links", "Names", "Titles", "Metadata", "Download"];

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static IN_CALLBACK: Cell<bool> = const { Cell::new(false) };
    static FACE: Cell<HBRUSH> = const { Cell::new(HBRUSH(std::ptr::null_mut())) };
    static WHITE: Cell<HBRUSH> = const { Cell::new(HBRUSH(std::ptr::null_mut())) };
    static WINDOW_BRUSH: Cell<HBRUSH> = const { Cell::new(HBRUSH(std::ptr::null_mut())) };
    // Wheel movement smaller than one step, kept for the next message. Touchpads send small deltas.
    static PAGE_WHEEL_REST: Cell<i32> = const { Cell::new(0) };
    static WORK_WHEEL_REST: Cell<i32> = const { Cell::new(0) };
    // Bold fonts for the characters that make a file name illegal: 12 pt, then 10 pt and
    // 8 pt for rows too short for it.
    static MARK_BIG: Cell<HFONT> = const { Cell::new(HFONT(std::ptr::null_mut())) };
    static MARK_SMALL: Cell<HFONT> = const { Cell::new(HFONT(std::ptr::null_mut())) };
    static MARK_TINY: Cell<HFONT> = const { Cell::new(HFONT(std::ptr::null_mut())) };
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Busy {
    Idle,
    Fetch,
    Download,
}

enum Action {
    None,
    Pass,
    Result(isize),
    Popup(String),
    /// A notice that is not a problem, shown with the information icon.
    Notice(String),
    Browse,
    StartFetch,
    StartDownload,
    Cancel,
    Close,
    Min,
    Max,
    DragCaption,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Anchor {
    Fixed,
    StretchX,
    Right,
    Fill,
}

struct Kid {
    hwnd: HWND,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    anchor: Anchor,
    right: i32,
    bottom: i32,
}

impl Kid {
    fn spec(&self) -> BoxSpec {
        BoxSpec {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
            anchor: self.anchor,
            right: self.right,
            bottom: self.bottom,
        }
    }
}

/// Stored box used to place a control after the window size changes.
#[derive(Clone, Copy)]
struct BoxSpec {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    anchor: Anchor,
    right: i32,
    bottom: i32,
}

fn fill_page_height(has_fill: bool, view_h: i32, built: i32) -> i32 {
    if has_fill { view_h } else { built }
}

/// Height of the page host: inner area below the caption, minus the title strip and the Back/Next bar.
fn chrome_view_h(inner_bottom: i32, caption_bottom: i32, header: i32, bar: i32) -> i32 {
    (inner_bottom - caption_bottom - header - bar).max(1)
}

/// True when this control sits in the reserved strip under a Fill sibling.
fn fill_footer(kid: &BoxSpec, page: &[BoxSpec]) -> bool {
    page.iter().any(|fill| {
        fill.anchor == Anchor::Fill
            && kid.y >= fill.y + fill.h
            && kid.y + kid.h <= fill.y + fill.h + fill.bottom + 2
    })
}

fn place_box(kid: &BoxSpec, host_w: i32, view_h: i32, pin_bottom: bool) -> (i32, i32, i32, i32) {
    let y = if pin_bottom {
        (view_h - kid.bottom - kid.h).max(0)
    } else {
        kid.y
    };
    match kid.anchor {
        Anchor::Fixed => (kid.x, y, kid.w, kid.h),
        Anchor::StretchX => (kid.x, y, (host_w - kid.x - kid.right).max(40), kid.h),
        Anchor::Right => (host_w - kid.right - kid.w, y, kid.w, kid.h),
        Anchor::Fill => {
            let w = (host_w - kid.x - kid.right).max(40);
            let h = (view_h - kid.y - kid.bottom).max(40);
            (kid.x, kid.y, w, h)
        }
    }
}

fn layout_page(kids: &[BoxSpec], host_w: i32, view_h: i32) -> Vec<(i32, i32, i32, i32)> {
    kids.iter()
        .map(|kid| {
            let pin = kid.anchor != Anchor::Fill && fill_footer(kid, kids);
            place_box(kid, host_w, view_h, pin)
        })
        .collect()
}

fn boxes_overlap(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
}

struct App {
    hwnd: HWND,
    host: HWND,
    sheet: HWND,
    back: HWND,
    next: HWND,
    tip: HWND,
    instance: HINSTANCE,
    dpi: i32,
    font: HFONT,
    title_font: HFONT,
    page: usize,
    memory: Memory,
    options: Options,
    active_profile: String,
    loading: bool,
    last_notice: String,
    links: String,
    kids: Vec<Kid>,
    scroll: i32,
    content_height: i32,
    groups: Vec<crate::model::Group>,
    name_tools: Vec<TextTools>,
    title_tools: Vec<TextTools>,
    fetched_text: String,
    fetch_ok: bool,
    media_stamp: Options,
    have_media: bool,
    download_done: bool,
    ytdlp: PathBuf,
    ffmpeg: PathBuf,
    node: PathBuf,
    shared: Option<std::sync::Arc<Shared>>,
    worker: Option<std::thread::JoinHandle<()>>,
    /// Cancel was pressed and the worker is still winding down. The window
    /// keeps working meanwhile; `poll_work` finishes the cancel when it is done.
    stopping: bool,
    busy: Busy,
    rows: Vec<WorkRow>,
    list_hwnd: HWND,
    list_scroll: i32,
    samples: VecDeque<(Instant, u64)>,
    sample_start: Instant,
    last_done: u32,
    base_done: Option<u32>,
    finished: u32,
    last_finish: f64,
    told_update: bool,
    temp: PathBuf,
    started: Option<Instant>,
    status_hwnd: HWND,
    progress_hwnd: HWND,
    percent_hwnd: HWND,
    readout: [HWND; 5],
    profile_hwnd: HWND,
    name_hwnd: HWND,
    location_hwnd: HWND,
    link_hwnd: HWND,
    res_hwnd: HWND,
    vcodec_hwnd: HWND,
    aqual_hwnd: HWND,
    acodec_hwnd: HWND,
    container_hwnd: HWND,
    audio_hwnd: HWND,
    reencode_hwnd: HWND,
    sponsorblock_hwnd: HWND,
    overwrite_hwnd: HWND,
    strip_genre_hwnd: HWND,
    min_hwnd: HWND,
    max_hwnd: HWND,
    icon: windows::Win32::UI::WindowsAndMessaging::HICON,
    caption_font: HFONT,
    active: bool,
    cap_hot: CapHot,
    cap_press: CapHot,
    /// The file name being typed in a Names list, and what it was before typing started.
    live_edit: Option<LiveEdit>,
    /// The control that had the keyboard focus when the window was last deactivated.
    last_focus: HWND,
}

/// One file name being edited in place. `original` is put back when the edit is cancelled with Esc.
#[derive(Clone, Debug, PartialEq, Eq)]
struct LiveEdit {
    group: usize,
    index: usize,
    original: String,
}

impl LiveEdit {
    /// Remembers the name before the first keystroke of an edit. Later keystrokes in the
    /// same row keep the first original. A different row starts a new edit.
    fn note(slot: &mut Option<LiveEdit>, group: usize, index: usize, committed: &str) {
        match slot {
            Some(edit) if edit.group == group && edit.index == index => {}
            _ => *slot = Some(LiveEdit { group, index, original: committed.to_string() }),
        }
    }

    /// The name to put back when the edit of this row is cancelled, if it was typed in.
    fn cancel(slot: &mut Option<LiveEdit>, group: usize, index: usize) -> Option<String> {
        match slot.take() {
            Some(edit) if edit.group == group && edit.index == index => Some(edit.original),
            _ => None,
        }
    }
}

pub fn run() -> i32 {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_WIN95_CLASSES,
        };
        let _ = InitCommonControlsEx(&icc);
        let module = GetModuleHandleW(None).unwrap_or_default();
        let instance = HINSTANCE(module.0);
        let face = CreateSolidBrush(rgb(192, 192, 192));
        let white = CreateSolidBrush(rgb(255, 255, 255));
        let window_brush = CreateSolidBrush(rgb(255, 255, 255));
        FACE.set(face);
        WHITE.set(white);
        WINDOW_BRUSH.set(window_brush);
        register_classes(instance, face);
        APP.with(|slot| *slot.borrow_mut() = Some(App::boot(instance)));
        APP.with(|slot| {
            if let Some(app) = slot.borrow().as_ref() {
                MARK_BIG.set(app.title_font);
                MARK_SMALL.set(make_font(10, app.dpi, true));
                MARK_TINY.set(make_font(8, app.dpi, true));
            }
        });
        let hwnd = create_main(instance);
        if hwnd.0.is_null() {
            return 1;
        }
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);
        let mut msg = MSG::default();
        loop {
            let got = GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0);
            if got.0 <= 0 {
                break;
            }
            if wheel(&msg) {
                continue;
            }
            if shortcut(&msg) {
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = CoUninitialize();
        0
    }
}

fn register_classes(instance: HINSTANCE, face: HBRUSH) {
    unsafe {
        let icon = load_icon(instance, 32);
        let small = load_icon(instance, 16);
        let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        let main = WNDCLASSW {
            style: CS_DBLCLKS | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            hIcon: icon,
            hCursor: cursor,
            hbrBackground: face,
            lpszClassName: w!("LinkDownloadWindow"),
            ..Default::default()
        };
        let host = WNDCLASSW {
            lpfnWndProc: Some(host_proc),
            hInstance: instance,
            hCursor: cursor,
            hbrBackground: face,
            lpszClassName: w!("LinkDownloadHost"),
            ..Default::default()
        };
        let list = WNDCLASSW {
            lpfnWndProc: Some(list_proc),
            hInstance: instance,
            hCursor: cursor,
            hbrBackground: white_brush(),
            lpszClassName: w!("LinkWorkList"),
            ..Default::default()
        };
        let _ = RegisterClassW(&main);
        let _ = RegisterClassW(&host);
        let _ = RegisterClassW(&list);
        let _ = small;
    }
}

fn load_icon(instance: HINSTANCE, px: i32) -> windows::Win32::UI::WindowsAndMessaging::HICON {
    unsafe {
        let handle = LoadImageW(instance, PCWSTR(1 as *const u16), IMAGE_ICON, px, px, Default::default());
        windows::Win32::UI::WindowsAndMessaging::HICON(handle.unwrap_or_default().0)
    }
}

fn frame_style() -> windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE {
    WS_OVERLAPPED | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_THICKFRAME | WS_CLIPCHILDREN
}

fn minimum_window() -> (i32, i32) {
    let dpi = screen_dpi();
    (scale(640, dpi), scale(480, dpi))
}

fn create_main(instance: HINSTANCE) -> HWND {
    let (w, h) = minimum_window();
    let style = frame_style();
    unsafe {
        let x = (GetSystemMetrics(SM_CXSCREEN) - w) / 2;
        let y = (GetSystemMetrics(SM_CYSCREEN) - h) / 2;
        CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0),
            w!("LinkDownloadWindow"),
            w!("Link Download"),
            style,
            x,
            y,
            w,
            h,
            HWND(std::ptr::null_mut()),
            HMENU(std::ptr::null_mut()),
            instance,
            None,
        )
        .unwrap_or_default()
    }
}

fn screen_dpi() -> i32 {
    unsafe {
        let dc = GetDC(None);
        let dpi = GetDeviceCaps(dc, LOGPIXELSX);
        let _ = ReleaseDC(None, dc);
        if dpi <= 0 { 96 } else { dpi }
    }
}

fn scale(px: i32, dpi: i32) -> i32 {
    px * dpi / 96
}

unsafe fn nccalcsize(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if wparam.0 != 0 {
        let params = &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS);
        if IsZoomed(hwnd).as_bool() || rect_covers_work(hwnd, params.rgrc[0]) {
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut mi).as_bool() {
                params.rgrc[0] = mi.rcWork;
            } else {
                let pad = GetSystemMetrics(SM_CXFRAME) + GetSystemMetrics(SM_CXPADDEDBORDER);
                let pady = GetSystemMetrics(SM_CYFRAME) + GetSystemMetrics(SM_CXPADDEDBORDER);
                params.rgrc[0].left += pad;
                params.rgrc[0].top += pady;
                params.rgrc[0].right -= pad;
                params.rgrc[0].bottom -= pady;
            }
        }
    }
    LRESULT(0)
}

fn rect_covers_work(hwnd: HWND, proposed: RECT) -> bool {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut mi).as_bool() {
            return false;
        }
        let pw = proposed.right - proposed.left;
        let ph = proposed.bottom - proposed.top;
        let ww = mi.rcWork.right - mi.rcWork.left;
        let wh = mi.rcWork.bottom - mi.rcWork.top;
        pw >= ww && ph >= wh
    }
}

fn fill_minmax(hwnd: HWND, lparam: LPARAM) {
    unsafe {
        let info = &mut *(lparam.0 as *mut MINMAXINFO);
        let (w, h) = minimum_window();
        info.ptMinTrackSize.x = w;
        info.ptMinTrackSize.y = h;
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut mi).as_bool() {
            info.ptMaxPosition.x = mi.rcWork.left - mi.rcMonitor.left;
            info.ptMaxPosition.y = mi.rcWork.top - mi.rcMonitor.top;
            info.ptMaxSize.x = mi.rcWork.right - mi.rcWork.left;
            info.ptMaxSize.y = mi.rcWork.bottom - mi.rcWork.top;
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCALCSIZE {
        return nccalcsize(hwnd, wparam, lparam);
    }
    if msg == WM_GETMINMAXINFO {
        fill_minmax(hwnd, lparam);
        return LRESULT(0);
    }
    if matches!(msg, 0x0138 | 0x0135 | 0x0133) {
        // WM_CTLCOLORSTATIC, WM_CTLCOLORBTN, WM_CTLCOLOREDIT
        return paint_control(msg, wparam, lparam);
    }
    if IN_CALLBACK.get() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let action = APP.with(|slot| {
        let Ok(mut borrow) = slot.try_borrow_mut() else {
            return Action::Pass;
        };
        let Some(app) = borrow.as_mut() else {
            return Action::Pass;
        };
        app.handle(hwnd, msg, wparam, lparam)
    });
    if let Action::Pass = action {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    if let Action::Result(value) = action {
        return LRESULT(value);
    }
    run_action(action);
    LRESULT(0)
}

unsafe extern "system" fn host_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_ERASEBKGND {
        let hdc = HDC(wparam.0 as *mut _);
        let mut rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut rect);
        let _ = FillRect(hdc, &rect, FACE.get());
        return LRESULT(1);
    }
    if matches!(msg, 0x0138 | 0x0135 | 0x0133 | 0x0134) {
        return paint_control(msg, wparam, lparam);
    }
    if msg == 0x002B {
        // WM_DRAWITEM. Only the Names page warning line is owner-drawn. It needs nothing from the app state.
        let item = &*(lparam.0 as *const DRAWITEMSTRUCT);
        if item.CtlType.0 == 5 && (ID_NAME_ERROR..ID_NAME_ERROR + GROUP_LIMIT as i32).contains(&(item.CtlID as i32)) {
            draw_name_error(item);
            return LRESULT(1);
        }
    }
    if msg == WM_NOTIFY {
        let header = &*(lparam.0 as *const NMHDR);
        if header.code == 0xFFFF_FFF4 && (ID_LIST..ID_ALBUM).contains(&child_id(header.hwndFrom)) && class_name(header.hwndFrom) == "SysListView32" {
            // NM_CUSTOMDRAW from a Names list.
            return LRESULT(name_row_draw(lparam));
        }
    }
    if matches!(msg, WM_COMMAND | WM_NOTIFY | WM_VSCROLL | WM_MOUSEWHEEL | 0x002B) && !IN_CALLBACK.get() {
        let parent = windows::Win32::UI::WindowsAndMessaging::GetParent(hwnd).unwrap_or(HWND(std::ptr::null_mut()));
        return SendMessageW(parent, msg, wparam, lparam);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn list_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if IN_CALLBACK.get() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    match msg {
        WM_PAINT => {
            paint_work(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_VSCROLL => {
            scroll_work(hwnd, wparam);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let lines = take_wheel(&WORK_WHEEL_REST, wparam, 3);
            if lines != 0 {
                nudge_work(hwnd, -lines);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Subclass of each Names list. Its in-place editor sends EN_CHANGE to the list, which
/// ignores it. Post it to the main window instead. Posting, not sending, because the
/// editor can change while the app is in the middle of handling a click.
unsafe extern "system" fn name_list_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    if msg == WM_COMMAND && ((wparam.0 >> 16) & 0xFFFF) as u32 == EN_CHANGE {
        let root = GetAncestor(hwnd, GA_ROOT);
        if !root.0.is_null() {
            let _ = PostMessageW(root, WM_LIVE_NAME, WPARAM(hwnd.0 as usize), LPARAM(0));
        }
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

fn paint_control(msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        let hdc = HDC(wparam.0 as *mut _);
        let child = HWND(lparam.0 as *mut _);
        let enabled = IsWindowEnabled(child).0 != 0;
        if msg == 0x0133 && enabled {
            // Enabled edits stay white.
            let _ = SetTextColor(hdc, COLORREF(GetSysColor(COLOR_WINDOWTEXT)));
            let _ = SetBkColor(hdc, COLORREF(GetSysColor(COLOR_WINDOW)));
            return LRESULT(WINDOW_BRUSH.get().0 as isize);
        }
        // Labels, checkboxes, and buttons stay transparent on the gray page.
        let color = if enabled { COLOR_BTNTEXT } else { COLOR_GRAYTEXT };
        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, COLORREF(GetSysColor(color)));
        let _ = SetBkColor(hdc, COLORREF(GetSysColor(COLOR_BTNFACE)));
        LRESULT(FACE.get().0 as isize)
    }
}

fn run_action(action: Action) {
    match action {
        Action::None | Action::Pass | Action::Result(_) => {}
        Action::Popup(text) => popup(&text),
        Action::Notice(text) => popup_as(&text, crate::popup::BoxIcon::Info),
        Action::Browse => browse(),
        Action::StartFetch => start_fetch(),
        Action::StartDownload => start_download(),
        Action::Cancel => cancel_work(),
        Action::Close => {
            let hwnd = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd).unwrap_or_default());
            unsafe { let _ = DestroyWindow(hwnd); }
        }
        Action::Min => {
            let hwnd = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd).unwrap_or_default());
            unsafe { let _ = ShowWindow(hwnd, SW_MINIMIZE); }
        }
        Action::Max => {
            let hwnd = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd).unwrap_or_default());
            unsafe {
                let cmd = if IsZoomed(hwnd).as_bool() { SW_RESTORE } else { SW_MAXIMIZE };
                let _ = ShowWindow(hwnd, cmd);
            }
        }
        Action::DragCaption => {
            let hwnd = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd).unwrap_or_default());
            unsafe {
                let _ = ReleaseCapture();
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                let packed = LPARAM((((pt.y as u32) << 16) | (pt.x as u32 & 0xFFFF)) as isize);
                let _ = SendMessageW(hwnd, WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), packed);
            }
        }
    }
}

fn popup(text: &str) {
    popup_as(text, crate::popup::BoxIcon::Warning);
}

fn popup_as(text: &str, icon: crate::popup::BoxIcon) {
    let hwnd = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd).unwrap_or_default());
    // A Windows 98 box drawn by the app. MessageBoxW gets the modern look on Windows 10 and 11.
    crate::popup::message(hwnd, "Link Download", text, icon);
}

fn browse() {
    let (hwnd, current) = APP.with(|slot| {
        let app = slot.borrow();
        let app = app.as_ref().unwrap();
        (app.hwnd, get_text(app.location_hwnd))
    });
    let Some(path) = pick_folder(hwnd, &current) else {
        return;
    };
    APP.with(|slot| {
        let mut borrow = slot.borrow_mut();
        let app = borrow.as_mut().unwrap();
        set_text(app.location_hwnd, &path);
        app.commit_location();
        app.note_options(false);
        store::save(&app.memory);
    });
}

fn start_fetch() {
    let hwnd = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd).unwrap_or_default());
    unsafe { let _ = UpdateWindow(hwnd); }
    match tools::load_bundle() {
        Err(err) => {
            // The tools next to the app are missing or broken, so nothing can be fetched.
            popup_as(&err, crate::popup::BoxIcon::Error);
            APP.with(|slot| {
                let mut borrow = slot.borrow_mut();
                let app = borrow.as_mut().unwrap();
                app.busy = Busy::Idle;
                app.page = 1;
                app.rebuild();
            });
        }
        Ok(bundle) => {
            APP.with(|slot| {
                let mut borrow = slot.borrow_mut();
                let app = borrow.as_mut().unwrap();
                app.store_bundle(bundle);
                app.told_update = false;
                app.spawn(Busy::Fetch);
            });
        }
    }
}

fn start_download() {
    APP.with(|slot| {
        let mut borrow = slot.borrow_mut();
        let app = borrow.as_mut().unwrap();
        if app.have_media && !same_media(&app.media_stamp, &app.options) {
            for group in &mut app.groups {
                for item in &mut group.items {
                    item.temp_path = None;
                }
            }
            app.download_done = false;
        }
        if app.download_done && app.files_ready() {
            app.refresh_nav();
            return;
        }
        app.download_done = false;
        app.spawn(Busy::Download);
    });
}

/// Cancel: stops the running programs and returns at once. It used to wait here,
/// on the window's own thread, for the worker to finish, which froze the window.
/// Now the 100 ms timer (`poll_work`) notices when the worker has stopped and
/// then goes back a page, as before.
fn cancel_work() {
    APP.with(|slot| {
        let mut borrow = slot.borrow_mut();
        let app = borrow.as_mut().unwrap();
        if app.stopping {
            return;
        }
        if let Some(shared) = &app.shared {
            run::kill_child(shared);
        }
        app.stopping = true;
        app.refresh_nav();
    });
}

impl App {
    fn boot(instance: HINSTANCE) -> Self {
        let dpi = screen_dpi();
        let memory = store::load();
        let active = memory.last_profile.clone();
        let mut options = memory
            .profiles
            .iter()
            .find(|profile| profile.name == active)
            .map(|profile| profile.options.clone())
            .unwrap_or_default();
        if memory.override_profile == active && !memory.override_location.is_empty() {
            options.location = memory.override_location.clone();
        }
        let temp = std::env::temp_dir().join("Link Download").join(std::process::id().to_string());
        let _ = std::fs::create_dir_all(&temp);
        let null = HWND(std::ptr::null_mut());
        Self {
            hwnd: null,
            host: null,
            sheet: null,
            back: null,
            next: null,
            tip: null,
            instance,
            dpi,
            font: make_font(8, dpi, false),
            title_font: make_font(12, dpi, true),
            page: 0,
            memory,
            options,
            active_profile: active,
            loading: false,
            last_notice: String::new(),
            links: String::new(),
            kids: Vec::new(),
            scroll: 0,
            content_height: 0,
            groups: Vec::new(),
            name_tools: Vec::new(),
            title_tools: Vec::new(),
            fetched_text: String::new(),
            fetch_ok: false,
            media_stamp: Options::default(),
            have_media: false,
            download_done: false,
            ytdlp: PathBuf::new(),
            ffmpeg: PathBuf::new(),
            node: PathBuf::new(),
            shared: None,
            worker: None,
            stopping: false,
            busy: Busy::Idle,
            rows: Vec::new(),
            list_hwnd: null,
            list_scroll: 0,
            samples: VecDeque::new(),
            sample_start: Instant::now(),
            last_done: 0,
            base_done: None,
            finished: 0,
            last_finish: 0.0,
            told_update: false,
            temp,
            started: None,
            status_hwnd: null,
            progress_hwnd: null,
            percent_hwnd: null,
            readout: [null; 5],
            profile_hwnd: null,
            name_hwnd: null,
            location_hwnd: null,
            link_hwnd: null,
            res_hwnd: null,
            vcodec_hwnd: null,
            aqual_hwnd: null,
            acodec_hwnd: null,
            container_hwnd: null,
            audio_hwnd: null,
            reencode_hwnd: null,
            sponsorblock_hwnd: null,
            overwrite_hwnd: null,
            strip_genre_hwnd: null,
            min_hwnd: null,
            max_hwnd: null,
            icon: load_icon(instance, 16),
            caption_font: classic::caption_font(dpi),
            active: true,
            cap_hot: CapHot::None,
            cap_press: CapHot::None,
            live_edit: None,
            last_focus: HWND(std::ptr::null_mut()),
        }
    }

    fn s(&self, px: i32) -> i32 {
        scale(px, self.dpi)
    }

    fn handle(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Action {
        match msg {
            0x0001 => {
                // WM_CREATE
                self.hwnd = hwnd;
                self.on_create();
                Action::None
            }
            WM_SIZE => {
                if wparam.0 != 1 {
                    // Freeze, move, then paint once. Painting inside the size message is what tore the window.
                    unsafe { SendMessageW(self.hwnd, 0x000B, WPARAM(0), LPARAM(0)); }
                    self.layout_chrome();
                    self.sync_fill_page_height();
                    self.update_scroll();
                    self.place();
                    self.place_sheet();
                    unsafe { SendMessageW(self.hwnd, 0x000B, WPARAM(1), LPARAM(0)); }
                    self.redraw();
                }
                Action::None
            }
            WM_PAINT => {
                self.paint_chrome();
                Action::None
            }
            WM_ERASEBKGND => Action::Result(1),
            x if x == WM_NCHITTEST => self.hit_test(lparam),
            x if x == WM_ACTIVATE => {
                self.active = (wparam.0 & 0xFFFF) != 0;
                let minimized = (wparam.0 >> 16) & 0xFFFF != 0;
                if self.active {
                    // A popup, the folder picker or Alt+Tab took the window away. Give the
                    // keyboard back to the box it was in, like a dialog does.
                    if !minimized {
                        self.restore_focus();
                    }
                } else {
                    self.remember_focus();
                }
                self.invalidate_caption();
                Action::None
            }
            0x0007 => {
                // WM_SETFOCUS on the main window itself, such as after a restore from the taskbar.
                self.restore_focus();
                Action::None
            }
            x if x == WM_MOUSEMOVE => self.cap_move(lparam),
            x if x == WM_LBUTTONDOWN => self.cap_down(lparam),
            x if x == WM_LBUTTONUP => self.cap_up(),
            x if x == WM_LBUTTONDBLCLK => self.cap_dbl(lparam),
            WM_TIMER => self.on_timer(),
            WM_COMMAND => self.on_command(wparam, lparam),
            WM_NOTIFY => self.on_notify(lparam),
            WM_VSCROLL => {
                self.on_scroll(wparam);
                Action::None
            }
            WM_MOUSEWHEEL => {
                let pixels = take_wheel(&PAGE_WHEEL_REST, wparam, self.s(36));
                if pixels != 0 {
                    self.scroll_by(-pixels);
                }
                Action::None
            }
            x if x == WM_LIVE_NAME => {
                self.live_name(HWND(wparam.0 as *mut _));
                Action::None
            }
            WM_CLOSE => Action::Close,
            WM_DESTROY => {
                self.shutdown();
                Action::None
            }
            _ => Action::Pass,
        }
    }

    fn on_create(&mut self) {
        // The viewport clips one sheet. The sheet holds the page controls and is what scrolls.
        let host = self.make(self.hwnd, "LinkDownloadHost", "", 0x40000000 | 0x10000000 | 0x02000000 | 0x04000000, 0, 0, 0, 10, 10, 0, 0);
        self.host = host;
        self.sheet = self.create_sheet();
        self.back = self.make(self.hwnd, "Button", "< &Back", button_bits(false), 0, 0, 0, 75, 23, ID_BACK, 0);
        self.next = self.make(self.hwnd, "Button", "Next >", button_bits(true), 0, 0, 0, 75, 23, ID_NEXT, 0);
        unsafe {
            let _ = SetWindowPos(self.back, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            let _ = SetWindowPos(self.next, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            let _ = SetTimer(self.hwnd, 1, 100, None);
            SendMessageW(self.hwnd, 0x0080, WPARAM(1), LPARAM(load_icon(self.instance, 32).0 as isize));
            SendMessageW(self.hwnd, 0x0080, WPARAM(0), LPARAM(load_icon(self.instance, 16).0 as isize));
        }
        self.layout_chrome();
        self.loading = true;
        self.rebuild();
        self.loading = false;
        self.last_notice = self.explain(false);
        self.tip_nav();
    }

    fn chrome(&self) -> classic::Layout {
        let (w, h) = client_of(self.hwnd);
        classic::layout(
            RECT { left: 0, top: 0, right: w, bottom: h },
            self.dpi,
            self.maximized(),
        )
    }

    fn maximized(&self) -> bool {
        unsafe { IsZoomed(self.hwnd).as_bool() }
    }

    fn layout_chrome(&self) {
        let chrome = self.chrome();
        let header = self.s(46);
        let bar = self.s(46);
        let x = chrome.inner.left;
        let y = chrome.caption.bottom;
        let w = (chrome.inner.right - chrome.inner.left).max(0);
        let h = (chrome.inner.bottom - y).max(0);
        unsafe {
            let _ = MoveWindow(self.host, x, y + header, w, (h - header - bar).max(0), false);
            let bw = self.s(75);
            let bh = self.s(23);
            let by = chrome.inner.bottom - bar + (bar - bh) / 2;
            let next_x = chrome.inner.right - self.s(8) - bw;
            let back_x = next_x - self.s(6) - bw;
            let _ = MoveWindow(self.back, back_x, by, bw, bh, false);
            let _ = MoveWindow(self.next, next_x, by, bw, bh, false);
            let _ = SetWindowPos(self.back, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            let _ = SetWindowPos(self.next, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }

    fn paint_chrome(&self) {
        unsafe {
            let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
            let hdc = BeginPaint(self.hwnd, &mut ps);
            let (w, h) = client_of(self.hwnd);
            let chrome = self.chrome();
            let header = self.s(46);
            let bar = self.s(46);
            classic::window_frame(hdc, chrome.window, chrome.border);
            classic::paint_caption(
                hdc,
                &chrome,
                "Link Download",
                self.caption_font,
                self.active,
                self.maximized(),
                self.cap_hot,
                self.cap_press,
            );
            let icon_px = chrome.icon.right - chrome.icon.left;
            if !self.icon.0.is_null() {
                let _ = DrawIconEx(
                    hdc,
                    chrome.icon.left,
                    chrome.icon.top,
                    self.icon,
                    icon_px,
                    icon_px,
                    0,
                    windows::Win32::Graphics::Gdi::HBRUSH(std::ptr::null_mut()),
                    DI_NORMAL,
                );
            }
            let head = RECT {
                left: chrome.inner.left,
                top: chrome.caption.bottom,
                right: chrome.inner.right,
                bottom: chrome.caption.bottom + header,
            };
            if w > 0 && header > 0 {
                let _ = FillRect(hdc, &head, WHITE.get());
                let mut line = RECT {
                    left: head.left,
                    top: head.bottom - 2,
                    right: head.right,
                    bottom: head.bottom,
                };
                let _ = DrawEdge(hdc, &mut line, EDGE_ETCHED, BF_RECT);
                let old = SelectObject(hdc, self.title_font);
                let _ = SetBkMode(hdc, TRANSPARENT);
                let _ = SetTextColor(hdc, rgb(0, 0, 128));
                let mut title = RECT {
                    left: head.left + self.s(12),
                    top: head.top,
                    right: head.right - self.s(12),
                    bottom: head.bottom - 2,
                };
                let mut name = wide(PAGE_NAMES[self.page]);
                let end = name.len().saturating_sub(1);
                let _ = DrawTextW(hdc, &mut name[..end], &mut title, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
                SelectObject(hdc, old);
            }
            let foot = RECT {
                left: chrome.inner.left,
                top: chrome.inner.bottom - bar,
                right: chrome.inner.right,
                bottom: chrome.inner.bottom,
            };
            let _ = FillRect(hdc, &foot, FACE.get());
            let mut bar_line = RECT {
                left: foot.left,
                top: foot.top,
                right: foot.right,
                bottom: foot.top + 2,
            };
            let _ = DrawEdge(hdc, &mut bar_line, EDGE_ETCHED, BF_RECT);
            if !self.maximized() {
                classic::gripper(hdc, RECT { left: 0, top: 0, right: w, bottom: h });
            }
            let _ = EndPaint(self.hwnd, &ps);
        }
    }

    fn hit_test(&self, lparam: LPARAM) -> Action {
        if self.maximized() {
            return Action::Result(HTCLIENT as isize);
        }
        let mut pt = POINT {
            x: (lparam.0 as i16) as i32,
            y: ((lparam.0 >> 16) as i16) as i32,
        };
        unsafe {
            let _ = ScreenToClient(self.hwnd, &mut pt);
        }
        let chrome = self.chrome();
        if classic::is_button(classic::hot_at(&chrome, pt)) {
            return Action::Result(HTCLIENT as isize);
        }
        let r = chrome.window;
        let edge = chrome.border.max(3);
        let left = pt.x < r.left + edge;
        let right = pt.x >= r.right - edge;
        let top = pt.y < r.top + edge;
        let bottom = pt.y >= r.bottom - edge;
        let code = if top && left {
            HTTOPLEFT
        } else if top && right {
            HTTOPRIGHT
        } else if bottom && left {
            HTBOTTOMLEFT
        } else if bottom && right {
            HTBOTTOMRIGHT
        } else if left {
            HTLEFT
        } else if right {
            HTRIGHT
        } else if top {
            HTTOP
        } else if bottom {
            HTBOTTOM
        } else {
            HTCLIENT
        };
        Action::Result(code as isize)
    }

    fn cap_point(lparam: LPARAM) -> POINT {
        POINT {
            x: (lparam.0 as i16) as i32,
            y: ((lparam.0 >> 16) as i16) as i32,
        }
    }

    fn invalidate_caption(&self) {
        let caption = self.chrome().caption;
        unsafe {
            let _ = InvalidateRect(self.hwnd, Some(&caption), false);
        }
    }

    fn cap_move(&mut self, lparam: LPARAM) -> Action {
        let hot = classic::hot_at(&self.chrome(), Self::cap_point(lparam));
        if hot == self.cap_hot {
            return Action::None;
        }
        let paint = classic::is_button(hot) || classic::is_button(self.cap_hot);
        self.cap_hot = hot;
        if paint {
            self.invalidate_caption();
        }
        Action::None
    }

    fn cap_down(&mut self, lparam: LPARAM) -> Action {
        let hot = classic::hot_at(&self.chrome(), Self::cap_point(lparam));
        self.cap_press = hot;
        self.cap_hot = hot;
        match hot {
            CapHot::Caption => Action::DragCaption,
            CapHot::Min | CapHot::Max | CapHot::Close | CapHot::Icon => {
                self.invalidate_caption();
                Action::None
            }
            CapHot::None => Action::None,
        }
    }

    fn cap_up(&mut self) -> Action {
        let press = self.cap_press;
        self.cap_press = CapHot::None;
        let mut pt = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut pt);
            let _ = ScreenToClient(self.hwnd, &mut pt);
        }
        let hot = classic::hot_at(&self.chrome(), pt);
        self.cap_hot = hot;
        self.invalidate_caption();
        if press != hot {
            return Action::None;
        }
        match press {
            CapHot::Close | CapHot::Icon => Action::Close,
            CapHot::Min => Action::Min,
            CapHot::Max => Action::Max,
            _ => Action::None,
        }
    }

    fn cap_dbl(&mut self, lparam: LPARAM) -> Action {
        let hot = classic::hot_at(&self.chrome(), Self::cap_point(lparam));
        match hot {
            CapHot::Caption | CapHot::Max => Action::Max,
            CapHot::Icon => Action::Close,
            _ => Action::None,
        }
    }

    fn redraw(&self) {
        unsafe {
            let flags = RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN;
            let _ = RedrawWindow(self.hwnd, None, None, flags);
            // The page controls are grandchildren. Invalidating the frame does not repaint them,
            // so a resize left their old pixels in place.
            if !self.host.0.is_null() {
                let _ = RedrawWindow(self.host, None, None, flags);
            }
            if !self.sheet.0.is_null() {
                let _ = RedrawWindow(self.sheet, None, None, flags);
            }
        }
    }

    fn rebuild(&mut self) {
        self.harvest_soft();
        self.live_edit = None;
        self.clear_kids();
        self.scroll = 0;
        self.list_scroll = 0;
        let bottom = match self.page {
            0 => self.build_options(),
            1 => self.build_links(),
            2 => self.build_names(),
            3 => self.build_titles(),
            4 => self.build_meta(),
            _ => self.build_work(),
        };
        self.content_height = fill_page_height(
            self.kids.iter().any(|kid| kid.anchor == Anchor::Fill),
            self.view_h(),
            bottom + self.s(8),
        );
        self.update_scroll();
        self.place();
        self.place_sheet();
        self.refresh_nav();
        self.retip();
        self.redraw();
    }

    fn build_options(&mut self) -> i32 {
        let w = self.host_width();
        let mut y = self.s(10);
        let label_w = self.s(108);
        let x = self.s(8);
        let field_x = x + label_w + self.s(8);
        let field_w = self.s(200);
        self.label(x, y + self.s(3), label_w, "&Profile");
        self.profile_hwnd = self.combo(field_x, y, field_w, ID_PROFILE);
        for profile in &self.memory.profiles {
            combo_add(self.profile_hwnd, &profile.name);
        }
        let sel = self.memory.profiles.iter().position(|profile| profile.name == self.active_profile).unwrap_or(0);
        set_combo(self.profile_hwnd, sel);
        let save = self.button(field_x + field_w + self.s(8), y, self.s(75), "&Save", ID_SAVE, false);
        let _ = save;
        y += self.s(28);
        self.label(x, y + self.s(3), label_w, "&Name");
        self.name_hwnd = self.edit(field_x, y, field_w + self.s(83), ID_NAME, false, false);
        set_text(self.name_hwnd, &self.active_profile);
        y += self.s(32);
        self.res_hwnd = self.labeled_combo(x, &mut y, label_w, field_x, field_w, "&Resolution", ID_RES, &Resolution::ALL.iter().map(|v| v.label()).collect::<Vec<_>>(), Resolution::ALL.iter().position(|v| *v == self.options.resolution).unwrap_or(0));
        self.vcodec_hwnd = self.labeled_combo(x, &mut y, label_w, field_x, field_w, "&Video codec", ID_VCODEC, &VideoCodec::ALL.iter().map(|v| v.label()).collect::<Vec<_>>(), VideoCodec::ALL.iter().position(|v| *v == self.options.video_codec).unwrap_or(0));
        self.aqual_hwnd = self.labeled_combo(x, &mut y, label_w, field_x, field_w, "Audio &quality", ID_AQUAL, &AudioQuality::ALL.iter().map(|v| v.label()).collect::<Vec<_>>(), AudioQuality::ALL.iter().position(|v| *v == self.options.audio_quality).unwrap_or(0));
        self.acodec_hwnd = self.labeled_combo(x, &mut y, label_w, field_x, field_w, "Audio c&odec", ID_ACODEC, &AudioCodec::ALL.iter().map(|v| v.label()).collect::<Vec<_>>(), AudioCodec::ALL.iter().position(|v| *v == self.options.audio_codec).unwrap_or(0));
        let containers = Container::listed_for(self.options.audio_only);
        let labels: Vec<&str> = containers.iter().map(|v| v.label()).collect();
        let which = containers.iter().position(|v| *v == self.options.container).unwrap_or(0);
        self.container_hwnd = self.labeled_combo(x, &mut y, label_w, field_x, field_w, "&Container", ID_CONTAINER, &labels, which);
        y += self.s(4);
        self.strip_genre_hwnd = self.check(x, y, self.s(280), "Strip &genre and description", ID_STRIP_GENRE, self.options.strip_genre);
        y += self.s(24);
        self.audio_hwnd = self.check(x, y, self.s(160), "Audio onl&y", ID_AUDIO, self.options.audio_only);
        y += self.s(24);
        self.reencode_hwnd = self.check(x, y, self.s(220), "Re-&encode", ID_REENCODE, self.options.reencode);
        y += self.s(24);
        self.sponsorblock_hwnd = self.check(x, y, self.s(220), "Sponsor&Block", ID_SPONSORBLOCK, self.options.sponsorblock);
        y += self.s(24);
        self.overwrite_hwnd = self.check(x, y, self.s(280), "Overwrite files &already saved", ID_OVERWRITE, self.options.overwrite);
        y += self.s(32);
        self.label(x, y + self.s(3), label_w, "&Minimum");
        self.min_hwnd = self.edit(field_x, y, self.s(48), ID_MIN, false, true);
        set_text(self.min_hwnd, &self.options.min_wait.to_string());
        self.label(field_x + self.s(54), y + self.s(3), self.s(52), "seconds");
        self.label(field_x + self.s(112), y + self.s(3), self.s(70), "Ma&ximum");
        self.max_hwnd = self.edit(field_x + self.s(184), y, self.s(48), ID_MAX, false, true);
        set_text(self.max_hwnd, &self.options.max_wait.to_string());
        self.label(field_x + self.s(238), y + self.s(3), self.s(52), "seconds");
        y += self.s(32);
        self.label(x, y + self.s(3), label_w, "&Location");
        let browse_w = self.s(75);
        let loc_w = (w - field_x - browse_w - self.s(16)).max(self.s(80));
        self.location_hwnd = self.edit(field_x, y, loc_w, ID_LOCATION, false, false);
        set_text(self.location_hwnd, &self.options.location);
        self.button(field_x + loc_w + self.s(8), y, browse_w, "Bro&wse", ID_BROWSE, false);
        y += self.s(28);
        self.apply_option_enables();
        y
    }

    fn build_links(&mut self) -> i32 {
        let w = self.host_width();
        let mut y = self.s(10);
        let blurb = self.static_text(self.s(8), y, w - self.s(16), self.s(32), "Paste the links into the box below. Put one link on each line.", true);
        let _ = blurb;
        y += self.s(36);
        let error_h = self.s(32);
        let edit_h = (self.view_h() - y - error_h - self.s(8)).max(self.s(80));
        self.link_hwnd = self.edit(self.s(8), y, w - self.s(16), ID_LINKS, true, false);
        unsafe { let _ = MoveWindow(self.link_hwnd, self.s(8), y, w - self.s(16), edit_h, true); }
        let link_w = w - self.s(16);
        let link_bottom = error_h + self.s(8);
        if let Some(kid) = self.kids.iter_mut().find(|kid| kid.hwnd == self.link_hwnd) {
            kid.h = edit_h;
            kid.w = link_w;
            kid.anchor = Anchor::Fill;
            kid.bottom = link_bottom;
        }
        set_text(self.link_hwnd, &self.links);
        y += edit_h + self.s(4);
        let error = self.static_text(self.s(8), y, w - self.s(16), error_h, "", true);
        set_id(error, ID_LINK_ERROR);
        y + error_h
    }

    fn build_names(&mut self) -> i32 {
        if !self.fetch_ok || self.busy == Busy::Fetch {
            return self.build_wait("Reading the links…");
        }
        self.fit_tools();
        let w = self.host_width();
        let mut y = self.s(8);
        if self.groups.is_empty() {
            self.static_text(self.s(8), y, w - self.s(16), self.s(32), "No videos were found.", false);
            return y + self.s(40);
        }
        let count = self.groups.len().min(GROUP_LIMIT);
        for group_index in 0..count {
            let url = self.groups[group_index].source_url.clone();
            let error = self.groups[group_index].error.clone();
            let titles: Vec<String> = self.groups[group_index].items.iter().map(|item| item.title.clone()).collect();
            y = self.url_line(y, w, &url);
            if !error.is_empty() {
                self.static_text(self.s(8), y, w - self.s(16), self.s(32), &error, true);
                y += self.s(36);
            }
            let tools = self.name_tools.get(group_index).cloned().unwrap_or_default();
            y = self.build_tool_row(group_index, y, w, &tools);
            let rows = titles.len().clamp(3, 8) as i32;
            let list_h = self.s(20) + rows * self.s(18) + self.s(4);
            let list = self.listview(self.s(8), y, w - self.s(16), list_h, ID_LIST + group_index as i32);
            fill_titles(list, &titles, w - self.s(24));
            y += list_h + self.s(4);
            // SS_OWNERDRAW, so the characters that make the name illegal can be drawn in bold.
            let error = self.make(self.host, "Static", "", 0x40000000 | 0x10000000 | 0x000D, 0, self.s(8), y, w - self.s(16), self.s(32), 0, 0);
            set_id(error, ID_NAME_ERROR + group_index as i32);
            show_name_error(error, &self.groups[group_index].items);
            y += self.s(36);
        }
        y
    }

    fn build_titles(&mut self) -> i32 {
        if !self.fetch_ok || self.busy == Busy::Fetch {
            return self.build_wait("Reading the links…");
        }
        self.fit_tools();
        let w = self.host_width();
        let mut y = self.s(8);
        if self.groups.is_empty() {
            self.static_text(self.s(8), y, w - self.s(16), self.s(32), "No videos were found.", false);
            return y + self.s(40);
        }
        let mut flat = 0usize;
        let count = self.groups.len().min(GROUP_LIMIT);
        let same_w = self.s(132);
        let all_w = self.s(168);
        for group_index in 0..count {
            let url = self.groups[group_index].source_url.clone();
            let error = self.groups[group_index].error.clone();
            let rows: Vec<(String, String, bool)> = self.groups[group_index]
                .items
                .iter()
                .map(|item| (item.title.clone(), item.meta_title.clone(), item.blocked.is_some()))
                .collect();
            y = self.url_line(y, w, &url);
            if !error.is_empty() {
                self.static_text(self.s(8), y, w - self.s(16), self.s(32), &error, true);
                y += self.s(36);
            }
            let tools = self.title_tools.get(group_index).cloned().unwrap_or_default();
            y = self.build_tool_row(group_index, y, w, &tools);
            let all = self.button(w - self.s(8) - all_w, y, all_w, "&All same as file name", ID_TITLE_SAME_ALL + group_index as i32, false);
            self.set_anchor(all, Anchor::Right);
            y += self.s(28);
            for (file_name, title, blocked) in rows {
                if ID_TITLE_SAME + flat as i32 > 65000 {
                    break;
                }
                // SS_NOPREFIX | SS_ENDELLIPSIS so the file name shows every character.
                self.make(self.host, "Static", &file_name, 0x40000000 | 0x10000000 | 0x80 | 0x4000, 0, self.s(8), y, w - self.s(16), self.s(16), 0, 0);
                y += self.s(16);
                let edit_w = (w - self.s(16) - same_w - self.s(8)).max(self.s(80));
                let field = self.edit(self.s(8), y, edit_w, ID_TITLE_FIELD + flat as i32, false, false);
                self.set_anchor(field, Anchor::StretchX);
                set_text(field, &title);
                let same = self.button(w - self.s(8) - same_w, y, same_w, "Same as file name", ID_TITLE_SAME + flat as i32, false);
                self.set_anchor(same, Anchor::Right);
                if blocked {
                    set_enabled(field, false);
                    set_enabled(same, false);
                }
                y += self.s(26);
                flat += 1;
            }
            y += self.s(8);
        }
        y
    }

    fn build_tool_row(&mut self, group_index: usize, mut y: i32, w: i32, tools: &TextTools) -> i32 {
        let row_h = self.s(22);
        self.static_text(self.s(8), y + self.s(3), self.s(52), row_h, "Remove", false);
        let remove = self.edit(self.s(62), y, self.s(120), ID_REMOVE + group_index as i32, false, false);
        set_text(remove, &tools.remove);
        self.static_text(self.s(188), y + self.s(3), self.s(40), row_h, "Insert", false);
        let insert = self.edit(self.s(230), y, self.s(110), ID_INSERT + group_index as i32, false, false);
        set_text(insert, &tools.insert);
        let mode = self.combo(self.s(346), y, self.s(110), ID_MODE + group_index as i32);
        for label in ["At the start", "At the end", "Before", "After"] {
            combo_add(mode, label);
        }
        set_combo(mode, insert_index(tools.at));
        y += self.s(28);
        self.static_text(self.s(8), y + self.s(3), self.s(40), row_h, "Find", false);
        let find = self.edit(self.s(50), y, self.s(140), ID_FIND + group_index as i32, false, false);
        set_text(find, &tools.find);
        self.check(self.s(200), y, self.s(220), "Strip the prefix and the note", ID_STRIP + group_index as i32, tools.strip);
        self.button(w - self.s(8) - self.s(156), y, self.s(75), "&Reset", ID_RESET + group_index as i32, false);
        self.button(w - self.s(8) - self.s(75), y, self.s(75), "Apply", ID_APPLY + group_index as i32, false);
        y += self.s(30);
        self.sync_apply(group_index);
        y
    }

    fn build_meta(&mut self) -> i32 {
        let w = self.host_width();
        let mut y = self.s(8);
        let mut flat = 0usize;
        let pages: Vec<(String, String, String, Vec<(String, String, bool)>)> = self
            .groups
            .iter()
            .take(GROUP_LIMIT)
            .map(|group| {
                (
                    group.source_url.clone(),
                    group.error.clone(),
                    group.album.clone(),
                    group
                        .items
                        .iter()
                        .map(|item| (item.title.clone(), item.artists.join("\r\n"), item.blocked.is_some()))
                        .collect(),
                )
            })
            .collect();
        for (group_index, (url, error, album_text, rows)) in pages.iter().enumerate() {
            y = self.url_line(y, w, url);
            if !error.is_empty() && rows.is_empty() {
                self.static_text(self.s(8), y, w - self.s(16), self.s(32), error, true);
                y += self.s(36);
                continue;
            }
            self.static_text(self.s(8), y + self.s(3), self.s(52), self.s(20), "Album", false);
            let album_w = (w - self.s(64) - self.s(90)).max(self.s(80));
            let album = self.edit(self.s(64), y, album_w, ID_ALBUM + group_index as i32, false, false);
            set_text(album, album_text);
            self.button(self.s(64) + album_w + self.s(8), y, self.s(75), "&Reset", ID_META_RESET + group_index as i32, false);
            y += self.s(28);
            for (file_name, artists_text, blocked) in rows {
                if flat + ID_ARTIST as usize > 65000 {
                    break;
                }
                self.static_text(self.s(8), y, w - self.s(16), self.s(18), file_name, false);
                y += self.s(18);
                let artists = self.edit(self.s(8), y, w - self.s(16), ID_ARTIST + flat as i32, true, false);
                let height = self.s(48);
                let artist_w = w - self.s(16);
                unsafe { let _ = MoveWindow(artists, self.s(8), y, artist_w, height, true); }
                if let Some(kid) = self.kids.last_mut() {
                    kid.h = height;
                    kid.w = artist_w;
                }
                set_text(artists, artists_text);
                if *blocked {
                    set_enabled(artists, false);
                }
                y += height + self.s(8);
                flat += 1;
            }
            y += self.s(6);
        }
        y
    }

    fn build_work(&mut self) -> i32 {
        let w = self.host_width();
        let mut y = self.s(8);
        self.status_hwnd = self.static_text(self.s(8), y, w - self.s(16), self.s(18), "", false);
        set_id(self.status_hwnd, ID_STATUS);
        y += self.s(22);
        let list_h = (self.view_h() - y - self.s(78)).max(self.s(120));
        self.list_hwnd = self.make(self.host, "LinkWorkList", "", 0x40000000 | 0x10000000 | 0x10000, 0x200, self.s(8), y, w - self.s(16), list_h, 52, 0);
        self.set_fill(self.list_hwnd, self.s(78));
        y += list_h + self.s(8);
        self.progress_hwnd = self.make(self.host, "msctls_progress32", "", 0x40000000 | 0x10000000, 0, self.s(8), y, w - self.s(70), self.s(16), ID_PROGRESS, 0);
        self.set_anchor(self.progress_hwnd, Anchor::StretchX);
        unsafe { SendMessageW(self.progress_hwnd, PBM_SETRANGE32, WPARAM(0), LPARAM(1000)); }
        self.percent_hwnd = self.static_text(w - self.s(58), y, self.s(50), self.s(16), "0%", false);
        set_id(self.percent_hwnd, ID_PERCENT);
        y += self.s(22);
        let labels = ["Speed", "Items per second", "Files left", "Time left", "Elapsed"];
        let col_w = ((w - self.s(16)) / 3).max(self.s(160));
        for (index, label) in labels.iter().enumerate() {
            let col = (index % 3) as i32;
            let row = (index / 3) as i32;
            let lx = self.s(8) + col * col_w;
            let ly = y + row * self.s(18);
            // Time left and Elapsed need room for 01:05:10:00 / Calculating...
            let timer = index == 3 || index == 4;
            let label_w = if timer {
                self.s(72)
            } else {
                (col_w - self.s(72)).clamp(self.s(96), self.s(150))
            };
            let value_w = (col_w - label_w - self.s(6)).max(if timer { self.s(88) } else { self.s(48) });
            self.static_text(lx, ly, label_w, self.s(16), label, false);
            self.readout[index] = self.static_text(lx + label_w, ly, value_w, self.s(16), "—", false);
            set_id(self.readout[index], ID_READOUT + index as i32);
        }
        y + self.s(40)
    }

    fn build_wait(&mut self, text: &str) -> i32 {
        let w = self.host_width();
        let y = self.s(12);
        self.status_hwnd = self.static_text(self.s(8), y, w - self.s(16), self.s(18), text, false);
        let bar_y = y + self.s(24);
        self.progress_hwnd = self.make(self.host, "msctls_progress32", "", 0x40000000 | 0x10000000, 0, self.s(8), bar_y, w - self.s(16), self.s(16), ID_PROGRESS, 0);
        unsafe { SendMessageW(self.progress_hwnd, PBM_SETRANGE32, WPARAM(0), LPARAM(1000)); }
        bar_y + self.s(24)
    }

    fn url_line(&mut self, y: i32, w: i32, url: &str) -> i32 {
        self.static_text(self.s(8), y, w - self.s(16), self.s(32), url, true);
        y + self.s(34)
    }

    fn labeled_combo(&mut self, x: i32, y: &mut i32, label_w: i32, field_x: i32, field_w: i32, caption: &str, id: i32, labels: &[&str], sel: usize) -> HWND {
        self.label(x, *y + self.s(3), label_w, caption);
        let combo = self.combo(field_x, *y, field_w, id);
        for label in labels {
            combo_add(combo, label);
        }
        set_combo(combo, sel);
        *y += self.s(28);
        combo
    }

    fn label(&mut self, x: i32, y: i32, w: i32, text: &str) -> HWND {
        self.static_text(x, y, w, self.s(16), text, false)
    }

    fn static_text(&mut self, x: i32, y: i32, w: i32, h: i32, text: &str, prefix_off: bool) -> HWND {
        let bits = if prefix_off { 0x800 } else { 0 };
        self.make(self.host, "Static", text, 0x40000000 | 0x10000000 | bits, 0, x, y, w, h, 0, 0)
    }

    fn edit(&mut self, x: i32, y: i32, w: i32, id: i32, multi: bool, number: bool) -> HWND {
        let mut bits = 0x40000000 | 0x10000000 | 0x10000 | 0x0080;
        if multi {
            bits |= 0x0004 | 0x0040 | 0x1000 | 0x00200000;
        }
        if number {
            bits |= 0x2000;
        }
        let hwnd = self.make(self.host, "Edit", "", bits, 0x200, x, y, w, self.s(if multi { 22 } else { 21 }), id, 0);
        install_edit_keys(hwnd);
        if number {
            unsafe { SendMessageW(hwnd, 0x00C5, WPARAM(4), LPARAM(0)); }
        }
        hwnd
    }

    fn combo(&mut self, x: i32, y: i32, w: i32, id: i32) -> HWND {
        // The window stays tall so the dropped list has room. The visible part is only the closed bar.
        let hwnd = self.make(self.host, "ComboBox", "", 0x40000000 | 0x10000000 | 0x10000 | 0x3 | 0x200 | 0x00200000, 0, x, y, w, self.s(160), id, 0);
        self.set_anchor(hwnd, Anchor::Fixed);
        self.clip_closed_combo(hwnd);
        hwnd
    }

    fn clip_closed_combo(&self, hwnd: HWND) {
        if hwnd.0.is_null() {
            return;
        }
        let mut rect = RECT::default();
        unsafe { let _ = GetWindowRect(hwnd, &mut rect); }
        let width = (rect.right - rect.left).max(1);
        let height = self.s(24).max(1);
        unsafe {
            let region = CreateRectRgn(0, 0, width, height);
            if region.0.is_null() {
                return;
            }
            if SetWindowRgn(hwnd, region, BOOL(1)) == 0 {
                let _ = DeleteObject(region);
            }
        }
    }

    fn check(&mut self, x: i32, y: i32, w: i32, text: &str, id: i32, on: bool) -> HWND {
        let hwnd = self.make(self.host, "Button", text, 0x40000000 | 0x10000000 | 0x10000 | 0x3, 0, x, y, w, self.s(18), id, 0);
        set_check(hwnd, on);
        hwnd
    }

    fn button(&mut self, x: i32, y: i32, w: i32, text: &str, id: i32, default: bool) -> HWND {
        self.make(self.host, "Button", text, button_bits(default), 0, x, y, w, self.s(23), id, 0)
    }

    fn listview(&mut self, x: i32, y: i32, w: i32, h: i32, id: i32) -> HWND {
        // WS_CLIPCHILDREN keeps the in-place editor above the row text. Without it the row paints through the editor.
        // LVS_REPORT | LVS_EDITLABELS | LVS_SINGLESEL | LVS_SHOWSELALWAYS | LVS_NOSORTHEADER
        let bits = 0x40000000 | 0x10000000 | 0x10000 | 0x02000000 | 0x00800000 | 0x0001 | 0x0200 | 0x0004 | 0x0008 | 0x8000;
        let list = self.make(self.host, "SysListView32", "", bits, 0x200, x, y, w, h, id, 0);
        if (ID_LIST..ID_ALBUM).contains(&id) {
            // The in-place editor reports each change to the list, not to us. Listen there.
            unsafe { let _ = SetWindowSubclass(list, Some(name_list_proc), 1, 0); }
        }
        list
    }

    fn make(&mut self, parent: HWND, class: &str, text: &str, style: u32, ex: u32, x: i32, y: i32, w: i32, h: i32, id: i32, _tab: i32) -> HWND {
        let class_w = wide(class);
        let text_w = wide(text);
        // Page controls live on the sheet. Callers still pass the viewport as parent.
        let on_page = parent == self.host;
        let parent = if on_page && !self.sheet.0.is_null() { self.sheet } else { parent };
        IN_CALLBACK.set(true);
        let hwnd = unsafe {
            CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(ex),
                PCWSTR(class_w.as_ptr()),
                PCWSTR(text_w.as_ptr()),
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(style),
                x,
                y,
                w.max(1),
                h.max(1),
                parent,
                HMENU(id as *mut _),
                self.instance,
                None,
            )
            .unwrap_or_default()
        };
        IN_CALLBACK.set(false);
        unsafe {
            SendMessageW(hwnd, WM_SETFONT, WPARAM(font_bits(self.font)), LPARAM(1));
            let _ = SetWindowTheme(hwnd, w!(""), w!(""));
        }
        if on_page {
            let host_w = self.host_width().max(1);
            let view_h = self.view_h().max(1);
            let right = (host_w - (x + w)).max(0);
            let in_view = y >= 0 && y + h <= view_h + 2;
            let bottom = if in_view { view_h - (y + h) } else { 0 };
            // Fill is set by the page that owns a stretching list or edit. Auto-Fill on any
            // tall control near the bottom pinned Names lists and stacked later groups.
            let anchor = if right <= 20 && w >= 160 {
                Anchor::StretchX
            } else if right <= 16 && w <= 110 {
                Anchor::Right
            } else {
                Anchor::Fixed
            };
            self.kids.push(Kid { hwnd, x, y, w, h, anchor, right, bottom });
        }
        hwnd
    }

    fn set_anchor(&mut self, hwnd: HWND, anchor: Anchor) {
        if let Some(kid) = self.kids.iter_mut().rev().find(|kid| kid.hwnd == hwnd) {
            kid.anchor = anchor;
        }
    }

    fn set_fill(&mut self, hwnd: HWND, bottom: i32) {
        if let Some(kid) = self.kids.iter_mut().rev().find(|kid| kid.hwnd == hwnd) {
            kid.anchor = Anchor::Fill;
            kid.bottom = bottom;
        }
    }

    fn sync_fill_page_height(&mut self) {
        if self.kids.iter().any(|kid| kid.anchor == Anchor::Fill) {
            self.content_height = self.view_h();
            self.scroll = 0;
        }
    }

    fn clear_kids(&mut self) {
        IN_CALLBACK.set(true);
        for kid in self.kids.drain(..) {
            unsafe { let _ = DestroyWindow(kid.hwnd); }
        }
        IN_CALLBACK.set(false);
        let null = HWND(std::ptr::null_mut());
        self.list_hwnd = null;
        self.link_hwnd = null;
        self.status_hwnd = null;
        self.progress_hwnd = null;
        self.percent_hwnd = null;
        self.readout = [null; 5];
        self.profile_hwnd = null;
        self.name_hwnd = null;
        self.location_hwnd = null;
    }

    fn place(&self) {
        let host_w = self.host_width().max(1);
        let view_h = self.view_h().max(1);
        let specs: Vec<BoxSpec> = self.kids.iter().map(Kid::spec).collect();
        let boxes = layout_page(&specs, host_w, view_h);
        // Positions are on the sheet. Scrolling moves the sheet, not each control.
        for (kid, (x, y, w, h)) in self.kids.iter().zip(boxes) {
            unsafe { let _ = MoveWindow(kid.hwnd, x, y, w, h, false); }
            let class = class_name(kid.hwnd);
            if class == "SysListView32" {
                unsafe { SendMessageW(kid.hwnd, 0x101E, WPARAM(0), LPARAM((w - 8).max(40) as isize)); }
            }
            if class == "ComboBox" {
                self.clip_closed_combo(kid.hwnd);
            }
            if class == "LinkWorkList" {
                unsafe { let _ = InvalidateRect(kid.hwnd, None, false); }
            }
        }
    }

    fn place_sheet(&self) {
        if self.sheet.0.is_null() {
            return;
        }
        let w = self.host_width().max(1);
        let h = self.content_height.max(self.view_h()).max(1);
        unsafe { let _ = MoveWindow(self.sheet, 0, -self.scroll, w, h, true); }
    }

    fn create_sheet(&self) -> HWND {
        let class_w = wide("LinkDownloadHost");
        IN_CALLBACK.set(true);
        let hwnd = unsafe {
            CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0),
                PCWSTR(class_w.as_ptr()),
                PCWSTR(wide("").as_ptr()),
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(0x40000000 | 0x10000000),
                0,
                0,
                10,
                10,
                self.host,
                HMENU(std::ptr::null_mut()),
                self.instance,
                None,
            )
            .unwrap_or_default()
        };
        IN_CALLBACK.set(false);
        unsafe {
            SendMessageW(hwnd, WM_SETFONT, WPARAM(font_bits(self.font)), LPARAM(0));
            let _ = SetWindowTheme(hwnd, w!(""), w!(""));
        }
        hwnd
    }

    fn pane(&self) -> HWND {
        if self.sheet.0.is_null() { self.host } else { self.sheet }
    }

    fn update_scroll(&self) {
        let view = self.view_h().max(1);
        let mut info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
            nMin: 0,
            nMax: (self.content_height - 1).max(0),
            nPage: view as u32,
            nPos: self.scroll,
            nTrackPos: 0,
        };
        unsafe {
            SetScrollInfo(self.host, SB_VERT, &info, true);
            let _ = ShowScrollBar(self.host, SB_VERT, self.content_height > view);
        }
    }

    fn on_scroll(&mut self, wparam: WPARAM) {
        let code = (wparam.0 & 0xFFFF) as u16;
        let view = self.view_h();
        let max_scroll = (self.content_height - view).max(0);
        let mut next = self.scroll;
        match code {
            0 => next -= self.s(18),
            1 => next += self.s(18),
            2 => next -= view,
            3 => next += view,
            5 | 4 => next = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32,
            6 => next = 0,
            7 => next = max_scroll,
            _ => {}
        }
        self.scroll = next.clamp(0, max_scroll);
        self.shift_kids();
    }

    fn scroll_by(&mut self, delta: i32) {
        let max_scroll = (self.content_height - self.view_h()).max(0);
        self.scroll = (self.scroll + delta).clamp(0, max_scroll);
        self.shift_kids();
    }

    fn reveal_host_rect(&mut self, rect: RECT) {
        let view = self.view_h();
        let margin = self.s(4);
        let delta = if rect.bottom - rect.top >= view - margin * 2 {
            if rect.top < margin { rect.top - margin } else { 0 }
        } else if rect.top < margin {
            rect.top - margin
        } else if rect.bottom > view - margin {
            rect.bottom - (view - margin)
        } else {
            0
        };
        if delta == 0 {
            return;
        }
        let max_scroll = (self.content_height - view).max(0);
        let next = (self.scroll + delta).clamp(0, max_scroll);
        if next == self.scroll {
            return;
        }
        self.scroll = next;
        self.shift_kids();
    }

    fn shift_kids(&self) {
        // One window moves. The controls stay put on it, so scrolling cannot leave their old picture behind.
        self.place_sheet();
        self.update_scroll();
    }

    fn view_h(&self) -> i32 {
        let chrome = self.chrome();
        chrome_view_h(chrome.inner.bottom, chrome.caption.bottom, self.s(46), self.s(46))
    }

    fn host_width(&self) -> i32 {
        let (w, _) = client_of(self.host);
        if w > 0 { w } else { self.s(640) }
    }

    fn refresh_nav(&mut self) {
        let ready = self.page_ready();
        set_enabled(self.back, self.page > 0 && !self.stopping);
        set_enabled(self.next, ready && !self.stopping);
        let caption = if self.page == 5 { "&Finish" } else { "Next >" };
        set_text(self.next, caption);
        let title = if self.stopping {
            "Link Download - Stopping".into()
        } else if self.busy != Busy::Idle {
            let left = self.rows.iter().filter(|row| matches!(row.kind, RowKind::Queued | RowKind::Active | RowKind::Waiting)).count();
            if left > 0 {
                format!("Link Download - {left} left")
            } else {
                "Link Download".into()
            }
        } else {
            "Link Download".into()
        };
        set_text(self.hwnd, &title);
        self.refresh_name_errors();
    }

    fn refresh_name_errors(&self) {
        if self.page != 2 {
            return;
        }
        for (index, group) in self.groups.iter().enumerate().take(GROUP_LIMIT) {
            if let Ok(label) = child(self.pane(), ID_NAME_ERROR + index as i32) {
                show_name_error(label, &group.items);
            }
        }
    }

    fn page_ready(&self) -> bool {
        if self.busy != Busy::Idle && self.page >= 2 {
            return false;
        }
        match self.page {
            0 => can_advance(&self.current_options()),
            1 => model::parse_links(&self.current_links()).is_ok(),
            2 => self.fetch_ok && self.names_ready() && self.name_problems().is_empty(),
            3 => self.fetch_ok,
            4 => self.fetch_ok,
            5 => self.download_done,
            _ => false,
        }
    }

    fn names_ready(&self) -> bool {
        self.groups.iter().any(|group| group.items.iter().any(|item| item.blocked.is_none()))
            && self.name_problems().is_empty()
    }

    fn name_problems(&self) -> String {
        for group in &self.groups {
            if let Some(line) = paths::name_problem_line(&group.items) {
                if !line.is_empty() {
                    return line;
                }
            }
        }
        String::new()
    }

    fn files_ready(&self) -> bool {
        self.groups.iter().any(|group| group.items.iter().any(|item| item.temp_path.is_some()))
    }

    fn remember_focus(&mut self) {
        let focus = unsafe { GetFocus() };
        if !focus.0.is_null() && focus != self.hwnd && unsafe { IsChild(self.hwnd, focus).as_bool() } {
            self.last_focus = focus;
        }
    }

    fn restore_focus(&mut self) {
        let target = self.last_focus;
        if target.0.is_null() {
            return;
        }
        let alive = unsafe { IsWindow(target).as_bool() && IsChild(self.hwnd, target).as_bool() };
        if !alive || !enabled_visible(target) {
            // The page was rebuilt since, or the control was turned off.
            self.last_focus = HWND(std::ptr::null_mut());
            return;
        }
        unsafe { let _ = SetFocus(target); }
    }

    fn on_command(&mut self, wparam: WPARAM, lparam: LPARAM) -> Action {
        let id = (wparam.0 & 0xFFFF) as i32;
        let code = ((wparam.0 >> 16) & 0xFFFF) as u16;
        if code == 7 || code == 8 {
            // CBN_DROPDOWN opens the full list. CBN_CLOSEUP clips it back to the bar.
            let hwnd = HWND(lparam.0 as *mut _);
            if class_name(hwnd) == "ComboBox" {
                if code == 7 {
                    unsafe { SetWindowRgn(hwnd, HRGN(std::ptr::null_mut()), BOOL(1)); }
                } else {
                    self.clip_closed_combo(hwnd);
                }
            }
        }
        if id == ID_BACK && code == 0 {
            return self.go_back();
        }
        if id == ID_NEXT && code == 0 {
            return self.go_next();
        }
        if id == ID_SAVE && code == 0 {
            return self.save_profile();
        }
        if id == ID_BROWSE && code == 0 {
            return Action::Browse;
        }
        if id == ID_PROFILE && code == CBN_SELCHANGE as u16 {
            self.change_profile();
            return Action::None;
        }
        if self.page == 0 && (code == CBN_SELCHANGE as u16 || code == 0 || code == EN_CHANGE as u16 || code == EN_KILLFOCUS as u16) {
            if id == ID_LOCATION && code == EN_KILLFOCUS as u16 {
                self.commit_location();
                store::save(&self.memory);
            }
            if id == ID_AUDIO && code == 0 {
                let audio_only = checked(self.audio_hwnd);
                let previous = Container::listed_for(!audio_only);
                let container = *previous.get(combo_sel(self.container_hwnd)).unwrap_or(&Container::Mp4);
                self.options = self.current_options();
                self.options.audio_only = audio_only;
                self.options.container = container;
                self.options.container = container_after_toggle(&self.options);
                self.refill_containers();
            }
            if matches!(id, ID_RES | ID_VCODEC | ID_AQUAL | ID_ACODEC | ID_CONTAINER | ID_AUDIO | ID_REENCODE | ID_SPONSORBLOCK | ID_STRIP_GENRE | ID_OVERWRITE | ID_MIN | ID_MAX | ID_LOCATION) {
                self.note_options(popup_for(id, code, self.leaving_for_own_control()));
            }
        }
        if self.page == 1 && id == ID_LINKS && code == EN_CHANGE as u16 {
            self.links = get_text(self.link_hwnd);
            let message = match model::parse_links(&self.links) {
                Ok(_) => String::new(),
                Err(_) if self.links.trim().is_empty() => String::new(),
                Err(err) => err,
            };
            if let Ok(error) = child(self.pane(), ID_LINK_ERROR) {
                set_text(error, &message);
            }
            self.refresh_nav();
        }
        if code == 0 && (ID_RESET..ID_META_RESET).contains(&id) {
            let group = (id - ID_RESET) as usize;
            if self.page == 3 {
                self.reset_titles(group);
            } else {
                self.reset_names(group);
            }
            return Action::None;
        }
        if code == 0 && (ID_META_RESET..ID_NAME_ERROR).contains(&id) {
            self.reset_meta((id - ID_META_RESET) as usize);
            return Action::None;
        }
        if code == 0 && (ID_TITLE_SAME_ALL..ID_TITLE_FIELD).contains(&id) {
            self.use_file_names((id - ID_TITLE_SAME_ALL) as usize);
            return Action::None;
        }
        if code == 0 && (ID_TITLE_SAME..).contains(&id) {
            self.use_file_name((id - ID_TITLE_SAME) as usize);
            return Action::None;
        }
        if (self.page == 2 || self.page == 3) && id >= ID_REMOVE && id < ID_ARTIST {
            let group = group_of(id);
            if code == 0 && id >= ID_APPLY && id < ID_LIST {
                if self.page == 3 {
                    self.apply_titles(group);
                } else {
                    self.apply_group(group);
                }
            } else if code == CBN_SELCHANGE as u16 || code == EN_CHANGE as u16 || code == 0 {
                self.sync_apply(group);
            }
        }
        Action::None
    }

    /// A key was typed in a Names list editor. The name, the warning line, and Next follow it
    /// right away. Nothing is written back into the editor, so the caret and undo are untouched.
    fn live_name(&mut self, list: HWND) {
        if self.page != 2 || list.0.is_null() {
            return;
        }
        let id = child_id(list);
        if !(ID_LIST..ID_ALBUM).contains(&id) {
            return;
        }
        let edit = unsafe { HWND(SendMessageW(list, LVM_GETEDITCONTROL, WPARAM(0), LPARAM(0)).0 as *mut _) };
        if edit.0.is_null() {
            // The edit already ended. Its end notice did the work.
            return;
        }
        install_edit_keys(edit);
        let index = list_focus_index(list);
        if index < 0 {
            return;
        }
        let group = (id - ID_LIST) as usize;
        let index = index as usize;
        let text = get_text(edit);
        let Some(item) = self.groups.get_mut(group).and_then(|group| group.items.get_mut(index)) else {
            return;
        };
        LiveEdit::note(&mut self.live_edit, group, index, &item.title);
        if item.title == text {
            return;
        }
        item.title = text;
        self.refresh_nav();
    }

    fn on_notify(&mut self, lparam: LPARAM) -> Action {
        unsafe {
            let header = &*(lparam.0 as *const NMHDR);
            if header.code == NM_CLICK || header.code == NM_DBLCLK {
                let id = child_id(header.hwndFrom);
                if (ID_LIST..ID_ALBUM).contains(&id) {
                    let activate = &*(lparam.0 as *const NMITEMACTIVATE);
                    if activate.iItem >= 0 {
                        SendMessageW(header.hwndFrom, LVM_EDITLABELW, WPARAM(activate.iItem as usize), LPARAM(0));
                        let edit = HWND(SendMessageW(header.hwndFrom, LVM_GETEDITCONTROL, WPARAM(0), LPARAM(0)).0 as *mut _);
                        install_edit_keys(edit);
                    }
                }
                return Action::None;
            }
            if header.code == LVN_ENDLABELEDITW {
                let info = &*(lparam.0 as *const NMLVDISPINFOW);
                let id = child_id(header.hwndFrom);
                if info.item.pszText.0.is_null() {
                    // Esc: the list keeps its old text. Put the model back to match it.
                    if (ID_LIST..ID_ALBUM).contains(&id) && info.item.iItem >= 0 {
                        let group = (id - ID_LIST) as usize;
                        let index = info.item.iItem as usize;
                        if let Some(original) = LiveEdit::cancel(&mut self.live_edit, group, index) {
                            if let Some(item) = self.groups.get_mut(group).and_then(|group| group.items.get_mut(index)) {
                                item.title = original;
                            }
                            self.refresh_nav();
                        }
                    }
                    return Action::Result(0);
                }
                self.live_edit = None;
                let title = pw_string(info.item.pszText);
                if (ID_LIST..ID_ALBUM).contains(&id) {
                    let group = (id - ID_LIST) as usize;
                    if let Some(item) = self.groups.get_mut(group).and_then(|group| group.items.get_mut(info.item.iItem as usize)) {
                        item.title = title;
                    }
                }
                self.refresh_nav();
                let _ = InvalidateRect(header.hwndFrom, None, true);
                return Action::Result(1);
            }
        }
        Action::None
    }

    fn go_back(&mut self) -> Action {
        if self.page == 0 {
            return Action::None;
        }
        self.harvest();
        if self.busy != Busy::Idle {
            return Action::Cancel;
        }
        if self.page == 5 {
            self.download_done = false;
        }
        self.page -= 1;
        self.rebuild();
        Action::None
    }

    fn go_next(&mut self) -> Action {
        self.harvest();
        match self.page {
            0 => {
                self.options = self.current_options();
                if !can_advance(&self.options) {
                    let text = self.explain(true);
                    if text.is_empty() {
                        return Action::None;
                    }
                    // Leaving a text box for Next may have queued the same message. Show it once.
                    PENDING.with(|cell| cell.borrow_mut().take());
                    self.last_notice = text.clone();
                    return Action::Popup(text);
                }
                // Checked here rather than while typing, so a half-typed path
                // (for example one that starts with a quote) never pops up a message.
                if let Err(text) = crate::paths::clean_location(&get_text(self.location_hwnd)) {
                    return Action::Popup(text);
                }
                match tools::load_bundle() {
                    Ok(bundle) => self.store_bundle(bundle),
                    Err(err) => return Action::Popup(err),
                }
                self.memory.last_profile = self.active_profile.clone();
                self.commit_location();
                store::save(&self.memory);
                self.page = 1;
                self.rebuild();
                Action::None
            }
            1 => match model::parse_links(&self.links) {
                Err(err) => Action::Popup(err),
                Ok(_) => {
                    let same = self.links == self.fetched_text && self.fetch_ok;
                    self.page = 2;
                    self.rebuild();
                    if same {
                        Action::None
                    } else {
                        self.fetch_ok = false;
                        self.rebuild();
                        Action::StartFetch
                    }
                }
            },
            2 => {
                if !self.names_ready() {
                    let problem = self.name_problems();
                    let text = if problem.is_empty() { "Type a file name for every video.".into() } else { problem };
                    return Action::Popup(text);
                }
                self.page = 3;
                self.rebuild();
                Action::None
            }
            3 => {
                self.page = 4;
                self.rebuild();
                Action::None
            }
            4 => {
                self.page = 5;
                self.rebuild();
                Action::StartDownload
            }
            _ => Action::Close,
        }
    }

    fn save_profile(&mut self) -> Action {
        let name = get_text(self.name_hwnd).trim().to_string();
        if name.is_empty() {
            return Action::Popup("Type a profile name.".into());
        }
        self.options = self.current_options();
        if let Some(profile) = self.memory.profiles.iter_mut().find(|profile| profile.name == name) {
            profile.options = self.options.clone();
        } else {
            self.memory.profiles.push(SavedProfile { name: name.clone(), options: self.options.clone() });
        }
        if self.memory.override_profile == name {
            self.memory.override_profile.clear();
            self.memory.override_location.clear();
        }
        self.active_profile = name.clone();
        self.memory.last_profile = name;
        self.loading = true;
        combo_clear(self.profile_hwnd);
        for profile in &self.memory.profiles {
            combo_add(self.profile_hwnd, &profile.name);
        }
        let sel = self.memory.profiles.iter().position(|profile| profile.name == self.active_profile).unwrap_or(0);
        set_combo(self.profile_hwnd, sel);
        self.loading = false;
        store::save(&self.memory);
        self.note_options(false);
        Action::None
    }

    fn change_profile(&mut self) {
        if self.loading {
            return;
        }
        self.commit_location();
        let sel = combo_sel(self.profile_hwnd);
        let Some(name) = self.memory.profiles.get(sel).map(|profile| profile.name.clone()) else {
            return;
        };
        let mut options = self.memory.profiles[sel].options.clone();
        if self.memory.override_profile == name && !self.memory.override_location.is_empty() {
            options.location = self.memory.override_location.clone();
        }
        self.options = options;
        self.active_profile = name;
        self.memory.last_profile = self.active_profile.clone();
        store::save(&self.memory);
        self.loading = true;
        self.rebuild();
        self.loading = false;
        self.last_notice = self.explain(false);
    }

    fn commit_location(&mut self) {
        if self.location_hwnd.0.is_null() {
            return;
        }
        let typed = get_text(self.location_hwnd);
        // Show the full folder the files will go to. If the text can't be used,
        // leave it as typed; the Options page explains why and Next stays off.
        let text = match crate::paths::clean_location(&typed) {
            Ok(full) => {
                if full != typed {
                    set_text(self.location_hwnd, &full);
                }
                full
            }
            Err(_) => typed,
        };
        let saved = self
            .memory
            .profiles
            .iter()
            .find(|profile| profile.name == self.active_profile)
            .map(|profile| profile.options.location.clone())
            .unwrap_or_default();
        if text != saved {
            self.memory.override_profile = self.active_profile.clone();
            self.memory.override_location = text;
        } else if self.memory.override_profile == self.active_profile {
            self.memory.override_profile.clear();
            self.memory.override_location.clear();
        }
    }

    fn current_options(&self) -> Options {
        if self.profile_hwnd.0.is_null() {
            return self.options.clone();
        }
        Options {
            resolution: *Resolution::ALL.get(combo_sel(self.res_hwnd)).unwrap_or(&Resolution::P1080),
            video_codec: *VideoCodec::ALL.get(combo_sel(self.vcodec_hwnd)).unwrap_or(&VideoCodec::H264),
            audio_quality: *AudioQuality::ALL.get(combo_sel(self.aqual_hwnd)).unwrap_or(&AudioQuality::Best),
            audio_codec: *AudioCodec::ALL.get(combo_sel(self.acodec_hwnd)).unwrap_or(&AudioCodec::Aac),
            container: *Container::listed_for(checked(self.audio_hwnd)).get(combo_sel(self.container_hwnd)).unwrap_or(&Container::Mp4),
            audio_only: checked(self.audio_hwnd),
            reencode: checked(self.reencode_hwnd),
            sponsorblock: checked(self.sponsorblock_hwnd),
            overwrite: checked(self.overwrite_hwnd),
            strip_genre: checked(self.strip_genre_hwnd),
            min_wait: parse_u32(&get_text(self.min_hwnd), 4),
            max_wait: parse_u32(&get_text(self.max_hwnd), 8),
            location: {
                let typed = get_text(self.location_hwnd);
                crate::paths::clean_location(&typed).unwrap_or_else(|_| typed.trim().to_string())
            },
        }
    }

    fn current_links(&self) -> String {
        if self.link_hwnd.0.is_null() { self.links.clone() } else { get_text(self.link_hwnd) }
    }

    fn note_options(&mut self, popup: bool) {
        if self.page != 0 || self.profile_hwnd.0.is_null() {
            return;
        }
        self.options = self.current_options();
        self.apply_option_enables();
        let text = self.explain(true);
        self.refresh_nav();
        if self.loading {
            self.last_notice = text;
            return;
        }
        if popup && !text.is_empty() && text != self.last_notice {
            self.last_notice = text.clone();
            // Posted after this returns via a nested call. Show it here only when the
            // window procedure has finished this message: stash by sending ourselves nothing.
            // The caller for combo changes returns Action::None. Show from a deferred path.
            self.defer_popup(text);
        } else if text.is_empty() {
            self.last_notice.clear();
        }
    }

    /// True when the focus is moving to another control of this window, and not to
    /// another program. Checked while a text box is losing the focus.
    fn leaving_for_own_control(&self) -> bool {
        let focus = unsafe { GetFocus() };
        !focus.0.is_null() && unsafe { IsChild(self.hwnd, focus).as_bool() }
    }

    fn defer_popup(&mut self, text: String) {
        // MessageBox from inside the window procedure re-enters it. The flag stops that
        // re-entry from touching this App. The box runs after handle() returns only when
        // the caller uses Action::Popup. Combo changes need the same, so queue it.
        let hwnd = self.hwnd;
        let wide_text = wide(&text);
        let title = wide("Link Download");
        // Run after the borrow by posting a timer-free direct call through the outer flag.
        IN_CALLBACK.set(true);
        // The actual box is shown by returning... we cannot both return and call.
        // Store and let the timer show it once.
        self.last_notice = String::from_utf16_lossy(&wide_text[..wide_text.len().saturating_sub(1)]);
        let _ = (hwnd, title);
        PENDING.with(|cell| *cell.borrow_mut() = Some(self.last_notice.clone()));
        IN_CALLBACK.set(false);
    }

    fn explain(&self, solved: bool) -> String {
        let opts = if self.profile_hwnd.0.is_null() { self.options.clone() } else { self.current_options() };
        problems(&opts)
            .into_iter()
            .filter(|problem| !(solved && problem.codec && opts.reencode))
            .map(|problem| problem.message)
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn apply_option_enables(&self) {
        if self.audio_hwnd.0.is_null() {
            return;
        }
        let opts = self.current_options();
        let codec = problems(&opts).iter().any(|problem| problem.codec);
        unsafe {
            let _ = enable(self.res_hwnd, !opts.audio_only);
            let _ = enable(self.vcodec_hwnd, !opts.audio_only);
            if !codec {
                set_check(self.reencode_hwnd, false);
            }
            let _ = enable(self.reencode_hwnd, codec);
        }
    }

    fn refill_containers(&mut self) {
        let list = Container::listed_for(self.options.audio_only);
        combo_clear(self.container_hwnd);
        for container in list {
            combo_add(self.container_hwnd, container.label());
        }
        let sel = list.iter().position(|container| *container == self.options.container).unwrap_or(0);
        set_combo(self.container_hwnd, sel);
    }

    fn harvest_soft(&mut self) {
        // Keep edits when the same page is rebuilt, and when leaving.
        if self.page == 1 && !self.link_hwnd.0.is_null() {
            self.links = get_text(self.link_hwnd);
        }
        if self.page == 0 && !self.profile_hwnd.0.is_null() {
            self.options = self.current_options();
        }
    }

    fn harvest(&mut self) {
        self.harvest_soft();
        if self.page == 2 && self.fetch_ok {
            self.read_names();
        }
        if self.page == 3 && self.fetch_ok {
            self.read_titles();
        }
        if self.page == 4 {
            self.read_meta();
        }
        if self.page == 0 {
            self.commit_location();
        }
    }

    fn read_names(&mut self) {
        let pane = self.pane();
        for (group_index, group) in self.groups.iter_mut().enumerate() {
            if let Ok(list) = child(pane, ID_LIST + group_index as i32) {
                for (index, item) in group.items.iter_mut().enumerate() {
                    item.title = list_text(list, index as i32);
                }
            }
        }
        self.capture_tools();
    }

    fn capture_tools(&mut self) {
        self.fit_tools();
        let titles = self.page == 3;
        for index in 0..self.groups.len().min(GROUP_LIMIT) {
            let tools = self.tools_of(index);
            if titles {
                self.title_tools[index] = tools;
            } else {
                self.name_tools[index] = tools;
            }
        }
    }

    fn tools_of(&self, group: usize) -> TextTools {
        TextTools {
            remove: text_of(self.pane(), ID_REMOVE + group as i32),
            insert: text_of(self.pane(), ID_INSERT + group as i32),
            at: match combo_sel_id(self.pane(), ID_MODE + group as i32) {
                1 => InsertAt::End,
                2 => InsertAt::Before,
                3 => InsertAt::After,
                _ => InsertAt::Start,
            },
            find: text_of(self.pane(), ID_FIND + group as i32),
            strip: checked_id(self.pane(), ID_STRIP + group as i32),
        }
    }

    fn sync_apply(&self, group: usize) {
        let tools = self.tools_of(group);
        let before = matches!(tools.at, InsertAt::Before | InsertAt::After);
        if let Ok(find) = child(self.pane(), ID_FIND + group as i32) {
            unsafe { let _ = enable(find, before); }
        }
        if let Ok(apply) = child(self.pane(), ID_APPLY + group as i32) {
            unsafe { let _ = enable(apply, names::tools_ready(&tools)); }
        }
    }

    fn apply_group(&mut self, group: usize) {
        let tools = self.tools_of(group);
        if !names::tools_ready(&tools) {
            return;
        }
        let Ok(list) = child(self.pane(), ID_LIST + group as i32) else {
            return;
        };
        let count = unsafe { SendMessageW(list, LVM_GETITEMCOUNT, WPARAM(0), LPARAM(0)).0.max(0) as usize };
        for index in 0..count {
            let next = names::apply_tools(&list_text(list, index as i32), &tools);
            set_list_text(list, index as i32, &next);
            if let Some(item) = self.groups.get_mut(group).and_then(|group| group.items.get_mut(index)) {
                item.title = next;
            }
        }
        if group < self.name_tools.len() {
            self.name_tools[group] = tools;
        }
        self.refresh_nav();
    }

    fn apply_titles(&mut self, group: usize) {
        let tools = self.tools_of(group);
        if !names::tools_ready(&tools) {
            return;
        }
        let Some(count) = self.groups.get(group).map(|group| group.items.len()) else {
            return;
        };
        let start = flat_start(&self.groups, group);
        for index in 0..count {
            if self.groups[group].items[index].blocked.is_some() {
                continue;
            }
            let id = ID_TITLE_FIELD + (start + index) as i32;
            let Ok(hwnd) = child(self.pane(), id) else {
                continue;
            };
            if hwnd.0.is_null() {
                continue;
            }
            let next = names::apply_tools(&get_text(hwnd), &tools);
            self.groups[group].items[index].meta_title = next.clone();
            set_text(hwnd, &next);
        }
        if group < self.title_tools.len() {
            self.title_tools[group] = tools;
        }
    }

    fn read_titles(&mut self) {
        let pane = self.pane();
        let mut flat = 0usize;
        for group in self.groups.iter_mut().take(GROUP_LIMIT) {
            for item in &mut group.items {
                if ID_TITLE_SAME + flat as i32 > 65000 {
                    break;
                }
                if let Ok(hwnd) = child(pane, ID_TITLE_FIELD + flat as i32) {
                    if !hwnd.0.is_null() {
                        item.meta_title = get_text(hwnd);
                    }
                }
                flat += 1;
            }
        }
        self.capture_tools();
    }

    fn use_file_name(&mut self, flat: usize) {
        let Some((group_index, item_index)) = locate_flat(&self.groups, flat) else {
            return;
        };
        if self.groups[group_index].items[item_index].blocked.is_some() {
            return;
        }
        let name = self.groups[group_index].items[item_index].title.clone();
        self.groups[group_index].items[item_index].meta_title = name.clone();
        if let Ok(hwnd) = child(self.pane(), ID_TITLE_FIELD + flat as i32) {
            set_text(hwnd, &name);
        }
    }

    fn use_file_names(&mut self, group_index: usize) {
        let Some(group) = self.groups.get(group_index) else {
            return;
        };
        let start = flat_start(&self.groups, group_index);
        let names: Vec<(usize, String, bool)> = group
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| (start + index, item.title.clone(), item.blocked.is_some()))
            .collect();
        for (flat, name, blocked) in names {
            if blocked {
                continue;
            }
            let item_index = flat - start;
            self.groups[group_index].items[item_index].meta_title = name.clone();
            if let Ok(hwnd) = child(self.pane(), ID_TITLE_FIELD + flat as i32) {
                set_text(hwnd, &name);
            }
        }
    }

    fn reset_names(&mut self, group: usize) {
        let Some(source) = self.groups.get_mut(group) else {
            return;
        };
        for item in &mut source.items {
            item.title = item.original_title.clone();
        }
        if let Ok(list) = child(self.pane(), ID_LIST + group as i32) {
            for (index, item) in self.groups[group].items.iter().enumerate() {
                set_list_text(list, index as i32, &item.title);
            }
        }
        self.refresh_nav();
    }

    fn reset_titles(&mut self, group: usize) {
        let Some(source) = self.groups.get_mut(group) else {
            return;
        };
        for item in &mut source.items {
            item.meta_title = item.original_title.clone();
        }
        let start = flat_start(&self.groups, group);
        let titles: Vec<String> = self.groups[group].items.iter().map(|item| item.meta_title.clone()).collect();
        for (index, title) in titles.iter().enumerate() {
            if let Ok(hwnd) = child(self.pane(), ID_TITLE_FIELD + (start + index) as i32) {
                set_text(hwnd, title);
            }
        }
    }

    fn reset_meta(&mut self, group: usize) {
        let Some(source) = self.groups.get_mut(group) else {
            return;
        };
        source.album = source.original_album.clone();
        for item in &mut source.items {
            item.artists = item.original_artists.clone();
        }
        let album = self.groups[group].album.clone();
        if let Ok(hwnd) = child(self.pane(), ID_ALBUM + group as i32) {
            set_text(hwnd, &album);
        }
        let mut flat = self.groups.iter().take(group).map(|group| group.items.len()).sum::<usize>();
        let rows: Vec<(String, bool)> = self.groups[group]
            .items
            .iter()
            .map(|item| (item.artists.join("\r\n"), item.blocked.is_some()))
            .collect();
        for (artists, blocked) in rows {
            if let Ok(hwnd) = child(self.pane(), ID_ARTIST + flat as i32) {
                set_text(hwnd, &artists);
                set_enabled(hwnd, !blocked);
            }
            flat += 1;
        }
    }

    fn read_meta(&mut self) {
        let pane = self.pane();
        let mut flat = 0usize;
        for (group_index, group) in self.groups.iter_mut().enumerate() {
            group.album = text_of(pane, ID_ALBUM + group_index as i32).trim().to_string();
            for item in &mut group.items {
                let text = text_of(pane, ID_ARTIST + flat as i32);
                item.artists = text.lines().map(|line| line.trim().to_string()).filter(|line| !line.is_empty()).collect();
                flat += 1;
            }
        }
    }

    fn store_bundle(&mut self, bundle: tools::Bundle) {
        self.ytdlp = bundle.ytdlp;
        self.ffmpeg = bundle.ffmpeg;
        self.node = bundle.node;
    }

    fn fit_tools(&mut self) {
        if self.name_tools.len() < self.groups.len() {
            self.name_tools.resize(self.groups.len(), TextTools::default());
        }
        if self.title_tools.len() < self.groups.len() {
            self.title_tools.resize(self.groups.len(), TextTools::default());
        }
    }

    fn spawn(&mut self, kind: Busy) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let shared = Shared::new();
        let shared_thread = shared.clone();
        let ytdlp = self.ytdlp.clone();
        let ffmpeg = self.ffmpeg.clone();
        let node = self.node.clone();
        let ffmpeg_dir = tools::ffmpeg_dir(&ffmpeg);
        let ffmpeg_bin = ffmpeg.clone();
        let opts = self.options.clone();
        let groups = self.groups.clone();
        let urls = model::parse_links(&self.links).unwrap_or_default();
        self.shared = Some(shared);
        self.busy = kind;
        self.rows.clear();
        self.reset_speed();
        self.started = Some(Instant::now());
        self.worker = Some(std::thread::spawn(move || match kind {
            Busy::Fetch => run::fetch(shared_thread, ytdlp, node, ffmpeg_dir, urls),
            Busy::Download => run::download(shared_thread, ytdlp, node, ffmpeg_bin, ffmpeg_dir, opts, groups),
            Busy::Idle => {}
        }));
        self.refresh_nav();
    }

    fn reset_speed(&mut self) {
        self.samples.clear();
        self.sample_start = Instant::now();
        self.last_done = 0;
        self.base_done = None;
        self.finished = 0;
        self.last_finish = 0.0;
    }

    fn on_timer(&mut self) -> Action {
        let Some(text) = PENDING.with(|cell| cell.borrow_mut().take()) else {
            return self.poll_work();
        };
        // A selection changed. Explain it once the procedure is idle.
        if !self.loading && self.page == 0 {
            return Action::Popup(text);
        }
        self.poll_work()
    }

    fn poll_work(&mut self) -> Action {
        if self.stopping {
            let finished = self.worker.as_ref().map(|worker| worker.is_finished()).unwrap_or(true);
            if !finished {
                return Action::None;
            }
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
            self.stopping = false;
            self.after_cancel();
            return Action::None;
        }
        let Some(shared) = self.shared.clone() else {
            return Action::None;
        };
        let live = match shared.live.lock() {
            Ok(live) => live.clone(),
            Err(_) => return Action::None,
        };
        if self.busy == Busy::Idle {
            return Action::None;
        }
        self.rows = live.rows.clone();
        self.track_speed(&live);
        self.paint_readouts(&live);
        if !self.list_hwnd.0.is_null() {
            clamp_list(self, self.rows.len());
            unsafe { let _ = InvalidateRect(self.list_hwnd, None, false); }
        }
        self.refresh_nav();
        if !live.done {
            return Action::None;
        }
        let taken = shared.groups.lock().ok().and_then(|mut slot| slot.take());
        if let Some(groups) = taken {
            self.groups = groups;
            self.fit_tools();
        }
        let kind = self.busy;
        self.busy = Busy::Idle;
        let notice = live.error.clone();
        match kind {
            Busy::Fetch => {
                self.fetch_ok = true;
                self.fetched_text = self.links.clone();
                self.rebuild();
                if !notice.is_empty() && !self.told_update {
                    self.told_update = true;
                    return Action::Notice(notice);
                }
            }
            Busy::Download => {
                self.download_done = true;
                self.have_media = true;
                self.media_stamp = self.options.clone();
                self.refresh_nav();
                unsafe { let _ = InvalidateRect(self.list_hwnd, None, false); }
            }
            Busy::Idle => {}
        }
        Action::None
    }

    fn track_speed(&mut self, live: &Live) {
        let now = Instant::now();
        let bytes = live.banked_bytes.saturating_add(live.current_bytes);
        self.samples.push_back((now, bytes));
        while self.samples.len() > 2 && now.saturating_duration_since(self.samples[0].0).as_millis() > 2000 {
            self.samples.pop_front();
        }
        // Items pinned before the run starts count as done but took no time. Leave them out.
        if live.total == 0 {
            return;
        }
        let done = live.ok.saturating_add(live.bad);
        let base = *self.base_done.get_or_insert(done);
        if done > self.last_done.max(base) {
            self.finished = done - base;
            self.last_finish = now.saturating_duration_since(self.sample_start).as_secs_f64();
        }
        self.last_done = done;
    }

    fn paint_readouts(&self, live: &Live) {
        if self.status_hwnd.0.is_null() {
            return;
        }
        let status = live.status.clone();
        set_text(self.status_hwnd, &status);
        if self.progress_hwnd.0.is_null() {
            return;
        }
        let ratio = work_ratio(live);
        unsafe { SendMessageW(self.progress_hwnd, PBM_SETPOS, WPARAM(readout::percent(ratio) as usize * 10), LPARAM(0)); }
        if !self.percent_hwnd.0.is_null() {
            set_text(self.percent_hwnd, &format!("{}%", readout::percent(ratio)));
        }
        if self.readout[0].0.is_null() {
            return;
        }
        let (speed, items) = speed_of(self);
        let left = live.rows.iter().filter(|row| matches!(row.kind, RowKind::Queued | RowKind::Active | RowKind::Waiting)).count();
        let progress = live.rows.iter().find(|row| row.kind == RowKind::Active).map(|row| (row.fraction + row.convert) * 0.5);
        let wait = if self.busy == Busy::Download { (self.options.min_wait + self.options.max_wait) as f64 * 0.5 } else { 0.0 };
        let estimate = readout::time_left(self.sample_start.elapsed().as_secs_f64(), self.finished, self.last_finish, left as u32, progress, wait);
        let eta = if live.done || left == 0 {
            "—".to_string()
        } else if let Some(seconds) = estimate {
            readout::format_eta(seconds)
        } else {
            "Calculating...".to_string()
        };
        let elapsed = self.started.map(|start| readout::format_elapsed(start.elapsed().as_secs())).unwrap_or_else(|| "0:00".into());
        let values = [
            readout::format_rate(speed),
            readout::format_items(items),
            left.to_string(),
            eta.to_string(),
            elapsed,
        ];
        for (index, value) in values.iter().enumerate() {
            set_text(self.readout[index], value);
        }
    }

    fn after_cancel(&mut self) {
        let kind = self.busy;
        self.busy = Busy::Idle;
        if kind != Busy::Fetch {
            let taken = self.shared.as_ref().and_then(|shared| shared.groups.lock().ok().and_then(|mut slot| slot.take()));
            if let Some(groups) = taken {
                self.groups = groups;
                self.fit_tools();
            }
        } else {
            self.fetch_ok = false;
        }
        if kind == Busy::Download {
            self.download_done = false;
        }
        if self.page > 0 {
            self.page -= 1;
        }
        self.rebuild();
    }

    fn shutdown(&mut self) {
        unsafe {
            let _ = KillTimer(self.hwnd, 1);
            if !self.caption_font.0.is_null() {
                let _ = DeleteObject(self.caption_font);
                self.caption_font = HFONT(std::ptr::null_mut());
            }
        }
        if let Some(shared) = &self.shared {
            run::kill_child(shared);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.harvest();
        run::discard_partial();
        self.memory.last_profile = self.active_profile.clone();
        store::save(&self.memory);
        let _ = std::fs::remove_dir_all(&self.temp);
        unsafe { PostQuitMessage(0); }
    }

    fn tip_nav(&mut self) {
        add_tip(self, self.back, "Previous page");
        add_tip(self, self.next, "Next page");
    }

    fn retip(&mut self) {
        if !self.tip.0.is_null() {
            unsafe { let _ = DestroyWindow(self.tip); }
        }
        self.tip = tooltip(self.hwnd, self.instance);
        self.tip_nav();
        let pairs: Vec<(HWND, &str)> = match self.page {
            0 => vec![
                (self.profile_hwnd, "Saved options"),
                (self.name_hwnd, "Name for this profile"),
                (self.res_hwnd, "Picture height"),
                (self.vcodec_hwnd, "Video to keep"),
                (self.aqual_hwnd, "Audio bitrate"),
                (self.acodec_hwnd, "Audio to keep"),
                (self.container_hwnd, "Finished file type"),
                (self.strip_genre_hwnd, "Drop the genre tag and the video description"),
                (self.audio_hwnd, "Download sound only"),
                (self.reencode_hwnd, "Convert when a copy will not fit"),
                (self.sponsorblock_hwnd, "Cut sponsors, self-promotion, and interaction reminders"),
                (self.overwrite_hwnd, "Download songs again and replace files that are already saved"),
                (self.min_hwnd, "Shortest pause between downloads"),
                (self.max_hwnd, "Longest pause between downloads"),
                (self.location_hwnd, "Folder for finished files"),
            ],
            1 => vec![(self.link_hwnd, "One link on each line")],
            2 | 3 | 4 => Vec::new(),
            _ => Vec::new(),
        };
        for (hwnd, text) in pairs {
            add_tip(self, hwnd, text);
        }
        if self.page == 2 {
            for index in 0..self.groups.len().min(GROUP_LIMIT) {
                if let Ok(reset) = child(self.pane(), ID_RESET + index as i32) {
                    add_tip(self, reset, "Put the file names back");
                }
            }
        }
        if self.page == 3 {
            for index in 0..self.groups.len().min(GROUP_LIMIT) {
                if let Ok(reset) = child(self.pane(), ID_RESET + index as i32) {
                    add_tip(self, reset, "Put the titles back");
                }
                if let Ok(all) = child(self.pane(), ID_TITLE_SAME_ALL + index as i32) {
                    add_tip(self, all, "Use each file name as the title");
                }
            }
            let mut flat = 0usize;
            for group in self.groups.iter().take(GROUP_LIMIT) {
                for _item in &group.items {
                    if ID_TITLE_SAME + flat as i32 > 65000 {
                        break;
                    }
                    if let Ok(same) = child(self.pane(), ID_TITLE_SAME + flat as i32) {
                        add_tip(self, same, "Use this file name as the title");
                    }
                    flat += 1;
                }
            }
        }
        if self.page == 4 {
            for index in 0..self.groups.len().min(GROUP_LIMIT) {
                if let Ok(reset) = child(self.pane(), ID_META_RESET + index as i32) {
                    add_tip(self, reset, "Put the album and artists back");
                }
            }
        }
        if self.page == 0 {
            if let Ok(save) = child(self.pane(), ID_SAVE) {
                add_tip(self, save, "Save these options");
            }
            if let Ok(browse) = child(self.pane(), ID_BROWSE) {
                add_tip(self, browse, "Choose a folder");
            }
        }
    }
}

thread_local! {
    static PENDING: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Whether a change on the Options page may explain a problem in a popup.
/// Drop-downs and check boxes explain right away. A text box only explains once you
/// leave it for another control here, never on a keystroke, so a half-typed number
/// such as the 1 of 12 does not interrupt you. Next always explains.
fn popup_for(id: i32, code: u16, leaving_for_own_control: bool) -> bool {
    if matches!(id, ID_MIN | ID_MAX | ID_LOCATION) {
        return code == EN_KILLFOCUS as u16 && leaving_for_own_control;
    }
    code != EN_CHANGE as u16 && code != EN_KILLFOCUS as u16
}

fn speed_of(app: &App) -> (f64, f64) {
    let speed = if app.samples.len() >= 2 {
        let older = app.samples.front().unwrap();
        let newer = app.samples.back().unwrap();
        let seconds = newer.0.saturating_duration_since(older.0).as_secs_f64().max(0.2);
        (newer.1.saturating_sub(older.1) as f64) / seconds
    } else {
        0.0
    };
    // Whole run, waits included, so it agrees with Time left.
    let run = app.sample_start.elapsed().as_secs_f64().max(0.25);
    let items = app.finished as f64 / run;
    (speed, items)
}

fn work_ratio(live: &Live) -> f64 {
    if live.total == 0 {
        return if live.done { 1.0 } else { 0.0 };
    }
    let active = live
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Active)
        .map(|row| (row.fraction + row.convert) * 0.5)
        .unwrap_or(0.0);
    ((live.ok.saturating_add(live.bad) as f64 + active) / live.total as f64).clamp(0.0, 1.0)
}

fn same_media(a: &Options, b: &Options) -> bool {
    a.resolution == b.resolution
        && a.video_codec == b.video_codec
        && a.audio_quality == b.audio_quality
        && a.audio_codec == b.audio_codec
        && a.audio_only == b.audio_only
        && a.reencode == b.reencode
        && a.sponsorblock == b.sponsorblock
        && a.strip_genre == b.strip_genre
}

fn clamp_list(app: &mut App, count: usize) {
    let visible = visible_rows(app.list_hwnd, app.dpi);
    let max_scroll = (count as i32 - visible).max(0);
    if app.list_scroll > max_scroll {
        app.list_scroll = max_scroll;
    }
    let mut info = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
        nMin: 0,
        nMax: (count as i32 - 1).max(0),
        nPage: visible.max(1) as u32,
        nPos: app.list_scroll,
        nTrackPos: 0,
    };
    unsafe { SetScrollInfo(app.list_hwnd, SB_VERT, &info, true); }
}

fn visible_rows(hwnd: HWND, dpi: i32) -> i32 {
    let (_, h) = client_of(hwnd);
    let header = scale(18, dpi);
    let row = scale(18, dpi).max(1);
    ((h - header) / row).max(1)
}

fn scroll_work(hwnd: HWND, wparam: WPARAM) {
    let code = (wparam.0 & 0xFFFF) as u16;
    let pos = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32;
    APP.with(|slot| {
        let Ok(mut borrow) = slot.try_borrow_mut() else {
            return;
        };
        let Some(app) = borrow.as_mut() else {
            return;
        };
        let visible = visible_rows(hwnd, app.dpi);
        let max_scroll = (app.rows.len() as i32 - visible).max(0);
        match code {
            0 => app.list_scroll -= 1,
            1 => app.list_scroll += 1,
            2 => app.list_scroll -= visible,
            3 => app.list_scroll += visible,
            4 | 5 => app.list_scroll = pos,
            6 => app.list_scroll = 0,
            7 => app.list_scroll = max_scroll,
            _ => {}
        }
        app.list_scroll = app.list_scroll.clamp(0, max_scroll);
        clamp_list(app, app.rows.len());
        unsafe { let _ = InvalidateRect(hwnd, None, false); }
    });
}

fn nudge_work(hwnd: HWND, delta: i32) {
    APP.with(|slot| {
        let Ok(mut borrow) = slot.try_borrow_mut() else {
            return;
        };
        let Some(app) = borrow.as_mut() else {
            return;
        };
        let visible = visible_rows(hwnd, app.dpi);
        let max_scroll = (app.rows.len() as i32 - visible).max(0);
        app.list_scroll = (app.list_scroll + delta).clamp(0, max_scroll);
        clamp_list(app, app.rows.len());
        unsafe { let _ = InvalidateRect(hwnd, None, false); }
    });
}

fn paint_work(hwnd: HWND) {
    unsafe {
        let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let (width, height) = client_of(hwnd);
        if width <= 0 || height <= 0 {
            let _ = EndPaint(hwnd, &ps);
            return;
        }
        let mem = CreateCompatibleDC(hdc);
        let bitmap = CreateCompatibleBitmap(hdc, width, height);
        let old_bmp = SelectObject(mem, bitmap);
        let white = white_brush();
        let face = FACE.get();
        let _ = FillRect(mem, &RECT { left: 0, top: 0, right: width, bottom: height }, white);
        APP.with(|slot| {
            let Ok(borrow) = slot.try_borrow() else {
                return;
            };
            let Some(app) = borrow.as_ref() else {
                return;
            };
            let font = SelectObject(mem, app.font);
            let header_h = app.s(18);
            let row_h = app.s(18);
            let mut header = RECT { left: 0, top: 0, right: width, bottom: header_h };
            let _ = FillRect(mem, &header, face);
            let _ = DrawEdge(mem, &mut header, EDGE_RAISED, BF_RECT);
            let (name_w, size_w, download_w, convert_w) = columns(width);
            let _ = SetBkMode(mem, TRANSPARENT);
            let _ = SetTextColor(mem, rgb(0, 0, 0));
            text_in(mem, "File", RECT { left: 6, top: 0, right: name_w - 4, bottom: header_h }, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
            text_in(mem, "Size", RECT { left: name_w, top: 0, right: name_w + size_w - 6, bottom: header_h }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            text_in(mem, "Download", RECT { left: name_w + size_w + 6, top: 0, right: name_w + size_w + download_w - 4, bottom: header_h }, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
            text_in(mem, "Convert", RECT { left: name_w + size_w + download_w + 6, top: 0, right: width - 4, bottom: header_h }, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
            if app.rows.is_empty() {
                text_in(mem, "Finished files leave this list.", RECT { left: 8, top: header_h + 8, right: width - 8, bottom: height - 8 }, DT_LEFT | DT_NOPREFIX);
            }
            let mut y = header_h;
            let mut index = app.list_scroll.max(0) as usize;
            while y < height && index < app.rows.len() {
                paint_row(mem, &app.rows[index], RECT { left: 0, top: y, right: width, bottom: y + row_h }, name_w, size_w, download_w, convert_w);
                y += row_h;
                index += 1;
            }
            SelectObject(mem, font);
            let _ = convert_w;
        });
        let _ = BitBlt(hdc, 0, 0, width, height, mem, 0, 0, SRCCOPY);
        SelectObject(mem, old_bmp);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(mem);
        let _ = EndPaint(hwnd, &ps);
    }
}

fn paint_row(hdc: HDC, row: &WorkRow, rect: RECT, name_w: i32, size_w: i32, download_w: i32, convert_w: i32) {
    let pinned = row.kind == RowKind::Pinned || (row.kind == RowKind::Failed && row.note != "Saved without a cover");
    let back = if pinned { rgb(255, 196, 196) } else { rgb(255, 255, 255) };
    let brush = unsafe { CreateSolidBrush(back) };
    unsafe {
        let _ = FillRect(hdc, &rect, brush);
        let _ = DeleteObject(brush);
        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, rgb(0, 0, 0));
    }
    text_in(hdc, &row.name, RECT { left: rect.left + 6, top: rect.top, right: rect.left + name_w - 4, bottom: rect.bottom }, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
    let size = if row.size > 0 { readout::format_bytes(row.size) } else { String::new() };
    text_in(hdc, &size, RECT { left: rect.left + name_w, top: rect.top, right: rect.left + name_w + size_w - 6, bottom: rect.bottom }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    if !row.note.is_empty() && row.kind != RowKind::Active {
        text_in(
            hdc,
            &row.note,
            RECT { left: rect.left + name_w + size_w + 6, top: rect.top, right: rect.right - 6, bottom: rect.bottom },
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        return;
    }
    let bar_h = ((rect.bottom - rect.top) - 6).clamp(8, 14);
    let top = rect.top + (rect.bottom - rect.top - bar_h) / 2;
    let download = RECT {
        left: rect.left + name_w + size_w + 6,
        top,
        right: rect.left + name_w + size_w + download_w - 8,
        bottom: top + bar_h,
    };
    let convert = RECT {
        left: rect.left + name_w + size_w + download_w + 6,
        top,
        right: rect.left + name_w + size_w + download_w + convert_w - 8,
        bottom: top + bar_h,
    };
    let download_fraction = if row.kind == RowKind::Pinned { 0.0 } else { row.fraction };
    chunks(hdc, download, download_fraction);
    chunks(hdc, convert, row.convert);
}

fn chunks(hdc: HDC, mut outer: RECT, fraction: f64) {
    unsafe {
        let _ = DrawEdge(hdc, &mut outer, EDGE_SUNKEN, BF_RECT);
        let _ = InflateRect(&mut outer, -2, -2);
        let chunk = 8;
        let gap = 2;
        let width = outer.right - outer.left;
        if width <= 0 {
            return;
        }
        let count = (width + gap) / (chunk + gap);
        let filled = (fraction.clamp(0.0, 1.0) * count as f64).round() as i32;
        let navy = CreateSolidBrush(rgb(0, 0, 128));
        for index in 0..filled.max(0) {
            let x = outer.left + index * (chunk + gap);
            let block = RECT { left: x, top: outer.top, right: (x + chunk).min(outer.right), bottom: outer.bottom };
            let _ = FillRect(hdc, &block, navy);
        }
        let _ = DeleteObject(navy);
    }
}

fn columns(width: i32) -> (i32, i32, i32, i32) {
    let mut bar = (width / 5).clamp(72, 160);
    let mut size = (width / 8).clamp(52, 80);
    let mut name = width - bar * 2 - size;
    if name < 72 {
        bar = ((width - 72 - 52) / 2).max(48);
        size = 52;
        name = (width - bar * 2 - size).max(40);
    }
    (name, size, bar, bar)
}

fn text_in(hdc: HDC, text: &str, mut rect: RECT, format: windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT) {
    let mut wide = wide(text);
    unsafe {
        let end = wide.len().saturating_sub(1);
        let _ = DrawTextW(hdc, &mut wide[..end], &mut rect, format);
    }
}

fn fill_titles(list: HWND, titles: &[String], width: i32) {
    let mut title = wide("File name");
    let column = LVCOLUMNW {
        mask: LVCF_TEXT | LVCF_WIDTH | LVCF_FMT,
        fmt: LVCFMT_LEFT,
        cx: width,
        pszText: PWSTR(title.as_mut_ptr()),
        ..Default::default()
    };
    unsafe {
        SendMessageW(list, LVM_SETEXTENDEDLISTVIEWSTYLE, WPARAM(0), LPARAM(LVS_EX_FULLROWSELECT as isize));
        SendMessageW(list, LVM_INSERTCOLUMNW, WPARAM(0), LPARAM(&column as *const _ as isize));
    }
    for (index, title) in titles.iter().enumerate() {
        let mut text = wide(title);
        let row = LVITEMW {
            mask: LVIF_TEXT,
            iItem: index as i32,
            pszText: PWSTR(text.as_mut_ptr()),
            ..Default::default()
        };
        unsafe { SendMessageW(list, LVM_INSERTITEMW, WPARAM(0), LPARAM(&row as *const _ as isize)); }
    }
}

/// Puts the problem line on the Names page warning label. The label keeps how many
/// characters at the start are the file name, so only those get the bold marks.
fn show_name_error(label: HWND, items: &[crate::model::Item]) {
    let (name, line) = paths::first_name_problem(items).unwrap_or_default();
    unsafe { SetWindowLongPtrW(label, GWLP_USERDATA, name.chars().count() as isize); }
    if get_text(label) == line {
        unsafe { let _ = InvalidateRect(label, None, true); }
    }
    set_text(label, &line);
}

fn draw_name_error(item: &DRAWITEMSTRUCT) {
    unsafe {
        let hdc = item.hDC;
        let rect = item.rcItem;
        let _ = FillRect(hdc, &rect, FACE.get());
        let label = item.hwndItem;
        let text = get_text(label);
        if text.is_empty() {
            return;
        }
        let normal = HFONT(SendMessageW(label, 0x0031, WPARAM(0), LPARAM(0)).0 as *mut _);
        let old = SelectObject(hdc, normal);
        let _ = SetBkMode(hdc, TRANSPARENT);
        let color = if IsWindowEnabled(label).0 != 0 { COLOR_BTNTEXT } else { COLOR_GRAYTEXT };
        let _ = SetTextColor(hdc, COLORREF(GetSysColor(color)));
        let chars: Vec<char> = text.chars().collect();
        let name_len = (GetWindowLongPtrW(label, GWLP_USERDATA).max(0) as usize).min(chars.len());
        let name: String = chars[..name_len].iter().collect();
        let marks = paths::name_marks(&name);
        if name_len == 0 || marks.iter().all(|(_, mark)| *mark == Mark::Plain) {
            // Same as a plain label: wrapped, no prefix characters.
            let mut wide: Vec<u16> = text.encode_utf16().collect();
            let mut area = rect;
            let _ = DrawTextW(hdc, &mut wide, &mut area, DT_LEFT | DT_NOPREFIX | DT_WORDBREAK | DT_EDITCONTROL);
            SelectObject(hdc, old);
            return;
        }
        // The name with its marks on the first line, the reason on the second.
        let (normal_h, _) = font_height(hdc, normal);
        let room = (rect.bottom - rect.top - normal_h).max(normal_h);
        let bold = mark_font(hdc, room);
        let (bold_h, _) = ink_height(hdc, bold);
        let first_h = bold_h.max(normal_h);
        let first = RECT { bottom: rect.top + first_h, ..rect };
        let mut line = marks;
        line.push((':', Mark::Plain));
        draw_marked(hdc, &first, &line, normal, bold, COLORREF(GetSysColor(color)), rgb(192, 192, 192));
        let rest: String = chars[name_len..].iter().collect();
        let rest = rest.trim_start_matches(':').trim_start();
        let mut wide: Vec<u16> = rest.encode_utf16().collect();
        let mut area = RECT { top: first.bottom, ..rect };
        SelectObject(hdc, normal);
        let _ = DrawTextW(hdc, &mut wide, &mut area, DT_LEFT | DT_NOPREFIX | DT_WORDBREAK | DT_EDITCONTROL | DT_END_ELLIPSIS);
        SelectObject(hdc, old);
    }
}

/// NM_CUSTOMDRAW for a Names list. Rows with a legal name are left to the list.
/// Rows with marks are drawn here, with the list's own colors for each state.
fn name_row_draw(lparam: LPARAM) -> isize {
    unsafe {
        let draw = &*(lparam.0 as *const NMLVCUSTOMDRAW);
        let stage = draw.nmcd.dwDrawStage.0;
        if stage == 0x0001 {
            // CDDS_PREPAINT: ask for each row.
            return 0x20;
        }
        if stage != 0x0001_0001 {
            return 0;
        }
        let list = draw.nmcd.hdr.hwndFrom;
        let index = draw.nmcd.dwItemSpec as i32;
        let marks = paths::name_marks(&list_text(list, index));
        if marks.iter().all(|(_, mark)| *mark == Mark::Plain) {
            return 0;
        }
        let hdc = draw.nmcd.hdc;
        // LVM_GETITEMRECT with LVIR_BOUNDS, then LVIR_LABEL.
        let mut bounds = RECT { left: 0, ..Default::default() };
        SendMessageW(list, 0x100E, WPARAM(index as usize), LPARAM(&mut bounds as *mut _ as isize));
        let mut label = RECT { left: 2, ..Default::default() };
        SendMessageW(list, 0x100E, WPARAM(index as usize), LPARAM(&mut label as *mut _ as isize));
        // LVM_GETITEMSTATE for LVIS_FOCUSED | LVIS_SELECTED.
        let state = SendMessageW(list, 0x102C, WPARAM(index as usize), LPARAM(0x0003)).0 as u32;
        let selected = state & 0x0002 != 0;
        let focused = state & 0x0001 != 0;
        let has_focus = GetFocus() == list;
        let enabled = IsWindowEnabled(list).0 != 0;
        let (back, fore) = row_colors(selected, has_focus, enabled);
        let _ = FillRect(hdc, &bounds, GetSysColorBrush(COLOR_WINDOW));
        let row = RECT { left: label.left, ..bounds };
        if back != COLOR_WINDOW {
            let _ = FillRect(hdc, &row, GetSysColorBrush(back));
        }
        let normal = HFONT(SendMessageW(list, 0x0031, WPARAM(0), LPARAM(0)).0 as *mut _);
        let old = SelectObject(hdc, normal);
        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, COLORREF(GetSysColor(fore)));
        let bold = mark_font(hdc, bounds.bottom - bounds.top);
        // The list starts its own text 5 pixels into the label and keeps 5 free at the end.
        let text = RECT { left: label.left + 5, right: label.right - 5, ..label };
        draw_marked(hdc, &text, &marks, normal, bold, COLORREF(GetSysColor(fore)), COLORREF(GetSysColor(back)));
        if focused && has_focus {
            let _ = DrawFocusRect(hdc, &row);
        }
        SelectObject(hdc, old);
        // CDRF_SKIPDEFAULT
        4
    }
}

/// Background and text colors of a list row, the way a classic list view picks them.
fn row_colors(selected: bool, list_focused: bool, enabled: bool) -> (SYS_COLOR_INDEX, SYS_COLOR_INDEX) {
    if !enabled {
        return (COLOR_WINDOW, COLOR_GRAYTEXT);
    }
    match (selected, list_focused) {
        (true, true) => (COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT),
        (true, false) => (COLOR_BTNFACE, COLOR_BTNTEXT),
        _ => (COLOR_WINDOW, COLOR_WINDOWTEXT),
    }
}

/// Character height and ascent of a font.
unsafe fn font_height(hdc: HDC, font: HFONT) -> (i32, i32) {
    let old = SelectObject(hdc, font);
    let mut metrics = TEXTMETRICW::default();
    let _ = GetTextMetricsW(hdc, &mut metrics);
    SelectObject(hdc, old);
    (metrics.tmHeight, metrics.tmAscent)
}

/// Height and ascent of the characters themselves, without the font's internal leading,
/// which is empty space above them. Used for the bold marks.
unsafe fn ink_height(hdc: HDC, font: HFONT) -> (i32, i32) {
    let old = SelectObject(hdc, font);
    let mut metrics = TEXTMETRICW::default();
    let _ = GetTextMetricsW(hdc, &mut metrics);
    SelectObject(hdc, old);
    (metrics.tmHeight - metrics.tmInternalLeading, metrics.tmAscent - metrics.tmInternalLeading)
}

/// The biggest bold font that fits in `height`: 12 pt, else 10 pt, else 8 pt.
unsafe fn mark_font(hdc: HDC, height: i32) -> HFONT {
    let fonts = [MARK_BIG.get(), MARK_SMALL.get(), MARK_TINY.get()];
    let heights = fonts.map(|font| if font.0.is_null() { i32::MAX } else { ink_height(hdc, font).0 });
    fonts[pick_mark(&heights, height)]
}

/// Index of the first font height that fits in `room`, or the last one when none does.
fn pick_mark(heights: &[i32], room: i32) -> usize {
    heights.iter().position(|h| *h <= room).unwrap_or(heights.len() - 1)
}

fn text_width(hdc: HDC, text: &[u16]) -> i32 {
    let mut size = SIZE::default();
    unsafe { let _ = GetTextExtentPoint32W(hdc, text, &mut size); }
    size.cx
}

/// Draws one line, marked characters in `bold` and the rest in `normal`, on one baseline
/// centered in `rect`. A last space or period is also drawn in reverse, `back` on `fore`,
/// so it can be seen. A line that does not fit ends with "...".
unsafe fn draw_marked(hdc: HDC, rect: &RECT, marks: &[(char, Mark)], normal: HFONT, bold: HFONT, fore: COLORREF, back: COLORREF) {
    let (normal_h, normal_up) = font_height(hdc, normal);
    let (bold_h, bold_up) = ink_height(hdc, bold);
    let mut widths = Vec::with_capacity(marks.len());
    for (ch, mark) in marks {
        SelectObject(hdc, if *mark == Mark::Plain { normal } else { bold });
        let mut buf = [0u16; 2];
        widths.push(text_width(hdc, ch.encode_utf16(&mut buf)));
    }
    SelectObject(hdc, normal);
    let dots: Vec<u16> = "...".encode_utf16().collect();
    let keep = fit_count(&widths, rect.right - rect.left, text_width(hdc, &dots));
    let up = normal_up.max(bold_up);
    let down = (normal_h - normal_up).max(bold_h - bold_up);
    let baseline = marked_baseline(rect.top, rect.bottom, normal_h, normal_up, bold_h, bold_up);
    let align = SetTextAlign(hdc, TEXT_ALIGN_OPTIONS(24));
    let mut x = rect.left;
    let mut start = 0;
    while start < keep {
        let mark = marks[start].1;
        let mut end = start;
        while end < keep && marks[end].1 == mark {
            end += 1;
        }
        let run: String = marks[start..end].iter().map(|(ch, _)| *ch).collect();
        let wide: Vec<u16> = run.encode_utf16().collect();
        SelectObject(hdc, if mark == Mark::Plain { normal } else { bold });
        let width = text_width(hdc, &wide);
        if mark == Mark::End {
            let block = RECT { left: x, top: baseline - up, right: x + width, bottom: baseline + down };
            let brush = CreateSolidBrush(fore);
            let _ = FillRect(hdc, &block, brush);
            let _ = DeleteObject(brush);
            let _ = SetTextColor(hdc, back);
            let _ = TextOutW(hdc, x, baseline, &wide);
            let _ = SetTextColor(hdc, fore);
        } else {
            let _ = TextOutW(hdc, x, baseline, &wide);
        }
        x += width;
        start = end;
    }
    if keep < marks.len() {
        SelectObject(hdc, normal);
        let _ = TextOutW(hdc, x, baseline, &dots);
    }
    SetTextAlign(hdc, TEXT_ALIGN_OPTIONS(align));
    SelectObject(hdc, normal);
}

/// The plain text sits where the list would put it, centered in the row. The bigger
/// bold characters share its baseline and are only moved when they would leave the row.
fn marked_baseline(top: i32, bottom: i32, normal_h: i32, normal_up: i32, bold_h: i32, bold_up: i32) -> i32 {
    let mut baseline = top + (bottom - top - normal_h) / 2 + normal_up;
    let low = baseline + (bold_h - bold_up).max(normal_h - normal_up);
    if low > bottom {
        baseline -= low - bottom;
    }
    let high = baseline - bold_up.max(normal_up);
    if high < top {
        baseline += top - high;
    }
    baseline
}

/// How many characters fit in `room`. When not all of them fit, room is kept for the dots.
fn fit_count(widths: &[i32], room: i32, dots: i32) -> usize {
    if widths.iter().sum::<i32>() <= room {
        return widths.len();
    }
    let mut used = 0;
    for (index, width) in widths.iter().enumerate() {
        if used + width + dots > room {
            return index;
        }
        used += width;
    }
    widths.len()
}

fn list_text(list: HWND, index: i32) -> String {
    let mut buf = [0u16; 1024];
    let item = LVITEMW {
        pszText: PWSTR(buf.as_mut_ptr()),
        cchTextMax: buf.len() as i32,
        ..Default::default()
    };
    unsafe { SendMessageW(list, LVM_GETITEMTEXTW, WPARAM(index as usize), LPARAM(&item as *const _ as isize)); }
    let end = buf.iter().position(|ch| *ch == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

fn set_list_text(list: HWND, index: i32, text: &str) {
    let mut wide = wide(text);
    let item = LVITEMW {
        pszText: PWSTR(wide.as_mut_ptr()),
        ..Default::default()
    };
    unsafe { SendMessageW(list, LVM_SETITEMTEXTW, WPARAM(index as usize), LPARAM(&item as *const _ as isize)); }
}

fn tooltip(parent: HWND, instance: HINSTANCE) -> HWND {
    unsafe {
        CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0x00000008),
            w!("tooltips_class32"),
            w!(""),
            windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(0x80000000 | 0x00000001 | TTS_ALWAYSTIP | TTS_NOPREFIX),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            parent,
            HMENU(std::ptr::null_mut()),
            instance,
            None,
        )
        .unwrap_or_default()
    }
}

fn add_tip(app: &App, hwnd: HWND, text: &str) {
    if hwnd.0.is_null() || app.tip.0.is_null() {
        return;
    }
    let mut wide = wide(text);
    let info = TTTOOLINFOW {
        cbSize: std::mem::size_of::<TTTOOLINFOW>() as u32,
        uFlags: TTF_IDISHWND | TTF_SUBCLASS,
        hwnd: app.hwnd,
        uId: hwnd.0 as usize,
        lpszText: PWSTR(wide.as_mut_ptr()),
        ..Default::default()
    };
    unsafe {
        SendMessageW(app.tip, TTM_ADDTOOLW, WPARAM(0), LPARAM(&info as *const _ as isize));
        SendMessageW(app.tip, TTM_SETMAXTIPWIDTH, WPARAM(0), LPARAM(240));
    }
}

fn pick_folder(owner: HWND, current: &str) -> Option<String> {
    unsafe {
        let dialog: IFileDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let options = dialog.GetOptions().ok()?;
        dialog.SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM).ok()?;
        dialog.SetTitle(w!("Location")).ok()?;
        if !current.is_empty() {
            let wide = wide(current);
            let item: windows::core::Result<IShellItem> = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None);
            if let Ok(item) = item {
                let _ = dialog.SetFolder(&item);
            }
        }
        if dialog.Show(owner).is_err() {
            return None;
        }
        let result = dialog.GetResult().ok()?;
        let name = result.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let text = name.to_string().unwrap_or_default();
        CoTaskMemFree(Some(name.0 as *const _));
        if text.is_empty() { None } else { Some(text) }
    }
}

/// Sends the wheel to the control under the pointer when that control can still scroll
/// that way, and to the page otherwise. A text box never keeps the wheel for nothing,
/// and a closed drop-down never changes its value from a scroll.
fn wheel(msg: &MSG) -> bool {
    if msg.message != WM_MOUSEWHEEL {
        return false;
    }
    unsafe {
        let point = POINT { x: (msg.lParam.0 & 0xFFFF) as i16 as i32, y: ((msg.lParam.0 >> 16) & 0xFFFF) as i16 as i32 };
        let target = WindowFromPoint(point);
        let up = wheel_delta(msg.wParam) > 0;
        let class = class_name(target);
        let multiline = class == "Edit" && (GetWindowLongPtrW(target, GWL_STYLE) as u32) & 0x0004 != 0;
        let can = !target.0.is_null() && bar_can_scroll(target, up);
        if wheel_to_control(&class, multiline, can) {
            SendMessageW(target, WM_MOUSEWHEEL, msg.wParam, msg.lParam);
            return true;
        }
        let mut page = GetAncestor(target, GA_ROOT);
        if class_name(page) != "LinkDownloadWindow" {
            page = GetAncestor(msg.hwnd, GA_ROOT);
        }
        if class_name(page) != "LinkDownloadWindow" {
            return false;
        }
        SendMessageW(page, WM_MOUSEWHEEL, msg.wParam, msg.lParam);
    }
    true
}

fn wheel_delta(wparam: WPARAM) -> i32 {
    ((wparam.0 >> 16) & 0xFFFF) as u16 as i16 as i32
}

fn bar_can_scroll(hwnd: HWND, up: bool) -> bool {
    let mut info = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
        ..Default::default()
    };
    if unsafe { GetScrollInfo(hwnd, SB_VERT, &mut info) }.is_err() {
        return false;
    }
    can_scroll(info.nMin, info.nMax, info.nPage, info.nPos, up)
}

/// Whether a scroll bar at `pos` can still move toward the top (`up`) or toward the bottom.
fn can_scroll(min: i32, max: i32, page: u32, pos: i32, up: bool) -> bool {
    let last = (max - page.max(1) as i32 + 1).max(min);
    if up { pos > min } else { pos < last }
}

/// True when the control under the pointer keeps the wheel. Everything else scrolls the page.
fn wheel_to_control(class: &str, multiline: bool, can_scroll: bool) -> bool {
    match class {
        // An open drop-down list floats over the page, so the page stays still under it.
        "ComboLBox" => true,
        "ListBox" | "LinkWorkList" | "SysListView32" => can_scroll,
        "Edit" => multiline && can_scroll,
        _ => false,
    }
}

fn take_wheel(rest: &'static std::thread::LocalKey<Cell<i32>>, wparam: WPARAM, per_notch: i32) -> i32 {
    rest.with(|cell| {
        let mut left = cell.get();
        let steps = wheel_steps(&mut left, wheel_delta(wparam), per_notch);
        cell.set(left);
        steps
    })
}

/// Turns a wheel delta into whole steps, `per_notch` steps for each 120. The part smaller
/// than a step stays in `rest` for the next message. Positive means toward the top.
fn wheel_steps(rest: &mut i32, delta: i32, per_notch: i32) -> i32 {
    if delta == 0 {
        return 0;
    }
    if (*rest > 0 && delta < 0) || (*rest < 0 && delta > 0) {
        // The direction changed. What was left over belongs to the old direction.
        *rest = 0;
    }
    let total = *rest + delta * per_notch;
    *rest = total % 120;
    total / 120
}

fn move_edited_line(dir: i32, tab: bool) -> bool {
    let focus = unsafe { GetFocus() };
    if class_name(focus) != "Edit" {
        return false;
    }
    let parent = unsafe { GetParent(focus).unwrap_or(HWND(std::ptr::null_mut())) };
    if class_name(parent) == "SysListView32" {
        let index = list_focus_index(parent);
        if index < 0 {
            return false;
        }
        if let Some((list, next)) = filename_neighbor(parent, index, dir) {
            // Focusing the list closes the editor and keeps the typed text.
            unsafe { let _ = SetFocus(parent); }
            edit_filename(list, next);
            return true;
        }
        if tab {
            unsafe { let _ = SetFocus(parent); }
            return false;
        }
        return true;
    }
    let id = child_id(focus);
    if !(ID_TITLE_FIELD..ID_TITLE_SAME).contains(&id) || !enabled_visible(focus) {
        return false;
    }
    if let Some(next) = title_neighbor(focus, dir) {
        reveal_window(next);
        unsafe {
            let _ = SetFocus(next);
            SendMessageW(next, 0x00B1, WPARAM(0), LPARAM(-1));
        }
        return true;
    }
    if tab {
        return false;
    }
    true
}

fn filename_neighbor(list: HWND, index: i32, dir: i32) -> Option<(HWND, i32)> {
    let lists = filename_lists();
    let pos = lists.iter().position(|(hwnd, _)| *hwnd == list)?;
    let count = lists[pos].1;
    let next = index + dir;
    if next >= 0 && next < count {
        return Some((list, next));
    }
    if dir > 0 {
        lists.iter().skip(pos + 1).find(|(_, count)| *count > 0).map(|(hwnd, _)| (*hwnd, 0))
    } else {
        lists.iter().take(pos).rev().find(|(_, count)| *count > 0).map(|(hwnd, count)| (*hwnd, count - 1))
    }
}

fn filename_lists() -> Vec<(HWND, i32)> {
    tab_children()
        .into_iter()
        .filter(|hwnd| class_name(*hwnd) == "SysListView32" && (ID_LIST..ID_ALBUM).contains(&child_id(*hwnd)))
        .map(|hwnd| {
            let count = unsafe { SendMessageW(hwnd, LVM_GETITEMCOUNT, WPARAM(0), LPARAM(0)).0.max(0) as i32 };
            (hwnd, count)
        })
        .collect()
}

fn title_neighbor(current: HWND, dir: i32) -> Option<HWND> {
    let fields: Vec<HWND> = tab_children()
        .into_iter()
        .filter(|hwnd| class_name(*hwnd) == "Edit" && (ID_TITLE_FIELD..ID_TITLE_SAME).contains(&child_id(*hwnd)) && enabled_visible(*hwnd))
        .collect();
    let pos = fields.iter().position(|hwnd| *hwnd == current)?;
    let next = pos as i32 + dir;
    if next < 0 || next >= fields.len() as i32 {
        None
    } else {
        Some(fields[next as usize])
    }
}

fn tab_children() -> Vec<HWND> {
    let host = APP.with(|slot| slot.borrow().as_ref().map(|app| app.pane()).unwrap_or(HWND(std::ptr::null_mut())));
    if host.0.is_null() {
        return Vec::new();
    }
    let mut stops = Vec::new();
    unsafe {
        let mut child_hwnd = neighbor(host, GW_CHILD);
        while !child_hwnd.0.is_null() {
            stops.push(child_hwnd);
            child_hwnd = neighbor(child_hwnd, GW_HWNDNEXT);
        }
    }
    stops.reverse();
    stops
}

fn list_focus_index(list: HWND) -> i32 {
    // LVNI_FOCUSED. The start index is -1 so the search begins at the first row.
    unsafe { SendMessageW(list, 0x100C, WPARAM(usize::MAX), LPARAM(0x0001)).0 as i32 }
}

fn edit_filename(list: HWND, index: i32) {
    unsafe { SendMessageW(list, 0x1013, WPARAM(index as usize), LPARAM(0)); }
    if let Some(rect) = list_item_host_rect(list, index) {
        reveal_host(rect);
    }
    unsafe {
        let edit = SendMessageW(list, LVM_EDITLABELW, WPARAM(index as usize), LPARAM(0));
        let edit = HWND(edit.0 as *mut _);
        if !edit.0.is_null() {
            install_edit_keys(edit);
            SendMessageW(edit, 0x00B1, WPARAM(0), LPARAM(-1));
        }
    }
}

fn list_item_host_rect(list: HWND, index: i32) -> Option<RECT> {
    let host = APP.with(|slot| slot.borrow().as_ref().map(|app| app.host).unwrap_or(HWND(std::ptr::null_mut())));
    if host.0.is_null() {
        return None;
    }
    let mut rect = RECT::default();
    let ok = unsafe { SendMessageW(list, 0x100E, WPARAM(index as usize), LPARAM(&mut rect as *mut RECT as isize)) };
    if ok.0 == 0 {
        return None;
    }
    map_rect(list, host, &mut rect);
    Some(rect)
}

fn reveal_window(hwnd: HWND) {
    let host = APP.with(|slot| slot.borrow().as_ref().map(|app| app.host).unwrap_or(HWND(std::ptr::null_mut())));
    if host.0.is_null() || hwnd.0.is_null() {
        return;
    }
    let mut rect = RECT::default();
    unsafe { let _ = GetWindowRect(hwnd, &mut rect); }
    map_rect(HWND(std::ptr::null_mut()), host, &mut rect);
    reveal_host(rect);
}

fn map_rect(from: HWND, to: HWND, rect: &mut RECT) {
    let mut points = [POINT { x: rect.left, y: rect.top }, POINT { x: rect.right, y: rect.bottom }];
    unsafe { MapWindowPoints(from, to, &mut points); }
    rect.left = points[0].x;
    rect.top = points[0].y;
    rect.right = points[1].x;
    rect.bottom = points[1].y;
}

fn reveal_host(rect: RECT) {
    APP.with(|slot| {
        let Ok(mut borrow) = slot.try_borrow_mut() else {
            return;
        };
        let Some(app) = borrow.as_mut() else {
            return;
        };
        app.reveal_host_rect(rect);
    });
}

fn enabled_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowEnabled(hwnd).0 != 0 && IsWindowVisible(hwnd).0 != 0 }
}

fn shortcut(msg: &MSG) -> bool {
    if msg.message == WM_KEYDOWN {
        let key = msg.wParam.0;
        let shift = unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0;
        let ctrl = unsafe { GetKeyState(VK_CONTROL.0 as i32) } < 0;
        let alt = unsafe { GetKeyState(VK_MENU.0 as i32) } < 0;
        if !alt && edit_key(key, shift, ctrl, msg.hwnd) {
            return true;
        }
        let up = key == VK_UP.0 as usize;
        let down = key == VK_DOWN.0 as usize;
        let tab = key == VK_TAB.0 as usize;
        if tab && (ctrl || alt) {
            // Ctrl+Tab is not a page tab here. Swallow it so a text box does not get a tab character.
            return true;
        }
        // Shift or Ctrl with Up or Down belongs to the text box (select, or move the caret).
        if (up || down) && !shift && !ctrl && !alt && move_edited_line(if up { -1 } else { 1 }, false) {
            return true;
        }
        if tab && move_edited_line(if shift { -1 } else { 1 }, true) {
            return true;
        }
        if tab {
            focus_next(shift);
            return true;
        }
        if key == VK_RETURN.0 as usize && !ctrl && !alt {
            return activate_default();
        }
    }
    if msg.message == WM_CHAR && edit_char(msg.wParam.0, msg.hwnd) {
        return true;
    }
    if msg.message == 0x0106 {
        // WM_SYSCHAR
        let ch = (msg.wParam.0 as u8 as char).to_ascii_uppercase();
        return activate_mnemonic(ch);
    }
    false
}

/// The Remove, Insert and Find boxes of a Names or Titles tool row.
fn is_tool_field(id: i32) -> bool {
    (ID_REMOVE..ID_MODE).contains(&id) || (ID_FIND..ID_STRIP).contains(&id)
}

fn install_edit_keys(hwnd: HWND) {
    if hwnd.0.is_null() {
        return;
    }
    unsafe {
        let _ = SetWindowSubclass(hwnd, Some(edit_keys_proc), EDIT_KEYS_SUBCLASS, 0);
    }
}

unsafe extern "system" fn edit_keys_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_KEYDOWN {
        let shift = GetKeyState(VK_SHIFT.0 as i32) < 0;
        let ctrl = GetKeyState(VK_CONTROL.0 as i32) < 0;
        let alt = GetKeyState(VK_MENU.0 as i32) < 0;
        if !alt && apply_edit_key(hwnd, wparam.0, shift, ctrl) {
            return LRESULT(0);
        }
    }
    if msg == WM_CHAR && apply_edit_char(hwnd, wparam.0) {
        return LRESULT(0);
    }
    if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(edit_keys_proc), EDIT_KEYS_SUBCLASS);
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

fn is_edit_class(class: &str) -> bool {
    class.eq_ignore_ascii_case("Edit")
}

fn combo_edit(combo: HWND) -> HWND {
    let child = neighbor(combo, GW_CHILD);
    if is_edit_class(&class_name(child)) {
        child
    } else {
        HWND(std::ptr::null_mut())
    }
}

fn as_edit(hwnd: HWND) -> Option<HWND> {
    if hwnd.0.is_null() {
        return None;
    }
    let class = class_name(hwnd);
    if is_edit_class(&class) {
        return Some(hwnd);
    }
    if class.eq_ignore_ascii_case("ComboBox") {
        let inner = combo_edit(hwnd);
        if !inner.0.is_null() {
            return Some(inner);
        }
    }
    if class == "SysListView32" {
        let edit = unsafe { HWND(SendMessageW(hwnd, LVM_GETEDITCONTROL, WPARAM(0), LPARAM(0)).0 as *mut _) };
        if !edit.0.is_null() {
            return Some(edit);
        }
    }
    None
}

fn focused_edit(msg_hwnd: HWND) -> HWND {
    let focus = unsafe { GetFocus() };
    if let Some(edit) = as_edit(focus) {
        return edit;
    }
    if let Some(edit) = as_edit(msg_hwnd) {
        return edit;
    }
    HWND(std::ptr::null_mut())
}

/// Keys the classic text box gets wrong or does not know. Returns true when handled.
/// - Left and Right with a selection go to its start and its end, like current Windows.
///   The classic box moves one character from the caret instead, so after Tab selected a
///   whole name, both keys landed at the right.
/// - Ctrl+A selects everything. The classic box ignores it and beeps.
/// - Ctrl+Backspace deletes the word before the caret. The classic box types a box
///   character instead. The deletion can be undone with Ctrl+Z.
/// - Ctrl+Delete deletes the word after the caret.
fn edit_key(key: usize, shift: bool, ctrl: bool, msg_hwnd: HWND) -> bool {
    let hwnd = focused_edit(msg_hwnd);
    if hwnd.0.is_null() {
        return false;
    }
    apply_edit_key(hwnd, key, shift, ctrl)
}

fn edit_char(ch: usize, msg_hwnd: HWND) -> bool {
    let hwnd = focused_edit(msg_hwnd);
    if hwnd.0.is_null() {
        // A closed drop-down still gets Ctrl+A and would beep.
        return ch == 1 || ch == 127;
    }
    apply_edit_char(hwnd, ch)
}

fn apply_edit_char(hwnd: HWND, ch: usize) -> bool {
    if ch == 1 {
        apply_edit_key(hwnd, b'A' as usize, false, true);
        return true;
    }
    if ch == 127 {
        return true;
    }
    false
}

fn apply_edit_key(hwnd: HWND, key: usize, shift: bool, ctrl: bool) -> bool {
    let left = key == VK_LEFT.0 as usize;
    let right = key == VK_RIGHT.0 as usize;
    if (left || right) && !shift && !ctrl {
        let (start, end) = edit_selection(hwnd);
        if start == end {
            return false;
        }
        let at = if left { start } else { end };
        unsafe {
            SendMessageW(hwnd, 0x00B1, WPARAM(at), LPARAM(at as isize));
            // EM_SCROLLCARET
            SendMessageW(hwnd, 0x00B7, WPARAM(0), LPARAM(0));
        }
        return true;
    }
    if key == b'A' as usize && ctrl && !shift {
        unsafe { SendMessageW(hwnd, 0x00B1, WPARAM(0), LPARAM(-1)); }
        return true;
    }
    if (key == VK_BACK.0 as usize || key == VK_DELETE.0 as usize) && ctrl {
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 };
        if style & 0x0800 != 0 {
            // ES_READONLY
            return true;
        }
        let (start, end) = edit_selection(hwnd);
        if start == end {
            let text: Vec<u16> = get_text(hwnd).encode_utf16().collect();
            let caret = end.min(text.len());
            let (from, to) = if key == VK_BACK.0 as usize {
                (word_start_before(&text, caret), caret)
            } else {
                (caret, word_end_after(&text, caret))
            };
            if from == to {
                return true;
            }
            unsafe { SendMessageW(hwnd, 0x00B1, WPARAM(from), LPARAM(to as isize)); }
        }
        let empty = [0u16; 1];
        unsafe {
            // EM_REPLACESEL with undo allowed.
            SendMessageW(hwnd, 0x00C2, WPARAM(1), LPARAM(empty.as_ptr() as isize));
        }
        return true;
    }
    false
}

/// EM_GETSEL: start and end of the selection, in UTF-16 units.
fn edit_selection(edit: HWND) -> (usize, usize) {
    let mut start = 0u32;
    let mut end = 0u32;
    unsafe {
        SendMessageW(edit, 0x00B0, WPARAM(&mut start as *mut u32 as usize), LPARAM(&mut end as *mut u32 as isize));
    }
    let (start, end) = (start as usize, end as usize);
    (start.min(end), start.max(end))
}

/// Where Ctrl+Backspace starts deleting: spaces before the caret, then one run of word
/// characters or one run of punctuation. A line break before the caret is deleted on its own.
fn word_start_before(text: &[u16], caret: usize) -> usize {
    let caret = caret.min(text.len());
    let at = |i: usize| text[i];
    let newline = |c: u16| c == b'\n' as u16 || c == b'\r' as u16;
    let space = |c: u16| c == b' ' as u16 || c == b'\t' as u16 || c == 0x00A0 || c == 0x3000;
    let word = |c: u16| {
        if (0xD800..0xE000).contains(&c) {
            return true;
        }
        char::from_u32(c as u32).map(|ch| ch.is_alphanumeric() || ch == '_' || ch == '\'').unwrap_or(true)
    };
    let mut i = caret;
    if i > 0 && newline(at(i - 1)) {
        i -= 1;
        if at(i) == b'\n' as u16 && i > 0 && at(i - 1) == b'\r' as u16 {
            i -= 1;
        }
        return i;
    }
    while i > 0 && space(at(i - 1)) {
        i -= 1;
    }
    if i == 0 || newline(at(i - 1)) {
        return i;
    }
    let kind = word(at(i - 1));
    while i > 0 && !space(at(i - 1)) && !newline(at(i - 1)) && word(at(i - 1)) == kind {
        i -= 1;
    }
    i
}

/// Where Ctrl+Delete stops: spaces after the caret, then one run of word characters or
/// one run of punctuation. A line break after the caret is deleted on its own.
fn word_end_after(text: &[u16], caret: usize) -> usize {
    let caret = caret.min(text.len());
    let at = |i: usize| text[i];
    let newline = |c: u16| c == b'\n' as u16 || c == b'\r' as u16;
    let space = |c: u16| c == b' ' as u16 || c == b'\t' as u16 || c == 0x00A0 || c == 0x3000;
    let word = |c: u16| {
        if (0xD800..0xE000).contains(&c) {
            return true;
        }
        char::from_u32(c as u32).map(|ch| ch.is_alphanumeric() || ch == '_' || ch == '\'').unwrap_or(true)
    };
    let mut i = caret;
    if i < text.len() && newline(at(i)) {
        if at(i) == b'\r' as u16 && i + 1 < text.len() && at(i + 1) == b'\n' as u16 {
            return i + 2;
        }
        return i + 1;
    }
    while i < text.len() && space(at(i)) {
        i += 1;
    }
    if i == text.len() || newline(at(i)) {
        return i;
    }
    let kind = word(at(i));
    while i < text.len() && !space(at(i)) && !newline(at(i)) && word(at(i)) == kind {
        i += 1;
    }
    i
}

fn focus_next(shift: bool) {
    APP.with(|slot| {
        let borrow = slot.borrow();
        let Some(app) = borrow.as_ref() else {
            return;
        };
        let mut stops = Vec::new();
        unsafe {
            let mut child_hwnd = neighbor(app.pane(), GW_CHILD);
            while !child_hwnd.0.is_null() {
                let style = GetWindowLongPtrW(child_hwnd, GWL_STYLE) as u32;
                if style & 0x00010000 != 0 && IsWindowVisible(child_hwnd).0 != 0 && IsWindowEnabled(child_hwnd).0 != 0 {
                    stops.push(child_hwnd);
                }
                child_hwnd = neighbor(child_hwnd, GW_HWNDNEXT);
            }
        }
        // GetWindow walks top-to-bottom z-order, which is reverse creation. Flip it.
        stops.reverse();
        if unsafe { IsWindowEnabled(app.back).0 != 0 } {
            stops.push(app.back);
        }
        if unsafe { IsWindowEnabled(app.next).0 != 0 } {
            stops.push(app.next);
        }
        if stops.is_empty() {
            return;
        }
        let focus = unsafe { GetFocus() };
        let current = stops.iter().position(|hwnd| *hwnd == focus);
        let next = match current {
            Some(index) if shift => (index + stops.len() - 1) % stops.len(),
            Some(index) => (index + 1) % stops.len(),
            None => 0,
        };
        unsafe { let _ = SetFocus(stops[next]); }
    });
}

fn activate_default() -> bool {
    let focus = unsafe { GetFocus() };
    let class = class_name(focus);
    if class == "Edit" && is_tool_field(child_id(focus)) {
        // Enter in Remove, Insert or Find applies that row, the same as its Apply button.
        let parent = unsafe { GetParent(focus).unwrap_or(HWND(std::ptr::null_mut())) };
        if let Ok(apply) = child(parent, ID_APPLY + group_of(child_id(focus)) as i32) {
            if enabled_visible(apply) {
                unsafe {
                    SendMessageW(apply, 0x00F5, WPARAM(0), LPARAM(0));
                    // A click moves focus to the button. Keep typing in the box instead.
                    let _ = SetFocus(focus);
                }
            }
        }
        return true;
    }
    if class == "Edit" {
        let style = unsafe { GetWindowLongPtrW(focus, GWL_STYLE) as u32 };
        if style & 0x1000 != 0 {
            return false;
        }
        let parent = class_name(unsafe { windows::Win32::UI::WindowsAndMessaging::GetParent(focus).unwrap_or(HWND(std::ptr::null_mut())) });
        if parent == "SysListView32" {
            return false;
        }
    }
    if class == "ComboBox" {
        let dropped = unsafe { SendMessageW(focus, 0x0157, WPARAM(0), LPARAM(0)).0 };
        if dropped != 0 {
            return false;
        }
    }
    if class == "Button" {
        unsafe { SendMessageW(focus, 0x00F5, WPARAM(0), LPARAM(0)); }
        return true;
    }
    APP.with(|slot| {
        let borrow = slot.borrow();
        let Some(app) = borrow.as_ref() else {
            return;
        };
        if unsafe { IsWindowEnabled(app.next).0 != 0 } {
            unsafe { SendMessageW(app.next, 0x00F5, WPARAM(0), LPARAM(0)); }
        }
    });
    true
}

fn activate_mnemonic(ch: char) -> bool {
    let targets = APP.with(|slot| {
        let borrow = slot.borrow();
        let Some(app) = borrow.as_ref() else {
            return Vec::new();
        };
        let mut found = Vec::new();
        collect_mnemonics(app.pane(), ch, &mut found);
        collect_mnemonics(app.hwnd, ch, &mut found);
        found
    });
    let Some(target) = targets.into_iter().find(|hwnd| unsafe { IsWindowEnabled(*hwnd).0 != 0 && IsWindowVisible(*hwnd).0 != 0 }) else {
        return false;
    };
    let class = class_name(target);
    unsafe { let _ = SetFocus(target); }
    if class == "Button" {
        unsafe { SendMessageW(target, 0x00F5, WPARAM(0), LPARAM(0)); }
    }
    true
}

fn collect_mnemonics(parent: HWND, ch: char, out: &mut Vec<HWND>) {
    unsafe {
        let mut child_hwnd = neighbor(parent, GW_CHILD);
        while !child_hwnd.0.is_null() {
            let text = get_text(child_hwnd);
            if accel_char(&text) == Some(ch) {
                let class = class_name(child_hwnd);
                if class == "Static" {
                    // The next tab stop is the control this label names. Creation order was reversed.
                } else {
                    out.push(child_hwnd);
                }
            }
            child_hwnd = neighbor(child_hwnd, GW_HWNDNEXT);
        }
    }
    if out.is_empty() {
        // Static label: focus the following tab stop, searched in creation order.
        let mut stops = Vec::new();
        unsafe {
            let mut child_hwnd = neighbor(parent, GW_CHILD);
            while !child_hwnd.0.is_null() {
                stops.push(child_hwnd);
                child_hwnd = neighbor(child_hwnd, GW_HWNDNEXT);
            }
        }
        stops.reverse();
        for (index, hwnd) in stops.iter().enumerate() {
            if accel_char(&get_text(*hwnd)) == Some(ch) && class_name(*hwnd) == "Static" {
                if let Some(next) = stops.iter().skip(index + 1).find(|candidate| {
                    let style = unsafe { GetWindowLongPtrW(**candidate, GWL_STYLE) as u32 };
                    style & 0x00010000 != 0
                }) {
                    out.push(*next);
                }
            }
        }
    }
}

fn enable(hwnd: HWND, value: bool) {
    unsafe { let _ = EnableWindow(hwnd, BOOL(i32::from(value))); }
}

fn neighbor(hwnd: HWND, cmd: windows::Win32::UI::WindowsAndMessaging::GET_WINDOW_CMD) -> HWND {
    unsafe { GetWindow(hwnd, cmd).unwrap_or(HWND(std::ptr::null_mut())) }
}

fn accel_char(text: &str) -> Option<char> {
    let chars: Vec<char> = text.chars().collect();
    for (index, ch) in chars.iter().enumerate() {
        if *ch == '&' && index + 1 < chars.len() && chars[index + 1] != '&' {
            return Some(chars[index + 1].to_ascii_uppercase());
        }
    }
    None
}

fn insert_index(at: InsertAt) -> usize {
    match at {
        InsertAt::Start => 0,
        InsertAt::End => 1,
        InsertAt::Before => 2,
        InsertAt::After => 3,
    }
}

fn flat_start(groups: &[crate::model::Group], group: usize) -> usize {
    groups.iter().take(group).map(|group| group.items.len()).sum()
}

fn locate_flat(groups: &[crate::model::Group], flat: usize) -> Option<(usize, usize)> {
    let mut seen = 0usize;
    for (group_index, group) in groups.iter().enumerate().take(GROUP_LIMIT) {
        if flat < seen + group.items.len() {
            return Some((group_index, flat - seen));
        }
        seen += group.items.len();
    }
    None
}

fn group_of(id: i32) -> usize {
    let base = match id {
        id if (ID_REMOVE..ID_INSERT).contains(&id) => ID_REMOVE,
        id if (ID_INSERT..ID_MODE).contains(&id) => ID_INSERT,
        id if (ID_MODE..ID_FIND).contains(&id) => ID_MODE,
        id if (ID_FIND..ID_STRIP).contains(&id) => ID_FIND,
        id if (ID_STRIP..ID_APPLY).contains(&id) => ID_STRIP,
        id if (ID_APPLY..ID_LIST).contains(&id) => ID_APPLY,
        id if (ID_LIST..ID_ALBUM).contains(&id) => ID_LIST,
        id if (ID_ALBUM..ID_ARTIST).contains(&id) => ID_ALBUM,
        _ => id,
    };
    (id - base) as usize
}

fn button_bits(default: bool) -> u32 {
    // Child, visible, tab stop, clip siblings. The last bit is the default-button style.
    0x40000000 | 0x10000000 | 0x10000 | 0x04000000 | if default { 1 } else { 0 }
}

fn parse_u32(text: &str, fallback: u32) -> u32 {
    text.trim().parse().unwrap_or(fallback)
}

fn set_id(hwnd: HWND, id: i32) {
    unsafe { SetWindowLongPtrW(hwnd, GWLP_ID, id as isize); }
}

fn child_id(hwnd: HWND) -> i32 {
    unsafe { GetWindowLongPtrW(hwnd, GWLP_ID) as i32 }
}

fn child(parent: HWND, id: i32) -> windows::core::Result<HWND> {
    unsafe { GetDlgItem(parent, id) }
}

fn text_of(parent: HWND, id: i32) -> String {
    child(parent, id).map(get_text).unwrap_or_default()
}

fn checked_id(parent: HWND, id: i32) -> bool {
    child(parent, id).map(checked).unwrap_or(false)
}

fn combo_sel_id(parent: HWND, id: i32) -> usize {
    child(parent, id).map(combo_sel).unwrap_or(0)
}

fn get_text(hwnd: HWND) -> String {
    if hwnd.0.is_null() {
        return String::new();
    }
    unsafe {
        let len = SendMessageW(hwnd, WM_GETTEXTLENGTH, WPARAM(0), LPARAM(0)).0;
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len as usize + 1];
        SendMessageW(hwnd, WM_GETTEXT, WPARAM(buf.len()), LPARAM(buf.as_mut_ptr() as isize));
        let end = buf.iter().position(|ch| *ch == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }
}

fn set_text(hwnd: HWND, text: &str) {
    if hwnd.0.is_null() || get_text(hwnd) == text {
        return;
    }
    let wide = wide(text);
    unsafe { let _ = SetWindowTextW(hwnd, PCWSTR(wide.as_ptr())); }
}

fn set_enabled(hwnd: HWND, on: bool) {
    if hwnd.0.is_null() {
        return;
    }
    let enabled = unsafe { IsWindowEnabled(hwnd).0 != 0 };
    if enabled != on {
        enable(hwnd, on);
    }
}

fn combo_add(hwnd: HWND, text: &str) {
    let wide = wide(text);
    unsafe { SendMessageW(hwnd, CB_ADDSTRING, WPARAM(0), LPARAM(wide.as_ptr() as isize)); }
}

fn combo_clear(hwnd: HWND) {
    unsafe { SendMessageW(hwnd, CB_RESETCONTENT, WPARAM(0), LPARAM(0)); }
}

fn combo_sel(hwnd: HWND) -> usize {
    let sel = unsafe { SendMessageW(hwnd, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 };
    if sel < 0 { 0 } else { sel as usize }
}

fn set_combo(hwnd: HWND, index: usize) {
    unsafe { SendMessageW(hwnd, CB_SETCURSEL, WPARAM(index), LPARAM(0)); }
}

fn checked(hwnd: HWND) -> bool {
    unsafe { SendMessageW(hwnd, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1 }
}

fn set_check(hwnd: HWND, on: bool) {
    unsafe { SendMessageW(hwnd, BM_SETCHECK, WPARAM(if on { 1 } else { 0 }), LPARAM(0)); }
}

fn pw_string(text: PWSTR) -> String {
    if text.0.is_null() {
        return String::new();
    }
    unsafe {
        let mut len = 0usize;
        while *text.0.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(text.0, len))
    }
}

fn class_name(hwnd: HWND) -> String {
    if hwnd.0.is_null() {
        return String::new();
    }
    let mut buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    if len <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

fn client_of(hwnd: HWND) -> (i32, i32) {
    let mut rect = RECT::default();
    unsafe { let _ = GetClientRect(hwnd, &mut rect); }
    (rect.right - rect.left, rect.bottom - rect.top)
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn rgb(r: u32, g: u32, b: u32) -> COLORREF {
    COLORREF(r | (g << 8) | (b << 16))
}

fn white_brush() -> HBRUSH {
    WHITE.get()
}

pub(crate) fn make_font(pt: i32, dpi: i32, bold: bool) -> HFONT {
    unsafe {
        CreateFontW(
            -((pt * dpi + 36) / 72),
            0,
            0,
            0,
            if bold { 700 } else { 400 },
            0,
            0,
            0,
            1,
            0,
            0,
            0,
            0,
            w!("MS Sans Serif"),
        )
    }
}

fn font_bits(font: HFONT) -> usize {
    font.0 as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mouse_notch_moves_a_whole_step() {
        let mut rest = 0;
        assert_eq!(wheel_steps(&mut rest, 120, 36), 36);
        assert_eq!(rest, 0);
        assert_eq!(wheel_steps(&mut rest, -240, 3), -6);
        assert_eq!(rest, 0);
    }

    #[test]
    fn small_touchpad_deltas_add_up() {
        let mut rest = 0;
        let mut lines = 0;
        for _ in 0..8 {
            lines += wheel_steps(&mut rest, 15, 3);
        }
        // Eight deltas of 15 are one notch, so three lines in total.
        assert_eq!(lines, 3);
        assert_eq!(rest, 0);
        let mut pixels = 0;
        rest = 0;
        for _ in 0..4 {
            pixels += wheel_steps(&mut rest, -10, 36);
        }
        assert_eq!(pixels, -12);
    }

    #[test]
    fn a_direction_change_drops_the_leftover() {
        let mut rest = 0;
        assert_eq!(wheel_steps(&mut rest, 30, 3), 0);
        assert_eq!(rest, 90);
        assert_eq!(wheel_steps(&mut rest, -30, 3), 0);
        assert_eq!(rest, -90);
        assert_eq!(wheel_steps(&mut rest, 0, 3), 0);
        assert_eq!(rest, -90);
    }

    #[test]
    fn scroll_limits_follow_the_bar() {
        // Ten lines, four visible, so positions 0 to 6.
        assert!(!can_scroll(0, 9, 4, 0, true));
        assert!(can_scroll(0, 9, 4, 0, false));
        assert!(can_scroll(0, 9, 4, 3, true));
        assert!(can_scroll(0, 9, 4, 3, false));
        assert!(can_scroll(0, 9, 4, 6, true));
        assert!(!can_scroll(0, 9, 4, 6, false));
        // Everything fits, so there is nowhere to go.
        assert!(!can_scroll(0, 2, 4, 0, true));
        assert!(!can_scroll(0, 2, 4, 0, false));
        assert!(!can_scroll(0, 0, 0, 0, false));
    }

    #[test]
    fn only_a_control_that_can_scroll_keeps_the_wheel() {
        assert!(!wheel_to_control("Edit", false, true));
        assert!(!wheel_to_control("Edit", false, false));
        assert!(wheel_to_control("Edit", true, true));
        assert!(!wheel_to_control("Edit", true, false));
        assert!(!wheel_to_control("ComboBox", false, true));
        assert!(wheel_to_control("ComboLBox", false, false));
        for class in ["ListBox", "LinkWorkList", "SysListView32"] {
            assert!(wheel_to_control(class, false, true));
            assert!(!wheel_to_control(class, false, false));
        }
        for class in ["Static", "Button", "SysHeader32", "LinkDownloadHost", ""] {
            assert!(!wheel_to_control(class, false, true));
        }
    }

    #[test]
    fn a_name_that_fits_is_drawn_whole() {
        assert_eq!(fit_count(&[5, 5, 5], 15, 6), 3);
        assert_eq!(fit_count(&[], 0, 6), 0);
    }

    #[test]
    fn a_long_name_leaves_room_for_the_dots() {
        // 4 characters of 5 do not fit in 18, so as many as fit beside 6 for the dots.
        assert_eq!(fit_count(&[5, 5, 5, 5], 18, 6), 2);
        assert_eq!(fit_count(&[5, 9, 5], 10, 6), 0);
    }

    #[test]
    fn marks_share_the_plain_baseline_inside_the_row() {
        // A 17 pixel row, 13 pixel plain text (ascent 11), 17 pixel bold (ascent 14).
        // Centered plain text would put the bold top 1 pixel above the row, so it moves down 1.
        let baseline = marked_baseline(0, 17, 13, 11, 17, 14);
        assert_eq!(baseline, 2 + 11 + 1);
        assert!(baseline - 14 >= 0 && baseline + 3 <= 17);
        // With room to spare nothing moves.
        assert_eq!(marked_baseline(0, 20, 13, 11, 16, 13), 3 + 11);
        // Plain text only: centered, the same as the list.
        assert_eq!(marked_baseline(10, 24, 13, 11, 13, 11), 10 + 0 + 11);
        // A bold font taller than the room keeps its top inside.
        assert_eq!(marked_baseline(0, 14, 13, 11, 16, 13), 13);
    }

    #[test]
    fn the_biggest_mark_font_that_fits_is_used() {
        // 12, 10 and 8 pt at 96 DPI.
        let heights = [16, 13, 13];
        assert_eq!(pick_mark(&heights, 18), 0);
        assert_eq!(pick_mark(&heights, 16), 0);
        assert_eq!(pick_mark(&heights, 14), 1);
        // At 120 DPI: 20, 17 and 13 pixels in a 15 pixel row.
        assert_eq!(pick_mark(&[20, 17, 13], 15), 2);
        assert_eq!(pick_mark(&[20, 17, 13], 10), 2);
    }

    #[test]
    fn row_colors_follow_the_classic_list() {
        assert_eq!(row_colors(true, true, true), (COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT));
        assert_eq!(row_colors(true, false, true), (COLOR_BTNFACE, COLOR_BTNTEXT));
        assert_eq!(row_colors(false, true, true), (COLOR_WINDOW, COLOR_WINDOWTEXT));
        assert_eq!(row_colors(false, false, true), (COLOR_WINDOW, COLOR_WINDOWTEXT));
        assert_eq!(row_colors(true, true, false), (COLOR_WINDOW, COLOR_GRAYTEXT));
    }
}

#[cfg(test)]
mod live_name_tests {
    use super::LiveEdit;

    #[test]
    fn the_first_keystroke_keeps_the_old_name() {
        let mut slot = None;
        LiveEdit::note(&mut slot, 0, 2, "Old name");
        LiveEdit::note(&mut slot, 0, 2, "Old nam");
        assert_eq!(slot, Some(LiveEdit { group: 0, index: 2, original: "Old name".into() }));
    }

    #[test]
    fn another_row_starts_a_new_edit() {
        let mut slot = None;
        LiveEdit::note(&mut slot, 0, 2, "First");
        LiveEdit::note(&mut slot, 1, 0, "Second");
        assert_eq!(slot, Some(LiveEdit { group: 1, index: 0, original: "Second".into() }));
    }

    #[test]
    fn esc_gives_back_the_old_name_once() {
        let mut slot = None;
        LiveEdit::note(&mut slot, 0, 2, "Old name");
        assert_eq!(LiveEdit::cancel(&mut slot, 0, 2), Some("Old name".into()));
        assert_eq!(slot, None);
        assert_eq!(LiveEdit::cancel(&mut slot, 0, 2), None);
    }

    #[test]
    fn esc_on_a_row_that_was_not_typed_in_changes_nothing() {
        let mut slot = None;
        LiveEdit::note(&mut slot, 0, 2, "Old name");
        assert_eq!(LiveEdit::cancel(&mut slot, 0, 3), None);
        assert_eq!(LiveEdit::cancel(&mut slot, 0, 2), None);
    }
}

#[cfg(test)]
mod text_key_tests {
    use super::{
        apply_edit_char, is_edit_class, is_tool_field, word_end_after, word_start_before, ID_FIND, ID_INSERT,
        ID_LINKS, ID_LOCATION, ID_MODE, ID_NAME, ID_REMOVE, ID_STRIP, ID_TITLE_FIELD,
    };
    use windows::Win32::Foundation::HWND;

    fn start(text: &str, caret: usize) -> usize {
        let wide: Vec<u16> = text.encode_utf16().collect();
        word_start_before(&wide, caret)
    }

    fn end(text: &str, caret: usize) -> usize {
        let wide: Vec<u16> = text.encode_utf16().collect();
        word_end_after(&wide, caret)
    }

    #[test]
    fn ctrl_backspace_deletes_one_word() {
        assert_eq!(start("hello world", 11), 6);
        assert_eq!(start("hello world  ", 13), 6);
        assert_eq!(start("hello", 5), 0);
        assert_eq!(start("hello world", 8), 6);
        assert_eq!(start("", 0), 0);
        assert_eq!(start("   ", 3), 0);
    }

    #[test]
    fn punctuation_is_its_own_word() {
        assert_eq!(start("name?? ", 7), 4);
        assert_eq!(start("Song - Artist", 6), 5);
        assert_eq!(start("C:\\Music", 8), 3);
        assert_eq!(start("don't", 5), 0);
    }

    #[test]
    fn a_line_break_goes_by_itself() {
        assert_eq!(start("one\r\ntwo", 5), 3);
        assert_eq!(start("one\r\n", 5), 3);
        assert_eq!(start("one\r\n  two", 10), 7);
        assert_eq!(start("one\r\n  ", 7), 5);
    }

    #[test]
    fn a_caret_past_the_end_is_clamped() {
        assert_eq!(start("abc", 99), 0);
    }

    #[test]
    fn only_remove_insert_and_find_apply_on_enter() {
        assert!(is_tool_field(ID_REMOVE));
        assert!(is_tool_field(ID_INSERT + 3));
        assert!(is_tool_field(ID_FIND + 2));
        assert!(!is_tool_field(ID_MODE));
        assert!(!is_tool_field(ID_STRIP));
        assert!(!is_tool_field(22000));
    }

    #[test]
    fn ctrl_a_is_for_every_text_box() {
        assert!(!is_tool_field(ID_LOCATION));
        assert!(!is_tool_field(ID_NAME));
        assert!(!is_tool_field(ID_LINKS));
        assert!(!is_tool_field(ID_TITLE_FIELD));
        assert!(is_edit_class("Edit"));
        assert!(is_edit_class("EDIT"));
    }

    #[test]
    fn ctrl_delete_deletes_one_word() {
        assert_eq!(end("hello world", 0), 5);
        assert_eq!(end("hello world", 5), 11);
        assert_eq!(end("hello  world", 5), 12);
        assert_eq!(end("hello", 5), 5);
        assert_eq!(end("", 0), 0);
    }

    #[test]
    fn ctrl_delete_treats_punctuation_and_breaks_like_backspace() {
        assert_eq!(end("name?? ", 4), 6);
        assert_eq!(end("Song - Artist", 4), 6);
        assert_eq!(end("one\r\ntwo", 3), 5);
        assert_eq!(end("abc", 99), 3);
    }

    #[test]
    fn leftover_ctrl_characters_do_not_beep() {
        let none = HWND(std::ptr::null_mut());
        assert!(apply_edit_char(none, 127));
        // Ctrl+C / Ctrl+V / Ctrl+Z still go to the box.
        assert!(!apply_edit_char(none, 3));
        assert!(!apply_edit_char(none, 22));
        assert!(!apply_edit_char(none, 26));
    }
}

#[cfg(test)]
mod typing_popup_tests {
    use super::{popup_for, CBN_SELCHANGE, EN_CHANGE, EN_KILLFOCUS, ID_AUDIO, ID_LOCATION, ID_MAX, ID_MIN, ID_RES};

    #[test]
    fn typing_in_a_box_never_pops_up() {
        for id in [ID_MIN, ID_MAX, ID_LOCATION] {
            assert!(!popup_for(id, EN_CHANGE as u16, true));
            assert!(!popup_for(id, EN_CHANGE as u16, false));
        }
    }

    #[test]
    fn leaving_a_box_for_another_control_explains() {
        for id in [ID_MIN, ID_MAX, ID_LOCATION] {
            assert!(popup_for(id, EN_KILLFOCUS as u16, true));
            // Alt+Tab to another program is not "done with the box".
            assert!(!popup_for(id, EN_KILLFOCUS as u16, false));
        }
    }

    #[test]
    fn drop_downs_and_check_boxes_still_explain_at_once() {
        assert!(popup_for(ID_RES, CBN_SELCHANGE as u16, true));
        assert!(popup_for(ID_AUDIO, 0, true));
    }
}

#[cfg(test)]
mod layout_tests {
    use super::{boxes_overlap, fill_page_height, layout_page, Anchor, BoxSpec};

    fn spec(x: i32, y: i32, w: i32, h: i32, anchor: Anchor, right: i32, bottom: i32) -> BoxSpec {
        BoxSpec { x, y, w, h, anchor, right, bottom }
    }

    fn download_page() -> Vec<BoxSpec> {
        // 640x480 client minus 46px header and footer: view 388, list above a 78px readout strip.
        vec![
            spec(8, 8, 624, 18, Anchor::StretchX, 8, 362),
            spec(8, 30, 624, 280, Anchor::Fill, 8, 78),
            spec(8, 318, 570, 16, Anchor::StretchX, 62, 54),
            spec(582, 318, 50, 16, Anchor::Right, 8, 54),
            spec(8, 340, 72, 16, Anchor::Fixed, 560, 32),
            spec(80, 340, 88, 16, Anchor::Fixed, 472, 32),
            spec(8, 358, 72, 16, Anchor::Fixed, 560, 14),
            spec(80, 358, 88, 16, Anchor::Fixed, 472, 14),
        ]
    }

    #[test]
    fn growing_the_window_keeps_the_list_above_the_readouts() {
        let page = download_page();
        let placed = layout_page(&page, 640, 500);
        let list = placed[1];
        assert_eq!(list, (8, 30, 624, 392));
        for footer in &placed[2..] {
            assert!(!boxes_overlap(list, *footer), "list {:?} covers {:?}", list, footer);
            assert!(footer.1 + footer.3 <= 500);
        }
        assert_eq!(placed[2].1, 430);
        assert_eq!(placed[4].1, 452);
        assert_eq!(placed[6].1, 470);
    }

    #[test]
    fn shrinking_the_window_keeps_the_readouts_on_the_page() {
        let page = download_page();
        let placed = layout_page(&page, 640, 300);
        let list = placed[1];
        assert_eq!(list.3, 192);
        for footer in &placed[2..] {
            assert!(!boxes_overlap(list, *footer));
            assert!(footer.1 >= 0);
            assert!(footer.1 + footer.3 <= 300);
        }
        assert_eq!(placed[2].1, 230);
        assert_eq!(placed[6].1, 270);
    }

    #[test]
    fn options_controls_stay_put_when_there_is_no_fill() {
        let page = vec![
            spec(124, 10, 200, 21, Anchor::Fixed, 316, 357),
            spec(124, 330, 400, 21, Anchor::StretchX, 116, 37),
            spec(540, 330, 75, 23, Anchor::Right, 25, 35),
        ];
        let grown = layout_page(&page, 800, 500);
        assert_eq!(grown[0], (124, 10, 200, 21));
        assert_eq!(grown[1], (124, 330, 560, 21));
        assert_eq!(grown[2], (700, 330, 75, 23));
    }

    #[test]
    fn the_links_error_stays_under_the_edit() {
        let page = vec![
            spec(8, 10, 624, 32, Anchor::Fixed, 8, 346),
            spec(8, 46, 624, 302, Anchor::Fill, 8, 40),
            spec(8, 352, 624, 32, Anchor::Fixed, 8, 4),
        ];
        let placed = layout_page(&page, 800, 500);
        assert_eq!(placed[1], (8, 46, 784, 414));
        assert_eq!(placed[2], (8, 464, 624, 32));
        assert!(!boxes_overlap(placed[1], placed[2]));
        assert_eq!(placed[1].1 + placed[1].3 + 4, placed[2].1);
    }

    #[test]
    fn a_fill_page_uses_the_view_height() {
        assert_eq!(fill_page_height(true, 500, 388), 500);
        assert_eq!(fill_page_height(false, 500, 388), 388);
    }

    #[test]
    fn the_page_view_sits_between_the_header_and_the_button_bar() {
        // 480 client, 3px border, 18px caption, 46px header, 46px Back/Next bar.
        assert_eq!(super::chrome_view_h(477, 21, 46, 46), 364);
        // Counting only the header and bar would leave the last rows under the bar.
        assert!(super::chrome_view_h(477, 21, 46, 46) < 480 - 46 - 46);
    }
}
