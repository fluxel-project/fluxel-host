# `fluxel-host`

`fluxel-host` provides native window and lifecycle primitives for Fluxel applications. It creates or adopts native windows, pumps ordered lifecycle events, and exposes standard `raw-window-handle` traits; it does not create a GPU device or own a rendering surface.

Repository: https://github.com/fluxel-project/fluxel-host

## Install

```toml
[dependencies]
fluxel-host = "0.1.0"
```

The minimal implementation creates owned windows on Windows. Android and iOS adopt native windows or views supplied by their application callbacks; `Window::new` intentionally returns `UnsupportedPlatform` on those targets. Enable an optional runtime feature for an event-driven window loop:

```toml
[dependencies]
fluxel-host = { version = "0.1.0", features = ["winit-runtime"] }
```

`winit-runtime` supports Windows, Linux/X11, Android, and iOS. Linux applications using Wayland can select `winit-runtime-wayland` instead. Winit remains private behind the host API.

## Minimal Win32 window

```rust
use fluxel_host::{Window, WindowConfig, WindowEvent, WindowVisibility};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = WindowConfig::new("Fluxel", 1280, 720)?
        .with_visibility(WindowVisibility::Hidden);
    let mut window = Window::new(config)?;
    // Create the renderer and submit its first frame before this call.
    window.show_background()?;
    for event in window.poll_events()? {
        if matches!(event, WindowEvent::CloseRequested) {
            window.close()?;
        }
    }
    Ok(())
}
```

New windows default to `WindowVisibility::Background`: on the minimal Win32
backend they are shown without activation at the bottom of the z-order. Use
`Hidden` when setup must finish before the first visible frame, then call
`show_background`. That method intentionally does not install a permanent
no-activate style, so a user can select the window normally. The optional
winit runtime asks platforms to create windows hidden and inactive. On Windows
its `show_background` has the same non-activating background transition; other
platforms receive a best-effort visibility request because winit does not
expose an equivalent z-order operation.

On Android and iOS, adopt platform-provided native objects during their valid lifecycle instead of calling `Window::new`; use the optional `HostRuntime` integration where supported. `Window` implements applicable `raw-window-handle` traits for RHI and other native consumers.

## Scope and lifecycle

The crate owns native window handles and ordered lifecycle notifications. It does not own GPU devices, contexts, surfaces, swapchains, frame scheduling, clocks, or renderer resource lifetime. Consumers must release presentation targets before the matching surface-destroyed boundary and must not retain borrowed native handles beyond the host object's lifetime.

See the [API documentation](https://docs.rs/fluxel-host), the [RHI verification guide](https://github.com/fluxel-project/fluxel-rhi/blob/main/documents/verification.md), and the [Fluxel ecosystem architecture](https://github.com/fluxel-project/.github/blob/main/ECOSYSTEM_ARCHITECTURE.md).

## Rendering consumer evidence

Rendering's bounded S7/S8 consumer result uses Host `8d10162` for native-window
lifecycle only. `WindowVisibility::Hidden` creates a non-visible host for
offscreen work; `show_background` makes its first Windows transition at the
bottom of the non-topmost z-order without foreground activation, and remains
idempotent after the window becomes visible. This supports native DX12/WGL demo
presentation and independent Windows Graphics Capture without changing focus or
z-order; the evidence oracle excludes only the two bottom 16x16 DWM decoration
corners, never RHI readback pixels. It is not a GPU, Surface, scheduling, or
renderer-ownership contract. The
[S7 historical closure](https://github.com/fluxel-project/fluxel-rendering/blob/main/documents/s7-deferred-closure.md)
and [S8 measured closure](https://github.com/fluxel-project/fluxel-rendering/blob/main/documents/s8-measured-closure.md)
do not establish browser support.

Requires Rust 1.87 or newer. Licensed under `MIT OR Apache-2.0`.
