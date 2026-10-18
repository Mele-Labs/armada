# Design System — UI & Voice

**Kind:** contract. **Governs:** static UI chrome, tokens, and the Voice &
Copy contract — the parent contract that nothing else may contradict.

Read before building a screen or component in `packages/components`.

---

Constraining what a surface may spell is what makes it drop into the
Electron app without restyling. A value or a word this document does not
sanction is a finding, not a judgement call.

UI tokens and the Voice & Copy contract are both in force. Two sibling
documents carry what building a surface does not need: the [Agent Copy
Contract](agent-copy.md) (text written at runtime by Drones, Judge and
Helm, with its surfaces and their samples in Armada Copy) and the [Voice
Contract — Engineering Requirements](voice-engineering.md).

---

<!-- split: one file per section; re-run the split on the whole doc -->

| Section |
|---|
| [Stack](design-system/stack.md) |
| [The product](design-system/the-product.md) |
| [Hard rules](design-system/hard-rules.md) |
| [Tokens](design-system/tokens.md) |
| [Typography](design-system/typography.md) |
| [Spacing and shape](design-system/spacing-and-shape.md) |
| [Motion](design-system/motion.md) |
| [Sound](design-system/sound.md) |
| [Touch](design-system/touch.md) |
| [Window and layout model](design-system/window-and-layout-model.md) |
| [Floating layers](design-system/floating-layers.md) |
| [Keyboard and command palette](design-system/keyboard-and-command-palette.md) |
| [Component → token mapping](design-system/component-token-mapping.md) |
| [The error treatment](design-system/the-error-treatment.md) |
| [Voice & Copy](design-system/voice-copy.md) |
| [Open questions](#open-questions) |

## Open questions

- **[decay-cap]** Does a Board with many changed rows cap how many carry a
  decay mark? Today every row whose status changes takes its mark for 45s, so
  thirty Jobs moving within a minute tint thirty rows at once and the mark stops
  pointing at anything. What decides it is seeing a busy Board: whether the
  marks still read at that density, and if not, whether a cap keeps the most
  recent few or the most urgent. Change detection and the ticker are in
  `packages/screens/src/recent.ts (deleted)`.

- **[pilot-exit-bindings]** What are the keys for Close as superseded and
  Override the verdict? Every action owes a verb, an icon and a shortcut, and
  these two have no binding. Both end or overrule a Job's record, so neither
  should take a spare letter by default — the destructive-key rule exists
  because a binding chosen for convenience is one a person reaches by accident.
  Three neighbouring acts took keys in the same pass: Observe `v`, Submit for
  verification `u`, Redispatch `e`.

- **[status-bar-onboarding]** During the hard-gated first-run sequence,
  does the Fleet panel read the same three runtime states as everywhere
  else, or does onboarding get its own reading?
  The panel states a healthy status out loud rather than implying it,
  because an empty panel reads the same whether Fleet is healthy or dead —
  that is settled and gives the three runtime states. But "Fleet is not
  running" is correct and reads as an error on a new user's first screen,
  and it was written for someone who already knows what Fleet is. Fleet is
  started by hand at M1 and becomes reachable partway through onboarding,
  so the panel changes state mid-journey either way; whether the panel is
  even present before the first onboarding step completes is part of the
  same question, since onboarding is not yet a Bridge surface. The slug
  predates Bridge/1088's move from the status bar to this panel and is
  unchanged, since a citation resolves by name.

Also bearing on this document, and written where each belongs: `[attested-verdict-glyph]` in `iconography.md`; `[verdict-artifact-rows]` in `voice-engineering.md`.
A question has one home — answering it in two places is how one of them goes stale.
