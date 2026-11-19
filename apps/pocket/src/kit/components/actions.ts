// What Bridge does with the act registry, and nothing the registry already says.
//
// **The map itself is generated.** `crates/core-model/domain/actions.toml` is
// the authority on what an act is called, what it draws and what it is bound
// to, and `./generated/actions.ts` is that file in TypeScript, copied
// from Bridge Classic's emit and trimmed to what Pocket draws. This file used to be a field-for-field
// transcription of the same table; a registry with two copies has two answers
// the day one of them is edited alone.
//
// What is left here is what the registry has no column for and should not: an
// alias a person might type, and which acts a palette offers where it is
// standing. Both are Bridge's readings of the map rather than facts about it.

import { boundKeyFor } from "./keymap";
import { type Action, ACTIONS } from "./generated/actions";

/**
 * One act, by the id the registry keys it on.
 *
 * **Nothing else may write a binding down.** `actions.toml` is the authority on
 * what a key does and the generator carries it here, and every caption on a
 * screen has to come through this — because a caption written by hand is a
 * promise about a key that nothing keeps in step. `Open the log` was captioned
 * `Enter` for four rounds of feedback after the registry moved it to `L`: the
 * shared configuration was there the whole time and that one call site simply
 * did not read it. `Open the diff` beside it said `f`, which was right by
 * coincidence and would have gone the same way on the next move.
 *
 * Throws on an id the registry does not carry, rather than rendering nothing:
 * a caption that quietly vanished is how a binding goes missing unnoticed.
 */
export function actionOf(id: string): Action {
  const found = ACTIONS.find((act) => act.id === id);
  if (found === undefined) {
    throw new Error(`no action \`${id}\` in the registry — actions.toml is the authority`);
  }
  return found;
}

/**
 * The key an act answers to now, for drawing beside it — the registry's, or what this person
 * rebound it to in Settings → Keyboard (`./keymap.ts`). `""` where they unbound it.
 *
 * A string rather than a `Kbd`, so the caller decides where it renders and
 * this decides only what it says.
 */
export function keyFor(id: string): string {
  actionOf(id);
  return boundKeyFor(id);
}
