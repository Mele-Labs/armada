# Keyboard and command palette

**Part of:** [Design System — UI & Voice](../design-system.md).

Foundational rather than additive. Both change the component inventory
and the focus model, which is why they are specified before the first
screen instead of retrofitted onto it.

**Principle: every action reachable by mouse is reachable by keyboard,
and nothing is keyboard-only.** The palette is a superset of the UI,
never a substitute for it. A capability that exists only behind a
shortcut is undiscoverable, and a capability that exists only behind a
click is unusable at speed.

## One artifact, three columns

Every action carries a **verb**, an **icon**, and a **shortcut**. The
source is `crates/core-model/domain/actions.toml` and the test is the
gate rule over it, which fails where an entry is missing any of the
three. This is the same discipline already applied to the enum→verb map
and the icon map, extended one column. A new action cannot ship with no
shortcut, and a shortcut cannot exist without a label to display next to
it.

**The glyph is named, never redrawn.** An entry's icon is a key in
`packages/icons/icons/`, which stays the authority on what a
silhouette may mean; the gate refuses one that is unregistered or
banned.

**A blank glyph column says why it is blank, and the gate counts the
blanks.** Copy debug info carries none by decision, for the reason the
error treatment gives below. Several acts carry none because no
registered glyph means them and assigning one is a decision for
[Iconography](../iconography.md). An entry that leaves the column empty and
says nothing fails — which is the closure working, since the alternative
is the back-fill by hand that this section existed to prevent.

## Two tiers

**Global — modifier-based, work anywhere.**

```
⌘K       command palette
⌘J       Helm, toggles the dock on every surface
⌥⌘C      capture a note onto the open Studio
⌘[ ⌘]    back / forward
⌘Enter   send the message in the field that has focus
⇧⌘Enter  Queue for later: save a Session's request as an idea, not started
⌘N       dispatch from a field, where n would type
⌥1–⌥9    choose a repository to dispatch to
⌥A       dispatch after another Job: a Job with an @ typed in the request
⌥M       message the Job: put the caret in the open Job's message box
⌘1–⌘8    go to a tab: Cockpit, Jobs, Studios, Logbook, Reviews, Issues, Projects or System, as the bar draws them
⌘,       open Settings
⌘O       open a file in the side panel
Esc      close an overlay, or return to the list from a detail route
⇧⌘R      refresh, re-reading everything Fleet holds
⇧⌘D      debug Bridge: shoot the window and start a Session on it  (pnpm dev only)
```

**`⌘Enter` and `⇧⌘Enter` are the Global bindings that need a focused field**, and
they are in this tier because they are modified rather than because they fire
from nowhere. Plain `Enter` in the drone message box is a new line — a redirect to a
drone is prose — so the key that sends there has to carry a modifier, and a
single-key binding would be suppressed inside a field anyway by the rule below.
`⌘Enter` is drawn on that Send button only while `⌘` is held, per `kbd` below.
`⇧⌘Enter` is its twin in the Dispatch panel's request: it saves a Session's
idea for later where `⌘Enter` starts one.
`⌥M` is the Job view's own family of `⌥` keys, with `⌥W`, `⌥1`–`⌥3`, `⌥F`, `⌥N` and `⌥L`.
It puts the caret in the open Job's message box from anywhere in the view, a field included, and does nothing with no Job open.

**Helm's box and a Session's are one box**: `Enter` sends and `⇧Enter` breaks
the line, and `⌘Enter` sends there too. In that box a `#` that starts a word
opens the repository's open issues and pull requests, and a number typed after
it that is not among them is looked up before it is refused. What the box offers
is read from Fleet, so a number that resolves to nothing stays text, in the box
and in Helm's thread alike. `docs/concepts/helm.md`, *The message box*.

**`⌘[ ⌘]` are the one Global pair a focused field keeps.** In a field they are
outdent and indent, which is what a person typing meant, so the press passes
through to it. Everywhere else they step through the places visited: the
surface, the open Job and its tab, the open Session, and the open Studio and
its node.

**Contextual — single-key, on the focused row or the open job.** This is
what makes triage fast: move down the queue and act without reaching for
a modifier.

