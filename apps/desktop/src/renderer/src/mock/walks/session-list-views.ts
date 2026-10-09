// The Sessions list in views. It opens on Active, which holds what is asking, working, idle or not started.
// Quiet and Ended are views of their own, and All holds every heading. Over the `session-list-views` scenario.

import { inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";
import type { Step } from "../walk";

function steps(): Step[] {
  const sessions = region("Sessions");
  const tab = (name: string) => inside(sessions, role("tab", name));
  return [
    { press: tab("Active"), say: "Active, where the list opens" },
    { look: inside(sessions, role("heading", "Needs you")), say: "What is asking is first" },
    { look: inside(sessions, role("heading", "Running")), say: "Then what is working" },
    { look: inside(sessions, role("heading", "Idle")), say: "Then idle" },
    { look: inside(sessions, role("heading", "Not started")), say: "Then not started" },
    { press: tab("Quiet"), say: "Quiet" },
    { look: inside(sessions, text("Why the reader drops lines")), say: "Only the terminal Session whose mod is not asking" },
    { press: tab("Ended"), say: "Ended" },
    { look: inside(sessions, text("Trim the retry loop")), say: "Only the one that ended" },
    { press: tab("All"), say: "All" },
    { look: inside(sessions, role("heading", "Ended")), say: "Every heading, as before" },
    { press: tab("Active"), say: "Back to Active" },
  ];
}

const listViews = walk("session-list-views", [toSessions, ...steps()]);

export { listViews as "session-list-views" };
