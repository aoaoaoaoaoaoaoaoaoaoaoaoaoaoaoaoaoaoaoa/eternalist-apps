# Eternalist Apps Glossary

This glossary admits only durable application terms whose law is shared across
products or fundamental to this crate's public boundary. Product nouns and
still-local implementation metaphors remain with their applications.

- **Product Identity**: one product's reverse-DNS identifier and display name,
  declared once in its contract crate. Platform directories and the crash
  report derive from it; no other spelling of the product is admitted here.
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
  Sheet** is the central UI that presents such settings; its Font Size row
  projects the Brass Font scale.
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

- **Capabilities**: the facts the host declares about one run, installed
  before the application is built. Applications consult one fact and never an
  operating system. The facts are `pointer` (a hovering precise pointer
  exists; tension, tooltips, and the Inspector actuator require it), `touch`
  (direct multi-touch manipulation exists; pinch and swipe are meaningful and
  mechanisms step up to a fingertip), `keyboard` (a physical keyboard with
  shortcuts is expected; shortcut hints and the command canon appear only
  where it holds), `power_unconstrained` (the host may spend the GPU
  continuously on ornament; water, radiators, and tension require it, and a
  Handheld never grants it), `configuration` (a human-edited Configuration
  exists), and `retirement` (the operating system may suspend or destroy the
  process at will; close never exits and the application checkpoints on
  suspension).
- **Handheld**: a phone or a tablet held in the hand. The word names the device
  class and the Capabilities constructor; code never branches on it.
- **Ingress**: the operating-system entry token handed to the native host:
  bare on a desktop, or Android's `NativeActivity` with its application
  handle.
- **Dock**: the Inspector's persistent left-side disposition.
- **Drawer**: the Inspector's bottom disposition for a Handheld, showing one
  Panel at a time with swipe paging.
- **Cue**: the touch gesture a Gesture names, such as `SWIPE ↔`, shown where
  Touch holds in place of key bindings.

Acceptance terms such as Target, Anchor, Observation, Story, and Oracle belong
to egui-tester. Physical terms such as Section, Rail, mechanism, and casing
belong to Brass Poolrooms.
