// Mod: a right press on anything opens a menu with Mod, and Mod starts a Job for modding Armada itself, carrying what was pointed at.
// A Mod Job sits under Mods in the Cockpit's Jobs list, and its right column lists the mods it made, each with its switch. The walk
// `mod-job` plays it. Mock only: the Job the mock Fleet starts under a `Mod ` title is marked `mod`, and it writes the dusk theme.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { MOD_TITLE } from "../../mod-job";
import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { mockModsMade } from "../mods-made";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";
import { DUSK_MOD, mockThemes } from "../themes";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const MODS = "~/Library/Application Support/Armada/mods/dusk";

const sessions = () => [
  hosted("01SESSIONAAAAAAAAAAAAAAAAA", { title: "Fix the flaky store test", attachments: [held("slot", "3"), held("branch", "fix/flaky-store")] }),
  hosted("01SESSIONBBBBBBBBBBBBBBBBB", { title: "Draft the migration notes", attachments: [held("slot", "4"), held("branch", "docs/migration")] }),
];

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(sessions()).scenario(board);
  return {
    ...served,
    name: "mod-job",
    says: "Mod in the right-click menu starts a Mod Job under Mods in the Jobs list, and its right column lists the mods it made",
    // A Fleet of its own for each window, so a walk told twice starts from the same Jobs.
    behaves: (handle) => {
      const fake = new FakeSessionsFleet(sessions());
      fake.startsNow = true;
      const own = fake.scenario(board).behaves!(handle);
      return {
        ...own,
        startSession: async (title, root, request, tuning, after) => {
          const started = await own.startSession!(title, root, request, tuning, after);
          if (!started.ok || title === undefined || !title.startsWith(MOD_TITLE)) return started;
          const record = { ...started.value, title, mod: true as const };
          fake.changed(record);
          return { ok: true, value: record };
        },
        sendSessionMessage: async (send) => {
          await own.watchSession!(send.session_id);
          const sent = await own.sendSessionMessage!(send);
          void fake
            .agentTurn(send.session_id, [
              { kind: "tool", id: "w1", at: "2026-10-23T09:00:05.000Z", text: `Write ${MODS}/mod.toml` },
              { kind: "tool", id: "w2", at: "2026-10-23T09:00:06.000Z", text: `Write ${MODS}/theme.css` },
              { kind: "message", id: "w3", at: "2026-10-23T09:00:08.000Z", from: { kind: "agent" }, text: "Dusk is saved and on." },
            ])
            .then(() => {
              mockThemes.install(DUSK_MOD);
              mockThemes.setActive(DUSK_MOD.name);
              mockModsMade.add(send.session_id, { name: DUSK_MOD.name, title: DUSK_MOD.title, kind: "theme", enabled: true });
            });
          return sent;
        },
      };
    },
  };
}

export const s323ModJob: Scenario = build();
