# Application Design Language

Dwemer Poolrooms is the visual and physical language. Install its chrome on
the egui context and compose its water frame through `NativeApp::water`.
Applications may build their own Poolrooms chrome directly.

Organize surfaces by semantic role rather than a universal panel sequence:

- a persistent inspector, when the product has durable libraries or controls;
- a primary canvas or gallery;
- a transient bottom shelf for working memory, results, or rich inspection;
- a bottom counsel/status surface for the next useful action and active work;
- overlays for modal or spatially anchored interaction.

Permanent collections belong in the inspector. Transient candidates and
working detail belong in the shelf. Status reports current work; it does not
repeat hidden implementation state. Applications without a natural inspector
must not manufacture one for fleet symmetry.

Begin from absence. Every label, symbol, section, control, and persisted state
must communicate a user fact, admit an action, or supply necessary feedback.
Use domain language, never renderer or storage terminology. Loading surfaces
must acknowledge life through Poolrooms motion and water; the underlying work
must progress independently of repaint animation.

Motion invalidates whole-frame pixel equality as a general test oracle. Test
semantic readiness through a presented witness, then judge a bounded rendered
feature with tolerance or a durable external effect.

Use `LivingWait::bouncer` for a gallery-scale initial wait: it paints the
standard central loading card and claims the same rectangle for Poolrooms'
living raft. Smaller concurrent waits may continue to use `LivingWait::claim`.