```
j / k / ↓ / ↑  move focus          (list only)
Enter          open the focused job. Acts on nothing   (list only)
o              open. The same act, named so the palette can display it   (list only)
c              copy debug info
n              dispatch
N              add a note           (open studio only)
V              add a link           (open studio only)
S              add a sketch         (open studio only)
Z              add a zone           (open studio only)
R              run                  (open studio only)
m              show the map   (dashboard only)
?              show the keys   (dashboard only)
w              bring a waiting call back   (dashboard only)
l b c q f a d  go to a panel: list, bays, calls, queue, flight controls, Autopilot, Dispatch   (deck only)
0–9            press a flight control   (deck only)
t              show the timeline   (deck only)
/              search the list   (deck only)
⌥↑ ⌥↓          move the row up or down   (deck only)
g              go to a page, then c, j, s or y   (deck only)
l r f          go to a part of Reviews: the list, the reader, the review tools   (review only)
L              open the lists   (review only)
/              search the pull requests   (review only)
N              Natural Search   (review only)
Q              go to the quick actions   (review only)
A              ask about the pull request   (review only)
D              show the description   (review only)
W              show where changes run   (review only)
l r            go to a part of Issues: the list, the reader   (issues only)
L              open the queries   (issues only)
/              search the issues   (issues only)
D              describe a query   (issues only)
s              start a Job on the issue   (issues only)
l r            go to a part of Projects: the left column, the cards   (projects only)
/              search the tasks   (projects only)
1 2 3          show all tasks, the ones Fleet asks about, the ones a Job holds   (projects only)
h              show or hide done tasks   (projects only)
N              new project   (projects only)
a              add a task   (projects only)
A              add a subtask   (projects only)
Backspace      delete or unlink   (projects only)
⌥↑ ⌥↓          move the row up or down   (projects only)
Space / x      select the card   (projects only)
Enter          edit the title   (projects only)
o              open the Job   (projects only)
u              unlink the Job   (projects only)
U              unlink the pull request   (projects only)
e              rename the project   (projects only)
d              tick or untick the task   (projects only)
s              dispatch the rows: the selected ones, or the focused one   (projects only)
D              mark done   (projects only)
K              keep open   (projects only)
1–9            pick an answer   (call only)
Enter          send the answer   (call only)
l              later   (call only)
d              dismiss for good   (call only)
e              expand the request   (call only)
b              make the best decision   (call only)
g              just get it done   (call only)
t              reply in words   (call only)
v              Open walk   (call only)
e              Edit the entry   (logbook only)
l              Lock or unlock the entry   (logbook only)
f              Forget the entry   (logbook only)   (confirms)
r              Adversarial review   (plan only)
j / k / ↓ / ↑  move between findings   (plan only)
Enter          send to agent   (plan only)
d / Backspace  dismiss the finding   (plan only)
```

**The Cockpit's keys and a call in front have scopes of their own**, `dashboard`
and `call`, because their keys act on neither a list nor a Job. A call in front
takes the digits, `Enter`, `l`, `d`, `e`, `b` and `g` while it is up and gives them
back when it goes; `l` and `Esc` both put it at the back of the stack, and
`b` and `g` stand only on a question an agent asked and could answer itself,
never on a call Fleet raises about a state. On a Job's question they answer with
`mode` `best` or `quick`, which Fleet turns into the instruction the Drone is told.
`m` shows the map, `w` brings a waiting call back and `?` lists the keys. None is
a palette row yet: each is read from the Cockpit's own sheet.

**The Logbook page has a scope of its own, `logbook`.** Its three keys act on the selected
entry: `e` edits its text in place, `l` locks it against rewriting or lets go, and `f` forgets it
after asking. `f` and not Backspace, which the text box in place takes. They stand down in a text
field and while a dialog is up.

**A plan waiting on its card has a scope of its own, `plan`.**
Its keys answer wherever focus is on the plan card or in the plan in full.
`r` runs an adversarial review of the plan, or runs it again where one failed or read an older version.
`j`, `k` and the arrows move between the findings not yet acted on, in the order the plan draws them; `Enter` sends the one in focus to the agent and `d` or Backspace dismisses it.
They stand down in a text field.

**System, Projects, Reviews and Issues share one search bar,** `SearchField` in `packages/components`: "Search" with the key drawn inside it, Esc clearing the words and a second Esc leaving it.
Its key is `/` on each page, and on a page with nothing focused the bar takes it.

