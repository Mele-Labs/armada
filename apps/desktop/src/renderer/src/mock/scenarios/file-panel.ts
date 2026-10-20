// A file open in the side panel's code editor: settings.json from Settings, a file a Session's agent showed with
// `show_file`, and whatever ⌘O's dialog answers. The files are a pretend disk (`files-disk.ts`), changed from outside
// by a walk's `later` step. Over the `sessions` scenario with one more Session. The walk `file-panel` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { NOTES_PATH } from "../files-disk";
import { s200Sessions } from "./sessions";

const noter: Session = {
  id: "s20",
  title: "Store clock notes",
  turn: { state: "idle" },
  rows: [
    { id: "n1", at: "14:44:10", kind: "message", from: { kind: "agent" }, text: "The clock reads are all in one place now. I wrote it up." },
    { id: "n2", at: "14:44:20", kind: "tool", text: `mcp__armada-fleet__show_file ${JSON.stringify({ path: NOTES_PATH })}` },
  ],
  attachments: [],
};

export const s250FilePanel: Scenario = {
  ...s200Sessions,
  name: "file-panel",
  says: "A file open in the side panel's code editor, from Settings, from a Session and from ⌘O",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [noter]) },
};
