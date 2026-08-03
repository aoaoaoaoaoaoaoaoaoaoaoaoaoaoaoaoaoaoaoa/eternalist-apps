# Native Verification

Each acceptance scenario is a complete user story executed against the
optimized product path. The acceptance executable lives in the product
repository, has no dependency on product internals, and controls the real
binary through `egui-tester`.

## Contract Boundary

Create a small product contract crate shared by the GUI and acceptance
executable. It owns stable semantic targets, closed wire enums, and a schema
fingerprint. It does not depend on egui, the product implementation, or
`egui-tester`.

The product's `egui-test` feature may add only one-way anchors and observations.
It must not change defaults, layout, timing policy, authority, persistence, or
offer a mutation channel. Every launch observation includes the schema
fingerprint and enough launch identity to reject stale frames.

## Story Law

Every consequential step separates:

1. a native gesture against a semantic target;
2. a later presented observation used only to release the wait;
3. an external verdict such as pixels, project files, process state, protocol
   traffic, or recovery after a cold restart.

An observation cannot satisfy the verdict it describes. Functional timeouts
bound hangs; production budgets independently measure from input through
surface presentation or another external effect.

The standard first suite covers:

1. boot to externally visible readiness;
2. one ordinary durable state transition;
3. restart and restoration;
4. one product-defining rich gesture;
5. inert launch without the observational feature.

Add stories by user obligation, not widget count. Prefer AccessKit identity for
ordinary accessible controls when it is stable; retain typed custom targets
for canvases and domain geometry that accessibility cannot honestly name.
Checked-in xdotool choreography is forbidden.

## Containment And Motion

The harness owns a private display, XDG tree, process group, network policy,
action transcript, last-good capture, and bounded failure artifacts. Fixture
seeding occurs outside the product or through public product behavior.

Poolrooms water and lawful animation remain enabled. Never wait for pixel
quiescence or require exact whole-frame equality. Synchronize on a presented
semantic predicate, then use tolerant bounded-region image evidence or a
durable external oracle. Performance runs use representative host graphics;
software graphics proves deterministic behavior only.
