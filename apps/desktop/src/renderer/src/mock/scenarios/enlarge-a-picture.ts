// A Session whose person sent a picture with a message. Over the `sessions` scenario. The walk
// `enlarge-a-picture` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const PICTURE =
  "data:image/svg+xml;utf8," +
  encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" width="960" height="640" viewBox="0 0 960 640"><rect width="960" height="640" fill="#2b2f36"/><rect x="40" y="40" width="880" height="64" rx="8" fill="#3b414b"/><rect x="40" y="136" width="260" height="464" rx="8" fill="#3b414b"/><rect x="332" y="136" width="588" height="200" rx="8" fill="#4a5260"/><rect x="332" y="368" width="588" height="232" rx="8" fill="#3b414b"/></svg>`,
  );

const sessions: Session[] = [
  {
    id: "s15",
    title: "Header looks wrong",
    turn: { state: "idle" },
    lastTurnAt: minutesAgo(1),
    rows: [
      {
        id: "r1",
        at: "14:39:50",
        kind: "message",
        from: { kind: "you" },
        text: "The header sits too low in this screenshot",
        files: [{ id: "f1", name: "header-screenshot.png", src: PICTURE }],
      },
      { id: "r2", at: "14:40:02", kind: "message", from: { kind: "agent" }, text: "Found it: the header has a stray margin." },
    ],
    attachments: [],
  },
];

export const s256EnlargeAPicture: Scenario = {
  ...s200Sessions,
  name: "enlarge-a-picture",
  says: "A Session whose person sent a picture, pressed to open it larger",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, sessions) },
};
