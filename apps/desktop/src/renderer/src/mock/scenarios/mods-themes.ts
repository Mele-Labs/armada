// Armada Mods, tier 1: themes. The real app over a Board with one Job and one Session Bridge hosts. The
// Session answers a request for a theme by writing `mods/dusk/` into the mod folder, and the theme
// shows in Settings and in Mods. Mod state is mock-only (`@armada/settings`'s `mods.ts`). The walk
// `modsThemes` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";
import { DUSK_MOD, installMod, resetMods } from "@armada/settings";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const MODS = "~/Library/Application Support/Armada/mods/dusk";

function build(): Scenario {
  const board = onBoard([job("running", { id: "01JOBRUNNING", handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" })], {
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const served = new FakeSessionsFleet([hosted("01SESSIONMODSAAAAAAAAAAAAA", { title: "A theme", attachments: [held("slot", "3")] })]).scenario(board);
  return {
    ...served,
    name: "mods-themes",
    says: "Settings → Theme, the Mods surface, and a Session that writes a theme",
    behaves: (handle) => {
      resetMods();
      const fake = new FakeSessionsFleet([hosted("01SESSIONMODSAAAAAAAAAAAAA", { title: "A theme", attachments: [held("slot", "3")] })]);
      const own = fake.scenario(board).behaves!(handle);
      return {
        ...own,
        sendSessionMessage: async (send) => {
          await own.watchSession!(send.session_id);
          const sent = await own.sendSessionMessage!(send);
          fake.row(send.session_id, { id: "w1", at: "2026-10-07T13:48:05.000Z", kind: "tool", text: `Write ${MODS}/mod.toml` });
          fake.row(send.session_id, { id: "w2", at: "2026-10-07T13:48:06.000Z", kind: "tool", text: `Write ${MODS}/theme.css` });
          fake.row(send.session_id, { id: "w3", at: "2026-10-07T13:48:08.000Z", kind: "message", from: { kind: "agent" }, text: "Dusk is in Settings, under Theme." });
          installMod(DUSK_MOD);
          return sent;
        },
      };
    },
  };
}

export const modsThemes: Scenario = build();
