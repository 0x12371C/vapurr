//! Vapurr-branded native authorize dialog (Win32). Outside HTML/renderer control.
use std::cell::Cell;
use std::sync::OnceLock;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, GetDC,
    InvalidateRect, ReleaseDC, SelectObject, SetBkMode, SetTextColor, CLEARTYPE_QUALITY,
    CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DRAW_TEXT_FORMAT, DT_CALCRECT, DT_CENTER,
    DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, FW_NORMAL,
    FW_SEMIBOLD, HBRUSH, HDC, HFONT, OUT_TT_PRECIS, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_RETURN};
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow, DispatchMessageW,
    DrawIconEx, GetClientRect, GetForegroundWindow, GetMessageW, GetSystemMetrics, GetWindowLongPtrW,
    GetWindowThreadProcessId, IsWindow, LoadCursorW, LoadImageW, MessageBoxW, RegisterClassExW,
    SetCursor, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage,
    CS_HREDRAW, CS_VREDRAW, DI_NORMAL, GWLP_USERDATA, HCURSOR, HICON, HWND_TOPMOST, IDC_ARROW,
    IDC_HAND, IDYES, IMAGE_ICON, LR_DEFAULTSIZE, MB_DEFBUTTON2, MB_ICONWARNING, MB_TOPMOST,
    MB_YESNO, MSG, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, SWP_NOZORDER, WINDOW_EX_STYLE, WM_CLOSE,
    WM_DESTROY, WM_KEYDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_SETCURSOR, WNDCLASSEXW,
    WS_CAPTION, WS_POPUP, WS_SYSMENU,
};

const VOID: COLORREF = COLORREF(0x000E_0E0E);
const LIME: COLORREF = COLORREF(0x0000_F8C0);
const STEEL: COLORREF = COLORREF(0x0027_231F);
const SNOW: COLORREF = COLORREF(0x00F4_F3F2);
const MUTED: COLORREF = COLORREF(0x0090_A08A);
const ON_LIME: COLORREF = COLORREF(0x000E_0E0E);
const STEEL_EDGE: COLORREF = COLORREF(0x0033_373B);

const ID_REJECT: isize = 1;
const ID_AUTHORIZE: isize = 2;

struct State {
    description: Vec<u16>,
    decided: Cell<Option<bool>>,
    icon: HICON,
    font_kicker: HFONT,
    font_title: HFONT,
    font_body: HFONT,
    font_btn: HFONT,
    brush_void: HBRUSH,
    brush_steel: HBRUSH,
    brush_lime: HBRUSH,
    reject: RECT,
    auth: RECT,
    hover: Cell<isize>,
    dpi: u32,
}