**The Reviews tab has a scope of its own, `review`.** Its list takes `j`, `k`, `Enter` and `o` as
any list does; the rest are its own: `l r f` go to the list, the reader and the review tools,
`L` opens the lists, `/` goes to the search bar that narrows the list on show and `N` is Natural Search, `Q` the quick actions, `A` the question box, `D` the
description and `W` the places changes run. They stand down in a text field and while a dialog is up.

**The Issues tab has a scope of its own, `issues`.** Its list takes `j`, `k`, `Enter` and `o` as any
list does; `l r` go to the list and the reader, `L` opens the queries, `/` goes to the search bar that narrows the loaded rows, `D` describes a query and `s`
starts a Job on the open issue. They stand down in a text field and while a dialog is up.

**The Projects tab has a scope of its own, `projects`.** `j`, `k` and the arrows move as any list does, through
the three views, the projects, the repositories and the cards, subtasks included; `l r` go to the left column and the
cards, `/` goes to the search field above the views and `1 2 3` show the three views.
In the search field, ↓ goes to the first result and Esc clears the words and goes back to the view that was open.
In the column, `Enter` opens a view, a project or a repository, `N` names a new
project, `e` renames the focused project and Backspace deletes it. `h` shows or hides the done tasks. A ⌘-click on a card selects it and a
⇧-click selects the range from the card last clicked. On a card, Space or `x` selects it, `Enter` edits a task's title in
place, `d` ticks a task, `a` goes to the field that adds a task and `A` opens one for a subtask inside the focused card,
`o` opens the Job holding it, `u` and `U` unlink its Job and its last pull request,
Backspace deletes a task or unlinks an issue, `⌥↑ ⌥↓` move it among its own list, `s` dispatches the cards selected, a request for each repository, or the focused one (a subtask on its own, with its parent as context),
and `D` and `K` answer a done ask. Esc leaves a field, closing it, then backs out a level. They stand down in a text
field and while a dialog is up.

