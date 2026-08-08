# Architecture

`eternalist-apps` supplies an application grammar for Eternalist-style products:
native lifecycle plus reusable high-level logical UI primitives. Its north star
is a product whose GUI is thin, explicit domain glue over these primitives and
whose behavior is verified externally through `egui-tester`.

This is a library-shaped DSL, not a total application schema. Products may
always descend to raw egui or Poolrooms when no shared law exists.

## Ownership

| Owner | Responsibility |
| --- | --- |
| application | domain model, commands, workers, product persistence projections, unpromoted UI, fixtures, oracles, and acceptance stories |
| `eternalist-apps` | native lifecycle and reusable logical application primitives: inspectors, managers, menus, storage interactions, loading assemblies, and other proved application-scale state machines |
| Dwemer Poolrooms | independently usable low-level physical GUI: geometry, material, buttons, rollers, sliders, tiles, frames, intrinsic control interaction, and water response |
| `egui-tester` | process containment, native input, capture, synchronization, timing, and failure artifacts |
| product contract crate | dependency-light semantic names and wire values shared by the GUI and its acceptance executable |

Ownership follows the governing invariant, not the everyday noun. Poolrooms
owns how a menu actuator is embodied; Eternalist owns the menu's logical model,
routing, storage, and placement. Eternalist may depend on Poolrooms. Poolrooms
must never depend on Eternalist and must remain sufficient for unrelated native
or WebGPU applications that use another application grammar.

Applications may depend directly on Poolrooms. Eternalist must not wrap every
physical mechanism or prevent product-specific composition.

The complete shipped surface is enumerated in the
[README](../README.md#present-surface). Architectural examples name ownership
territory, not hidden or promised APIs.

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

`Cabinet` is also optional. It owns a persistent collection's global entry
identity, root and one-level shelf order, entry and shelf drag placement, shelf
folds and naming, optional inline entry-name editing, and their common
Poolrooms body. Renaming is an opt-in projection that emits a refined,
collision-free action; it is not a callback executed during layout. An
application owns what each entry means, which entry is active, how actions
alter the domain, and how the cabinet is projected into product storage. It is
neither a storage backend nor a document manager. `serde` support is opt-in
because serialization is useful to some projections but is not part of the
collection law.

## Application Primitives

A high-level primitive owns one reusable logical interaction law. It accepts
explicit state and dependencies, composes Poolrooms mechanisms, emits standard
witness anchors, and returns typed responses or actions for the product to
interpret. It may own persistence-neutral UI state. It does not call domain
commands, discover product services, or dictate a product storage schema.

Primitives are ordinary modules in this crate by default. A new crate is
justified only by a materially different dependency universe, target claim, or
release authority, not by the existence of another reusable widget.

No global panel registry, service locator, declarative product schema, or
closed inventory of application roles is admitted. Shared primitives must
remain independently composable with raw egui, Poolrooms, and product-local UI.

## Promotion Law

Shared code crosses this repository boundary through either promotion gate:

1. Two applications use the primitive with the same behavioral and failure
   law, and a further independent reuse is plainly expected.
2. Three applications use the primitive identically, whether or not that reuse
   was predicted.

The complete promotion is:

```text
incubate in an application
→ prove the common behavioral and failure law with executable evidence
→ satisfy a promotion gate
→ state the common law
→ extract
→ migrate every adopter
→ delete every local rival
```

Structural resemblance is insufficient. Product nouns and speculative options
remain local. A promoted primitive is the smallest law common to its adopters,
not a configurable memorial of every local variation.

## Platform Coordinate

Linux/X11 is the sole current native-host coordinate. Wayland, macOS, Windows,
multi-window orchestration, tray behavior, and native dialogs remain outside
that claim until a product needs and proves them. Logical UI primitives should
not acquire native assumptions merely because they share a release with the
host.
