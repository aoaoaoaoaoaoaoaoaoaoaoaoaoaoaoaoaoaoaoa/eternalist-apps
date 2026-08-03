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

Publish the smallest useful shell and each independent substrate as soon as
its own prerequisites exist. Later armament must not reset the viewport,
replace the workbench, withdraw already presented content, or erase intervening
input.

The host records canonical spans under `eternalist::*`:

```text
window.event
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
