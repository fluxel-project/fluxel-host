# Fluxel Host

`fluxel-host` is the monorepo for native Fluxel application hosts that produce
Windows executables, Android APK/AAB packages, and iOS IPA applications.

## Scope

This repository owns the native platform capabilities that turn rendering into
a runnable application. Its internal crates and host packages are extracted
only as real targets prove the boundary; the outline below is not a promise
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
RenderGraph behavior, renderer `Surface`/swapchain ownership, or the public cross-platform language SDK. It consumes
the rendering library and supplies native capabilities around it. Browser and
mini-game adapters, along with their developer-facing JavaScript API, belong to
`fluxel-jsbridge`, not to this repository.

## Dependencies

`fluxel-host` depends one way on `fluxel-bases` for shared mechanisms and on
`fluxel-rendering` for rendering. Neither dependency may import this
repository. A host creates and owns platform resources, loads data, drives
application frames, and passes the required surface and prepared inputs to the
renderer.

## Status and roadmap

This repository records the native-host ownership boundary ahead of the Windows
playable-runtime work; it does not claim completed EXE, APK/AAB, or IPA support.
The current Stage 1.1 slice provides only a Windows `Window` primitive: fixed
non-zero client extent, non-blocking message pump, close observation, explicit
RAII destruction, and `raw-window-handle` 0.6 interoperability. It deliberately
does not yet create a complete host runtime or a playable application.
The authoritative sequence, target evidence, and cross-repository contracts are
maintained in the [Fluxel roadmap](https://github.com/fluxel-project/.github/blob/main/ROADMAP.md)
and [ecosystem architecture](https://github.com/fluxel-project/.github/blob/main/ECOSYSTEM_ARCHITECTURE.md).
