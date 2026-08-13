# Responsiveness

Responsiveness is a product contract enforced across the host, product, and
acceptance boundary.

**Reaction latency** runs from an admitted native gesture to the first
presented frame that visibly acknowledges it. **Frame work** is event-loop wall
time spent producing one frame. **Cadence** is the distribution of intervals
between presented frames during sustained interaction. These are distinct
measurements.

The event loop performs work proportional only to visible UI, resident
resources, and bounded result drains. Filesystem work, decoding, indexing,
sorting, network acquisition, durable writes, corpus-scale geometry, and GPU
preparation are background work by presumption. Worker results carry generation
identity; drains have item and wall-time ceilings and reject stale work.
`DrainBudget::arm` mints one per-frame allowance that related receivers share;
`Drain::receive` stops polling receivers when either ceiling closes. A receiver
that remains nonempty requests another frame instead of enlarging the current
transaction.

UI-to-worker commands obey the same law in the other direction: the event-loop
thread never waits for mailbox capacity. Use `superseding_channel` when queued
work is serial-tagged or idempotent and only the latest not-yet-claimed demand
retains value. Its single pending slot is replaced atomically; work already
claimed by the consumer remains the application's stale-result problem.

Workers and platform callbacks use `NativeWake`, not an egui repaint request,
to publish domain results, reveal, or exit. Egui deliberately coalesces repaint
requests inside its frame lifecycle; once presentation policy suppresses such
a request, it is not a reliable cross-thread event signal. `NativeWake::wake`
delivers a nonvisual control signal, while `request_repaint` asks for exactly
one policy-governed externally caused frame.

Result streams that may induce more work use
`NativeWake::request_foreground_repaint` or its delayed form. While unfocused,
the wake remains observable but the frame is refused; bounded result channels
then exert backpressure, and focus restoration supplies one catch-up frame.
Unconditional `request_repaint` is reserved for finite, independently bounded
changes whose visible publication still matters while unfocused. A worker
completion must never form a result → frame → fresh-demand oscillator in the
background.

Presentation has three states. A focused window renders on demand. An
unfocused window may admit one frame caused by new input, an OS event, or an
external domain wake, but a repaint requested by that frame cannot produce a
successor. Known minimized, occluded, zero-sized, or deliberately hidden
windows render nothing. Worker repaint requests still wake the event loop so
terminal application signals are observed; concealed presentation resumes
with one fresh frame when the window returns. No visual animation may be the
clock for domain work.

Surface acquisition may transiently report occlusion while a newly mapped
window is becoming presentable. The host admits one finite, backoff-spaced
acquisition barrage, then sleeps until a platform transition supplies fresh
authority. A missing first present cannot become a permanent black window, and
persistent occlusion cannot become a polling cadence.

Semantic clocks use `NativeApp::service_deadline`, never
`request_repaint_after`. The host services a matured deadline even while the
window is concealed and requests a frame only when its visible projection
changed. The callback must advance or retire every matured deadline before it
returns. This separates retries, polling, and persistence settlement from GPU
presentation without suspending the underlying application.

Every resident loop must have a named product purpose, a blocking wait, bounded
mailboxes and retained state, a terminal or steady-state cadence, and an
orderly stop condition. Inventory intentional background services separately
from presentation: a crawler, mirror, cache custodian, or updater may continue
without a window, but that permission does not buy frames or an unmetered CPU,
network, disk, or memory budget. Its activity and pause control must be visible
to the user. Verify both domain-quiescent rest and each elected background
service under load; “the application is not idle” is not an explanation for a
busy loop.

`SettledScribe` is the standard durable-write boundary: mutations restart one
settlement clock, matured snapshots enter a latest-wins background mailbox,
explicit save actions may submit immediately without waiting, outcomes carry
submission sequence identity, and orderly retirement waits for one final
receipt. The product still owns its projection, paths, format, atomic-write
implementation, retry policy, and error language. A failed background write is
reported once rather than establishing an automatic I/O retry cadence; a later
mutation, explicit action, or orderly retirement may try again.

This distinction is the portability fallback. Some window systems do not tell
a client whether it is minimized or fully occluded. Lack of that signal never
licenses an optimistic animation cadence: focus loss suppresses all
frame-originated continuation, while genuine external events remain capable of
publishing one bounded frame. Do not poll platform state to synthesize a
visibility oracle.

Same-window concealment is likewise a capability, not an OS-name guess.
`CloseDisposition::HideOrExit` keeps the process resident only where the host
can both hide and later reveal its native window; an incapable backend follows
ordinary close semantics. A visible but logically concealed frozen window is
never an admissible fallback.

Every animation and physical response owns a finite wake lease. Stable hover,
focus, loading state, or unchanged geometry may not renew that lease merely
because another frame happened. Repaint requests must arise from new input,
new semantic state, or an unexpired visual transition. A settled application
with the pointer resting over a control must return to an event-driven wait.

The native host favors wgpu's memory-usage allocator policy, requests one
frame of swapchain latency, and performs nonblocking device maintenance after
every present. Product GPU caches still require explicit byte ceilings.
Unchanged uniform or texture content must not be uploaded again: temporary
staging allocations participate in the real resident-memory budget even when
the durable GPU payload is bounded.

Publish the smallest useful shell and each independent substrate as soon as
its own prerequisites exist. Later armament must not reset the viewport,
replace the workbench, withdraw already presented content, or erase intervening
input.

The host records canonical spans under `eternalist::*`:

