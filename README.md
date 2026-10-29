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
use fluxel_host::{Window, WindowConfig, WindowEvent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new(WindowConfig::new("Fluxel", 1280, 720)?)?;
    for event in window.poll_events()? {
        if matches!(event, WindowEvent::CloseRequested) {
            window.close()?;
        }
    }
    Ok(())
}
```

On Android and iOS, adopt platform-provided native objects during their valid lifecycle instead of calling `Window::new`; use the optional `HostRuntime` integration where supported. `Window` implements applicable `raw-window-handle` traits for RHI and other native consumers.

## Scope and lifecycle

The crate owns native window handles and ordered lifecycle notifications. It does not own GPU devices, contexts, surfaces, swapchains, frame scheduling, clocks, or renderer resource lifetime. Consumers must release presentation targets before the matching surface-destroyed boundary and must not retain borrowed native handles beyond the host object's lifetime.

See the [API documentation](https://docs.rs/fluxel-host), the [RHI verification guide](https://github.com/fluxel-project/fluxel-rhi/blob/main/documents/verification.md), and the [Fluxel ecosystem architecture](https://github.com/fluxel-project/.github/blob/main/ECOSYSTEM_ARCHITECTURE.md).

Requires Rust 1.87 or newer. Licensed under `MIT OR Apache-2.0`.
