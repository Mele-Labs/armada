// A Session's retro. The Sessions list with the Retro press on each row, over the Retros page that
// holds two Jobs' retros. Pressing it writes the Session's retro (the mock takes a moment, so the
// mark is seen moving) and opens it on the Retros page. The walk `session-retro` plays it.

import type { Lesson, RepositorySummary } from "@armada/protocol";
import { retroFixtures, sessionRetro } from "@armada/jobs/fake";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const now = () => new Date().toISOString();

const records = () => [
  hosted("01WORKINGBBBBBBBBBBBBBBBBB", { title: "Draft the migration notes", last_turn_at: now(), hosted: { turn: { state: "working" }, mode: "auto", running: true } }),
  hosted("01IDLECCCCCCCCCCCCCCCCCCCC", { title: "Fix the flaky store test", last_turn_at: now(), attachments: [held("slot", "3")] }),
];

function build(refusal?: { code: string; message: string }): Scenario {
  const board = onBoard([job("escalated", { id: "01JOBSTOPPED", handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" })], {
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const { retros, lessons } = retroFixtures();
  const before: Lesson[] = [...lessons];
  const pressed: Record<string, number> = {};
  const served = new FakeSessionsFleet(records(), {}).scenario(board);
  return {
    ...served,
    name: refusal === undefined ? "session-retro" : "session-retro-refused",
    says: "The Sessions list with the Retro press on each row, and the Retros page it opens",
    retros,
    lessons,
    behaves: (handle) => {
      const fleet = new FakeSessionsFleet(records(), {});
      // **The press writes into the arrays the Retros page reads**, in place, and each mount starts from the Jobs' two.
      lessons.splice(0, lessons.length, ...before);
      for (const id of Object.keys(retros)) if (!before.some((one) => one.job_id === id)) delete retros[id];
      for (const id of Object.keys(pressed)) delete pressed[id];
      fleet.refusesRetro = refusal;
      fleet.onRetro = (id, title) => {
        // **Each press is the Session's next retro**: held under `{id}-r{n}` for the numbered read, and under the id as the newest.
        const n = (pressed[id] = (pressed[id] ?? 0) + 1);
        const written = sessionRetro({ id, ...(title === undefined ? {} : { title }) }, now(), n);
        retros[id] = written.retro;
        retros[`${id}-r${n}`] = written.retro;
        lessons.splice(0, 0, ...written.lessons);
      };
      return fleet.scenario(board).behaves!(handle);
    },
  };
}

export const s205SessionRetro: Scenario = build();

/** The same, with Fleet refusing the retro: another is already being written. */
export const s205SessionRetroRefused: Scenario = build({ code: "fleet.session_retro_being_written", message: "A retro is already being written for this Session" });
