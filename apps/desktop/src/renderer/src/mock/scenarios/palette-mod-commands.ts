// The rows a Commands mod brings, read the way the real window reads them: a mod on the list and the `commands` Fleet checked for
// it, not rows handed to the palette. One row for each of the four acts; the shell line prints from a fake Fleet. Over the palette like Raycast's, with its own commands
// left out. Played by the walk of the same name.

import type { CommandsFile, ModChecked, SessionRunMessage } from "@armada/protocol";

import type { Scenario } from "../moment";
import { sharedPanelReads } from "../shared-panel-reads";
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

/** A Fleet that runs the line it is given in a directory: it prints what an audit prints, then ends. Output is sent as main sends it, by run id. */
function fakeRun() {
  const hearers = new Set<(runId: string, message: SessionRunMessage) => void>();
  const ok = async () => ({ ok: true, value: { run_id: "run-audit" } }) as const;
  return {
    startTerminalRun: async (command: string, dir: string) => {
      const send = (message: SessionRunMessage) => hearers.forEach((one) => one("run-audit", message));
      setTimeout(() => send({ kind: "output", text: `${dir}\r\n$ ${command}\r\n` }), 150);
      setTimeout(() => send({ kind: "output", text: "found 0 vulnerabilities in 412 packages\r\n" }), 400);
      setTimeout(() => send({ kind: "exited", code: 0, stopped: false }), 650);
      return ok();
    },
    writeSessionRun: ok,
    resizeSessionRun: ok,
    stopSessionRun: ok,
    onSessionRunMessage: (on: (runId: string, message: SessionRunMessage) => void) => {
      hearers.add(on);
      return () => void hearers.delete(on);
    },
  };
}

export const s333PaletteModCommands: Scenario = {
  ...base,
  name: "palette-mod-commands",
  says: "The palette with the rows of a Commands mod: a prompt with two blanks, an address, a chain and a shell line that runs in a side panel",
  state: { ...base.state, mods: { mods: [{ name: MOD, origin: "local", kind: "commands", version: "1", enabled: true, valid: true, changed_at: "2026-10-27T09:00:00Z" }] } },
  draft: {
    ...base.draft,
    palette: { ...base.draft!.palette!, commands: [], pinned: [], recent: [] },
    sharedPanel: () => ({ ...sharedPanelReads(base.state.mergeLines ?? { lines: [] }), terminalRun: fakeRun() }),
  },
  behaves: (fleet) => ({ ...(base.behaves?.(fleet) ?? {}), validateMod: async (name) => checked(name) }),
};