impl State {
    unsafe fn drop_gdi(&self) {
        let _ = DeleteObject(self.font_kicker);
        let _ = DeleteObject(self.font_title);
        let _ = DeleteObject(self.font_body);
        let _ = DeleteObject(self.font_btn);
        let _ = DeleteObject(self.brush_void);
        let _ = DeleteObject(self.brush_steel);
        let _ = DeleteObject(self.brush_lime);
        if !self.icon.is_invalid() {
            let _ = DestroyIcon(self.icon);
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn scale(dpi: u32, px: i32) -> i32 {
    (i64::from(px) * i64::from(dpi) / 96) as i32
}

fn pt_in(r: &RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

fn class_atom() -> u16 {
    static ATOM: OnceLock<u16> = OnceLock::new();
    *ATOM.get_or_init(|| unsafe {
        let h_inst = GetModuleHandleW(None).unwrap_or_default();
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: h_inst.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: CreateSolidBrush(VOID),
            lpszClassName: w!("VAPURR.AuthorizeDialog"),
            ..Default::default()
        };
        let atom = RegisterClassExW(&wc);
        if atom == 0 {
            1
        } else {
            atom
        }
    })
}

fn make_font(dpi: u32, px: i32, weight: i32) -> HFONT {
    unsafe {
        CreateFontW(
            -scale(dpi, px),
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET.0 as u32,
            OUT_TT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            CLEARTYPE_QUALITY.0 as u32,
            0,
            w!("Segoe UI"),
        )
    }
}

fn paint_btn(
    hdc: HDC,
    r: &RECT,
    fill: COLORREF,
    border: COLORREF,
    label: &str,
    fg: COLORREF,
    font: HFONT,
    hover: bool,
) {
    unsafe {
        let mut fill_c = fill;
        if hover {
            let v = fill_c.0;
            let b = ((v >> 16) & 0xff).saturating_add(12).min(255);
            let g = ((v >> 8) & 0xff).saturating_add(12).min(255);
            let rr = (v & 0xff).saturating_add(12).min(255);
            fill_c = COLORREF((b << 16) | (g << 8) | rr);
        }
        let edge = CreateSolidBrush(border);
        let br = CreateSolidBrush(fill_c);
        FillRect(hdc, r, edge);
        let inner = RECT {
            left: r.left + 1,
            top: r.top + 1,
            right: r.right - 1,
            bottom: r.bottom - 1,
        };
        FillRect(hdc, &inner, br);
        let _ = DeleteObject(edge);
        let _ = DeleteObject(br);
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, fg);
        let old = SelectObject(hdc, font);
        let mut text = wide(label);
        let mut tr = *r;
        let _ = DrawTextW(
            hdc,
            &mut text,
            &mut tr,
            DRAW_TEXT_FORMAT(DT_CENTER.0 | DT_VCENTER.0 | DT_SINGLELINE.0 | DT_NOPREFIX.0),
        );
        SelectObject(hdc, old);
    }
}

unsafe fn decide(hwnd: HWND, st: &State, ok: bool) {
    if st.decided.get().is_some() {
        return;
    }
    st.decided.set(Some(ok));
    let _ = DestroyWindow(hwnd);
}

unsafe fn paint(hwnd: HWND, hdc: HDC, st: &State) {
    let mut client = RECT::default();
    let _ = GetClientRect(hwnd, &mut client);
    FillRect(hdc, &client, st.brush_void);

    let dpi = st.dpi;
    let pad = scale(dpi, 18);
    let panel = RECT {
        left: pad,
        top: pad,
        right: client.right - pad,
        bottom: client.bottom - pad,
    };
    FillRect(hdc, &panel, st.brush_steel);

    let accent = RECT {
        left: panel.left,
        top: panel.top,
        right: panel.right,
        bottom: panel.top + scale(dpi, 3),
    };
    FillRect(hdc, &accent, st.brush_lime);

    let mut y = panel.top + scale(dpi, 18);
    let left = panel.left + scale(dpi, 18);
    let right = panel.right - scale(dpi, 18);

    if !st.icon.is_invalid() {
        let isz = scale(dpi, 36);
        let _ = DrawIconEx(
            hdc,
            left,
            y,
            st.icon,
            isz,
            isz,
            0,
            HBRUSH::default(),
            DI_NORMAL,
        );
    }
    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, SNOW);
    let old = SelectObject(hdc, st.font_title);
    let mut brand = wide("vapurr");
    let mut br = RECT {
        left: left + scale(dpi, 48),
        top: y,
        right,
        bottom: y + scale(dpi, 36),
    };
    let _ = DrawTextW(
        hdc,
        &mut brand,
        &mut br,
        DRAW_TEXT_FORMAT(DT_LEFT.0 | DT_VCENTER.0 | DT_SINGLELINE.0 | DT_NOPREFIX.0),
    );
    SelectObject(hdc, old);
    y += scale(dpi, 48);

    SetTextColor(hdc, MUTED);
    let old = SelectObject(hdc, st.font_kicker);
    let mut kicker = wide("THIS DEVICE");
    let mut kr = RECT {
        left,
        top: y,
        right,
        bottom: y + scale(dpi, 18),
    };
    let _ = DrawTextW(
        hdc,
        &mut kicker,
        &mut kr,
        DRAW_TEXT_FORMAT(DT_LEFT.0 | DT_SINGLELINE.0 | DT_NOPREFIX.0),
    );
    SelectObject(hdc, old);
    y += scale(dpi, 22);

    SetTextColor(hdc, SNOW);
    let old = SelectObject(hdc, st.font_title);
    let mut title = wide("Authorize");
    let mut tr = RECT {
        left,
        top: y,
        right,
        bottom: y + scale(dpi, 34),
    };
    let _ = DrawTextW(
        hdc,
        &mut title,
        &mut tr,
        DRAW_TEXT_FORMAT(DT_LEFT.0 | DT_SINGLELINE.0 | DT_NOPREFIX.0),
    );
    SelectObject(hdc, old);
    y += scale(dpi, 40);

    SetTextColor(hdc, MUTED);
    let old = SelectObject(hdc, st.font_body);
    let mut desc = st.description.clone();
    let mut dr = RECT {
        left,
        top: y,
        right,
        bottom: st.reject.top - scale(dpi, 16),
    };
    let _ = DrawTextW(
        hdc,
        &mut desc,
        &mut dr,
        DRAW_TEXT_FORMAT(DT_LEFT.0 | DT_WORDBREAK.0 | DT_NOPREFIX.0 | DT_END_ELLIPSIS.0),
    );
    SelectObject(hdc, old);

    let hover = st.hover.get();
    paint_btn(
        hdc,
        &st.reject,
        STEEL,
        STEEL_EDGE,
        "Reject",
        SNOW,
        st.font_btn,
        hover == ID_REJECT,
    );
    paint_btn(
        hdc,
        &st.auth,
        LIME,
        LIME,
        "Authorize",
        ON_LIME,
        st.font_btn,
        hover == ID_AUTHORIZE,
    );
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !state_ptr.is_null() {
                paint(hwnd, hdc, &*state_ptr);
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if !state_ptr.is_null() {
                let st = &*state_ptr;
                let x = (lparam.0 as i32) & 0xffff;
                let y = ((lparam.0 as i32) >> 16) & 0xffff;
                let mut h = 0isize;
                if pt_in(&st.reject, x, y) {
                    h = ID_REJECT;
                } else if pt_in(&st.auth, x, y) {
                    h = ID_AUTHORIZE;
                }
                if st.hover.get() != h {
                    st.hover.set(h);
                    let _ = InvalidateRect(hwnd, None, false);
                }
            }
            LRESULT(0)
        }
        WM_SETCURSOR => {
            if !state_ptr.is_null() && (*state_ptr).hover.get() != 0 {
                if let Ok(hand) = LoadCursorW(None, IDC_HAND) {
                    SetCursor(HCURSOR(hand.0));
                    return LRESULT(1);
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONUP => {
            if !state_ptr.is_null() {
                let st = &*state_ptr;
                let x = (lparam.0 as i32) & 0xffff;
                let y = ((lparam.0 as i32) >> 16) & 0xffff;
                if pt_in(&st.reject, x, y) {
                    decide(hwnd, st, false);
                } else if pt_in(&st.auth, x, y) {
                    decide(hwnd, st, true);
                }
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if !state_ptr.is_null() {
                let vk = wparam.0 as i32;
                // Fail-closed default matches MB_DEFBUTTON2.
                if vk == VK_ESCAPE.0 as i32 || vk == VK_RETURN.0 as i32 {
                    decide(hwnd, &*state_ptr, false);
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            if !state_ptr.is_null() {
                decide(hwnd, &*state_ptr, false);
            } else {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn layout_buttons(st: &mut State, client_w: i32, client_h: i32) {
    let dpi = st.dpi;
    let pad = scale(dpi, 18);
    let btn_h = scale(dpi, 44);
    let gap = scale(dpi, 10);
    let inner = scale(dpi, 18);
    let bottom = client_h - pad - inner;
    st.auth = RECT {
        left: pad + inner,
        top: bottom - btn_h,
        right: client_w - pad - inner,
        bottom,
    };
    st.reject = RECT {
        left: st.auth.left,
        top: st.auth.top - gap - btn_h,
        right: st.auth.right,
        bottom: st.auth.top - gap,
    };
}

/// Show the branded authorize dialog. Returns true only if Authorize was clicked.
pub fn show(description: &str) -> bool {
    let _ = class_atom();
    unsafe {
        let h_inst = GetModuleHandleW(None).unwrap_or_default();
        let owner = {
            let fg = GetForegroundWindow();
            let mut pid = 0u32;
            let _ = GetWindowThreadProcessId(fg, Some(&mut pid));
            if pid == std::process::id() && !fg.is_invalid() {
                fg
            } else {
                HWND::default()
            }
        };
        let dpi = if !owner.is_invalid() {
            GetDpiForWindow(owner).max(96)
        } else {
            96
        };
        let width = scale(dpi, 420);

        let measure_dc = GetDC(None);
        let font_body = make_font(dpi, 14, FW_NORMAL.0 as i32);
        let old = SelectObject(measure_dc, font_body);
        let mut desc_w = wide(description);
        let text_w = width - scale(dpi, 18) * 4;
        let mut calc = RECT {
            left: 0,
            top: 0,
            right: text_w,
            bottom: 0,
        };
        let _ = DrawTextW(
            measure_dc,
            &mut desc_w,
            &mut calc,
            DRAW_TEXT_FORMAT(DT_LEFT.0 | DT_WORDBREAK.0 | DT_CALCRECT.0 | DT_NOPREFIX.0),
        );
        SelectObject(measure_dc, old);
        let _ = ReleaseDC(None, measure_dc);
        let desc_h = (calc.bottom - calc.top).clamp(scale(dpi, 48), scale(dpi, 220));
        let height = scale(dpi, 18) * 2
            + scale(dpi, 18)
            + scale(dpi, 48)
            + scale(dpi, 22)
            + scale(dpi, 40)
            + desc_h
            + scale(dpi, 16)
            + scale(dpi, 44) * 2
            + scale(dpi, 10)
            + scale(dpi, 18);

        let scr_w = GetSystemMetrics(SM_CXSCREEN);
        let scr_h = GetSystemMetrics(SM_CYSCREEN);

        let icon = LoadImageW(
            h_inst,
            PCWSTR(1usize as *const u16),
            IMAGE_ICON,
            scale(dpi, 36),
            scale(dpi, 36),
            LR_DEFAULTSIZE,
        )
        .map(|h| HICON(h.0)).unwrap_or_default();

        let mut state = Box::new(State {
            description: wide(description),
            decided: Cell::new(None),
            icon,
            font_kicker: make_font(dpi, 11, FW_SEMIBOLD.0 as i32),
            font_title: make_font(dpi, 22, FW_SEMIBOLD.0 as i32),
            font_body,
            font_btn: make_font(dpi, 14, FW_SEMIBOLD.0 as i32),
            brush_void: CreateSolidBrush(VOID),
            brush_steel: CreateSolidBrush(STEEL),
            brush_lime: CreateSolidBrush(LIME),
            reject: RECT::default(),
            auth: RECT::default(),
            hover: Cell::new(0),
            dpi,
        });
        layout_buttons(&mut state, width, height);

        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("VAPURR.AuthorizeDialog"),
            w!("vapurr - authorize"),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            0,
            0,
            width,
            height,
            owner,
            None,
            h_inst,
            None,
        ) {
            Ok(h) if !h.is_invalid() => h,
            _ => {
                state.drop_gdi();
                return fallback_message_box(description);
            }
        };

        let dark = windows::Win32::Foundation::BOOL::from(true);
        let caption: u32 = 0x000E_0E0E;
        let text: u32 = 0x00F4_F3F2;
        let corner = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark as *const _ as *const _,
            std::mem::size_of_val(&dark) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            &caption as *const _ as *const _,
            std::mem::size_of_val(&caption) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            &caption as *const _ as *const _,
            std::mem::size_of_val(&caption) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TEXT_COLOR,
            &text as *const _ as *const _,
            std::mem::size_of_val(&text) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner as *const _ as *const _,
            std::mem::size_of_val(&corner) as u32,
        );

        let mut cr = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };
        let _ = AdjustWindowRectEx(
            &mut cr,
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            false,
            WINDOW_EX_STYLE(0),
        );
        let win_w = cr.right - cr.left;
        let win_h = cr.bottom - cr.top;
        let wx = (scr_w - win_w) / 2;
        let wy = (scr_h - win_h) / 2;
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, wx, wy, win_w, win_h, SWP_NOZORDER);

        let state_ptr = Box::into_raw(state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);

        let mut msg = MSG::default();
        while IsWindow(hwnd).as_bool() {
            let ok = GetMessageW(&mut msg, None, 0, 0);
            if ok.0 == 0 || ok.0 == -1 {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let state = Box::from_raw(state_ptr);
        let result = state.decided.get().unwrap_or(false);
        state.drop_gdi();
        result
    }
}

fn fallback_message_box(description: &str) -> bool {
    unsafe {
        use windows::core::HSTRING;
        MessageBoxW(
            None,
            &HSTRING::from(description),
            &HSTRING::from("vapurr - authorize"),
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2 | MB_TOPMOST,
        ) == IDYES
    }
}