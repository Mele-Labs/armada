// Every way a Job starts opens on the same start card, with the first message the agent will read in the Message box to edit before Start:
// a Job dispatched from the Dispatch sheet, a Job another Job's agent started, and a Job started from the annotation layer, where the box
// holds the note as the agent will read it. Over a Fleet that holds the start as Fleet would (`../sessions-fleet.ts`). The walk
// `job-start-card-everywhere` plays it. Mock only: the layer's note turn is composed here in the shape of `crates/fleet/src/session_host/noting.rs`.

import type { RepositorySummary, SendSessionNote, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

/** A repository served beside armada, set up, so the Dispatch sheet draws its row of chips as it does for four. */
function elsewhere(name: string): RepositorySummary {
  return {
    root: `/Users/user/code/${name}`,
    records_root: `/records/${name}`,
    manifest: { ...repository().manifest!, id: name, repository: name, path: `${name}/armada.yml` },
  };
}

const SERVED = [ARMADA, elsewhere("storefront"), elsewhere("billing"), elsewhere("docs-site")];

const AT = "2026-10-12T09:14:02.000Z";
const DEBUG = "01DEBUGCAPAAAAAAAAAAAAAAAA";

const records = () => [hosted(DEBUG, { title: "Debug the retry cap", last_turn_at: AT, attachments: [held("slot", "3"), held("branch", "fix/retry-cap")] })];

const threads = (): Record<string, SessionRow[]> => ({
  [DEBUG]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "The retry loop ran 5 attempts with the cap set to 3. Find out why." },
    { kind: "message", id: "m2", at: AT, from: { kind: "agent" }, text: "The loop reads the cap once at startup. Separately, the settings loader seems to ignore a cap set for one repository." },
  ],
});

const START = "Read crates/settings/src/loader.rs and find out whether a cap set for one repository is ever read. Report where the value is dropped.";

const HANDOFF = "Handoff: deliver the way the owner chose when Fleet asked how. A change the owner would see waits for his walk before it lands.";

/** What the agent reads of a pinned note: his words, then the lines about the spot, then the handoff, as `noting.rs` composes them. */
function turnOf(note: SendSessionNote): string {
  const { element, stamp, component, owners } = note.place;
  const named = (element.label ?? "").trim() || element.text.trim();
  const lines = [`Pointed at: ${element.tag}${named === "" ? "" : ` "${named}"`}${note.note_id === undefined ? "" : ` (note ${note.note_id})`}`];
  if (stamp === undefined) lines.push("No line on screen matched the code in /Users/user/code/armada");
  else lines.push("Candidates:", `1. ${stamp} — the page's stamp, checked`, `   <${element.tag} aria-label="${named === "" ? component : named}">`);
  lines.push(`Page: ${note.address}`);
  const chain = [...(component === undefined ? [] : [component]), ...(owners ?? [])];
  if (chain.length > 0) lines.push(`Inside: component chain ${chain.join(" ← ")}`);
  return [note.said.trim(), ...lines, HANDOFF].join("\n");
}

function build(): Scenario {
  const board = onBoard({ repositories: SERVED });
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return {
    ...served,
    name: "job-start-card-everywhere",
    says: "A Job dispatched, a Job another Job started and a Job started from the annotation layer each open on the start card, with the first message to edit",
    // A Fleet of its own for each window, so a walk told twice starts from the same Job.
    behaves: (handle) => {
      const fake = new FakeSessionsFleet(records(), threads());
      const own = fake.scenario(board).behaves!(handle);
      /** The Jobs held on a card for the layer's notes, and the turns of the notes sent to each. */
      const noted = new Map<string, string[]>();
      /** What each Job's agent does once Start is pressed, or its message is sent. */
      fake.onSend = (send) => {
        const said = send.text.toLowerCase();
        if (send.session_id === DEBUG) {
          if (!said.includes("start a session")) return;
          void fake.agentTurn(DEBUG, [
            { kind: "tool", id: "a1", at: AT, text: "Start session Settings loader and the cap" },
            () => {
              const title = "Settings loader and the cap";
              const id = fake.agentStarts(DEBUG, { title, message: START });
              return { kind: "started", id: "a2", at: AT, session: { id, title }, fork: false };
            },
          ]);
          return;
        }
        void fake.agentTurn(send.session_id, [
          { kind: "tool", id: `p1-${send.session_id}`, at: AT, text: "Read crates/store/src/client.rs" },
          { kind: "message", id: `p2-${send.session_id}`, at: AT, from: { kind: "agent" }, text: "Read the first message. Starting with the client." },
        ]);
      };
      return {
        ...own,
        // The layer starts a Job with a title and no words, and Fleet would hold it on its card as a Dispatch start is held:
        // the note it is sent next is the first message, composed the way the agent reads it.
        startSession: async (title, root, request, tuning, after, holds) => {
          if (title === undefined || request !== undefined || after !== undefined || holds === true) return own.startSession!(title, root, request, tuning, after, holds);
          const started = await own.startSession!(undefined, root, undefined, undefined, undefined, true);
          if (!started.ok) return started;
          const record = { ...started.value, title, hosted: { ...started.value.hosted!, start: { asks_delivery: true, picked: { choice: "open_pr" as const, from: "usual" as const } } } };
          fake.changed(record);
          noted.set(record.id, []);
          return { ok: true, value: record };
        },
        stageAttachment: async (_bytes, filename) => ({ path: `/tmp/${filename}` }),
        sendSessionMessage: async (send) => {
          await own.watchSession!(send.session_id);
          return own.sendSessionMessage!(send);
        },
        // A pinned note: on a held Job its turn joins the first message; on a live one the thread keeps his words alone.
        sendSessionNote: async (send) => {
          await own.watchSession!(send.session_id);
          const turns = noted.get(send.session_id);
          if (turns !== undefined) turns.push(turnOf(send));
          const sent = await own.sendSessionMessage!({ session_id: send.session_id, text: turns === undefined ? send.said : turns.join("\n\n") });
          return sent.ok ? { ok: true, value: { session: sent.value, candidates: [] } } : sent;
        },
      };
    },
  };
}

export const s330JobStartCardEverywhere: Scenario = build();
