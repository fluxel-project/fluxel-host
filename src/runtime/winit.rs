use core::fmt;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, Ime, MouseScrollDelta, WindowEvent as WinitWindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key as WinitKey, NamedKey},
    window::{Window as WinitWindow, WindowAttributes, WindowId},
};

use crate::{Key, MouseButton, WindowConfig, WindowError, WindowEvent};

/// A native window created by [`HostContext`] while the platform permits it.
///
/// It forwards the standard `raw-window-handle` traits directly.  The window
/// is thread-affine because the underlying platform object is thread-affine.
pub struct HostWindow {
    inner: WinitWindow,
}

impl HostWindow {
    /// Requests one redraw callback for this native drawable.
    pub fn request_redraw(&self) {
        self.inner.request_redraw();
    }

    /// Returns the client drawable extent in physical pixels.
    pub fn inner_size(&self) -> [u32; 2] {
        let size = self.inner.inner_size();
        [size.width, size.height]
    }
}

impl HasWindowHandle for HostWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.inner.window_handle()
    }
}

impl HasDisplayHandle for HostWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.inner.display_handle()
    }
}

/// Callbacks delivered by the minimal cross-platform native runtime.
///
/// `resumed` is the only place portable applications may create their first
/// window.  On Android and iOS it is also the place to recreate a presentation
/// target after a suspended/destroyed interval.
pub trait HostApplication {
    /// The application may create a window through `host` in this callback.
    fn resumed(&mut self, host: &mut HostContext<'_>);

    /// A lifecycle or window fact was observed in platform order.
    fn window_event(&mut self, host: &mut HostContext<'_>, event: WindowEvent);
}

/// Temporary capability passed into [`HostApplication`] callbacks.
///
/// It does not own a GPU object.  A consumer must only borrow [`HostWindow`]
/// while its matching surface interval is alive, and must release any RHI
/// presentation state after `SurfaceDestroyed` before the next resume.
pub struct HostContext<'a> {
    event_loop: &'a ActiveEventLoop,
    window: &'a mut Option<HostWindow>,
}

impl<'a> HostContext<'a> {
    /// Creates the one minimal presentation window, if absent.
    pub fn create_window(&mut self, config: WindowConfig) -> Result<&HostWindow, WindowError> {
        if self.window.is_none() {
            let attributes = WindowAttributes::default()
                .with_title(config.title())
                .with_inner_size(PhysicalSize::new(
                    config.client_width().get(),
                    config.client_height().get(),
                ));
            let window = self
                .event_loop
                .create_window(attributes)
                .map_err(|error| WindowError::Platform(error.to_string()))?;
            window.set_ime_allowed(true);
            *self.window = Some(HostWindow { inner: window });
        }
        Ok(self.window.as_ref().expect("window was just created"))
    }

    /// Returns the live window, if the application created one.
    pub fn window(&self) -> Option<&HostWindow> {
        self.window.as_ref()
    }

    /// Stops the platform event loop after this callback returns.
    pub fn exit(&self) {
        self.event_loop.exit();
    }
}

/// Failure while starting the event-driven host runtime.
#[derive(Debug)]
pub struct HostRuntimeError(String);

impl fmt::Display for HostRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for HostRuntimeError {}

/// An event-driven native runtime for Windows, Android, and iOS test/examples.
pub struct HostRuntime {
    event_loop: EventLoop<()>,
}

impl HostRuntime {
    /// Builds the platform event loop.  Android/iOS applications must invoke
    /// this from their platform entry point/main thread.
    pub fn new() -> Result<Self, HostRuntimeError> {
        #[cfg(target_os = "android")]
        {
            return Err(HostRuntimeError(
                "Android requires HostRuntime::from_android_app(android_main's AndroidApp)"
                    .to_owned(),
            ));
        }
        #[cfg(not(target_os = "android"))]
        EventLoop::new()
            .map(|event_loop| Self { event_loop })
            .map_err(|error| HostRuntimeError(error.to_string()))
    }

    /// Builds an Android runtime from the `AndroidApp` received by the APK's
    /// `android_main` entry point. The type is re-exported by this crate so
    /// the application never adds a second `android-activity` dependency.
    #[cfg(target_os = "android")]
    pub fn from_android_app(
        app: winit::platform::android::activity::AndroidApp,
    ) -> Result<Self, HostRuntimeError> {
        use winit::platform::android::EventLoopBuilderExtAndroid;

        let mut builder = EventLoop::builder();
        builder.with_android_app(app);
        builder
            .build()
            .map(|event_loop| Self { event_loop })
            .map_err(|error| HostRuntimeError(error.to_string()))
    }

    /// Runs until [`HostContext::exit`] or a close request.
    pub fn run<A: HostApplication + 'static>(self, application: A) -> Result<(), HostRuntimeError> {
        let mut runner = Runner {
            application,
            window: None,
            surface_live: false,
        };
        self.event_loop
            .run_app(&mut runner)
            .map_err(|error| HostRuntimeError(error.to_string()))
    }
}

struct Runner<A> {
    application: A,
    window: Option<HostWindow>,
    surface_live: bool,
}

