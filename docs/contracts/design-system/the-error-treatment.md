# The error treatment

**Part of:** [Design System — UI & Voice](../design-system.md).

A failed Job is Armada working. An error is Armada failing. Both are red,
and they are told apart by shape.

What an error carries and how it crosses the wire is the [Error
Contract](../error-contract.md). This section governs what a person sees.

## One red, told apart by shape

**`--error` aliases `--status-completed-failed` and carries no value of its
own.** One red, and no ninth hue to keep in step when the state machine
moves.

**Shape separates an error from a status, on two channels.** An error is the
only solid fill on a data surface, where every Job status is a 12% tint in a
chip; and an error always carries a code, which a status never does.

**The code is always shown, in mono at `--text-2xs`.** It is what a person
reads back to someone else, and the wire guarantees one on every error.

**No generic alarm glyph.** `triangle-alert` is Doctor's and `octagon-alert`
is `stalled`'s, so an error carries the code and the sentence instead.

```
code chip  solid --error or --degraded fill · --fg-inverse text
           --h-badge · --space-2 horizontal padding · --radius-sm
           --text-2xs mono · weight 500 — the status badge's geometry exactly
edge       leading, --error-edge or --degraded-edge. Never a box: the solid
           fill in this treatment belongs to the chip
surface    the placement's own. Inline adds none; banner and full-surface
           take --bg-raised; toast takes --bg-overlay and a shadow
```

## Two fault classes, and only one is red

| Class | Edge | Headline | Dot | Means |
| --- | --- | --- | --- | --- |
| Fault | `--error-edge` in `--error` | `--error` | none | Armada cannot do the thing |
| Degraded | `--degraded-edge` in `--degraded` | `--fg-default` | `--degraded-dot` | Armada cannot refresh what it shows |

**Unreachable Fleet and dropped events are degraded, not faults.** The fixes
are opposite — restarting Fleet is wrong when the process is alive — so the
dot has to differ.

**The dot is amber rather than red.** Amber already means a person is waited
on rather than something being broken, and stale data is a wait.

**One value, not a ladder.** Placement carries blast radius, so severity picks
nothing but the edge.

## The four placements

| Placement | Where | Rule |
| --- | --- | --- |
| Inline | In the row, or beside the act | Contained to the thing you touched |
| Toast | Bottom trailing, inset `--space-6`, shadowed | A confirmation, which may carry no act and goes on its own; or a press that failed or was not sent, which carries its acts and stays until dismissed; or a call that has come for the owner while he is away from the cockpit, which is pressable as a whole, opens that call's card, and goes when the call is dealt with or after fifteen seconds |
| Banner | Above the surface, inside it | Persistent. The surface works beneath |
| Full-surface | Replaces the surface | The one placement that takes the screen |

**Blast radius picks the placement, never severity.** Approve-refused is
red-serious and affects one row, so it renders in that row and nowhere else.

**Rows around an inline error are undisturbed.** Same height, same badges, and
the pulse continues.

**A press that failed is a toast; a state is a banner.** A command Fleet
refused or did not answer, a route it does not serve yet, and a throw no
boundary saw each pop up as a toast over every layer, because they come from a
press and a panel may be open over the surface. Fleet unreachable, a stale
window and every other standing condition stay banners above the surface:
they are true until something changes, not something that happened.

**A press the form would not send is a toast as well.** "Fleet is not
connected. Nothing was sent." and the rest of the form's guidance pop up the
same way, with the sentence and Dismiss and no Copy debug info, since there is
no code to hand on. One stands per kind, so a press repeated replaces it.

**Several failure toasts stack, newest nearest the corner, and none is on a
timer.** A failure is evidence, and one that expired while somebody read the
panel it came from is one they cannot get back.

**Every placement names the failure and the act.** A confirmation toast is the
one exception, because it reports something already over.

