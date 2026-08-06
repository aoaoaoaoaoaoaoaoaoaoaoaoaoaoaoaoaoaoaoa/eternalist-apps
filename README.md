# Eternalist Apps

`eternalist-apps` is the shared application kit for Eternalist-style egui
products. It owns the native winit/egui/wgpu lifecycle and the reusable
high-level primitives from which applications acquire a uniform interaction
grammar: inspectors, living waits, managers, menus, storage surfaces, and other
logical assemblies once their common law has been proved.

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

## Adoption

Use the bundled `eternalist-apps` bootstrap skill for a fresh application or a
retrofit. Its source lives at
[`assets/codex-skills/eternalist-apps`](assets/codex-skills/eternalist-apps).

The concrete contracts are documented in:

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
acceptance stories.

## License

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at
your option.
