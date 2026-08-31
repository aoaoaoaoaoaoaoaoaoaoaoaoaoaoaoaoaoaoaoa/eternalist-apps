# Eternalist Apps Glossary

This glossary admits only durable application terms whose law is shared across
products or fundamental to this crate's public boundary. Product nouns and
still-local implementation metaphors remain with their applications.

- **Application Header**: the persistent application name with the standard
  Help and Settings actuators, placed above the nearest persistent control
  surface.
- **Inspector**: the optional persistent left-side application control surface.
  It owns concealment, scrolling, and its boundary actuator; it is neither a
  Brass Rail nor a logical Panel.
- **Panel**: one application-owned logical unit participating in Inspector
  focus containment and traversal. Its visible disclosure is a Brass Section.
- **Command**: a semantic user-invokable action declared once for routing,
  availability, labels, consequences, shortcuts, and help.
- **Shortcut**: one portable key combination that may invoke a Command. A
  **binding** is the association between a Command and a Shortcut; a
  **mnemonic** is the permanent Alt-modified label glyph declared by a command.
- **Gesture**: a target-relative interaction interpreted by the focused
  control rather than globally routed as a Command.
- **Guide Group**: one application-named group of related Gesture rows in the
  Command Guide. An idiom is a reusable convention, not a guide-group value.
- **Setting**: one user-adjustable declaration and value. The **Settings
  Sheet** is the central UI that presents such settings.
- **Font Scale**: the closed Standard (100%), Large (125%), or Extra Large
  (150%) multiplier applied to semantic application fonts before layout and
  rasterization. Extra Large is the desktop compatibility ceiling.
- **Configuration**: the typed aggregate admitted from the human-edited
  configuration file. A **Configuration Ledger** owns strict admission,
  format-preserving revision, fault state, reload, and settled writes; it is
  not a product document store.
- **Cabinet**: the shared logical projection for a persistent, reorderable
  collection with globally unique entries and at most one named grouping
  level. The application remains the storage and domain authority.
- **Living Wait**: visible, finite evidence that real background preparation is
  progressing without using animation as the work's clock.
- **Native Wake**: a cross-thread native event-loop signal. It may request a
  policy-governed frame but is not itself an egui repaint request.
- **Settled Scribe**: the sequenced background-write boundary for a latest-wins
  durable projection after a finite settlement interval.

Acceptance terms such as Target, Anchor, Observation, Story, and Oracle belong
to egui-tester. Physical terms such as Section, Rail, mechanism, and casing
belong to Brass Poolrooms.
