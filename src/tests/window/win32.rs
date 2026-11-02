use std::sync::Arc;

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
        // Showing a newly-created Win32 window may itself enqueue an
        // initial size notification; it is a real platform event.
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
    let mut window =
        Window::new(WindowConfig::new("fluxel-host lifecycle", 320, 240).unwrap()).unwrap();
    let hwnd = window.state.hwnd.get().unwrap();

    // Isolate the subsequently posted close from the creation/show event.
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
    let mut window =
        Window::new(WindowConfig::new("fluxel-host events", 320, 240).unwrap()).unwrap();
    let hwnd = window.state.hwnd.get().unwrap();
    let resize = LPARAM((240_isize << 16) | 320);

    // Isolate explicit messages from the creation/show notification.
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
    let mut window =
        Window::new(WindowConfig::new("fluxel-host destroy events", 320, 240).unwrap())
            .unwrap();
    let hwnd = window.state.hwnd.get().unwrap();

    // Isolate explicit messages from the creation/show notification.
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
