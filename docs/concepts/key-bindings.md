# Key bindings

**What it is:** The keys each act answers, as this person has them: the ones `actions.toml` ships, with whatever they rebound in Settings → Keyboard shortcuts laid over them, kept by Fleet and read by every key handler in Bridge.

---

**Kind:** Concept.

You would rather `p` opened Dispatch than `n`.
Settings → Keyboard shortcuts lists every act by where you stand when you press it; you press Dispatch's key, then `p`.
The Cockpit and every caption that drew `n` now answer and draw `p`, and the next Bridge on this Fleet does too.

## Two layers

> **Rule.** `crates/core-model/domain/actions.toml` says what an act ships bound to; the keymap says what it is bound to now. No handler compares a press to a letter.
> Why: a handler that spells its key is a binding Settings cannot move, and a caption that spells one is a promise nothing keeps.

The keymap is `packages/components/src/keymap.ts`.
A handler asks `isPressed(id, event)`, `pressedSlot(id, event)` for an act with several keys, or `pressedDigit(id, event)` for a row of digits; a caption draws `keyFor(id)`.

| An act's spelling | Slots | Rebinding takes |
|---|---|---|
| `⌘K`, `x`, `B` | One key | The next key pressed |
| `⌘[ ⌘]`, `j / k / ↓ / ↑` | One per key, each named — Back, Forward | The next key pressed, for that slot |
| `⌘1–⌘6`, `1–6` | One row of digits | The modifiers held with any digit |

Esc lets go of a key without changing it and Delete unbinds it. Reset puts one act's keys back, and Reset all to defaults puts every one back.

## Where two acts want one key

> **Rule.** A key two acts answer where both are listening is drawn as a warning on both rows, and kept.
> Why: moving two keys at once passes through a moment where both hold one, and refusing it would make the swap impossible.

Where an act listens is its scope.
A key may serve two acts that never listen on the same screen, so the shipped map has no conflict; a test holds it to that.
The Cockpit's keys are in the `deck` scope: `l b c q f a d` go to a panel, `1–6` press a flight control, `v` and `t` swap the stage, `g` then a letter goes to a tab, and `⌘1–⌘6` are the tabs.
`⌘,` is Settings, anywhere.
`o` is also Open on the deck's lists, which are in the `list` scope, so the deck reads that row and has none of its own.
`l` is also Later on a call, which answers only while a call is in front, and the deck stands down for it.
See [Cockpit](cockpit.md).

An act whose row is `unbuilt` listens to nothing yet, so it counts against no key; it counts again the day its issue closes.

## Kept by Fleet

The owner's bindings ride the `key_bindings` preference, `{ "version": 1, "bindings": { "<act id>": ["<key>", …] } }`, one list per act in the registry's slot order and `""` for an unbound slot.
Fleet refuses text `ipc::key_bindings` cannot read with 422 `fleet.unacceptable_key_bindings`, and an empty save takes the bindings back.
An act id this build does not have is kept and ignored.

## What it does not reach

| Not rebound | Why |
|---|---|
| Esc in a dialog, menu or popover | The primitive's own; it closes on Esc whatever Close is bound to |
| Arrows, Home and End inside a tab row, list box or resize handle | The control's own keyboard, as the platform has it |
| The capture window, and a Job's page | Separate windows the main process keys, which keep `⌥⌘C` and Esc |
| The annotation layer's `⌥⌘A`, and its `⌥⌘S` while it is on | Dev only, and not acts in the registry |
