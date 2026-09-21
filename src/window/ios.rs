//! iOS adopted-`UIView` test-host primitive.
//!
//! UIKit owns view creation and teardown. This module deliberately does not
//! import UIKit, create a `CAMetalLayer`, or name a Metal object: it turns a
//! host-owned `UIView` lifetime into the standard raw-window-handle contract.

use core::{
    cell::{Cell, RefCell},
    ffi::c_void,
    marker::PhantomData,
    ptr::NonNull,
};
use std::rc::Rc;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle,
    UiKitWindowHandle, WindowHandle,
};

use super::{WindowConfig, WindowError, WindowEvent};

/// A main-thread-affine borrow of a UIKit `UIView`.
///
/// The application/test runner owns the actual view and reports its lifecycle.
/// RHI may privately attach platform-specific presentation state to the raw
/// handle, but neither that state nor a Metal layer belongs to this type.
pub struct Window {
    view: Cell<Option<NonNull<c_void>>>,
    events: RefCell<Vec<WindowEvent>>,
    close_requested: Cell<bool>,
    _thread_affinity: PhantomData<Rc<()>>,
}

impl Window {
    /// UIKit supplies views through the application lifecycle; use
    /// [`Self::from_ui_view`] instead of creating one in this RHI-free crate.
    pub fn new(_: WindowConfig) -> Result<Self, WindowError> {
        Err(WindowError::UnsupportedPlatform)
    }

    /// Borrows a live main-thread UIKit `UIView`.
    ///
    /// # Safety
    ///
    /// `ui_view` must remain a valid UIKit view on the main thread until
    /// [`Self::surface_destroyed`] is called. The caller retains all Objective-C
    /// ownership; this method neither retains nor releases it.
    pub unsafe fn from_ui_view(ui_view: NonNull<c_void>) -> Self {
        Self {
            view: Cell::new(Some(ui_view)),
            events: RefCell::new(vec![WindowEvent::SurfaceCreated]),
            close_requested: Cell::new(false),
            _thread_affinity: PhantomData,
        }
    }

    /// Records the drawable size most recently reported by UIKit.
    pub fn resized(&self, width: u32, height: u32) {
        self.events
            .borrow_mut()
            .push(WindowEvent::Resized { width, height });
    }

    /// Records a foreground suspension event.
    pub fn suspended(&self) {
        self.events.borrow_mut().push(WindowEvent::Suspended);
    }

    /// Records a foreground resume event.
    pub fn resumed(&self) {
        self.events.borrow_mut().push(WindowEvent::Resumed);
    }

    /// Records UIKit's request to redraw the active drawable.
    pub fn redraw_requested(&self) {
        self.events.borrow_mut().push(WindowEvent::RedrawRequested);
    }

    /// Ends the borrowed view/drawable interval.
    pub fn surface_destroyed(&self) {
        if self.view.replace(None).is_some() {
            self.events.borrow_mut().push(WindowEvent::SurfaceDestroyed);
        }
    }

    /// Returns lifecycle facts in callback order.
    pub fn poll_events(&self) -> Result<Vec<WindowEvent>, WindowError> {
        Ok(core::mem::take(&mut *self.events.borrow_mut()))
    }

    /// Returns whether this host object was explicitly closed.
    pub fn close_requested(&self) -> bool {
        self.close_requested.get()
    }

    /// Marks the borrowed view terminal without releasing UIKit-owned state.
    pub fn close(&mut self) -> Result<(), WindowError> {
        if !self.close_requested.replace(true) {
            self.events.borrow_mut().push(WindowEvent::CloseRequested);
        }
        self.surface_destroyed();
        Ok(())
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let view = self.view.get().ok_or(HandleError::Unavailable)?;
        let raw = UiKitWindowHandle::new(view);
        // SAFETY: the UIKit owner retains `view` while this host object reports
        // a live surface, and the returned handle borrows this object.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::UiKit(raw)) })
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::uikit())
    }
}

#[cfg(test)]
mod tests {
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
}
