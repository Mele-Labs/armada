// Mod: a right press on anything opens a menu with Mod, and Mod starts a Job for modding Armada itself, carrying what was pointed at.
// A Mod Job sits under Mods in the Cockpit's Jobs list, and its right column lists the mods it made, each with its switch. The walk
// `mod-job` plays it, and `create-mod` plays the same Job started empty and described in its box. Mock only: the Job the mock Fleet starts is marked `mod` as Fleet marks it, and it writes the dusk theme.

import type { ModAim, ModSummary, RepositorySummary, SessionNote } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";
import { DUSK_MOD, mockThemes } from "../themes";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const MODS = "~/Library/Application Support/Armada/mods/dusk";

/** The first message's lines about the spot, as the Mod brief and the note give them. */
function pointedAt(note: SessionNote): string {
  const { tag, text, label } = note.place.element;
  const named = (label ?? "").trim() || text.trim();
  return [`Pointed at: ${tag}${named === "" ? "" : ` "${named}"`}`, `Page: ${note.place.location}`].join("\n");
}

/** The first line of his words, cut at a word where it runs past the width. */
function titleOf(said: string): string {
  const line = (said.split("\n").find((one) => one.trim() !== "") ?? "").trim();
  if (line.length <= 40) return line;
  const cut = line.slice(0, 40);
  return (cut.includes(" ") ? cut.slice(0, cut.lastIndexOf(" ")) : cut).trimEnd();
}

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
      // The mod list Fleet publishes: the dusk theme once the Job has written it, `made_by` the Job, and this machine's switch.
      let mods: ModSummary[] = [];
      const publish = () => handle.publish({ mods: { mods } });
      const empty = new Set<string>();
      // The agent writes the dusk theme. Where he aimed at colours the reply says it is a theme, so the walk shows the pick arrived.
      const writesDusk = (id: string, aim?: ModAim): void => {
        void fake
          .agentTurn(id, [
            { kind: "tool", id: "w1", at: "2026-10-23T09:00:05.000Z", text: `Write ${MODS}/mod.toml` },
            { kind: "tool", id: "w2", at: "2026-10-23T09:00:06.000Z", text: `Write ${MODS}/theme.css` },
            { kind: "message", id: "w3", at: "2026-10-23T09:00:08.000Z", from: { kind: "agent" }, text: aim === "theme" ? "Dusk is a theme, saved and on." : "Dusk is saved and on." },
          ])
          .then(() => {
            mockThemes.install(DUSK_MOD);
            mockThemes.setActive(DUSK_MOD.name);
            mods = [...mods, { name: DUSK_MOD.name, kind: "theme", enabled: true, valid: true, made_by: id }];
            publish();
          });
      };
      return {
        ...own,
        setModEnabled: async (name, enabled) => {
          mods = mods.map((one) => (one.name === name ? { ...one, enabled } : one));
          mockThemes.setEnabled(name, enabled);
          if (enabled) mockThemes.setActive(name);
          publish();
          return { ok: true };
        },
        // What Fleet does for the real operation: starts the Job marked `mod`, and sends it the note where there is one. Neither a note nor
        // words starts it empty, titled `Mod` until his first message, then `Mod <first line>`.
        startModSession: async (start) => {
          const started = await own.startSession!(start.title, undefined, undefined, undefined, undefined);
          if (!started.ok) return started;
          const record = { ...started.value, title: start.title ?? "Mod", mod: true as const };
          fake.changed(record);
          await own.watchSession!(record.id);
          const first = start.note ? pointedAt(start.note) : start.said;
          if (first === undefined) {
            empty.add(record.id);
            return { ok: true, value: { session_id: record.id, title: record.title } };
          }
          const sent = await own.sendSessionMessage!({ session_id: record.id, text: first });
          writesDusk(record.id);
          return { ok: true, value: { session_id: record.id, title: record.title, ...(sent.ok ? {} : { send_failed: "not sent" }) } };
        },
        // His first message to an empty Mod Job names it and sets the agent to work.
        sendSessionMessage: async (send) => {
          const sent = await own.sendSessionMessage!(send);
          if (sent.ok && empty.delete(send.session_id)) {
            fake.changed({ ...sent.value, title: `Mod ${titleOf(send.text)}` });
            writesDusk(send.session_id, send.aim);
          }
          return sent;
        },
      };
    },
  };
}

export const s323ModJob: Scenario = build();