**A toast used to clear the status bar rather than cover it**, because the
bar spanned the window's bottom edge and a bottom-right toast could
otherwise sit over it. Fleet's liveness statement is now the Fleet panel, a popover on the title row's dot, nowhere near a bottom-right toast. `Toast.css`, the app
shell's own toast region and `ErrorNotice.css` all carried the stale inset
against the deleted bar; all three now sit at the plain inset this document
names, and `--h-status-bar` is gone.

## The debug payload, and what each placement does with it

**Every error carries the payload. The four placements differ only in whether
it is shown, offered or expandable** — which is the placement's blast radius
again, not a second decision.

| Placement | Form |
| --- | --- |
| Inline | Ghost control, copying directly. A row has no room for an expanded view |
| Toast | A failure toast carries the notice whole: Copy debug info, the fold, File an issue and Dismiss. It stays until dismissed, so it is read as well as quoted |
| Banner | Copy, plus **Details** opening the expanded view. A standing condition gets read, not only quoted |
| Full-surface | Shown rather than offered. Nothing else is on the screen |

**The act is called "Copy debug info" wherever it appears, and it is bound to
`c`.** One verb for one act, from the contextual key map above — the control,
the palette entry and the tooltip all say it. It names the artifact rather than
what somebody is about to do with it, because the decision being taken is
whether to paste a machine record into a public issue.

**The control carries no glyph and no kbd.** No glyph, because nothing in the
error treatment carries one and `triangle-alert` and `octagon-alert` are
spoken for. No kbd, because a binding is discovered in the tooltip and in Settings under
Keyboard shortcuts, and a kbd inside
every button that has a binding would put the reference material on the thing
it describes.

**The key runs the control's own function, never its own copy.** A binding that
reimplemented the write would be a second artifact the day either side changed,
and this whole treatment rests on there being one producer.

**The expanded view renders the exact string the control copies.** One producer
formats it, so what was read on screen is what arrives in the issue body — not
two renderings agreeing about field order on the day they were written. What
the artifact holds is the [Error Contract](../error-contract.md).

**A clipboard write is silent, so a toast confirms it — and the toast carries
no status dot.** A leading dot carries a Job state and is never chosen, and a
clipboard write is not a Job state.

**The act is not an error's alone.** A Helm session carries the same payload
under the same word, bound to the same key, with one producer and an expanded
view rendering the string the control copies — `../concepts/helm.md`,
*Reporting a bad answer*. It takes the banner's form, because a dock is a
standing surface: the copy acts on one press and a second control opens the
reading beside it. Its safety sentence is its own — nothing in it is a
structured field a type bounds, so the sentence below cannot be reused over
it.

**One sentence about safety, in the expanded view only, stating the mechanism
rather than promising an outcome.** It is bounded to what the mechanism
reaches: structured fields carry primitives and a credential does not compile
into one, while the message and the chain are prose an error wrote and nothing
bounds those. A claim over the whole artifact would be a promise the type
system does not make, and it makes none about the wider context — see the
file-an-issue flow, which is not bounded this way.

## Filing, which is a second act with a review

**Copying stays on the machine. Filing leaves it.** So `Copy debug info` acts on
one press and **File an issue** opens a dialog first, naming every item that
would go, showing its text, and offering a control to take it out. **Send is
never one press from an error.**

**It appears on the full-surface state, in the expanded view and on a failure
toast, and nowhere else.** A review needs the artifact legible in full; an
inline error has no room for one. A failure toast stays until it is dismissed
and its review opens over it, so it offers one; a confirmation toast is gone
before a review would be read, and offers none.

**Armada makes no scrub claim, and the dialog says what it does not do.** Every
row carries a sentence naming what is unbounded about that item — which is the
read-this mark, made specific — and the row that cannot be removed carries the
payload's own safety sentence rather than a claim written for the dialog. A
promise Armada cannot keep is worse than the work of reading.

**The confirm copies.** Nothing in Armada opens anything in a tracker, and the
dialog states that in those words. What the artifact holds, and what the drawing
asked for that follows from having no transport, is the [Error
Contract](../error-contract.md).
