// A Session's retro. The Sessions list with the Retro press on each row, over the Retros page that
// holds two Sessions' retros. Pressing it puts a `retro` row in the Session's thread that Fleet replaces
// as the writing goes: asking the agent, reading the record, the items settling, written. The row's
// `Open retro` link opens the kept retro on the Retros page; the press itself moves nobody. The live
// one holds at each stop of the writing until the walk `session-retro` says time passes.

import type { Lesson, RepositorySummary } from "@armada/protocol";
import { retroFixtures } from "@armada/screens/src/fixtures/retros";
import { sessionRetro } from "@armada/screens/src/lessons/session-retro";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted, terminal } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const now = () => new Date().toISOString();

const records = () => [
  hosted("01WORKINGBBBBBBBBBBBBBBBBB", { title: "Draft the migration notes", last_turn_at: now(), hosted: { turn: { state: "working" }, mode: "auto", running: true } }),
  hosted("01IDLECCCCCCCCCCCCCCCCCCCC", { title: "Fix the flaky store test", last_turn_at: now(), attachments: [held("slot", "3")] }),
  // A terminal Session has no `retro` row, its thread being a transcript.
  terminal("01TERMINALDDDDDDDDDDDDDDDDD", { title: "Tidy the release notes", last_turn_at: now() }),
];

/** The items the model writes, as they settle: the retro's own, with the fields the model writes. */
const written = () =>
  sessionRetro({ id: "script" }, now()).retro.items!.map((one) => ({
    ...(one.title === undefined ? {} : { title: one.title }),
    ...(one.what === undefined ? {} : { what: one.what }),
    ...(one.fix === undefined ? {} : { fix: one.fix }),
    who: one.who,
    ...(one.lands_in === undefined ? {} : { lands_in: one.lands_in }),
  }));

function build(options: { refusal?: { code: string; message: string }; live?: boolean; nothingNew?: boolean } = {}): Scenario {
  const { refusal, live = false, nothingNew = false } = options;
  const board = onBoard({
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const { retros, lessons } = retroFixtures();
  const before: Lesson[] = [...lessons];
  const pressed: Record<string, number> = {};
  const served = new FakeSessionsFleet(records(), {}).scenario(board);
  return {
    ...served,
    name: refusal !== undefined ? "session-retro-refused" : live ? "session-retro-live" : nothingNew ? "session-retro-nothing-new" : "session-retro",
    says: live
      ? "A Session's retro being written in its thread, held at each stop until time passes"
      : "The Sessions list with the Retro press on each row, and the retro written in the Session's thread",
    retros,
    lessons,
    behaves: (handle) => {
      const fleet = new FakeSessionsFleet(records(), {});
      // **The press writes into the arrays the Retros page reads**, in place, and each mount starts from the two Sessions'.
      lessons.splice(0, lessons.length, ...before);
      for (const id of Object.keys(retros)) if (!before.some((one) => one.session.id === id)) delete retros[id];
      for (const id of Object.keys(pressed)) delete pressed[id];
      fleet.refusesRetro = refusal;
      fleet.retroHeld = live;
      fleet.retroNothingNew = nothingNew;
      fleet.retroStepMs = live ? 250 : 40;
      fleet.retroItems = written();
      fleet.onRetro = (id, title) => {
        // **Each press is the Session's next retro**: held under `{id}-r{n}` for the numbered read, and under the id as the newest.
        const n = (pressed[id] = (pressed[id] ?? 0) + 1);
        const kept = sessionRetro({ id, ...(title === undefined ? {} : { title }) }, now(), n);
        retros[id] = kept.retro;
        retros[`${id}-r${n}`] = kept.retro;
        lessons.splice(0, 0, ...kept.lessons);
        return n;
      };
      return fleet.scenario(board).behaves!(handle);
    },
  };
}

export const s205SessionRetro: Scenario = build();

/** The same, held at each stop of the writing so a walk can look at it. `later` goes on. */
export const s205SessionRetroLive: Scenario = build({ live: true });

/** The same, where Fleet answers a press with the Session's last retro and puts no row in the thread: nothing is new since it. */
export const s205SessionRetroNothingNew: Scenario = build({ nothingNew: true });

/** The same, with Fleet refusing the retro: another is already being written. */
export const s205SessionRetroRefused: Scenario = build({ refusal: { code: "fleet.session_retro_being_written", message: "A retro is already being written for this Session" } });
