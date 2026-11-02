//! Android's adopted `ANativeWindow` test-host primitive.
//!
//! Android owns creation and destruction of the native window. This type never
//! creates, retains, releases, or otherwise manages that NDK object; it merely
//! borrows it for the interval announced by the activity lifecycle. That keeps
//! the host independent of Vulkan, GLES, and RHI while still giving a test or
//! example one standard raw-window-handle source.

use core::{
    cell::{Cell, RefCell},
    ffi::c_void,
    marker::PhantomData,
    ptr::NonNull,
};
use std::rc::Rc;

use raw_window_handle::{
    AndroidNdkWindowHandle, DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle,
    RawWindowHandle, WindowHandle,
};

use super::{WindowConfig, WindowError, WindowEvent};

/// A thread-affine borrow of an Android activity's current `ANativeWindow`.
///
/// Construct this only from the activity callback that supplied `window`; the
/// pointer must remain valid until [`Self::surface_destroyed`] is called. The
/// Android activity owns the native reference. RHI backends that need a longer
/// lease acquire their own native reference privately.
pub struct Window {
    native: Cell<Option<NonNull<c_void>>>,
    events: RefCell<Vec<WindowEvent>>,
    close_requested: Cell<bool>,
    _thread_affinity: PhantomData<Rc<()>>,
}

impl Window {
    /// Android supplies its native drawable rather than allowing a Rust host to
    /// create one. Use [`Self::from_native_window`] in the activity callback.
    pub fn new(_: WindowConfig) -> Result<Self, WindowError> {
        Err(WindowError::UnsupportedPlatform)
    }

    /// Adopts the current activity drawable for the caller-controlled lifetime.
    ///
    /// # Safety
    ///
    /// `native_window` must be a live `ANativeWindow` received from Android's
    /// native-window-created callback. The caller must call
    /// [`Self::surface_destroyed`] before Android invalidates that pointer, and
    /// must use the window on the Android UI/activity thread.
    pub unsafe fn from_native_window(native_window: NonNull<c_void>) -> Self {
        Self {
            native: Cell::new(Some(native_window)),
            events: RefCell::new(vec![WindowEvent::SurfaceCreated]),
            close_requested: Cell::new(false),
            _thread_affinity: PhantomData,
        }
    }

    /// Records the current drawable extent supplied by the activity.
    pub fn resized(&self, width: u32, height: u32) {
        self.events
            .borrow_mut()
            .push(WindowEvent::Resized { width, height });
    }

    /// Records a redraw request from the Android activity.
    pub fn redraw_requested(&self) {
        self.events.borrow_mut().push(WindowEvent::RedrawRequested);
    }

    /// Records that Android suspended foreground rendering.
    pub fn suspended(&self) {
        self.events.borrow_mut().push(WindowEvent::Suspended);
    }

    /// Records that Android resumed foreground rendering.
    pub fn resumed(&self) {
        self.events.borrow_mut().push(WindowEvent::Resumed);
    }

    /// Ends the borrowed native-window interval before Android destroys it.
    pub fn surface_destroyed(&self) {
        if self.native.replace(None).is_some() {
            self.events.borrow_mut().push(WindowEvent::SurfaceDestroyed);
        }
    }

    /// Returns pending lifecycle facts in their original callback order.
    pub fn poll_events(&self) -> Result<Vec<WindowEvent>, WindowError> {
        Ok(core::mem::take(&mut *self.events.borrow_mut()))
    }

    /// Android has no closeable window primitive; this reports whether the
    /// embedding activity asked this host object to stop producing frames.
    pub fn close_requested(&self) -> bool {
        self.close_requested.get()
    }

    /// Marks this host object terminal without destroying Android-owned state.
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
        let native = self.native.get().ok_or(HandleError::Unavailable)?;
        let raw = AndroidNdkWindowHandle::new(native);
        // SAFETY: `native` is borrowed from Android and is cleared before the
        // callback's destruction boundary; the returned handle borrows `self`.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::AndroidNdk(raw)) })
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::android())
    }
}

#[cfg(test)]
#[path = "../tests/window/android.rs"]
mod tests;
