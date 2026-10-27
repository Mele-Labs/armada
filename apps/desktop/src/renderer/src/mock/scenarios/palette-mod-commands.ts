// The rows a Commands mod brings, read the way the real window reads them: a mod on the list and the `commands` Fleet checked for
// it, not rows handed to the palette. One row for each of the four acts. Over the palette like Raycast's, with its own commands
// left out. Played by the walk of the same name.

import type { CommandsFile, ModChecked } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s332PaletteLikeRaycast as base } from "./palette-like-raycast";

const MOD = "team-commands";

const FILE: CommandsFile = {
  command: [
    { name: "Security pass on a pull request", aliases: ["sec"], hotkey: "⌥⌘S", prompt: "Review {pr} for {focus} problems", blanks: ["pr", "focus"] },
    { name: "Open pull request on the forge", aliases: ["forge"], url: "https://forge.example/mele-labs/armada/pull/{pr}", blanks: ["pr"] },
    { name: "Triage the board", chain: ["nav:reviews", "nav:projects"] },
    { name: "Audit dependencies", shell: "pnpm audit" },
  ],
};

const checked = (name: string): ModChecked | null => (name === MOD ? { name, valid: true, problems: [], commands: FILE } : null);

export const s333PaletteModCommands: Scenario = {
  ...base,
  name: "palette-mod-commands",
  says: "The palette with the rows of a Commands mod: a prompt with two blanks, an address, a chain and a shell line",
  state: { ...base.state, mods: { mods: [{ name: MOD, origin: "local", kind: "commands", version: "1", enabled: true, valid: true, changed_at: "2026-10-27T09:00:00Z" }] } },
  draft: { ...base.draft, palette: { ...base.draft!.palette!, commands: [], pinned: [], recent: [] } },
  behaves: (fleet) => ({ ...(base.behaves?.(fleet) ?? {}), validateMod: async (name) => checked(name) }),
};
