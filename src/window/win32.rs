//! Win32 implementation of the deliberately narrow `Window` primitive.
//!
//! The native callback only transitions close state. GPU waiting, surface
//! teardown, and HWND destruction stay outside the callback so a renderer can
//! first stop its frame loop and retire its accepted work.

use core::{cell::Cell, ffi::c_void, marker::PhantomData, num::NonZeroIsize};
use std::{rc::Rc, sync::OnceLock};

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle,
    Win32WindowHandle, WindowHandle,
};
use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AdjustWindowRectEx, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW,
            DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA, IDC_ARROW, LoadCursorW,
            MSG, PM_REMOVE, PeekMessageW, RegisterClassW, SW_SHOW, ShowWindow, TranslateMessage,
            WINDOW_EX_STYLE, WM_CLOSE, WM_DESTROY, WM_NCCREATE, WM_NCDESTROY, WNDCLASSW,
            WS_OVERLAPPEDWINDOW,
        },
    },
    core::PCWSTR,
};

use super::{WindowConfig, WindowError};

const CLASS_NAME: &[u16] = &[
    b'F' as u16,
    b'l' as u16,
    b'u' as u16,
    b'x' as u16,
    b'e' as u16,
    b'l' as u16,
    b'H' as u16,
    b'o' as u16,
    b's' as u16,
    b't' as u16,
    b'W' as u16,
    b'i' as u16,
    b'n' as u16,
    b'd' as u16,
    b'o' as u16,
    b'w' as u16,
    0,
];

static WINDOW_CLASS: OnceLock<Result<(), String>> = OnceLock::new();

/// A thread-affine fixed-size Win32 window.
///
/// It is intentionally neither `Send` nor `Sync`; cloning an `Arc<Window>`
/// still works on its owning thread, allowing an RHI surface to retain the
/// window's lifetime without making a Win32 HWND cross-thread safe.
pub struct Window {
    state: Box<WindowState>,
    _thread_affinity: PhantomData<Rc<()>>,
}

struct WindowState {
    hwnd: Cell<Option<HWND>>,
    hinstance: HINSTANCE,
    close_requested: Cell<bool>,
}

impl WindowState {
    fn request_close(&self) {
        self.close_requested.set(true);
    }

    /// Records terminal native destruction before the owning `Box` may be dropped.
    fn native_destroyed(&self) {
        self.request_close();
        self.hwnd.set(None);
    }
}

impl Window {
    /// Creates and shows a Win32 overlapped window with the requested client extent.
    pub fn new(config: WindowConfig) -> Result<Self, WindowError> {
        let hinstance = unsafe { GetModuleHandleW(None) }.map_err(platform_error)?;
        let hinstance = HINSTANCE(hinstance.0);
        ensure_window_class(hinstance)?;

        let mut state = Box::new(WindowState {
            hwnd: Cell::new(None),
            hinstance,
            close_requested: Cell::new(false),
        });
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: config.client_width().get() as i32,
            bottom: config.client_height().get() as i32,
        };
        let style = WS_OVERLAPPEDWINDOW;
        unsafe { AdjustWindowRectEx(&mut rect, style, false, WINDOW_EX_STYLE::default()) }
            .map_err(platform_error)?;
        let title = wide_nul(config.title());
        let state_ptr = (&mut *state) as *mut WindowState;
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PCWSTR(CLASS_NAME.as_ptr()),
                PCWSTR(title.as_ptr()),
                style,
                0x80000000_u32 as i32,
                0x80000000_u32 as i32,
                rect.right - rect.left,
                rect.bottom - rect.top,
                None,
                None,
                Some(hinstance),
                Some(state_ptr.cast::<c_void>()),
            )
        }
        .map_err(platform_error)?;
        state.hwnd.set(Some(hwnd));
        // SAFETY: `hwnd` was created above and the current thread owns it.
        // Showing may legitimately report that the window was not previously visible.
        let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };
        Ok(Self {
            state,
            _thread_affinity: PhantomData,
        })
    }

    /// Dispatches all messages already queued for this thread without blocking.
    pub fn poll_events(&self) -> Result<(), WindowError> {
        let mut message = MSG::default();
        // SAFETY: `message` is valid writable storage; null HWND selects this
        // thread's queue, which is precisely this window primitive's contract.
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            // SAFETY: the message came directly from this thread's queue.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        Ok(())
    }

    /// Returns whether `WM_CLOSE` or `WM_DESTROY` has stopped future frames.
    pub fn close_requested(&self) -> bool {
        self.state.close_requested.get()
    }

    /// Explicitly destroys the HWND if it has not already been destroyed.
    ///
    /// Requiring exclusive access prevents destruction while a standard
    /// `WindowHandle` borrowed from this window is still live.
    pub fn close(&mut self) -> Result<(), WindowError> {
        self.state.request_close();
        if let Some(hwnd) = self.state.hwnd.get() {
            // `WM_DESTROY` updates the state before this call returns.
            unsafe { DestroyWindow(hwnd) }.map_err(platform_error)?;
        }
        Ok(())
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // A Drop implementation cannot communicate a platform error.  Explicit
        // `close` is available for callers that require reporting.
        let _ = self.close();
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let hwnd = self.state.hwnd.get().ok_or(HandleError::Unavailable)?;
        let hwnd = NonZeroIsize::new(hwnd.0 as isize).ok_or(HandleError::Unavailable)?;
        let mut raw = Win32WindowHandle::new(hwnd);
        raw.hinstance = NonZeroIsize::new(self.state.hinstance.0 as isize);
        // SAFETY: both handles remain valid while `self` is borrowed; close
        // must happen only after the rendering surface releases its borrow.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(raw)) })
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::windows())
    }
}

