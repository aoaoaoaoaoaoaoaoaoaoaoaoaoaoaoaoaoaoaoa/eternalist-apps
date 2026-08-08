# Eternalist Apps

`eternalist-apps` is an application kit for Eternalist-style egui products. It
owns the native winit/egui/wgpu lifecycle and proved high-level logical
primitives that give applications a uniform interaction grammar without taking
authority over their domain.

Dwemer Poolrooms remains the independently usable low-level visual and physical
substrate. It owns buttons, rollers, sliders, tiles, frames, material response,
and living water. Eternalist primitives compose Poolrooms mechanisms; Poolrooms
never depends on Eternalist. Products retain their domain model, workers,
product persistence projections, contracts, fixtures, oracles, and any UI whose
reuse law has not yet earned promotion.

The north star is an application written as thin, explicit domain glue over
typed Eternalist primitives and verified from outside through `egui-tester`.
Raw egui and Poolrooms remain lawful escape hatches: the kit supplies a
library-shaped DSL, not a registry-shaped framework.

## Present Surface

| Surface | Coordinate | Owns | Leaves To The Application |
| --- | --- | --- | --- |
| `WindowSpec`, `ResponsivenessSpec`, and `CloseDisposition` | native | initial window identity and geometry, ordinary frame-work budget, and close-request vocabulary | dynamic title, product close policy, and every window beyond the sole host window |
| `NativeApp` | native | the explicit hooks consumed by the host, including drawing, water sealing, GPU resource registration, post-present settlement, and optional one-way observation | domain state, workers, persistence, commands, tray behavior, and dialogs |
| `run` | native | one-window winit/egui/wgpu lifecycle, Poolrooms-water composition, repaint scheduling, surface recovery, responsiveness spans and trace deadline, and optional post-present witness publication | application construction, product shutdown work, multi-window policy, and recovery after a terminal host error |
| `Inspector`, `InspectorResponse`, and `inspector::WIDTH` | renderer-neutral | optional fixed left-rail geometry, vertical scrolling, application return value, panel response, and resulting scroll offset | sections, fold state, actions, persistence, and water forcing |
| `LivingWait` | renderer-neutral | one-frame largest-region arbitration and the standard central loading bouncer | task ownership, progress, cancellation, retry, error handling, and copy beyond the standard bouncer |
| `Cabinet`, `CabinetEntry`, `CabinetKey`, `CabinetAction`, `CabinetBerth`, `CabinetShelfBerth`, `CabinetShelf`, `CabinetEntryEdit`, and `CabinetShelfEdit` | renderer-neutral | globally unique entry identity, root and one-level shelf ordering, entry and shelf drag berths, shelf folds and naming, optional inline entry renaming, shared Poolrooms body, and semantic targets | entry meaning, active-document policy, product commands, storage projection, and persistence timing |
| `TraceGuard` and `responsiveness` | native | opt-in production-path trace capture, named instrumentation spans, and over-budget reporting | optimization policy, representative hardware, and product latency thresholds |

There is no generic menu model, storage backend, command bus, worker runtime,
service locator, or persistence framework in the crate today. `Cabinet` is a
persistence-neutral collection law, not a repository: it accepts an explicit
product projection and returns actions for the product to interpret. Other
logical nouns belong here only after a real common law is proved; they do not
advertise latent APIs.

The native host's sole release-tested coordinate is Linux/X11. The WebGPU
atelier below is a design surface for renderer-neutral primitives, not a web
application-host claim.

The optional `egui-test` feature adds only `NativeApp::Observation`,
`NativeApp::observe`, and post-present witness publication inside `run`. It
does not alter the renderer-neutral surface.

## Visual Atelier

The tabbed atelier is the living visual contract for `Inspector`, `LivingWait`,
and `Cabinet`. Its native path is a thin `NativeApp` over the public `run` host.
The browser build executes the same egui composition over Poolrooms' direct
water render graph through an example-local host; no web host is exported.

```sh
cargo run --example atelier
scripts/web-atelier serve
```

Open `http://127.0.0.1:4174`. Pass another port as
`scripts/web-atelier serve 8080`; an occupied interactive port yields a printed
ephemeral address rather than displacing another process.

`scripts/web-atelier build` emits the static browser bundle beneath Cargo's
target directory. It requires WebGPU rather than silently falling back to a
software canvas.

For a persistent fixed address, install the repository-aware systemd user
service:

```sh
scripts/atelier-service install        # http://127.0.0.1:4174
scripts/atelier-service install 8080   # choose another fixed port
```

The installer builds once, writes
`$XDG_CONFIG_HOME/systemd/user/eternalist-apps-atelier.service` (falling back
to `~/.config/systemd/user`), and enables it for the user session. The bundle
remains rebuildable Cargo target output; every later
`scripts/web-atelier build` or `scripts/check` replaces it atomically and
becomes visible on browser refresh without restarting the service. A fixed-port
collision fails instead of silently changing the address. Inspect or remove the
integration with `scripts/atelier-service status` and
`scripts/atelier-service uninstall`.

## Adoption

Use the bundled `eternalist-apps` bootstrap skill for a fresh application or a
retrofit. Its source lives at
[`assets/codex-skills/eternalist-apps`](assets/codex-skills/eternalist-apps).

The remaining contracts are documented in:

- [architecture](docs/architecture.md)
- [fresh bootstrap](docs/bootstrap-fresh.md)
- [retrofit](docs/bootstrap-retrofit.md)
- [verification](docs/verification.md)
- [responsiveness](docs/responsiveness.md)
- [CI and release evidence](docs/ci.md)

## Verification

```sh
scripts/check
scripts/audit
```

The library compiles and tests its own laws. A native product adopter must also
exercise lifecycle and high-level primitives through its optimized black-box
acceptance stories. `scripts/check` includes the release WebAssembly build and
browser-bundle integrity check for the atelier.

## License

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at
your option.
