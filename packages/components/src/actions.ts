// What Bridge does with the act registry, and nothing the registry already says.
//
// **The map itself is generated.** `crates/core-model/domain/actions.toml` is
// the authority on what an act is called, what it draws and what it is bound
// to, and `./generated/actions.ts` is that file in TypeScript, emitted by
// `pnpm --filter @armada/desktop codegen` and held to the registry by
// `cargo xtask verify-foundations`. This file used to be a field-for-field
// transcription of the same table; a registry with two copies has two answers
// the day one of them is edited alone.
//
// What is left here is what the registry has no column for and should not: an
// alias a person might type, and which acts a palette offers where it is
// standing. Both are Bridge's readings of the map rather than facts about it.

import { ACTIONS } from "./generated/actions";
import type { Action, ActionScope } from "./generated/actions";
import { boundKeyFor } from "./keymap";

// The map, its types and `ACTION` reach `@armada/components`' consumers through
// here, so an importer neither knows nor cares which half of this pair a name
// came from. Moving the transcription into a generated file is not an API
// change and should not read as one at a call site.
export * from "./generated/actions";

/**
 * What a person might type looking for an act, and never what renders.
 *
 * **Bridge's, not the registry's.** `actions.toml` carries no alias column and
 * should not: it is the authority on what an act *is called*, and the lexicon's
 * whole point is that one word wins. The search index is a different artifact —
 * the contract says so in as many words, that the index may carry aliases so
 * "terminate" finds Kill and the alias never renders.
 *
 * So a word here is a way in, and adding one costs nothing. What it may never
 * do is reach the screen: the palette is where a person learns Armada's
 * vocabulary, and a surface that answers "terminate" with the word "terminate"
 * has taught them the wrong one.
 */
export const ALIASES: Readonly<Record<string, readonly string[] | undefined>> = {
  // "dispatch" left the alias list on 2026-09-14 (#1087): it is the verb now,
  // and an alias identical to the label it finds teaches nothing.
  new_job: ["start", "compose", "session", "new session"],
  copy_debug_info: ["clipboard", "support", "envelope"],
  debug_bridge: ["screenshot", "diagnose", "state"],
  state_filter: ["tab", "narrow", "status"],
};

/**
 * The contexts the palette is drawn in.
 *
 * **Not the same thing as a scope.** A scope says where a binding is offered
 * and the registry decides how many there are; a context is where a person is
 * standing. `open studio` is a place: the acts
 * scoped to it act on the board rather than on anything focused, so a Studio
 * open on its whiteboard is neither the board nor a job read whole.
 */
export type ActionContext = "board" | "detail" | "studio";

/**
 * Which contexts each scope is offered in.
 *
 * **Keyed by scope rather than by context, so a new scope cannot be silent.**
 * `ActionScope` is generated from the rows of `actions.toml`, so a scope added
 * there widens this record's key set and the build stops here naming it. The
 * other direction — a list of scopes under each context — compiles clean and
 * drops the new scope out of every palette, which is the same defect the
 * registry exists to prevent, one layer down.
 */
const CONTEXTS_FOR: Readonly<Record<ActionScope, readonly ActionContext[]>> = {
  anywhere: ["board", "detail", "studio"],
  list: ["board"],
  "list and detail": ["board", "detail"],
  "open studio": ["studio"],
  // Keys on the Dashboard's panel and on a call in front: found from its own `?` sheet, not drawn as
  // palette rows, which would each need a handler the panel owns.
  dashboard: [],
  // The flight deck's keys: found from the deck's own `?` sheet.
  deck: [],
  // The Reviews tab's keys: found from Settings → Keyboard and the tab's own caps.
  review: [],
  // The Issues tab's keys: found from Settings → Keyboard and the tab's own caps.
  issues: [],
  // The Projects tab's keys: found from Settings → Keyboard and the tab's own caps.
  projects: [],
  call: [],
  // The Logbook's keys: found from Settings → Keyboard and the caps on its acts.
  logbook: [],
  // A plan card's keys: found from Settings → Keyboard and the tooltips on its acts.
  plan: [],
};

/**
 * The acts a context offers on what is in front of you, in registry order.
 *
 * **Contextual tier only.** The global acts reach a surface rather than a Job,
 * so they do not belong under a block titled with the job it acts on — a
 * palette that says `job_2d90bb` and then offers Toggle sidebar under that
 * heading is a heading that lies. `globalActs` is where they are.
 *
 * **Motions are dropped.** A Motion acts on nothing, so a row for it would be
 * a row that does nothing when chosen — the same reading the contract gives
 * unbuilt rows, one step further along.
 */
export function actsIn(context: ActionContext): readonly Action[] {
  return ACTIONS.filter(
    (action) =>
      action.kind === "Action" &&
      action.tier === "Contextual" &&
      CONTEXTS_FOR[action.scope].includes(context),
  );
}

/**
 * The global acts, which reach a surface rather than a Job.
 *
 * **Three rows are left out and none is an omission.** `command_palette` is
 * the one act that cannot be reached from inside the palette, since the
 * palette is what is already open; `deck_tabs` is one registry row
 * standing for the title bar's tabs, because the contract's rule there is the
 * tab's place in the bar rather than the digits; and `open_settings` is the
 * Settings destination's own key, which the roster's Settings row already
 * draws. A palette draws the destinations one per row, from the roster —
 * which is the only thing that knows what `⌘3` currently reaches.
 */
export function globalActs(): readonly Action[] {
  return ACTIONS.filter(
    (action) =>
      action.kind === "Action" &&
      action.tier === "Global" &&
      action.id !== "command_palette" &&
      action.id !== "deck_tabs" &&
      action.id !== "open_settings",
  );
}

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