impl<A: HostApplication> Runner<A> {
    fn with_context(
        &mut self,
        event_loop: &ActiveEventLoop,
        callback: impl FnOnce(&mut A, &mut HostContext<'_>),
    ) {
        let mut context = HostContext {
            event_loop,
            window: &mut self.window,
        };
        callback(&mut self.application, &mut context);
    }

    fn emit(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        self.with_context(event_loop, |application, context| {
            application.window_event(context, event)
        });
    }
}

impl<A: HostApplication> ApplicationHandler for Runner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.with_context(event_loop, |application, context| {
            application.resumed(context)
        });
        if self.window.is_some() && !self.surface_live {
            self.surface_live = true;
            self.emit(event_loop, WindowEvent::SurfaceCreated);
        }
        if self.window.is_some() {
            self.emit(event_loop, WindowEvent::Resumed);
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            self.emit(event_loop, WindowEvent::Suspended);
        }
        if self.surface_live {
            self.surface_live = false;
            self.emit(event_loop, WindowEvent::SurfaceDestroyed);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WinitWindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.inner.id()) != Some(window_id) {
            return;
        }
        if let WinitWindowEvent::KeyboardInput { event, .. } = &event {
            let key = translate_key(&event.logical_key);
            self.emit(
                event_loop,
                WindowEvent::KeyboardInput {
                    key,
                    pressed: event.state == ElementState::Pressed,
                    repeat: event.repeat,
                },
            );
            if event.state == ElementState::Pressed {
                if let Some(text) = &event.text {
                    if !text.is_empty() && !text.chars().any(char::is_control) {
                        self.emit(event_loop, WindowEvent::TextInput(text.to_string()));
                    }
                }
            }
            return;
        }
        let translated = match event {
            WinitWindowEvent::CursorMoved { position, .. } => Some(WindowEvent::CursorMoved {
                x: position.x,
                y: position.y,
            }),
            WinitWindowEvent::CursorLeft { .. } => Some(WindowEvent::CursorLeft),
            WinitWindowEvent::Focused(focused) => Some(WindowEvent::Focused(focused)),
            WinitWindowEvent::MouseInput { state, button, .. } => Some(WindowEvent::MouseInput {
                button: match button {
                    winit::event::MouseButton::Left => MouseButton::Left,
                    winit::event::MouseButton::Right => MouseButton::Right,
                    winit::event::MouseButton::Middle => MouseButton::Middle,
                    winit::event::MouseButton::Back => MouseButton::Other(4),
                    winit::event::MouseButton::Forward => MouseButton::Other(5),
                    winit::event::MouseButton::Other(value) => MouseButton::Other(value),
                },
                pressed: state == ElementState::Pressed,
            }),
            WinitWindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x, y),
                    MouseScrollDelta::PixelDelta(p) => (p.x as f32 / 40.0, p.y as f32 / 40.0),
                };
                Some(WindowEvent::MouseWheel { x, y })
            }
            WinitWindowEvent::Ime(Ime::Commit(text)) if !text.is_empty() => {
                Some(WindowEvent::TextInput(text))
            }
            WinitWindowEvent::Resized(size) => Some(WindowEvent::Resized {
                width: size.width,
                height: size.height,
            }),
            WinitWindowEvent::RedrawRequested => Some(WindowEvent::RedrawRequested),
            WinitWindowEvent::CloseRequested => {
                event_loop.exit();
                Some(WindowEvent::CloseRequested)
            }
            _ => None,
        };
        if let Some(event) = translated {
            self.emit(event_loop, event);
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if self.surface_live {
            self.surface_live = false;
            self.emit(event_loop, WindowEvent::SurfaceDestroyed);
        }
    }
}

fn translate_key(key: &WinitKey) -> Key {
    match key {
        WinitKey::Character(value) => {
            let mut chars = value.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Key::Character(c.to_ascii_lowercase()),
                _ => Key::Other,
            }
        }
        WinitKey::Named(named) => match named {
            NamedKey::Escape => Key::Escape,
            NamedKey::Enter => Key::Enter,
            NamedKey::Tab => Key::Tab,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::Insert => Key::Insert,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::PageDown => Key::PageDown,
            NamedKey::ArrowLeft => Key::ArrowLeft,
            NamedKey::ArrowRight => Key::ArrowRight,
            NamedKey::ArrowUp => Key::ArrowUp,
            NamedKey::ArrowDown => Key::ArrowDown,
            NamedKey::Space => Key::Space,
            NamedKey::Shift => Key::Shift,
            NamedKey::Control => Key::Control,
            NamedKey::Alt => Key::Alt,
            NamedKey::Super => Key::Super,
            _ => Key::Other,
        },
        _ => Key::Other,
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;
    #[test]
    fn logical_keys_are_normalized_and_named_keys_are_preserved() {
        assert_eq!(
            translate_key(&WinitKey::Character("A".into())),
            Key::Character('a')
        );
        assert_eq!(
            translate_key(&WinitKey::Named(NamedKey::ArrowLeft)),
            Key::ArrowLeft
        );
        assert_eq!(
            translate_key(&WinitKey::Named(NamedKey::Control)),
            Key::Control
        );
        assert_eq!(translate_key(&WinitKey::Character("ab".into())), Key::Other);
    }
}
