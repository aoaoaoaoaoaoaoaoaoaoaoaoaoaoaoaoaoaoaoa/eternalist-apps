# CI And Release Evidence

[`foundry.toml`](../foundry.toml) declares this crate's executable evidence.
The pinned Foundry workflow schedules those product-owned commands; workflow
YAML does not invent additional capability, and every command remains runnable
locally.

| Gate | Evidence | Claim |
| --- | --- | --- |
| `scripts/check` | formatting, strict native all-target/all-feature lints, browser-target linting, tests, warning-free rustdoc, release WebAssembly compilation, and browser-bundle integrity | the checked source and documented package surface cohere |
| `scripts/test-atelier` | optimized Atelier under egui-tester's private X11, XDG, process, and software-Vulkan universe | command routing, focus traversal, generated help, Poolrooms controls, witness focus, and the native host compose end to end |
| Foundry `host` proof | ordinary host compilation and tests on Linux, macOS, and Windows | portability regressions are caught; runtime support is not implied |
| `scripts/audit` | the locked dependency graph against the checked policy | known dependency advisories are adjudicated |
| `cargo package --locked` | the exact publishable archive and its dependency resolution | crates.io receives the intended source, examples, and documentation |

The WebGPU gate proves that the renderer-neutral primitives, atelier host, and
static browser artifact compile together. Hosted CI does not turn that design
surface into a supported web application host or prove hardware rendering.

The Atelier story proves only this library's shared interaction assembly. This
repository does not own an adopting product's installation, uninstallation,
XDG persistence, workers, latency budgets, or domain stories. An application
proves those obligations in its own repository according to [Native
Verification](verification.md). Compilation on a matrix coordinate does not
create a platform claim.

The `native-acceptance` proof requests Foundry's private X11 substrate on the
Linux coordinate, then invokes `scripts/test-atelier`. Display setup belongs to
the scheduler; the interaction story and its verdict remain owned here.

## Library Releases

A release identifies one exact source commit. Publish only from a clean,
tagged checkout after `scripts/check`, `scripts/audit`, and `cargo package
--locked` pass. The manifest version, tag, packaged metadata, and Git commit
must identify the same source. `--allow-dirty` is forbidden.

Publication claims only that this package satisfies its documented contract.
Applications adopt a release deliberately and prove the resulting product
through their own source, lifecycle, and acceptance gates.
