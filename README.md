# Eternalist Apps

`eternalist-apps` is an application kit for Eternalist-style egui products. It
owns the native winit/egui/wgpu lifecycle and proved high-level logical
primitives that give applications a uniform interaction grammar without taking
authority over their domain.

Brass Poolrooms remains the independently usable low-level visual and physical
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
| `NativeApp` | native | the explicit hooks consumed by the host, including drawing, water sealing, GPU resource registration, post-present settlement, one-shot reveal and exit signals, rendering-independent service deadlines, optional crash-report enrollment, and optional one-way observation | domain state, service meaning, workers, persistence projection, typed command consequences, product state-directory selection, tray behavior, and dialogs |
| `run` and `run_with` | native | one-window winit/egui/wgpu lifecycle, optional pre-construction recovery boundary, Poolrooms-water composition, focus- and concealment-aware repaint authority, rendering-independent service wakeups, surface recovery, responsiveness spans and trace deadline, local crash-capsule recovery, explicit report consent, and optional post-present witness publication | construction policy and dependencies, product shutdown work, multi-window policy, and faults outside the recoverable Rust host boundary |
| `CrashProduct` and `CrashReportSpec` | native | a closed report identity, one bounded local capsule, sanitized panic evidence, next-launch consent, and one short background delivery attempt | enrollment, XDG state placement, server operation, returned host or environmental errors, and every fault that cannot execute a Rust panic hook |
| `ApplicationHeader` and `ApplicationHeaderResponse` | renderer-neutral | persistent application identity opposite right-justified Help and Settings actuators above a control surface | application name, placement of that control surface, settings condition, and product-specific witnesses |
| `Inspector`, `InspectorResponse`, and `inspector::WIDTH` | renderer-neutral | optional fixed left-rail geometry, F9 and a zero-layout hover-revealed boundary actuator, translated slide motion, vertical scrolling, application return value, and shared water forcing | sections, fold state, actions, and persistence |
| `commands::{CommandCanon, CommandSpec, CommandScope, CommandStatus, CommandDispatch, CommandButtonResponse, Shortcut, ShortcutKey, ShortcutModifiers, TextFocusPolicy, RepeatPolicy}` | renderer-neutral | validated typed command metadata, stable IDs, portable exact accelerators, reserved-idiom interlocks, visible Alt mnemonics, context precedence, disabled refusal, text-focus and repeat ownership, generated buttons, and one effective binding projection | command enum and scope enum, dynamic availability, domain execution, feedback presentation, and keymap persistence |
| `command_guide::{CommandGuide, GuideSection, GuideGesture}` | renderer-neutral | F1/question-mark discovery, focus-restoring modal help generated from the command canon, current/all pages, and universal keyboard and guide sections | every target-specific section in product language, scope names, command availability, and domain help beyond interaction guidance |
| `settings::{SettingsSheet, SettingsFile, SettingsUi, SettingSpec, SettingsResponse}` | renderer-neutral | shared F2 and platform settings accelerators, focus-restoring and wheel-owning central sheet, persistent Poolrooms actuator, grouped preference layout, configuration-fault presentation, reload action, and semantic witnesses | setting declarations and values, storage, validation, reload execution, and which controls also appear in context |
| `panel_navigation::{PanelNavigator, PanelFrame, PanelResponse}` | renderer-neutral | one active inspector panel, Poolrooms section composition, contained Tab/Shift+Tab traversal, Control+Tab panel traversal, pointer activation, and dynamic-panel reconciliation | panel identity, ordering, contents, fold defaults, persistence, and application actions |
| `LivingWait` | renderer-neutral | one-frame largest-region arbitration and the standard central loading bouncer | task ownership, progress, cancellation, retry, error handling, and copy beyond the standard bouncer |
| `Cabinet`, `CabinetEntry`, `CabinetKey`, `CabinetAction`, `CabinetBerth`, `CabinetShelfBerth`, `CabinetShelf`, `CabinetEntryEdit`, and `CabinetShelfEdit` | renderer-neutral | globally unique entry identity, root and one-level shelf ordering, entry and shelf drag berths, shelf folds and naming, optional inline entry renaming, shared Poolrooms body, and semantic targets | entry meaning, active-document policy, product commands, storage projection, and persistence timing |
| `TraceGuard` and `responsiveness::{DrainBudget, Drain}` | native | opt-in production-path trace capture, named instrumentation spans, over-budget reporting, and item-plus-wall-clock admission of worker results | product event meaning, stale-generation rejection, optimization policy, representative hardware, and product latency thresholds |
| `responsiveness::{superseding_channel, SupersedingSender, SupersedingReceiver}` | native | a nonblocking one-slot, latest-demand-wins mailbox for single-producer work whose queued predecessors have become worthless | demand meaning, cancellation of work already claimed, result transport, and stale-result rejection |
| `NativeWake` | native | reliable cross-thread control wakes plus unconditional and foreground-only frame requests that cannot be stranded behind egui's coalesced repaint state | the meaning of the signal, whether an unfocused frame is warranted, and the application state consumed after it wakes |
| `SettledScribe` and `ScribeOutcome` | native | restartable settlement timing, immediate nonblocking submission, sequenced latest-snapshot coalescing, background writes, completion wakeups, and a blocking final retirement receipt | serialized projection, storage paths and format, atomic-write law, fault copy, and dirty-domain selection |
| `configuration::{Configuration, ConfigurationLedger, ConfigurationFault}` | native | strict typed TOML admission, unknown-key rejection, semantic validation seam, settled worker writes, per-key optimistic merge, surgical scalar edits, symlink-respecting atomic replacement, explicit reload, and blocking fault state | product schema and defaults, platform-correct path, semantic invariants, setting copy, and deciding when external edits are reread |

