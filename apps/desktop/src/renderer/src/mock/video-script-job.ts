// A Job that writes a video script as its document and puts each revision to him, as a sketch is put to him. Two moments of it,
// each a scenario: `document-waits-in-needs-you` with revision 1 waiting, and `document-asks-again` with revision 2 waiting after
// revision 1 was approved. The page carries what Fleet's artifact does: `asked`, `revision` and, once answered, `settled`, which a
// new revision drops.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";

import { VIDEO_SCRIPT_1, VIDEO_SCRIPT_2, VIDEO_SCRIPT_PATH, VIDEO_SCRIPT_R1, VIDEO_SCRIPT_R2, VIDEO_SCRIPT_TITLE } from "./document-pages";
import { holding } from "./holding";
import type { Scenario } from "./moment";
import { sessionsStore } from "./sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const ID = "s58";
const ITEM = `document:${VIDEO_SCRIPT_PATH}`;

const FIRST: readonly SessionRow[] = [
  { id: "v1", at: "10:02:00", kind: "message", from: { kind: "you" }, text: "Write the script for a 60 second demo video of Armada for the landing page." },
  { id: "v1a", at: "10:02:06", kind: "tool", text: "Read docs/scope.md" },
  { id: "v1b", at: "10:02:12", kind: "tool", text: "Read docs/journeys/first-job.md" },
  { id: "v1c", at: "10:02:30", kind: "message", from: { kind: "agent" }, text: "Four shots: the Board, a plan card, a walk and the merge queue, with a line of voice over each." },
  { id: "v1d", at: "10:03:10", kind: "tool", text: "Write .armada/documents/video-script.html" },
];

const SECOND: readonly SessionRow[] = [
  { id: "v2", at: "10:09:00", kind: "message", from: { kind: "you" }, text: "Make the close shorter." },
  { id: "v2a", at: "10:09:40", kind: "tool", text: "Write .armada/documents/video-script.html" },
];

function job(stage: 1 | 2): Session {
  const page: SessionAttachment = {
    kind: "artifact",
    form: "document",
    id: VIDEO_SCRIPT_PATH,
    title: VIDEO_SCRIPT_TITLE,
    revisions: stage === 1 ? [VIDEO_SCRIPT_R1] : [VIDEO_SCRIPT_R1, VIDEO_SCRIPT_R2],
    asked: true,
    revision: String(stage),
    kept: ago(1),
  };
  return {
    id: ID,
    title: "Demo marketing video",
    turn: { state: "idle" },
    standing: "idle",
    lastTurnAt: ago(1),
    work: { landing: "document" },
    rows: stage === 1 ? FIRST : [...FIRST, ...SECOND],
    attachments: [page],
    waitingFor: [{ id: ITEM, text: VIDEO_SCRIPT_TITLE, since: ago(1), source: "document", act: { kind: "message", target: ID } }],
  };
}

export function videoScriptScenario(name: string, says: string, stage: 1 | 2): Scenario {
  const base = holding(name, says);
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: {
      sessions: () => {
        const store = sessionsStore([job(stage)]);
        return {
          ...store,
          // Each revision is a file Fleet keeps, which the reading layer draws.
          readArtifact: async (id: string, path: string) => {
            const html = path === VIDEO_SCRIPT_R1 ? VIDEO_SCRIPT_1 : path === VIDEO_SCRIPT_R2 ? VIDEO_SCRIPT_2 : undefined;
            return html === undefined ? (store.readArtifact?.(id, path) ?? { ok: false as const, why: "unreadable" as const }) : { ok: true as const, bytes: new TextEncoder().encode(html), type: "text/html" };
          },
          // Fleet settles the revision he was shown on the page's artifact and takes the item off, as `answerSketch` does for a sketch.
          answerDocument: ({ session_id: id, document, revision, how, words }) => {
            const at = new Date().toISOString();
            const settled = how === "sent_back" ? { how, words: words ?? "", at } : { how, at };
            store.edit(id, (one) => ({
              ...one,
              attachments: one.attachments.map((a) => (a.kind === "artifact" && `document:${a.id}` === document && a.revision === revision ? { ...a, settled } : a)),
              waitingFor: (one.waitingFor ?? []).filter((item) => item.id !== document),
            }));
          },
        };
      },
    },
  };
}
