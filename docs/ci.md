# CI And Release Evidence

CI schedules app-owned evidence; workflow YAML does not own product law. A
native product exposes complete local commands for:

| Unit | Obligation |
| --- | --- |
| source | format, strict lints, unit and integration behavior, documentation |
| security | locked dependency audit with explicit policy |
| lifecycle | install into a sterile non-default prefix, inert public probes, native launch smoke, documented uninstall, and proof of removal |
| native acceptance | optimized black-box user stories on every release-tested GUI coordinate |

Declare release-tested, supported, and unclaimed platform sets before writing
a matrix. Jobs exist only for claimed capabilities. An OS, window system,
installer, archive, updater, tray, dialog, or package manager requires evidence
that can falsify that claim.

Hosted software graphics may run deterministic functional stories. It cannot
pass or excuse production latency budgets; those run on a named
representative-host coordinate. Third-party actions are commit-pinned, receive
least privilege, and only invoke the app-owned commands.

An `egui-tester` X11 runner must provide Bubblewrap, Xauth, a normalized
read-only lavapipe root, and a canonical `systemd --user` manager with a D-Bus
socket at `/run/user/$UID/bus`; its kernel must admit the unprivileged
namespaces used by the systemd and Bubblewrap sandboxes. A disposable hosted
runner may enable those namespaces, raise that manager, and project its
distribution's multiarch Mesa package into the harness's fixed
software-Vulkan layout during workflow setup. Do not add a containment-free
harness mode merely to accommodate deficient CI.

Uninstall removes application-owned machinery. Projects and user-owned data
survive unless the product separately defines and tests an explicit purge.

## Shared Release Law

A shared package release is an exact source claim. Publication is admitted only
from a clean, tagged checkout after the package's source gate, `cargo package
--locked`, and every registered downstream head canary pass. The manifest
version, tag, packaged metadata, and Git commit must identify the same source.
`--allow-dirty` is forbidden.

Foundational releases proceed in dependency order:

```text
Dwemer Poolrooms and egui-tester witness/controller
→ eternalist-apps
→ grouped application lockfile updates
```

Application manifests use released compatible ranges; checked-in lockfiles and
the fleet cohort record supply exact reproducibility. A release bot may update
locks, but it may not publish a shared layer before its consumers compile and
their affected stories pass against that exact candidate.
