// Notes on what is being read, sent to a Job without closing it: a Job whose agent drew a sketch and showed a picture,
// and the files ⌘O opens outside any Job (`files-disk.ts`). A send lands in the Job's thread as a message from the
// person, a picture's crop along with it. Over the `sessions` scenario with one more Session. The walk `job-notes` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import type { BayControl, SessionsStore } from "../sessions/script";
import { READS, READS_TITLE } from "../sessions/sketches";
import { s200Sessions } from "./sessions";

export const NOTES_JOB = "Pin the store clock";
export const FAILURE = "store-clock-failure.png";
const FAILURE_PATH = `/tmp/armada/${FAILURE}`;

// What the CI page drew when store_flaky failed: a run's rows, one red. Sized, so it opens large.
const PICTURE = `<svg xmlns="http://www.w3.org/2000/svg" width="960" height="600" viewBox="0 0 960 600"><rect width="960" height="600" fill="#161b22"/><rect x="0" y="0" width="960" height="56" fill="#21262d"/><rect x="32" y="20" width="220" height="16" rx="4" fill="#3d444d"/><rect x="32" y="96" width="896" height="64" rx="6" fill="#1f2a1f"/><rect x="56" y="122" width="320" height="12" rx="3" fill="#3fb950"/><rect x="32" y="176" width="896" height="64" rx="6" fill="#1f2a1f"/><rect x="56" y="202" width="280" height="12" rx="3" fill="#3fb950"/><rect x="32" y="256" width="896" height="64" rx="6" fill="#3a1d1f"/><rect x="56" y="282" width="360" height="12" rx="3" fill="#f85149"/><rect x="32" y="336" width="896" height="200" rx="6" fill="#0d1117"/><rect x="56" y="364" width="520" height="10" rx="3" fill="#8b949e"/><rect x="56" y="390" width="600" height="10" rx="3" fill="#f85149"/><rect x="56" y="416" width="440" height="10" rx="3" fill="#8b949e"/><rect x="56" y="442" width="380" height="10" rx="3" fill="#8b949e"/></svg>`;

const at = (seconds: number) => `14:4${String(Math.floor(seconds / 60))}:${String(seconds % 60).padStart(2, "0")}`;

const noter: Session = {
  id: "s27",
  title: NOTES_JOB,
  // The checkout ⌘O's files are in, so a note on one is shown from here in the thread.
  cwd: "/Users/user/Development/armada",
  turn: { state: "idle" },
  rows: [
    { id: "j1", at: at(10), kind: "message", from: { kind: "agent" }, text: "The clock is read in one place now. This is how a write reaches it." },
    { id: "j2", at: at(20), kind: "tool", text: `mcp__armada-fleet__show_sketch ${JSON.stringify({ title: READS_TITLE, scene: READS })}` },
    { id: "j3", at: at(30), kind: "message", from: { kind: "agent" }, text: `The last red run is in \`${FAILURE}\`.` },
  ],
  attachments: [
    { kind: "sketch", id: `sketch:${READS_TITLE}`, title: READS_TITLE, by: "agent", drawing: READS },
    { kind: "artifact", form: "image", id: FAILURE_PATH, title: FAILURE, shown: "2026-10-23T14:40:30.000Z", by: "agent" },
  ],
};

function sessions(board: BayControl): SessionsStore {
  const store = s200Sessions.draft!.sessions!(board, [noter]);
  const read = store.readArtifact;
  return {
    ...store,
    readArtifact: async (id, path) => (path === FAILURE_PATH ? { ok: true, bytes: new TextEncoder().encode(PICTURE), type: "image/svg+xml" } : (read?.(id, path) ?? { ok: false, why: "unreadable" })),
  };
}

export const s321JobNotes: Scenario = {
  ...s200Sessions,
  name: "job-notes",
  says: "Notes on a sketch, a file and a picture, held on each and sent to a Job as one message",
  draft: { sessions },
};
