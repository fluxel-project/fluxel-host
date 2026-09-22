# Fluxel Host

`fluxel-host` is the monorepo for native Fluxel application-host foundations.
It is intended to support Fluxel applications targeting Windows executables,
Android APK/AAB packages, and iOS IPA applications.

## Scope

This repository owns native platform capabilities and the application-layer
composition needed to package them as runnable Fluxel applications. Its
platform-library crates and its final runtime/executable composition have
different dependency roles; internal crates and host packages are extracted
only as real targets prove the boundary. The outline below is not a promise
that every component already exists.

Expected ownership includes:

- process entry points, native windows/targets, and application lifecycle;
- native time and input collection;
- filesystem, storage, networking, loading, audio, and video implementations;
- native diagnostic sinks;
- platform and language-VM integration; and
- Windows, Android, and iOS packaging and lifecycle adapters.

## Non-scope

`fluxel-host` does not define rendering semantics, GPU resource management,
RenderGraph behavior, renderer `Surface`/swapchain ownership, or the public
cross-platform language SDK. Its platform-library crates are platform leaves:
they supply windows, display handles, and lifecycle facts, but do not depend on
`fluxel-rendering` or any RHI crate. An application-layer runtime or final
EXE/APK/IPA composition in this repository may combine those platform crates
with `fluxel-rendering`. That composition is a consumer of both libraries, not
a dependency of the platform-library boundary.
Browser and mini-game adapters, along with their developer-facing JavaScript
API, belong to `fluxel-jsbridge`, not to this repository.

## Dependencies

Platform-library crates in `fluxel-host` may depend on platform-neutral
foundation crates such as `fluxel-bases`, but never on `fluxel-rendering`. A
platform host owns platform resources and reports their lifecycle. The
application-layer composition owns the frame loop, creates the RHI
provider/surface through RHI's host-handle entry point, and decides how to
react to resize, suspension, and device loss.

## Current foundation

This repository establishes the native-host ownership boundary. It does not
claim that complete EXE, APK/AAB, or IPA application delivery is available.
The cross-platform test-host foundation provides:

- Windows creates an owned Win32 `Window`, with ordered close/resize/minimize/
  restore events and explicit RAII destruction.
- Android adopts the `ANativeWindow` supplied by the activity callback for its
  valid lifecycle interval.
- iOS adopts the `UIView` supplied by the UIKit application for its valid
  lifecycle interval.
- All three expose the selected `raw-window-handle` API plus ordered
  drawable-created, drawable-destroyed, resize, suspend/resume, redraw, and
  close facts. They do not create a GPU device, context, layer, surface, or
  swapchain.

For platform smoke tests and examples, enable the optional `winit-runtime`
feature. `HostRuntime` provides one event-driven callback contract across
Windows, Android, and iOS while keeping `winit` private to the implementation.
Windows/iOS use `HostRuntime::new()` on their UI thread; Android calls
`HostRuntime::from_android_app()` from `android_main` using the re-exported
`fluxel_host::AndroidApp`. No RHI type is involved in this platform runtime. A
runtime callback creates its `HostWindow` only from `resumed`, consumes ordered
`WindowEvent`s, and hands the borrowed raw handle to an application-layer
composition, RHI-side test, or example.

Browser host lifecycle is deliberately implemented by
[`@fluxel/browser`](https://github.com/fluxel-project/fluxel-jsbridge/tree/main/packages/browser):
DOM canvas, CSS × DPR sizing, RAF, visibility, and browser context-loss are not
native-host concerns and must not be duplicated here.

This platform runtime is intentionally sufficient for RHI examples and
conformance runners, not a complete application runtime. DPI policy, input,
clock, Android/iOS package bootstraps, and playable-application orchestration
are not provided by this foundation.

## Planned milestones

The native-host plan is to prove runnable application composition as real
targets require it, while preserving the platform-library boundary:

- add application-layer composition that can combine host lifecycle with
  rendering without making platform-library crates depend on rendering;
- establish platform packaging and lifecycle evidence for Windows, Android, and
  iOS; and
- grow platform services such as input, time, storage, networking, media, and
  diagnostics only when a target demonstrates their contract.

A runnable application may compose this foundation with rendering at the
application layer without changing the platform-library dependency rule. The
authoritative sequence, target evidence, and cross-repository contracts are
maintained in the [Fluxel roadmap](https://github.com/fluxel-project/.github/blob/main/ROADMAP.md)
and [ecosystem architecture](https://github.com/fluxel-project/.github/blob/main/ECOSYSTEM_ARCHITECTURE.md).
