# CI And Release Evidence

CI schedules this crate's own executable evidence; workflow YAML does not
invent additional capability. The canonical commands remain runnable locally.

| Gate | Evidence | Claim |
| --- | --- | --- |
| `scripts/check` | formatting, strict native all-target/all-feature lints, browser-target linting, tests, warning-free rustdoc, release WebAssembly compilation, and browser-bundle integrity | the checked source and documented package surface cohere |
| platform jobs | ordinary host compilation and tests on Linux, macOS, and Windows | portability regressions are caught; runtime support is not implied |
| `scripts/audit` | the locked dependency graph against the checked policy | known dependency advisories are adjudicated |
| `cargo package --locked` | the exact publishable archive and its dependency resolution | crates.io receives the intended source, examples, and documentation |

The WebGPU gate proves that the renderer-neutral primitives, atelier host, and
static browser artifact compile together. Hosted CI does not turn that design
surface into a supported web application host or prove hardware rendering.

This repository does not own product installation, uninstallation, XDG
persistence, workers, latency budgets, or black-box user stories. An adopting
application proves those obligations in its own repository according to
[Native Verification](verification.md). Compilation on a matrix coordinate
does not create a platform claim.

## Library Releases

A release identifies one exact source commit. Publish only from a clean,
tagged checkout after `scripts/check`, `scripts/audit`, and `cargo package
--locked` pass. The manifest version, tag, packaged metadata, and Git commit
must identify the same source. `--allow-dirty` is forbidden.

Publication claims only that this package satisfies its documented contract.
Applications adopt a release deliberately and prove the resulting product
through their own source, lifecycle, and acceptance gates.
