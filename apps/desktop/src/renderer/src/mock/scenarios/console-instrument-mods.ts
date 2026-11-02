// The console with instruments mods supply: Reviews drawing, Spend faulted, and Make one in the rack, whose Mod Job lives on the bay it builds in. The Fleet here answers the mod list,
// `validate_mod` and the Mod Job's start as the real one does, with real `instrument.js` texts. The Job is a fixture that scaffolds, asks one question, and answering it hands back Main build.
// The claims are in `walks/console-instrument-mods.ts`.

import type { ModSummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { forgetBays } from "../../cockpit/deck/ConsoleBays";
import { MAIN_BUILD, REVIEWS, SPEND, STARTER } from "../instrument-samples";
import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

const JOB = "mod-main-build";
const FILE = "~/Library/Application Support/Armada/mods/main-build/instrument.js";
const job: Session = {
  id: JOB,
  title: "Mod Main build",
  mod: true,
  modOfBay: true,
  attachments: [],
  rows: [],
  turn: {
    state: "idle",
  },
  standing: "idle",
};
const SCAFFOLDS_AT = 300;
const ASKS_AT = 1500;

const rowOf = (name: string, glyph: string | undefined, made_by?: string): ModSummary => ({
  name,
  kind: "instrument",
  enabled: true,
  valid: true,
  changed_at: new Date().toISOString(),
  ...(glyph === undefined
    ? {}
    : {
        glyph,
      }),
  ...(made_by === undefined
    ? {}
    : {
        made_by,
      }),
});

/** Made with the window's fake: `behaves` and the draft reach one another through these. */
let store: ReturnType<NonNullable<NonNullable<Scenario["draft"]>["sessions"]>> | undefined;
let built: () => void = () => undefined;

export const s263ConsoleInstrumentMods: Scenario = {
  ...ship,
  name: "console-instrument-mods",
  says: "The console with instruments mods supply, a faulted one, and Make one, whose Job lives on its bay",
  behaves: (fleet) => {
    const code = new Map([
      [
        "reviews",
        REVIEWS,
      ],
      [
        "spend",
        SPEND,
      ],
    ]);
    let mods = [
      rowOf("reviews", "git-pull-request"),
      rowOf("spend", undefined),
    ];
    const publish = (next: ModSummary[]) => {
      mods = next;
      fleet.publish({
        mods: {
          mods,
        },
      });
    };
    publish(mods);
    built = () => {
      code.set("main-build", MAIN_BUILD);
      publish(
        mods.map((one) =>
          one.name === "main-build"
            ? {
                ...one,
                changed_at: new Date().toISOString(),
              }
            : one,
        ),
      );
    };
    return {
      ...ship.behaves?.(fleet),
      // Fleet's start of a Mod Job with the aim `instrument`: the Job is marked for a bay and told the words.
      startModSession: async (start) => {
        const said = start.said ?? "";
        store?.edit(JOB, (one) => ({
          ...one,
          turn: {
            state: "working",
          },
          standing: "working",
          rows: [
            {
              id: "b1",
              at: "13:40:00",
              kind: "message",
              from: {
                kind: "you",
              },
              text: said,
            },
          ],
        }));
        window.setTimeout(() => {
          code.set("main-build", STARTER);
          publish([
            ...mods,
            rowOf("main-build", "hammer", JOB),
          ]);
          store?.edit(JOB, (one) => ({
            ...one,
            rows: [
              ...one.rows,
              {
                id: "b2",
                at: "13:40:03",
                kind: "tool",
                text: "scaffold_mod main-build",
              },
            ],
          }));
        }, SCAFFOLDS_AT);
        window.setTimeout(
          () =>
            store?.edit(JOB, (one) => ({
              ...one,
              asked: {
                command: `Write ${FILE}`,
              },
              rows: [
                ...one.rows,
                {
                  id: "b3",
                  at: "13:40:09",
                  kind: "message",
                  from: {
                    kind: "agent",
                  },
                  text: "Writing the face.",
                },
              ],
            })),
          ASKS_AT,
        );
        return {
          ok: true,
          value: {
            session_id: JOB,
            title: job.title!,
          },
        };
      },
      validateMod: async (name) => {
        const text = code.get(name);
        return text === undefined
          ? null
          : {
              name,
              valid: true,
              problems: [],
              instrument: {
                code: text,
              },
            };
      },
      setModEnabled: async (name, enabled) => {
        publish(
          mods.map((one) =>
            one.name === name
              ? {
                  ...one,
                  enabled,
                }
              : one,
          ),
        );
        return {
          ok: true,
        };
      },
    };
  },
  draft: {
    ...ship.draft!,
    sessions: (control, more = []) => {
      const held = ship.draft!.sessions!(control, [
        ...more,
        job,
      ]);
      store = held;
      window.localStorage.setItem(
        "armada.console.bays",
        JSON.stringify([
          "mod:reviews",
          "beacon",
          "mod:spend",
          "scope",
        ]),
      );
      forgetBays();
      return {
        ...held,
        answer: (id, ...rest) => {
          held.answer(id, ...rest);
          if (id !== JOB) return;
          held.edit(JOB, (one) => ({
            ...one,
            turn: {
              state: "idle",
            },
            standing: "idle",
            rows: [
              ...one.rows,
              {
                id: "b4",
                at: "13:40:30",
                kind: "message",
                from: {
                  kind: "agent",
                },
                text: "Main build is drawn. Runs over 50 minutes are amber.",
              },
            ],
          }));
          built();
        },
      };
    },
  },
};