**The flight deck is the Cockpit, and its keys are in the `deck` scope (#2301).**
The deck takes `w` and `?` and a list's `j`, `k` and `Enter`; the acts that need a row in
hand (`c` copy debug info) are not on the deck, so
the keymap does not count them against `c`. `l b c q f a d` go to a panel, `m` shows
the map, `0–9` press a flight control, `t` shows the timeline, `/` opens the list's Filter menu with its search focused, and `g` then `c`, `j`, `s` or `y` goes to the Cockpit, Jobs, Sessions or
System. A call pane in front takes the bare digits and letters the `call` scope names
while it is up, and the deck's handlers stand down for it. A replay takes Space, `←`, `→`,
Home and Esc while it is up. The rail, its `⌘1–⌘9` and `⌘\` went with it, and so did
the Dashboard panel's `[ ]` and `⌥1–3`: the deck has no filter to step.

**This is the map, not a pattern.** It was settled by drawing the Job
Board and the command palette together, which is what forced the two
halves into one artifact — the palette displays a binding beside every
entry, so an unreconciled map is a palette that cannot be drawn.

**Both blocks are transcribed from
`crates/core-model/domain/actions.toml`**, which is the artifact above
and the authority. They stay here because a binding is read beside the
rule that governs it rather than followed to a data file; the gate reads
both and fails where they disagree, on the binding, the verb and the
annotations in brackets.

**Job detail's bindings are in the map before the screen is rebuilt.**
The run tree, the chapters, the produced files and the phase strip each
carry one. The run tree roves on the same `j`/`k` as a list rather than
taking a second pair of keys — moving between steps and moving between
rows are one act — and expanding a step's facts is the same act as
opening a log entry's payload, so both are one binding on the other
axis. #265 builds the screen they belong to. **The run tree and the
chapters were retired with Overview's reframe of 29 Sep 2026**, and `[` `]`,
which moved between chapters, went with them: a key that lands on nothing
is not in the map. **The rest of this was retired on 2 Oct 2026**, when the
activity log went (#1761): with no run tree and no log on detail, `j`/`k`
roved nothing there and `h`/`l` opened nothing, so the owner had both
removed. `j`/`k` are the list's alone, and expand-and-collapse has no key.

**`not built` means the binding is registered and nothing answers it.**
The map was settled by drawing, so it holds acts nobody has written, and
the palette displays a binding beside every entry — a row a person
presses and gets nothing from is worse than one that is absent. The
registry's `unbuilt` column names the issue that gives each of them an
act, and the gate holds the annotation and the column to each other. So
the palette can draw these rows disabled, or leave them out, on a fact
rather than on a list of exceptions kept in the app.

Three reversals against what stood before, each with a reason:

- **`a` is deleted.** Nothing on a list approves. Approval is a second
  act from detail, which [Job Board](../../concepts/job-board.md) has always
  said and the built row contradicted.
- **`r` is review, and redirect moves to `d`.** Review is on every
  needs-you row and is the most-pressed contextual key in the app;
  redirect is reached from a job that has already gone wrong.
- **`⌘F` is deleted.** It duplicated `/`, and two bindings for one act
  breaks the one-artifact rule above. `/` is deleted too, on 28 Sep 2026:
  the Job Board held the only field it focused, and the command palette is
  what searches now.

**`x` for kill and every safety rule below are unchanged.** Neither was
in play, and the destructive-key rule is what kept `x` off `k`.

**A Studio's keys are shifted, and `open studio` is a place rather than
an object.** Every other contextual scope names what the act acts on —
the focused row, the open job, the dispatch card. These act on the board
a person is looking at, and the node they make lands where they are
looking rather than at the origin. They are shifted because the letters
the design drew are all spoken for: `n` is dispatch and its scope is
`anywhere`, `v` is observe, `s` is restart step and `r` is review and the
run sheet, so an unshifted key would answer twice on one press. `R` opens
the rail's Run menu rather than starting anything, as a press on Run does.
See [Studio](../../concepts/studio.md).

**`⌘1`–`⌘8` are the title bar's tabs and Settings is `⌘,`.** The tabs are Cockpit, Jobs,
Studios, Logbook, Reviews, Issues, Projects and System, in the order the bar draws them, and System opens
the page last used there. The rule is the tab's place in the bar, not the number: every tab
drawn takes the next digit, a tab that is not drawn takes none, and the digits behind it close up.
The owner decided this on 26 Oct 2026, when Reviews and Issues joined the bar and the digits stopped
matching what he saw. Studios and Retros took `⌘4` and `⌘5` on 15 Oct 2026
(the owner: *"Studios and retros should live in the main bar at the top of the app"*), which
moved System to `⌘6` and took Settings off `⌘5`; it has `⌘,`, the platform's own key for
preferences. Doctor left the System page the same day for the title bar's menu (*"Doctor
should live in the menu in the top right"*). Every other page is reached from a System
entry, a link on the deck or the palette by name, and takes no key.

**The rail's digits are gone with the rail (#2301).** Until then `⌘1`–`⌘9` followed the
rail's rows, and each arrival of a row moved the digits behind it: Held worktrees (now
Worktree Slots) took `⌘5` on 2026-09-03, Overview took `⌘1` on 2026-09-13, Settings took
`⌘7` on 2026-09-14, Studios took `⌘3` on 2026-09-17, and Kit took `⌘8` on 2026-09-18. A
learned key does not move quietly, and this paragraph is the noise: a person who
pressed `⌘3` for Studios now presses it for Sessions, and the palette, which displays
the binding beside every entry, is where they find out. **Helm moved from `⌘6` to `⌘J`
on 2026-09-13** (#948) and keeps it.

**`1`–`6` and `⌘1`–`⌘6` are different acts on one row of keys.** One is
modified and one is not, which is the whole separation. It was drawn
that way deliberately rather than by omission.

## Safety rules for single-key actions

These are constraints on the map, not suggestions.

- **Destructive keys are never adjacent to navigation keys.** Kill is
  `x`, never `k`, because `k` sits against `j` and a mistyped navigation
  keystroke must not be able to end a running job.
- **Every destructive action confirms**, even from the keyboard. A
  confirmation is a dialog or a hold, never nothing. In the
  confirmation dialog **Cancel holds initial focus, so `Enter` cancels**
  — `Enter` fires whatever holds focus, and Cancel is what holds it. The
  kbd is drawn on Cancel, where the key actually fires; confirming is a
  deliberate move to the other control, and `Esc` cancels as well. A
  destructive action is never one keystroke from a focused row, which is
  what the two rules together mean.
  **A dialog that collects a field is the exception**, and it is not a
  weakening of the rule: the field *is* the confirmation, nothing is
  destroyed by pressing it, and the kbd is drawn on the confirm because
  that is where `Enter` fires there. Every dialog that collects a field
  before it confirms works this way: Redirect, Overrule, Report,
  Raise the cost cap, Raise the turn cap, Add task.
  This line read "**Cancel holds initial focus**, `Enter` confirms,
  `Esc` cancels" until 2026-09-02. Both halves were true of something
  and the sentence did not say which won, so the implementation guessed
  — it bound `Enter` on the window and confirmed past the focused
  Cancel, which is exactly the one-keystroke destruction the rule above
  refuses.
- **A hold confirms in place.** Kill on Job detail fills while held and
  commits only once it has been held for `--duration-hold`; releasing,
  leaving the control or cancelling the pointer before then does
  nothing, and it behaves the same from Space or Enter. It sits beside
  the dialog rather than replacing it: `x` and a row's menu still reach
  the dialog. **Under `prefers-reduced-motion` the hold is not offered**
  and the control opens the dialog, because the fill is the only thing
  that shows how long is left.
- **Single-key shortcuts are suppressed whenever a text input holds
  focus.** Typing "axe" into a filter box must not approve, kill, and
  open something.
- **Pilot is exempt.** Once the terminal has focus, every keystroke
  belongs to the terminal. Only `Esc Esc` releases it.

## Focus model

Focused and selected are different states and can coexist. A 1px ring
around a full-width table row is nearly invisible, so the row does
something stronger.

```
focused row   2px --accent left edge bar + --bg-hover
selected row  --select-ground fill and a --select-frame hairline
focused ctrl  2px --accent ring at 2px offset, per the global focus rule
```

Focus is visible at all times during keyboard navigation, not only on
`:focus-visible` heuristics — if the person is driving with `j`/`k`, the
ring is the cursor.

## Command palette

A floating layer, so `--bg-overlay` and a shadow.

```
surface    glass (Depth): --bg-overlay at 78%, blurred · 1px light edge ·
           --radius-lg · --shadow-overlay
width      560px · max-height 400px · scroll-area beyond
anchor     horizontally centred across the window; field top-aligned at
           20% of viewport height; never vertically centered
input      --text-base · no border · --fg-default · placeholder --fg-subtle
           bottom rule --border-subtle
input row  48px · --text-base · set off by a hairline
row        32px · --radius-md · --text-sm · --fg-default
           16px leading icon · shortcut right-aligned as kbd
hover row  one flat step, --bg-step
active row 16% accent wash and a 2px accent bar, no fill block
section    28px · mono caps --text-2xs · --fg-subtle
```

Top-anchored rather than centered because a centered dialog shifts
vertically as the result count changes, and a target that moves while
you type is a target you misclick.

**What is typed can be dispatched as well as searched.** Once there is a
query, a row offers it as a Job, as a Session where Sessions are served, or as
an Autopilot where it is, and Tab steps through them. That row leads when nothing else matches or when the words are a link; otherwise it waits at the foot, so ↵ still opens
the match — a Job is searched by its title, and a title is a sentence. `n` and `⌘N` do not open the palette: they open
the Dispatch sheet directly (the owner, 15 Oct 2026). `⌘↵` dispatches the words whichever row is picked, by opening that
sheet holding them. The sheet reads the repository from them and asks in its own frame when there is none. The full composer for a file or a sketch is opened from a Studio's Sketch.

**What is typed can also be added to Projects.**
The query offers a Task and a Project beside the dispatch rows, and Tab to either moves it to the top so ↵ adds.
A Task then picks its project, or none, and goes to the repository the last task did. A Project is his and asks for no repository.
What a project is: [Project](../../concepts/project.md).

**Any task is found by its title.**
Tasks and subtasks of the Projects tab are listed while something is typed, in a Tasks block after the Jobs, each with its project or No project and its repository on a quiet second line.
Choosing one goes to the Projects tab on that task's project, or All tasks for a task in no project, shows done tasks if it is done, and scrolls its card into view with the focus on it.

**Contents, in order:** actions available on the current context,
navigation, jobs by id or name, sessions by any word of the title or by id,
tasks by title (while something is typed), settings. Only work in hand is listed: a Job that is not over, a Session
that is live.

**The palette obeys the lexicon.** Displayed labels always use the
lexicon term — Kill, Drone, Job Board. The search index may carry aliases
so that "terminate" finds Kill, but the alias never renders. This is
where the lexicon earns its keep: one vocabulary, searchable, with the
shortcut shown beside every entry.

**A matched span is marked by contrast**: `--fg-default` against the rest
of its label at `--fg-muted`, and never by a hue or a fill. Status hue is
never chosen and the accent is reserved to interactive affordance, so a
step within the row's one hue is the channel left. A danger row marks the
match by underline instead, because red has no contrast step. **An
alias hit marks nothing**, because the match was on a word that never
renders and there is nothing on the lexicon term to mark.

**The leading glyph read `12px` here until 2026-09-03.**
[Iconography](../iconography.md) is the authority on what a silhouette
means and at what size it holds, and this line was transcribing it
wrongly: every glyph the Navigation section draws is assigned 16px
there, and `hard-drive`'s row records that its interior marks merge into
the band above them below that size. The record was wrong rather than
holding a second opinion, so it is the record that moved. A 32px row
takes 16px comfortably.

**A row that cannot act draws dimmed and says why**, in `--fg-subtle` at
`--text-2xs`, beside its binding. Two kinds reach it: a binding the
registry carries that nothing answers, where `unbuilt` names the issue,
and an act the app cannot reach from where you are standing. Both are
facts rather than a list of exceptions, which is the same reasoning the
`not built` annotation carries under Two tiers.

**One exception: an act on a focused row is left out.**
Open and Close need a row to act on. With nothing under the cursor, dimmed rows
all saying *nothing focused* led the palette and told a person nothing they could
act on. A `not built` row is never left out. Settled 2026-09-17.

**The palette is the discovery surface.** It is how a person learns
forty shortcuts without a cheat sheet, which is why every entry displays
its binding and why no action may exist outside it.

## kbd

The one non-shadcn primitive. Used in palette rows, dropdown-menu items,
and tooltips.

```
surface  --bg-raised · --border-default · --radius-sm
lip      bottom edge at twice --border-width
type     --text-2xs mono · --fg-muted
size     20px height, border included · 4px horizontal padding
```

Never `--fg-default` — a shortcut hint is reference material sitting
beside the thing it describes, and rendering it at full contrast makes
it compete with the label.

**Raised, and edged like a card rather than ruled like a table.** This
was `--bg-sunken` behind `--border-subtle`, and on any hovered or
focused row the edge was not visible: `--border-subtle` is `#232B35`
and `--bg-hover` is `#212A34`. A key drawn on a row appears only while
that row is focused, so the one ground it was guaranteed to sit on was
the one it disappeared against, leaving a letter loose beside a button.
`--border-subtle` rules a table; an edge is `--border-default`.

The fill follows the same argument. A key is pressed, so it stands off
its surface; `--bg-sunken` is a well you type into, which is the
opposite claim. The doubled bottom edge is the lip, and it is what
makes the box read as a key rather than as a small chip.

## Consequences elsewhere in this document

- **Tooltips gain a trailing kbd** where the action has a binding. The
  400ms delay stands.
- **Dropdown-menu items gain a right-aligned kbd.** Item height is
  unchanged by it.
- **A confirmation's buttons gain a trailing kbd**, on the one control
  `Enter` fires — Cancel on a plain confirmation, the confirm on a
  dialog carrying a field. One per dialog, because two would be the
  ambiguity the safety rule above was rewritten to remove.
- **A control may draw its own Global binding while `⌘` is held**, and
  never at rest. Helm's reopen button and the Send
  button in a message box each do; the badge mounts on the hold and goes
  on release. This is not the rule under the debug payload — that one
  refuses a kbd standing permanently on a button, which is reference
  material sitting on the thing it describes. A hold is asked for: a
  person pressing `⌘` is looking for what it reaches, and a badge that is
  gone a moment later cannot compete with the label. Every reveal reads
  one listener through `ShortcutRevealProvider`, because a hold watched in
  two places ends in two places. Contextual bindings are never revealed
  this way — holding `⌘` fires none of them.
- **One exception: the Cockpit deck's key caps.** The deck's keys are
  contextual bindings, and their caps are hidden until `⌘` is held. The owner chose this on 26 Oct 2026, over the
  rule above, because the caps crowded the small deck. The cost: a held `⌘`
  then a letter is `⌘`+letter, so a person releases `⌘` before pressing the
  key. `docs/concepts/cockpit.md`, *Density*.
