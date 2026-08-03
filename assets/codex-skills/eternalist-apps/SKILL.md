---
name: eternalist-apps
description: Bootstrap, retrofit, verify, or release an Eternalist-style native Rust egui application using eternalist-apps, Dwemer Poolrooms, egui-tester, XDG product conduct, responsiveness doctrine, and capability-honest CI. Use when creating a new Eternalist app, migrating an existing egui/wgpu/winit app onto the shared host, adding the standard optional inspector layout, establishing native user-story acceptance tests, or preparing an app for fleet conformance and release.
---

# Eternalist Apps

Treat the `eternalist_apps` repository as the source of truth. This skill is a
router, not an independent framework specification.

## Grounding

1. Read the target repository's `AGENTS.md` and current implementation.
2. Load `$style-doctrine`, `$product-doctrine`, and `$ui-doctrine`. Load
   `$rust-bootstrap` for a fresh Rust project or lint retrofit.
3. Read [architecture](../../../docs/architecture.md) and
   [design language](../../../docs/design-language.md).
4. Inspect the current `eternalist-apps`, Dwemer Poolrooms, and `egui-tester`
   APIs. Do not code from remembered versions.
5. Classify the work as fresh bootstrap, retrofit, verification, or release.

## Fresh Bootstrap

Read [fresh bootstrap](../../../docs/bootstrap-fresh.md),
[verification](../../../docs/verification.md),
[responsiveness](../../../docs/responsiveness.md), and
[CI](../../../docs/ci.md). Implement the smallest useful product end to end:

- declare product-owned data, XDG artifacts, platform coordinates, and first
  useful publication;
- compose the product through `NativeApp` and Poolrooms directly;
- add `Inspector` only when persistent left-side controls or libraries earn it;
- keep corpus-scale work and durable writes off the event loop;
- create a dependency-light product contract plus product-owned acceptance
  executable;
- prove boot, one durable action, restart, one rich native gesture, install,
  uninstall, and the declared platform coordinate.

## Retrofit

Read [retrofit](../../../docs/bootstrap-retrofit.md) and the same verification
documents. Establish black-box stories before replacing the host. Migrate one
lifecycle, preserve product behavior, then delete the old event loop, renderer,
trace spine, and witness publisher. A compatibility shell or product-named host
branch is a failed extraction.

## Guardrails

- The GUI is the product frontend. A CLI, when present, is a debug frontend
  over the same engine.
- Poolrooms owns visual and physical primitives. `eternalist-apps` owns native
  host mechanics and optional macroscopic layout, not application chrome.
- Product repositories own domain contracts, state, fixtures, oracles, and
  acceptance stories.
- Witnesses synchronize; external effects and rendered evidence judge.
- Production motion remains enabled. Never use whole-frame pixel equality or
  stillness as readiness.
- X11 is the sole current native coordinate. Do not imply Wayland, macOS, or
  Windows support from compilation.
- Do not extract a mechanism because it may become useful. Require a proved
  law and a live adopter, then delete every local rival.
- Do not impose an inspector, bottom shelf, tray, project model, cartography,
  or persistence scheme on an app whose product ontology does not require it.

Before finishing, run the app-owned source, audit, lifecycle, and native
acceptance commands. Use `$x11-gui-testing` for native inspection and
`$release-inquest` before publication. Report any claimed coordinate that lacks
executable evidence.
