---
name: eternalist-apps
description: Bootstrap, retrofit, verify, or release an Eternalist-style Rust egui application using eternalist-apps lifecycle and high-level UI primitives, Brass Poolrooms physical controls and water, egui-tester, XDG product conduct, responsiveness doctrine, and capability-honest CI. Use when creating an Eternalist app, migrating duplicated lifecycle or logical application primitives, establishing native user-story acceptance tests, or preparing an app for release.
---

# Eternalist Apps

Treat the `eternalist_apps` repository as the source of truth. This skill is a
router, not an independent framework specification.

## Grounding

1. Read the target repository's `AGENTS.md` and current implementation.
2. Load `$style-doctrine`, `$product-doctrine`, and `$ui-doctrine`. Load
   `$unit-test-doctrine` whenever unit tests are in scope. Load
   `$rust-bootstrap` for a fresh Rust project or lint retrofit.
3. Read [architecture](../../../docs/architecture.md) and
   [design language](../../../docs/design-language.md).
4. Inspect the current `eternalist-apps`, Brass Poolrooms, and `egui-tester`
   APIs. Do not code from remembered versions.
5. Classify the work as fresh bootstrap, retrofit, verification, or release.

## Fresh Bootstrap

Read [fresh bootstrap](../../../docs/bootstrap-fresh.md),
[verification](../../../docs/verification.md),
[responsiveness](../../../docs/responsiveness.md), and
[CI](../../../docs/ci.md). Implement the smallest useful product end to end:

- declare product-owned data, XDG artifacts, platform coordinates, and first
  useful publication;
- compose the product through `NativeApp`, proved Eternalist application
  primitives, and Poolrooms directly where no high-level law exists;
- add `Inspector` only when persistent left-side controls or libraries earn it;
- declare recurring keyboard actions through a typed `CommandCanon`, render
  their labels and help from that canon, and use `PanelNavigator` only when an
  inspector has multiple persistent control panels;
- keep corpus-scale work and durable writes off the event loop;
- create a dependency-light product contract plus product-owned acceptance
  executable;
- establish the smallest product-owned acceptance basis that discriminates
  boot, durable action, restart, rich native interaction, installation,
  removal, and the declared platform coordinate where those risks exist.

## Retrofit

Read [retrofit](../../../docs/bootstrap-retrofit.md) and the same verification
documents. Establish black-box stories before replacing the host. Migrate one
lifecycle, preserve product behavior, then delete the old event loop, renderer,
trace spine, and witness publisher. A compatibility shell or product-named host
branch is a failed extraction.

## Guardrails

- The GUI is the product frontend. A CLI, when present, is a debug frontend
  over the same engine.
- Poolrooms owns independently usable low-level physical GUI mechanisms and
  water. `eternalist-apps` owns native lifecycle and reusable high-level
  logical application primitives. Eternalist may compose Poolrooms; Poolrooms
  must never depend on Eternalist.
- Product repositories own domain contracts, state, fixtures, oracles, and
  acceptance stories.
- Applications own typed command consequences. Eternalist command metadata,
  exact routing, generated guidance, and panel traversal must not become a
  callback bus, widget registry, or speculative keymap editor.
- Command buttons, routing, and help read effective shortcuts through
  `CommandCanon`; declaration defaults are not a second runtime projection.
- Focused controls consume only their exact keys. Text entry receives ordinary
  typing unless a command explicitly declares capture. Tab remains inside the
  active panel; physical Control+Tab crosses panels.
- Witnesses synchronize; external effects and rendered evidence judge.
- Production motion remains enabled. Never use whole-frame pixel equality or
  stillness as readiness.
- The event-loop thread never waits on worker capacity or durable I/O. Give
  result drains item and wall ceilings; use the shared superseding mailbox for
  latest-demand-wins work, and use `SettledScribe` for settled background
  persistence when the product needs it.
- Workers and platform callbacks use `NativeWake` for domain-result repaint,
  reveal, and exit signals. Streams of progress, tiles, thumbnails, or other
  results whose consumption can create more demand use the foreground-only
  repaint methods, leaving bounded-channel backpressure in authority while the
  window is unfocused. Do not treat egui's coalescing repaint request as a
  reliable cross-thread event channel.
- Retries, surveys, persistence settlement, and other semantic clocks use
  `NativeApp::service_deadline`. They never borrow visual repaint cadence and
  must advance every matured deadline before returning to the host.
- Inventory every resident loop. Each needs a named product purpose, blocking
  wait, bounded queues and retained state, terminal or steady-state cadence,
  and shutdown law. Measure elected background services independently from
  domain-quiescent rest; continued domain work never licenses presentation or
  an unmetered resource budget.
- The host is portable across Linux/X11, Linux/Wayland, macOS, and Windows and
  selects one native GPU backend per target. Compilation alone is not a product
  support claim; require app-owned runtime, lifecycle, and installation
  evidence.
- Exclude platform-exclusive dependencies, modules, assets, workers, and
  initialization paths at compile time with target or feature `cfg`s. A runtime
  no-op is not a lean target. Keep runtime capability detection for facts such
  as concealment or tray availability that the OS name cannot decide.
- Every advertised coordinate also requires the real-hardware settled-rest
  preflight in `docs/responsiveness.md`; hosted compilation and software
  rendering cannot prove compositor, driver, or GPU-memory conduct.
- Promote a logical primitive after two applications prove the same law and a
  further reuse is evident, or after three applications use it identically.
  Require executable evidence, migrate every adopter, and delete every local
  rival.
- Do not impose an inspector, bottom shelf, tray, project model, cartography,
  or persistence scheme on an app whose product ontology does not require it.

Before finishing, run the app-owned source, audit, lifecycle, and native
acceptance commands. Use `$x11-gui-testing` for native inspection and
`$release-inquest` before publication. Report any claimed coordinate that lacks
executable evidence.