There is no generic menu registry, general storage backend, command bus, worker runtime,
service locator, or keymap editor in the crate today. `SettledScribe` owns only
the common timing and thread boundary of persistence; it neither knows nor
chooses what, where, or how an application stores.
`ConfigurationLedger` is the deliberate narrow exception: it owns the complete
native TOML mechanics for application settings while the product owns the typed
schema and path. Invalid syntax, types, semantics, and unknown keys block UI
mutation and are never laundered into a rewritten file.
The command canon describes and routes typed application commands; it never
executes them. Its stable IDs and optionally serializable shortcut values leave
one clean effective-binding seam for future persisted keymaps without adding
override storage or an editor prematurely. `Cabinet` is a
persistence-neutral collection law, not a repository: it accepts an explicit
product projection and returns actions for the product to interpret. Other
logical nouns belong here only after a real common law is proved; they do not
advertise latent APIs.

The native host compiles on Linux/X11, Linux/Wayland, macOS, and Windows and
selects exactly one wgpu backend for each operating system: Vulkan, Metal, or
DX12. That portability is a library contract, not a product support claim;
each application still proves its declared coordinates through its own native
acceptance and lifecycle evidence. The WebGPU atelier below is a design
surface for renderer-neutral primitives, not a web application-host claim.

The optional `egui-test` feature adds one-way observation and response anchors,
post-present witness publication inside `run`, and an isolated-endpoint
constructor used only by the crash-path acceptance. It does not alter the
renderer-neutral surface or production endpoint.

## Visual Atelier

The tabbed atelier is the living visual contract for `Inspector`, `LivingWait`,
`Cabinet`, `SettingsSheet`, and the command/help/panel-navigation assembly. Its
native path is a thin `NativeApp` over the public `run` host.
The browser build executes the same egui composition over Poolrooms' direct
water render graph through an example-local host; no web host is exported.
Eternalist permanently hosts this exact example bundle at
`https://eternalist.moe/demos/eternalist-apps/`. That public witness does not
make the library a portfolio project or establish a supported web surface.

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
scripts/test-atelier
scripts/audit
```

The library compiles and tests its own laws. A native product adopter must also
exercise lifecycle and high-level primitives through its optimized black-box
acceptance stories. The Linux `atelier_acceptance` example drives Alt
mnemonics, exact command refusal and capture, focus-contained panel traversal,
F9 inspector concealment, generated help, focus restoration, and focused or
hovered value adjustment through a
private egui-tester X11 universe. `scripts/check` includes the release
WebAssembly build and browser-bundle integrity check for the atelier.
After deploying an isolated fault-intake stack,
`scripts/test-crash-report INTAKE_URL REPORTS_BUCKET` detonates a real native
process, restarts it, clicks consent, crosses the public network boundary,
compares the stored object byte-for-byte with the local capsule, and deletes
the specimen. It uses the ordinary admission gate; there is no test bypass.

## License

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at
your option.