```text
window.event
repaint.callback
repaint.schedule
app.service_deadline
frame
  frame.input
  frame.ui
  frame.platform_output
  frame.tessellate
  frame.water
  frame.render
    render.encoder
    render.prepare
    render.acquire_surface
    render.egui_pass
    render.water_compose
    render.submit
    render.water_after_submit
    render.free_textures
    render.present
    render.maintain
  frame.after_present
```

Products refine this spine with stable semantic spans; source function names
and temporary types are not trace vocabulary. Enable a Chrome/Perfetto trace
with:

```sh
ETERNALIST_TRACE=/tmp/product-trace.json \
ETERNALIST_TRACE_SECONDS=60 \
product
```

Trace instrumentation is dormant when `ETERNALIST_TRACE` is absent.
Acceptance, not product instrumentation, judges performance. Report at least
p50, p95, and worst cadence plus p95 frame work for sustained interactions.
Before release, also observe a fully armed foreground rest, an unfocused rest,
and a concealed rest after every finite deadline has expired. Record CPU over
a fixed interval and RSS at the beginning and end of a representative soak.
When GPU memory is involved, distinguish Rust heap, driver mappings, allocator
allocated bytes, allocator reserved bytes, and device-local memory; RSS or
`nvidia-smi` alone cannot identify the owner.

## Settled-Rest Preflight

Before runtime measurement, inspect each advertised target's compiled product,
not merely its source tree. Platform-exclusive dependencies, modules, assets,
workers, and initialization paths must be excluded with target or feature
`cfg`s when the target cannot use them; a runtime no-op still bloats the binary
and may retain initialization cost. Conversely, do not infer behavior from an
OS name where the window-system capability is observable only at runtime.
Build every claimed target and inspect its enabled feature and dependency graph
before applying the native matrix below.

Run this preflight on real hardware for every advertised window-system and GPU
backend coordinate: Linux/X11/Vulkan, Linux/Wayland/Vulkan, macOS/Metal, and
Windows/DX12 where the product claims them. Compilation, software rendering,
and a hosted virtual runner do not establish native presentation or driver
resource conduct. A backend- or vendor-specific failure expands the matrix to
that affected GPU vendor until the repair has matching evidence.

| State | Required evidence |
| --- | --- |
| Fully armed foreground rest | Leave the pointer over a tension-bearing control and keyboard focus inside the UI. After every declared lease expires, the trace has no new frames until input, a domain event, or a real deadline arrives. |
| Unfocused but visible | A real input, OS, or explicitly unconditional domain event may cause one frame. That frame's own repaint requests cause no successor; the trace returns immediately to zero. Streaming results use foreground-only wakes, their queues remain bounded, and backpressure rather than presentation becomes their idle governor. |
| Minimized or compositor-occluded | When the backend reports concealment, the trace has no UI, submission, or presentation work. When it cannot, each genuine external wake may cause at most one attempted frame and never establishes cadence. Exit and ordinary OS restoration still wake the host. Record which law the coordinate can actually observe. |
| Application-hidden | When the product supports tray concealment, hidden time has no frames or GPU submissions. Explicit reveal and exit signals work without a polling frame. |
| Elected background service | Run each crawler, mirror, cache custodian, or updater with presentation suppressed. It blocks between bounded units, stays within its declared CPU/network/disk/memory envelope, exposes status and pause where activity is material, and neither depends on nor manufactures frames. |
| Restored interaction | The first restored frame contains current state; input, water, custom GPU callbacks, and post-present settlement resume without a burst drain or stale generation. |
| Saturated workers | Hold the window unfocused and concealed while producers run. Result channels exert bounded backpressure, terminal signals remain observable, and restoration does not admit an unbounded event-loop transaction. |

For each state, retain the frame trace, CPU consumed over a fixed interval,
RSS/private bytes at the start and end, and native GPU/driver memory evidence.
Warm the complete product before measuring. A release candidate receives at
least a fifteen-minute settled soak on every claimed coordinate; a change to
the host, surface lifecycle, allocator, custom renderer, or upload path also
receives an overnight soak on representative hardware. Passing means no
positive secular memory slope after warm-up, no unexplained driver-mapping
growth, no validation error, and frame production equal to the state law above.

Use native observability; one platform's counters are not aliases for
another's:

| Coordinate | Minimum native observation |
| --- | --- |
| Linux/X11/Vulkan | Name the X server, window manager, Vulkan driver, and GPU. Retain `/proc` private-memory and mapping evidence, Vulkan validation output, and allocator plus vendor device-memory readings. Exercise focus loss, another workspace, minimization where the manager supports it, and restoration separately. |
| Linux/Wayland/Vulkan | Name the compositor, Vulkan driver, and GPU. Record which focus, minimize, and occlusion transitions the client can actually observe; apply the conservative unfocused law where the protocol supplies no visibility fact. Retain the same process, allocator, validation, and device-memory evidence as X11. |
| macOS/Metal | Name the OS, GPU, and display arrangement. Retain process footprint and VM-region evidence plus Metal validation and device-memory observations. Exercise miniaturization, app switching, full occlusion, sleep/wake, and restoration. |
| Windows/DX12 | Name the OS build, GPU, and driver. Retain private working set and commit, D3D12 debug-layer output, DXGI budget observations, and allocator readings. Exercise minimize, task switching, display sleep, device recovery, and restoration. |

These are runtime coordinates, not four ceremonial jobs. A discrepancy between
GPU vendors, compositors, window managers, display arrangements, or driver
families splits the affected coordinate until the causal boundary is known.
