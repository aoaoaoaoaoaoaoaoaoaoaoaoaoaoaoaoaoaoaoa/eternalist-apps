# Architecture

`eternalist-apps` owns a native process boundary, not an application
framework. Its public laws are deliberately few.

## Ownership

| Owner | Responsibility |
| --- | --- |
| application | domain model, commands, project state, persistence, background work, product UI, fixtures, oracles, and acceptance stories |
| `eternalist-apps` | native event loop, window and surface lifecycle, egui/wgpu submission, Poolrooms water composition, post-present witness publication, trace spine, and opt-in inspector geometry |
| Dwemer Poolrooms | fonts, palette, widgets, chrome, water primitives, and physical response |
| `egui-tester` | process containment, native input, capture, synchronization, timing, and failure artifacts |
| product contract crate | dependency-light semantic names and wire values shared by the GUI and its acceptance executable |

Applications may depend directly on Poolrooms. The host must not wrap every
visual primitive or prevent application-specific flourish.

## Native Seam

`NativeApp` admits one frame builder, post-present settlement, water
composition, GPU callback registration, and an observation type when the
`egui-test` feature is enabled. It does not admit domain callbacks, panel
registries, persistence hooks, or a service locator.

`after_present` is the only host-owned commit fence. Return `true` when the
commit requires another frame. Expensive preparation, filesystem work, and
complete queue drains never belong there.

`Inspector` is optional. It owns fixed left-rail geometry and vertical scroll
behavior only. An application chooses whether it exists, what it contains,
which sections are open, how state persists, and how scrolling agitates water.
A canvas-only application uses no inspector API.

## Promotion Law

Shared code crosses this repository boundary only through:

```text
incubate in an application
→ prove with executable evidence
→ encounter another live consumer
→ state the common law
→ extract
→ migrate every adopter
→ delete every local rival
```

Structural resemblance is insufficient. Do not add general helpers, domain
widgets, cartography, contract macros, capability registries, or panel
archetypes until a second adopter proves the same semantics and failure
contract.

## Platform Coordinate

Linux/X11 is the sole current native coordinate. Wayland, macOS, Windows,
multi-window orchestration, tray behavior, and native dialogs remain outside
the crate's claim until a product needs and proves them.
