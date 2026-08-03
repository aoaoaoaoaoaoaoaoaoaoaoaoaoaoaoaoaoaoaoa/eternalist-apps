# Eternalist Apps

`eternalist-apps` is the thin native host shared by Eternalist-style egui
applications. It owns the winit event loop, egui/wgpu surface lifecycle,
Dwemer Poolrooms water composition, responsiveness trace spine, and optional
post-present `egui-tester` witness publication.

The crate also supplies an optional fixed-width [`Inspector`] rail. Applications
without a left inspector do not construct one and inherit no panel policy.
Poolrooms remains the visual and physical primitive library; product
repositories retain their domain model, UI, persistence, contracts, fixtures,
and acceptance stories.

Trailgen is the first proving adopter. HRRR and Adequate Booru Viewer are the
next intended migrations; their needs, rather than speculation, govern later
extraction.

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

The library compiles and tests its own laws. A native product adopter must
also exercise the host through its optimized black-box acceptance stories.

## License

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at
your option.
