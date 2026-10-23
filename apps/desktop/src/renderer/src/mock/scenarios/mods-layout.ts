// Armada Mods, tier 2: layout. The real app over a merge line and one Session Bridge hosts. The
// Session answers a request for a layout by writing `mods/tidy/layout.json` into the mod folder, and
// the Dashboard and the rail re-lay out. Layout state is mock-only (`../layout.ts`).

import type { MergeLine } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import { mockLayout, TIDY_DASHBOARD, writeTidy } from "../layout";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const MODS = "~/Library/Application Support/Armada/mods/tidy";
const PULL = "https://git.example/armada/pull/";
const SESSION = "01SESSIONLAYOUTAAAAAAAAAA";

function mergeLine(): MergeLine {
  const entry = (place: number, number: number, branch: string, state: string) => ({ place, branch, pull_request: { number, url: `${PULL}${number}` }, state });
  return {
    root: repository().root,
    line: [entry(1, 1861, "fix/pin-store-clock", "gating"), entry(2, 1862, "fix/order-store-migrations", "waiting"), entry(3, 1863, "docs/typo-in-the-readme", "waiting")],
    off: [],
    landed: [],
    sent_back: [],
    hub: { main: { state: "green", commit: "9f3c2d1a7b", read_at: "2026-10-08T09:00:00.000Z" } },
  };
}

const base = holding("mods-layout", "Settings → Layout, the Mods surface, and a Session that writes a layout");

function build(): Scenario {
  const board: Scenario = { ...base, state: { ...base.state, repository: repository().root, mergeLines: { lines: [mergeLine()] } } };
  const sessions = () => new FakeSessionsFleet([hosted(SESSION, { title: "A layout", manifest_id: repository().manifest!.id, attachments: [held("slot", "3")] })]);
  return {
    ...sessions().scenario(board),
    name: "mods-layout",
    says: "A Session that writes a layout: the Cockpit, a Job and the tabs follow, and Settings → Layout takes it back",
    behaves: (handle) => {
      mockLayout.reset();
      const fake = sessions();
      const own = fake.scenario(board).behaves!(handle);
      let asked = 0;
      return {
        ...own,
        sendSessionMessage: async (send) => {
          await own.watchSession!(send.session_id);
          const sent = await own.sendSessionMessage!(send);
          asked += 1;
          fake.row(send.session_id, { id: `w${asked}1`, at: "2026-10-08T09:10:05.000Z", kind: "tool", text: asked === 1 ? `Write ${MODS}/mod.toml` : `Edit ${MODS}/layout.json` });
          if (asked === 1) fake.row(send.session_id, { id: "w12", at: "2026-10-08T09:10:06.000Z", kind: "tool", text: `Write ${MODS}/layout.json` });
          fake.row(send.session_id, {
            id: `w${asked}3`,
            at: "2026-10-08T09:10:08.000Z",
            kind: "message",
            from: { kind: "agent" },
            text: "Activity is off the Cockpit, Workflow is first in a Job, and the Logbook is off the tabs.",
          });
          writeTidy(TIDY_DASHBOARD);
          // The turn is over, so the next request can be sent.
          fake.changed(hosted(SESSION, { title: "A layout", manifest_id: repository().manifest!.id, attachments: [held("slot", "3")] }));
          return sent;
        },
      };
    },
  };
}

export const s207ModsLayout: Scenario = build();
