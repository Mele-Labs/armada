// A Session's walk waiting on approval, on the call in front: Open walk beside Open Session, on v.
// Over `dashboard-needs-you`, where the Session walk is the seventh call.

import { button, dialog, inside, region, role, walk } from "../walk";

const cockpit = region("Dashboard");
const call = (kind: RegExp) => region(kind);
const later = { key: "l", on: cockpit, say: "Later" } as const;

const cockpitOpenWalk = walk("dashboard-needs-you", [
  { look: call(/^Plan question/), say: "Several calls wait" },
  { ...later },
  { ...later },
  { ...later },
  { ...later },
  { ...later },
  { look: call(/^Session question/), say: "A Session's question: Open Session only" },
  { look: inside(call(/^Session question/), button(/Open Session/)), say: "o opens the Session" },
  { ...later },
  { look: call(/^Session walk/), say: "A walk waiting on approval" },
  { look: inside(call(/^Session walk/), role("radio", /Approve/)), say: "Approve answers it" },
  { look: inside(call(/^Session walk/), button("Open walk v")), say: "Open walk, on v, beside Open Session on o" },
  { key: "v", on: cockpit, say: "v opens the walk" },
  { look: dialog(/^Bridge's window on Docs for the pairing screen/), say: "The walk's page, in Bridge's window" },
  { press: button("Close window"), say: "Closed" },
  { look: call(/^Session walk/), say: "The call stays: Approve still clears it" },
]);

export { cockpitOpenWalk as "cockpit-open-walk" };
