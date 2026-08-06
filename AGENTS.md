# Eternalist Apps Agent Guidance

Read and obey the `$style-doctrine`, `$product-doctrine`, and `$ui-doctrine`
skills before changing public code or doctrine.

The crate owns native lifecycle and reusable high-level logical application
primitives. Its north star is a product expressed as thin domain glue over
typed, composable Eternalist primitives. It does not own domain behavior,
workers, product-specific persistence projections, or low-level physical GUI
elements. Dwemer Poolrooms owns that visual and mechanical substrate and must
remain independently usable without Eternalist, including through WebGPU.

Before changing the host lifecycle or latency instrumentation, read
[docs/responsiveness.md](docs/responsiveness.md). Before changing the public
application seam or adding shared machinery, read
[docs/architecture.md](docs/architecture.md). Before changing test support,
read [docs/verification.md](docs/verification.md).

Promote a logical primitive after two applications prove the same law and a
further reuse is evident, or after three applications use it identically.
Promotion requires executable evidence, migration of every adopter, and
deletion of every local rival. Similar source without common semantics remains
local.
