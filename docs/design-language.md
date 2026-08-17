# Application Design Language

Brass Poolrooms is the low-level visual and physical language. It owns the
embodiment of controls and surfaces: geometry, material, constrained motion,
interaction response, and displaced water. Install its chrome on the egui
context and compose its water frame through `NativeApp::water`. Applications
may build their own Poolrooms chrome directly.

Eternalist Apps is the high-level logical language. Its renderer-neutral
primitives are the persistent [`Inspector`](../src/inspector.rs),
[`LivingWait`](../src/living_wait.rs), shelved [`Cabinet`](../src/cabinet.rs),
typed [`commands`](../src/commands.rs), generated
[`command guide`](../src/command_guide.rs), and active
[`panel navigation`](../src/panel_navigation.rs). A future manager, menu,
storage interaction, layout, or application-scale state machine belongs here
only after its common law is proved. Such primitives accept explicit state and
return typed actions; the product supplies domain meaning.

Eternalist-style applications use several useful semantic roles:

- a persistent inspector, when the product has durable libraries or controls;
- a primary canvas or gallery;
- a transient bottom shelf for working memory, results, or rich inspection;
- a bottom counsel/status surface for the next useful action and active work;
- overlays for modal or spatially anchored interaction.

These are design vocabulary, not a promise that every role has a library type
or a universal panel sequence.

Commands have one stable identity, one visible label, one consequence
description, and one effective shortcut projection. Default accelerators,
permanent Alt mnemonics, routing, button legends, and help must derive from
that declaration rather than drift as independent strings. Reserve mnemonics
for common consequential actions; saturation destroys their value.

## Basic Controls

Apply every idiom whose target exists; omission is correct only when the
corresponding navigation is absent:

- Tab and Shift+Tab traverse the active inspector panel. From a collapsed
  header they continue to the adjacent panel instead of cycling through absent
  contents.
- Physical Control+Tab and Control+Shift+Tab cross panel boundaries.
- Enter and Space actuate the focused control.
- Left and Right move one item through a result surface's primary order.
- Alt+Left and Alt+Right traverse a subordinate or grouped order when one
  exists; they never silently replace primary-order navigation.
- Arrows and the wheel adjust controls that visibly admit adjustment.
- Page Up and Page Down move exactly one rendered row through an ordered result
  surface.
- Home returns an ordered result surface to its first item.
- Escape closes only the topmost transient layer.

A target-relative gesture belongs in help but is not a global command
shortcut; do not advertise a shared gesture in an application that lacks its
target. Commands and panel traversal remain dormant behind a modal layer.
Ordinary labels remain selectable. Native hosts export their copy, cut, and
paste commands through the platform clipboard.
Permanent collections ordinarily belong in an inspector; transient candidates
and working detail ordinarily belong in a shelf. Status reports current work;
it does not repeat hidden implementation state. Applications without a natural
inspector must not manufacture one for superficial symmetry.

Begin from absence. Every label, symbol, section, control, and persisted state
must communicate a user fact, admit an action, or supply necessary feedback.
Use domain language, never renderer or storage terminology. Loading surfaces
must acknowledge life through Poolrooms motion and water; the underlying work
must progress independently of repaint animation.

Explanatory prose is not ambient furniture. A self-explanatory selector stands
alone; never paraphrase it in a note immediately beneath it. Put command
consequences in the generated guide and transient facts in status. When a
domain term still needs interpretation, attach a compact glossary card to the
term or control and use the platform Help cursor on hover. Such cards have one
short title and one direct definition; ordinary hover text remains sufficient
for a terse action hint or disabled reason.

Motion invalidates whole-frame pixel equality as a general test oracle. Test
semantic readiness through a presented witness, then judge a bounded rendered
feature with tolerance or a durable external effect.

Use `LivingWait::bouncer` for a gallery-scale initial wait: it paints the
standard central loading card and claims the same rectangle for Poolrooms'
living raft. Smaller concurrent waits may continue to use `LivingWait::claim`.

Use `Cabinet` when entries have globally unique textual identities, user-owned
order, and at most one level of named shelves. Implement `CabinetEntry` on the
product value and interpret each returned `CabinetAction` in the application.
Use its ordinary projection for fixed entry identities and its renamable
projection when users may edit them; shelf reordering remains available in
either projection.
Do not force trees, tags, recents, search results, or immutable built-ins into
the cabinet merely because they are collections.
