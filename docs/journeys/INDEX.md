# User journeys

The journeys carried out of the Armada User Journeys database in Notion, and the ones designed since.
Each names a trigger — the moment a person reaches for Armada — and the surfaces that answer it.
**These describe intent, not build order.** Notion's own phased plan is explicit that design order is not build order: journeys are designed in the order their conventions get reused, not in the order the milestones build them.

Every journey names a **Milestone** — what Armada v2 builds is decided in its issues and milestones, and the milestone is where this journey's answer is currently expected to land, not a promise of when.

Read the journey that covers what you are about to design or build, before you build it.

| Journey | Read it before you |
| --- | --- |
| [Check System Health](check-system-health.md) | Design or build Doctor's health grid, or add a module to it |
| [Set up the phone app](set-up-the-phone-app.md) | Run `armada pocket`, put it on the tailnet, pair a phone and allow push |
| [Set Up a Project (Manifest)](set-up-a-project-manifest.md) | Design or build onboarding a new repo — Locate, Scan, the picker, the proposal sheet, ports, Write and Verify |
| [Run and edit a Manifest](run-and-edit-a-manifest.md) | Design or build Bridge's Manifest surface — running a Check or Command on demand, editing a manifest, or Verify's drift-plus-dry-run |
| [Guild Setup & Configuration](guild-setup-and-configuration.md) | Design or build the Kit/Machine settings surface — first-time init or an ongoing settings edit |
| [Consult Helm](consult-helm.md) | Design or build Helm's entry point — starting a job, or what it can read |
| [First-Run Onboarding](first-run-onboarding.md) | Design or build the hard-gated first-launch sequence that chains Guild Setup, Set Up a Project, Check System Health and a first Job together |
| [Take a design to a milestone](take-a-design-to-a-milestone.md) | Design or build what turns a finished design into a milestone of issues worked as Jobs — reading it against the code, keeping the answers it forced, and filing the work it implies |

## Both ways

Every file above is a real file in this directory, and every real file in this directory (besides this one) is listed above.
A gate rule checks both directions, so an entry with no file, or a file with no entry, fails the build.
