// The console with instruments mods supply: Reviews drawing, Spend faulted, and Make one in the rack, whose Mod Job lives on the bay it builds in. Mock only: the Job is a fixture
// that asks one question, and answering it hands back Main build. The claims are in `walks/console-instrument-mods.ts`.

import type { Session } from "@armada/screens/src/draft/sessions";

import { forgetBays } from "../../cockpit/deck/ConsoleBays";
import { installInstruments } from "../../cockpit/deck/instrument-mods";
import { MAIN_BUILD, MAIN_BUILD_BUILDING, REVIEWS, SPEND } from "../instrument-samples";
import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

const JOB = "mod-main-build";
const FILE = "~/Library/Application Support/Armada/mods/main-build/instrument.js";
const job: Session = { id: JOB, title: "Mod Main build", mod: true, modOfBay: true, attachments: [], rows: [], turn: { state: "idle" }, standing: "idle" };

export const s263ConsoleInstrumentMods: Scenario = {
  ...ship,
  name: "console-instrument-mods",
  says: "The console with instruments mods supply, a faulted one, and Make one, whose Job lives on its bay",
  draft: {
    ...ship.draft!,
    sessions: (control, more = []) => {
      const store = ship.draft!.sessions!(control, [...more, job]);
      window.localStorage.setItem("armada.console.bays", JSON.stringify(["mod:reviews", "beacon", "mod:spend", "scope"]));
      forgetBays();
      let finish: () => void = () => undefined;
      installInstruments([REVIEWS, SPEND], (said) => {
        store.edit(JOB, (one) => ({ ...one, turn: { state: "working" }, standing: "working", rows: [{ id: "b1", at: "13:40:00", kind: "message", from: { kind: "you" }, text: said }, { id: "b2", at: "13:40:03", kind: "tool", text: "scaffold_mod main-build" }] }));
        setTimeout(() => store.edit(JOB, (one) => ({ ...one, asked: { command: `Write ${FILE}` }, rows: [...one.rows, { id: "b3", at: "13:40:09", kind: "message", from: { kind: "agent" }, text: "Writing the face." }] })), 1500);
        const built = new Promise<typeof MAIN_BUILD>((done) => {
          finish = () => {
            store.edit(JOB, (one) => ({ ...one, turn: { state: "idle" }, standing: "idle", rows: [...one.rows, { id: "b4", at: "13:40:30", kind: "message", from: { kind: "agent" }, text: "Main build is drawn. Runs over 50 minutes are amber." }] }));
            done({ ...MAIN_BUILD, job: JOB });
          };
        });
        return { stub: { ...MAIN_BUILD_BUILDING, job: JOB }, built };
      });
      return {
        ...store,
        answer: (id, ...rest) => {
          store.answer(id, ...rest);
          if (id === JOB) finish();
        },
      };
    },
  },
};
