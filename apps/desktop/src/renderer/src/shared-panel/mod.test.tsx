// Change it, in a mod's side panel, opens a box on the change, worded by what the mod is. A tweak is laid over a theme, so it reads as one.

import { expect, test } from "vitest";
import type { ModKind } from "@armada/protocol";

import { changeWords } from "./mod";

test("the box opens on the change, named by the mod's kind, the caret after the colon", () => {
  const said: Record<ModKind, string> = {
    theme: "Change the Dusk theme: ",
    tweak: "Change the Dusk theme: ",
    layout: "Change the Dusk layout: ",
    words: "Change the Dusk words: ",
    fleet: "Change the Dusk Fleet rule: ",
    tracker: "Change the Dusk tracker: ",
    commands: "Change the Dusk Commands: ",
    tab: "Change the Dusk tab: ",
  };
  for (const [kind, words] of Object.entries(said)) expect(changeWords("Dusk", kind as ModKind)).toBe(words);
});
