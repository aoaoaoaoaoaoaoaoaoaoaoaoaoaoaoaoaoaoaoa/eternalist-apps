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

Focus is a presented synchronization fact, not application state. Instrumented
egui controls publish their real response through
`egui_tester_witness::egui::record_response`; painter-only geometry uses
`record_rect` and cannot claim focus. Acceptance may use `Probe::wait_focus` to
fence keyboard traversal, then judges the resulting command, pixels, or durable
effect independently.

An application that adopts the shared command and panel grammar proves at
least one keyboard-only path through its consequential controls. Exercise an
Alt mnemonic, forward and backward traversal inside a panel, Control+Tab panel
movement, Enter or Space activation, one focused adjustable control, text-entry
deferral or explicit command capture, disabled-command refusal, generated help,
Escape closure, and focus restoration where those laws exist in the product.
Modified activation keys must remain available to their exact owner, and
application commands or panel traversal must not bleed through a modal layer.

The release `atelier` plus `atelier_acceptance` examples are the library's own
executable specimen. Their hermetic Linux story drives the public command,
guide, panel-navigation, Poolrooms, host, witness, and egui-tester surfaces as
one optimized product path. Product stories remain necessary because the
atelier cannot prove product-specific consequences or persistence.

## Containment And Motion

The harness owns a private display, XDG tree, process group, network policy,
action transcript, last-good capture, and bounded failure artifacts. Fixture
seeding occurs outside the product or through public product behavior.

Poolrooms water and lawful animation remain enabled. Never wait for pixel
quiescence or require exact whole-frame equality. Synchronize on a presented
semantic predicate, then use tolerant bounded-region image evidence or a
durable external oracle. Performance runs use representative host graphics;
software graphics proves deterministic behavior only.