fn ensure_window_class(hinstance: HINSTANCE) -> Result<(), WindowError> {
    let result = WINDOW_CLASS.get_or_init(|| {
        let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }.map_err(|error| error.to_string())?;
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: hinstance,
            hCursor: cursor,
            lpszClassName: PCWSTR(CLASS_NAME.as_ptr()),
            ..Default::default()
        };
        if unsafe { RegisterClassW(&class) } == 0 {
            return Err(windows::core::Error::from_thread().to_string());
        }
        Ok(())
    });
    result.clone().map_err(WindowError::Platform)
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        // SAFETY: `lparam` is a CREATESTRUCTW supplied by CreateWindowExW;
        // lpCreateParams is the stable Box<WindowState> pointer passed above.
        let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        if !create.lpCreateParams.is_null() {
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                    hwnd,
                    GWLP_USERDATA,
                    create.lpCreateParams as isize,
                )
            };
        }
    }
    let state = unsafe {
        let pointer =
            windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        (pointer as *const WindowState).as_ref()
    };
    match message {
        WM_CLOSE => {
            if let Some(state) = state {
                state.request_close();
            }
            // The application owns the shutdown sequence: stop rendering,
            // retire its surface/GPU work, then explicitly call `Window::close`.
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Some(state) = state {
                state.request_close();
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            if let Some(state) = state {
                state.native_destroyed();
            }
            // The HWND will no longer retain a pointer into the `Window` Box.
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0)
            };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn wide_nul(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn platform_error(error: windows::core::Error) -> WindowError {
    WindowError::Platform(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use raw_window_handle::{HasDisplayHandle as _, HasWindowHandle as _};
    use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

    use super::*;

    #[test]
    #[allow(clippy::arc_with_non_send_sync)]
    fn create_handles_and_drop_can_repeat() {
        for _ in 0..2 {
            let window = Arc::new(
                Window::new(WindowConfig::new("fluxel-host test", 320, 240).unwrap()).unwrap(),
            );
            assert!(window.window_handle().is_ok());
            assert!(window.display_handle().is_ok());
            window.poll_events().unwrap();
            let mut window = match Arc::try_unwrap(window) {
                Ok(window) => window,
                Err(_) => unreachable!("the test holds the only Arc"),
            };
            window.close().unwrap();
            assert!(window.close_requested());
        }
    }

    #[test]
    fn close_request_does_not_destroy_the_native_window_intent() {
        let state = WindowState {
            hwnd: Cell::new(Some(HWND(std::ptr::dangling_mut()))),
            hinstance: HINSTANCE(std::ptr::null_mut()),
            close_requested: Cell::new(false),
        };

        state.request_close();

        assert!(state.close_requested.get());
        assert!(state.hwnd.get().is_some());
    }

    #[test]
    fn native_destruction_makes_repeated_close_idempotent() {
        let state = WindowState {
            hwnd: Cell::new(Some(HWND(std::ptr::dangling_mut()))),
            hinstance: HINSTANCE(std::ptr::null_mut()),
            close_requested: Cell::new(false),
        };

        state.native_destroyed();
        state.request_close();

        assert!(state.close_requested.get());
        assert_eq!(state.hwnd.get(), None);
    }

    #[test]
    fn close_message_preserves_window_until_explicit_close() {
        let mut window =
            Window::new(WindowConfig::new("fluxel-host lifecycle", 320, 240).unwrap()).unwrap();
        let hwnd = window.state.hwnd.get().unwrap();

        // SAFETY: `hwnd` is owned by this test's window on the current thread.
        unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) }.unwrap();
        window.poll_events().unwrap();

        assert!(window.close_requested());
        assert!(window.window_handle().is_ok());

        window.close().unwrap();
        assert!(matches!(
            window.window_handle(),
            Err(HandleError::Unavailable)
        ));
        window.close().unwrap();
    }
}
