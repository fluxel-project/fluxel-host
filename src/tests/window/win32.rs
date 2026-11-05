use std::sync::Arc;

use windows::Win32::UI::WindowsAndMessaging::{
    GW_HWNDPREV, GWL_EXSTYLE, GetForegroundWindow, GetWindow, GetWindowLongPtrW, IsWindowVisible,
    PostMessageW, WS_EX_TOPMOST,
};

use super::*;

#[test]
#[allow(clippy::arc_with_non_send_sync)]
fn create_handles_and_drop_can_repeat() {
    for _ in 0..2 {
        let window = Arc::new(
            Window::new(
                WindowConfig::new("fluxel-host test", 320, 240)
                    .unwrap()
                    .with_visibility(WindowVisibility::Hidden),
            )
            .unwrap(),
        );
        assert!(window.window_handle().is_ok());
        assert!(window.display_handle().is_ok());
        // Hidden creation must not disturb the desktop during native tests.
        assert!(!unsafe { IsWindowVisible(window.state.hwnd.get().unwrap()) }.as_bool());
        let _ = window.poll_events().unwrap();
        let mut window = match Arc::try_unwrap(window) {
            Ok(window) => window,
            Err(_) => unreachable!("the test holds the only Arc"),
        };
        window.close().unwrap();
        assert!(window.close_requested());
    }
}

#[test]
#[ignore = "requires a caller-coordinated interactive Windows desktop"]
fn background_presentation_preserves_foreground_and_is_idempotent() {
    let foreground_before = unsafe { GetForegroundWindow() };
    assert_ne!(foreground_before, HWND(std::ptr::null_mut()));

    let mut window = Window::new(
        WindowConfig::new("fluxel-host background policy", 320, 240)
            .unwrap()
            .with_visibility(WindowVisibility::Hidden),
    )
    .unwrap();
    let hwnd = window.state.hwnd.get().unwrap();
    assert!(!unsafe { IsWindowVisible(hwnd) }.as_bool());
    assert_eq!(unsafe { GetForegroundWindow() }, foreground_before);

    window.show_background().unwrap();
    assert!(unsafe { IsWindowVisible(hwnd) }.as_bool());
    assert_eq!(unsafe { GetForegroundWindow() }, foreground_before);
    let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    assert_eq!(ex_style & WS_EX_TOPMOST.0, 0);

    let predecessor = unsafe { GetWindow(hwnd, GW_HWNDPREV) }.unwrap();
    assert_ne!(predecessor, HWND(std::ptr::null_mut()));
    let mut above = predecessor;
    let mut foreground_is_above = false;
    while above != HWND(std::ptr::null_mut()) {
        if above == foreground_before {
            foreground_is_above = true;
            break;
        }
        above = unsafe { GetWindow(above, GW_HWNDPREV) }.unwrap();
    }
    assert!(foreground_is_above);

    // Once visible, presentation is intentionally inert so a user-selected
    // window is not demoted by a duplicate first-frame notification.
    window.show_background().unwrap();
    assert_eq!(unsafe { GetForegroundWindow() }, foreground_before);
    assert_eq!(
        unsafe { GetWindow(hwnd, GW_HWNDPREV) }.unwrap(),
        predecessor
    );

    window.close().unwrap();
}

#[test]
fn close_request_does_not_destroy_the_native_window_intent() {
    let state = WindowState {
        hwnd: Cell::new(Some(HWND(std::ptr::dangling_mut()))),
        hinstance: HINSTANCE(std::ptr::null_mut()),
        close_requested: Cell::new(false),
        minimized: Cell::new(false),
        events: RefCell::new(Vec::new()),
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
        minimized: Cell::new(false),
        events: RefCell::new(Vec::new()),
    };

    state.push_event(state.size_event(WPARAM(2), LPARAM((240_isize << 16) | 320)));
    state.native_destroyed();
    state.request_close();

    assert!(state.close_requested.get());
    assert_eq!(state.hwnd.get(), None);
    assert_eq!(
        state.take_events(),
        vec![
            WindowEvent::Resized {
                width: 320,
                height: 240,
            },
            WindowEvent::CloseRequested,
            WindowEvent::SurfaceDestroyed,
        ]
    );
}

