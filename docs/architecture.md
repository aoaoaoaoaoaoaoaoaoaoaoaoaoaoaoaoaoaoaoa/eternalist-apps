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
| application | domain model, typed command values and consequences, workers, product persistence projections, unpromoted UI, fixtures, oracles, and acceptance stories |
| `eternalist-apps` | native lifecycle and reusable logical application primitives: command metadata and routing, inspectors, managers, menus, storage interactions, loading assemblies, and other proved application-scale state machines |
| Brass Poolrooms | independently usable low-level physical GUI: geometry, material, buttons, rollers, sliders, tiles, frames, intrinsic control interaction, and water response |
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

## Crash Recovery

A product enrolls through `NativeApp::crash_reports` and supplies its lawful
state directory. Fallible products enter through `run_with`, which arms
recovery before their constructor runs. The host retains at most one private,
bounded JSON capsule.
It contains only closed product and platform identity, a fault class, a
source-relative panic location when available, and sanitized function names.
Panic text is excluded because it may contain paths, input, or domain state.

The next launch places a modal consent surface above the ordinary application.
The exact payload is inspectable. Declining deletes the capsule; consenting
performs one five-second delivery attempt on a worker thread. Failure keeps the
capsule for an explicit retry or discard. There is no automatic retry, always-
send mode, usage telemetry, ambient state collection, or networking before the
user's gesture. Application input is quarantined while the modal owns the
surface.

This is recoverable Rust-panic reporting, not a process oracle. It records
panics after the native host is armed. Returned construction, event-loop,
window, GPU, rendering, tracing, and witness errors are deliberately excluded:
a generic terminal error does not prove that a non-bespoke code change can
repair the affected machine. It also cannot record an out-of-memory kill,
abort, segmentation fault, external signal, machine loss, or a panic before
either native entry point is called. Unknown failures are unreported rather
than laundered into an actionable category.

Every native release coordinate executes the filesystem and transport seams:
starting from a nonexistent state directory, it persists and reloads a real
capsule, then sends a deliberately invalid body through the production HTTPS
edge and requires the closed refusal response. The probe stores no report.
Linux separately proves the complete crash, restart, consent, delivery, and
stored-object equality path through a hermetic GUI process.

`Inspector` is optional. It owns fixed left-rail geometry, vertical scrolling,
session visibility, F9 and zero-layout hover-revealed boundary-actuator routing, translated slide motion,
and the resulting button, scroll, and moving-wall water law. An application
chooses whether it exists, what it contains, which sections are open, and how
domain state persists. A fully concealed inspector does not evaluate its body;
the body's `Default` result is the empty application action. A canvas-only
application uses no inspector API.

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

## Command Canon

An application declares a closed command value, a closed context value, and a
static `CommandSpec` for each command. `CommandCanon` validates stable
lowercase dotted IDs and rejects duplicate command values, IDs, mnemonics, or
bindings wherever scopes overlap. It also rejects chords owned by shared help,
panel traversal, or focused-control interaction; those target-relative
gestures never enter global command routing. The application still executes
every `CommandDispatch`; the canon has no domain callback or command bus.

Routing consumes only an exact chord and its matching textual projection.
Active contexts are ordered from most to
least specific and outrank global commands. Hidden commands relinquish their
chord; disabled commands own it and return their refusal reason. Text entry
receives a command by default unless that command explicitly declares capture.
Fresh keys dispatch once, rejected repeats are consumed, and repeatable
commands dispatch at most once per input frame. Generated buttons accept
pointer, accessibility, or fresh unmodified Enter/Space activation, so a
modified chord cannot leak through the focused actuator. A preceding modal
layer suspends ordinary application command routing without consuming its
keys. An application-owned modal may route its own context through
`route_in_modal`; the route remains dormant when any later modal is above its
declared layer.

`CommandSpec::default_shortcuts` names only the shipped declaration.
`CommandCanon::shortcuts`, routing, generated button legends, and the command
guide all consult the canon's effective projection. Stable command IDs and
optionally serializable `Shortcut` values make that projection the sole future
insertion point for persisted keymaps. No override storage, merge law,
conflict UI, or keymap editor exists yet.

Alt mnemonics are separate from replaceable accelerators: the declared glyph is
permanently underlined by Poolrooms and its exact Alt chord is validated with
the rest of the canon. `CommandGuide` renders the same metadata and dynamic
availability used by routing. F1 always toggles the guide; question mark
defers to focused text entry. Closing the modal restores its prior focus target
when that target remains available. While open, the guide owns wheel input as
well as pointer and keyboard interaction: `take_shortcuts` quarantines wheel
motion before application layout and `show` returns it only to the guide's
scroll surface. Universal keyboard and guide sections are automatic. Every
target-specific gesture section is application-owned, written in product
vocabulary, and supplied only where its target exists. Low-level physical
control classes never export inheritable help sections.

## Panel Grammar

`PanelNavigator` composes Poolrooms `Section` disclosures into one active
inspector panel. Tab and Shift+Tab cycle through its header and focusable
contents. A collapsed panel has no interior cycle, so Tab and Shift+Tab move
from its header to the adjacent panel header. Physical Control+Tab and
Control+Shift+Tab move to the next or previous panel header regardless of fold
state; Control is deliberate because Command+Tab belongs to the macOS
application switcher. Pointer engagement or header focus activates a panel.
Dynamic insertion or removal is reconciled after each frame. Modal layers
suspend panel traversal and F9 inspector concealment, leaving their keys to the
topmost layer.

Traversal does not create a widget registry. The caller supplies stable panel
IDs, presentation order, contents, fold defaults, and water handling through
ordinary composition. Controls outside the navigator retain ordinary egui
focus behavior and are never pulled into an active panel's Tab loop.

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

The native host compiles for Linux/X11, Linux/Wayland, macOS, and Windows and
admits only the operating system's native wgpu backend: Vulkan, Metal, or DX12.
Compilation establishes library portability, not an application's support
claim. Each product must prove startup, first presentation, installation, and
its own native behavior on every coordinate it advertises. Multi-window
orchestration, tray behavior, and native dialogs remain outside this crate's
claim. Logical UI primitives do not acquire native assumptions merely because
they share a release with the host.
