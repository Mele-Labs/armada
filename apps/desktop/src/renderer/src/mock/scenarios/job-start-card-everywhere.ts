// Every way a Job starts opens held, with the first message the agent will read in the composer to edit before Start:
// a Job dispatched from the Dispatch sheet, a Job another Job's agent started, and a Job started from the annotation layer, where the box
// holds the note as the agent will read it. Over a Fleet that holds the start as Fleet would (`../sessions-fleet.ts`). The walk
// `job-start-card-everywhere` plays it. Mock only: the layer's note turn is composed here in the shape of `crates/fleet/src/session_host/noting.rs`.

import type { RepositorySummary, SessionNote, SessionRow, WorkflowSummary } from "@armada/protocol";
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

/** The repository's workflows, which the Workflow select lists. */
const WORKFLOWS = [
  { id: "feature", name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"] },
  { id: "bug", name: "Bug", steps: ["Reproduce", "Fix", "Verify", "Pull request"] },
  { id: "review", name: "Review", steps: ["Read", "Review", "Report"] },
].map((one): WorkflowSummary => ({ id: one.id, name: one.name, version: 1, manifest_id: "armada", steps: one.steps.map((label) => ({ step_id: label.toLowerCase(), label, checks: [], judge_checks: [] })) }) as unknown as WorkflowSummary);

/** How long the proposer reads a dispatched request before it lands on a workflow. */
const READ_MS = 1200;

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

/** What the agent reads of a pinned note: his words, then the lines about the spot, then the handoff, as `noting.rs` composes them. */
function turnOf(note: SessionNote): string {
  const { element, stamp, component, owners } = note.place;
  const named = (element.label ?? "").trim() || element.text.trim();
  const lines = ["Where he pointed, to find the code; not the request:", `Pointed at: ${element.tag}${named === "" ? "" : ` "${named}"`}${note.note_id === undefined ? "" : ` (note ${note.note_id})`}`];
  if (stamp !== undefined) lines.push("Candidates:", `1. ${stamp} — the page's stamp, checked`, `   <${element.tag} aria-label="${named === "" ? component : named}">`);
  lines.push(`Page: ${note.address}`);
  const chain = [...(component === undefined ? [] : [component]), ...(owners ?? [])].slice(0, 3);
  if (chain.length > 0) lines.push(`Inside: component chain ${chain.join(" ← ")}`);
  return [note.said.trim(), "", ...lines].join("\n");
}

function build(): Scenario {
  const board = onBoard({ repositories: SERVED, workflows: WORKFLOWS });
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return {
    ...served,
    name: "job-start-card-everywhere",
    says: "A Job dispatched, a Job another Job started and a Job started from the annotation layer each open held, with the first message in the composer to edit",
    // A Fleet of its own for each window, so a walk told twice starts from the same Job.
    behaves: (handle) => {
      const fake = new FakeSessionsFleet(records(), threads());
      const own = fake.scenario(board).behaves!(handle);
      /** The Jobs held for the layer's notes, and the turns of the notes sent to each. */
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
        // The layer starts a Job held, with a title and no words, as a Dispatch start is held:
        // the note it is sent next is the first message, composed the way the agent reads it.
        startSession: async (title, root, request, tuning, after, holds) => {
          // A request, from a dispatch or from a note, is read for a moment: the proposer picks the workflow and his usual landing comes with it.
          if (request !== undefined && after === undefined && holds === true) {
            const started = await own.startSession!(title, root, request, tuning, after, holds);
            if (!started.ok) return started;
            const id = started.value.id;
            // A Job the layer started has a title: the notes sent next are its first message.
            if (title !== undefined) noted.set(id, []);
            fake.proposes(id, { reading: true });
            window.setTimeout(() => fake.proposes(id, { workflow: "feature", picked: ["workflow"], landing: { choice: "auto_land", from: "usual" } }), READ_MS);
            return started;
          }
          if (title === undefined || request !== undefined || after !== undefined) return own.startSession!(title, root, request, tuning, after, holds);
          const started = await own.startSession!(undefined, root, undefined, undefined, undefined, true);
          if (!started.ok) return started;
          const record = { ...started.value, title, hosted: { ...started.value.hosted!, start: { asks_delivery: true } } };
          fake.changed(record);
          noted.set(record.id, []);
          return { ok: true, value: record };
        },
        stageAttachment: async (_bytes, filename) => ({ path: `/tmp/${filename}` }),
        sendSessionMessage: async (send) => {
          await own.watchSession!(send.session_id);
          return own.sendSessionMessage!(send);
        },
        // Pinned notes, one turn: on a held Job each note's part joins the first message; on a live one the thread keeps his words alone.
        sendSessionNotes: async (send) => {
          await own.watchSession!(send.session_id);
          const notes = send.notes.map(({ note }) => note);
          const turns = noted.get(send.session_id);
          if (turns !== undefined) turns.push(...notes.map(turnOf));
          const said = notes.length === 1 ? notes[0]!.said : notes.map((one, at) => `${at + 1}. ${one.said}`).join("\n");
          const sent = await own.sendSessionMessage!({ session_id: send.session_id, text: turns === undefined ? said : turns.join("\n\n") });
          return sent.ok ? { ok: true, value: { session: sent.value, candidates: notes.map(() => []) } } : sent;
        },
      };
    },
  };
}

export const s330JobStartCardEverywhere: Scenario = build();