#[test]
fn close_message_preserves_window_until_explicit_close() {
    let mut window = Window::new(
        WindowConfig::new("fluxel-host lifecycle", 320, 240)
            .unwrap()
            .with_visibility(WindowVisibility::Hidden),
    )
    .unwrap();
    let hwnd = window.state.hwnd.get().unwrap();

    // Isolate the subsequently posted close from creation-time messages.
    let _ = window.poll_events().unwrap();

    // SAFETY: `hwnd` is owned by this test's window on the current thread.
    unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) }.unwrap();
    assert_eq!(
        window.poll_events().unwrap(),
        vec![WindowEvent::CloseRequested]
    );

    assert!(window.close_requested());
    assert!(window.window_handle().is_ok());

    window.close().unwrap();
    assert!(matches!(
        window.window_handle(),
        Err(HandleError::Unavailable)
    ));
    window.close().unwrap();
}

#[test]
fn ordered_size_events_preserve_resize_minimize_restore_and_zero_extent() {
    let state = WindowState {
        hwnd: Cell::new(None),
        hinstance: HINSTANCE(std::ptr::null_mut()),
        close_requested: Cell::new(false),
        minimized: Cell::new(false),
        events: RefCell::new(Vec::new()),
    };

    // Win32 reports ordinary drag-resizes as SIZE_RESTORED too; without a
    // preceding minimize they remain ordinary Resized events.
    state.push_event(state.size_event(
        WPARAM(SIZE_RESTORED as usize),
        LPARAM((480_isize << 16) | 640),
    ));
    state.push_event(state.size_event(WPARAM(SIZE_MINIMIZED as usize), LPARAM(0)));
    state.push_event(state.size_event(WPARAM(SIZE_RESTORED as usize), LPARAM(0)));
    state.push_event(state.size_event(WPARAM(2), LPARAM((720_isize << 16) | 1280)));

    assert_eq!(
        state.take_events(),
        vec![
            WindowEvent::Resized {
                width: 640,
                height: 480,
            },
            WindowEvent::Minimized,
            WindowEvent::Restored {
                width: 0,
                height: 0,
            },
            WindowEvent::Resized {
                width: 1280,
                height: 720,
            },
        ]
    );
}

#[test]
fn queued_win32_messages_emit_events_in_dispatch_order() {
    let mut window = Window::new(
        WindowConfig::new("fluxel-host events", 320, 240)
            .unwrap()
            .with_visibility(WindowVisibility::Hidden),
    )
    .unwrap();
    let hwnd = window.state.hwnd.get().unwrap();
    let resize = LPARAM((240_isize << 16) | 320);

    // Isolate explicit messages from creation-time notifications.
    let _ = window.poll_events().unwrap();

    // SAFETY: the test owns this HWND and posts only messages handled by
    // this window's procedure on the current thread.
    unsafe {
        PostMessageW(Some(hwnd), WM_SIZE, WPARAM(2), resize).unwrap();
        PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap();
    }

    assert_eq!(
        window.poll_events().unwrap(),
        vec![
            WindowEvent::Resized {
                width: 320,
                height: 240,
            },
            WindowEvent::CloseRequested,
        ]
    );
    assert!(window.close_requested());
    window.close().unwrap();
}

#[test]
fn destroy_message_appends_one_terminal_event_after_prior_resize() {
    let mut window = Window::new(
        WindowConfig::new("fluxel-host destroy events", 320, 240)
            .unwrap()
            .with_visibility(WindowVisibility::Hidden),
    )
    .unwrap();
    let hwnd = window.state.hwnd.get().unwrap();

    // Isolate explicit messages from creation-time notifications.
    let _ = window.poll_events().unwrap();

    // SAFETY: the test owns this HWND and uses messages handled by this
    // window procedure. Posting WM_DESTROY exercises external terminal
    // notification without asking the OS to destroy the test HWND twice.
    unsafe {
        PostMessageW(
            Some(hwnd),
            WM_SIZE,
            WPARAM(2),
            LPARAM((400_isize << 16) | 800),
        )
        .unwrap();
        PostMessageW(Some(hwnd), WM_DESTROY, WPARAM(0), LPARAM(0)).unwrap();
    }

    assert_eq!(
        window.poll_events().unwrap(),
        vec![
            WindowEvent::Resized {
                width: 800,
                height: 400,
            },
            WindowEvent::CloseRequested,
        ]
    );
    assert!(window.close_requested());

    // The synthetic WM_DESTROY did not destroy the HWND; explicit close
    // dispatches a second terminal message which the helper must dedupe.
    window.close().unwrap();
    assert_eq!(
        window.poll_events().unwrap(),
        vec![WindowEvent::SurfaceDestroyed]
    );
}
