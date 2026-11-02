use super::*;

#[test]
fn adopted_window_reports_ordered_lifecycle_and_revokes_handle_on_destroy() {
    // No Android API is dereferenced: the host contract stores and returns
    // the opaque handle only while the test-controlled lifecycle is live.
    let window = unsafe { Window::from_native_window(NonNull::dangling()) };
    assert!(matches!(
        window.window_handle().unwrap().as_raw(),
        RawWindowHandle::AndroidNdk(_)
    ));
    assert_eq!(
        window.poll_events().unwrap(),
        vec![WindowEvent::SurfaceCreated]
    );

    window.resized(320, 240);
    window.suspended();
    window.resumed();
    window.redraw_requested();
    window.surface_destroyed();
    window.surface_destroyed();
    assert_eq!(
        window.poll_events().unwrap(),
        vec![
            WindowEvent::Resized {
                width: 320,
                height: 240,
            },
            WindowEvent::Suspended,
            WindowEvent::Resumed,
            WindowEvent::RedrawRequested,
            WindowEvent::SurfaceDestroyed,
        ]
    );
    assert!(matches!(
        window.window_handle(),
        Err(HandleError::Unavailable)
    ));
}
