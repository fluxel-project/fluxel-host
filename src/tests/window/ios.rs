use super::*;

#[test]
fn adopted_view_reports_ordered_lifecycle_and_revokes_handle_on_destroy() {
    // No UIKit object is messaged: this validates only the opaque-handle
    // lifetime contract that a real UIKit owner must uphold.
    let window = unsafe { Window::from_ui_view(NonNull::dangling()) };
    assert!(matches!(
        window.window_handle().unwrap().as_raw(),
        RawWindowHandle::UiKit(_)
    ));
    assert_eq!(
        window.poll_events().unwrap(),
        vec![WindowEvent::SurfaceCreated]
    );

    window.resized(640, 480);
    window.suspended();
    window.resumed();
    window.redraw_requested();
    window.surface_destroyed();
    assert_eq!(
        window.poll_events().unwrap(),
        vec![
            WindowEvent::Resized {
                width: 640,
                height: 480,
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
