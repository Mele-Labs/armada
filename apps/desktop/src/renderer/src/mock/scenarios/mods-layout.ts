// Armada Mods, tier 2: layout. The real app over a Board with a Job at work, a Job done and a merge
// line, and one Session Bridge hosts. The Session answers a request for a layout by writing
// `mods/tidy/layout.json` into the mod folder, and the Dashboard, the rail and a Job's tabs re-lay
// out. Layout state is mock-only (`../layout.ts`). The walk `modsLayout` plays it.

import type { MergeLine } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";
import { completedSuccess } from "@armada/jobs/fixtures/build/index";
import { featureRunning } from "@armada/jobs/fake";

import { asRow, holding } from "../holding";
import { mockLayout, TIDY_DASHBOARD, TIDY_JOB, writeTidy } from "../layout";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const MODS = "~/Library/Application Support/Armada/mods/tidy";
const PULL = "https://git.example/armada/pull/";
const SESSION = "01SESSIONLAYOUTAAAAAAAAAA";

const working = asRow(featureRunning(), 71, "dash-running", "Pin the store clock");
const done = asRow(completedSuccess(), 72, "dash-done", "Fold the two notification routes into one");

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

const owned = (fixture: ReturnType<typeof asRow>) => ({ ...fixture, job: { ...fixture.job, owner_manifest_id: repository().manifest!.id } });
// A Job that was cleared after it ran is on the Board's Running section instead.
const { reclaimed_at: _cleared, ...running } = owned(working).job;

const base = holding("mods-layout", "Settings → Layout, the Mods surface, and a Session that writes a layout", [{ ...owned(working), job: running }, owned(done)]);

function build(): Scenario {
  const board: Scenario = { ...base, state: { ...base.state, repository: repository().root, mergeLines: { lines: [mergeLine()] } } };
  const sessions = () => new FakeSessionsFleet([hosted(SESSION, { title: "A layout", attachments: [held("slot", "3")] })]);
  return {
    ...sessions().scenario(board),
    name: "mods-layout",
    says: "A Session that writes a layout: the Dashboard, the rail and a Job's tabs follow, and Settings → Layout takes it back",
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
          const first = asked === 1;
          fake.row(send.session_id, { id: `w${asked}1`, at: "2026-10-08T09:10:05.000Z", kind: "tool", text: first ? `Write ${MODS}/mod.toml` : `Edit ${MODS}/layout.json` });
          if (first) fake.row(send.session_id, { id: "w12", at: "2026-10-08T09:10:06.000Z", kind: "tool", text: `Write ${MODS}/layout.json` });
          fake.row(send.session_id, {
            id: `w${asked}3`,
            at: "2026-10-08T09:10:08.000Z",
            kind: "message",
            from: { kind: "agent" },
            text: first ? "Merge line is first on the Dashboard. Retros is off the rail." : "In a Job, Record is before Plan and Pulse is hidden. Jobs open on Plan.",
          });
          writeTidy(first ? TIDY_DASHBOARD : TIDY_JOB);
          // The turn is over, so the next request can be sent.
          fake.changed(hosted(SESSION, { title: "A layout", attachments: [held("slot", "3")] }));
          return sent;
        },
      };
    },
  };
}

export const s207ModsLayout: Scenario = build();
