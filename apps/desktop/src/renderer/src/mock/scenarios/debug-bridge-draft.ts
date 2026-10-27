// Debug Bridge, ⇧⌘D, over a Fleet that serves Sessions: it opens a new Session holding the note and the
// picture as a draft, with no turn running, and a Session mid-turn beside it that Stop ends.
// The walk `debug-bridge-draft` plays it. The layer is the mock's own, off (`../annotating.tsx`).

import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const AT = "2026-10-12T14:00:00.000Z";
const READ_MS = 700;
const BACKFILL = "01SESSIONAAAAAAAAAAAAAAAAA";

const sessions = () => [
  hosted(BACKFILL, {
    title: "Backfill dry run",
    attachments: [held("slot", "3"), held("branch", "fix/backfill")],
    hosted: { turn: { state: "working" }, mode: "auto", running: true },
  }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [BACKFILL]: [
    { kind: "message", id: "w1", at: AT, from: { kind: "you" }, text: "Dry-run the backfill against the copy." },
    { kind: "tool", id: "w2", at: AT, text: "Read crates/store/src/backfill.rs" },
    { kind: "tool", id: "w3", at: AT, text: "Bash cargo run -p store --example backfill -- --dry-run" },
  ],
});

/** What the window looked like, as far as a thumbnail goes: the rail and the panel, drawn once. */
async function windowShot(): Promise<Uint8Array> {
  const canvas = new OffscreenCanvas(288, 180);
  const paint = canvas.getContext("2d")!;
  paint.fillStyle = "#0b1220";
  paint.fillRect(0, 0, 288, 180);
  paint.fillStyle = "#16213a";
  paint.fillRect(8, 20, 56, 150);
  paint.fillRect(72, 20, 140, 150);
  paint.fillRect(220, 20, 60, 150);
  paint.fillStyle = "#2b3d63";
  paint.fillRect(8, 4, 272, 8);
  return new Uint8Array(await (await canvas.convertToBlob({ type: "image/png" })).arrayBuffer());
}

function build(): Scenario {
  const board = onBoard({
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const served = new FakeSessionsFleet(sessions(), threads()).scenario(board);
  return {
    ...served,
    name: "debug-bridge-draft",
    says: "Debug Bridge opens a Session with its note as a draft and no turn running; Stop ends a running turn",
    // A Fleet of its own for each window, so a walk told twice starts from the same Session.
    behaves: (handle) => {
      const fake = new FakeSessionsFleet(sessions(), threads());
      const own = fake.scenario(board).behaves!(handle);
      return {
        ...own,
        // A Session Fleet starts takes its title from the call, so the fake names it here.
        // Held on its start card with the note's words as the request: the proposer reads them for a moment and picks the Workflow.
        startSession: async (title, root, request, tuning, after, holds) => {
          const started = await own.startSession!(title, root, request, tuning, after, holds);
          if (!started.ok) return started;
          if (title !== undefined) await own.renameSession!({ session_id: started.value.id, title });
          if (request !== undefined && holds === true) {
            const id = started.value.id;
            fake.proposes(id, { reading: true });
            window.setTimeout(() => fake.proposes(id, { workflow: "feature", picked: ["workflow"], landing: { choice: "auto_land", from: "usual" } }), READ_MS);
          }
          return started;
        },
        readSessionFile: async () => ({ ok: true, bytes: await windowShot(), type: "image/png" }),
        sendSessionMessage: async (send) => {
          await own.watchSession!(send.session_id);
          return own.sendSessionMessage!(send);
        },
      };
    },
  };
}

export const s210DebugBridgeDraft: Scenario = build();
