// A Session that waits on another and starts on its own. Dispatch has an After row: the Session to wait on, then when this one starts, and
// ⌥A moves to it. The waiting Session is a row of its own in the Work list, the Session it waits on is told in its thread, its agent
// changes when the other starts, and a pull request pushed starts it with the handoff as its first message. Over `session-after-another`.

import { button, inside, region, role, walk } from "../walk";

const cockpit = region("Work", { exact: true });
const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const after = inside(frame, role("combobox", "After Session"));
const thread = role("region", "Thread", { exact: true });
const waiting = region("Session: Add the rotation page", { exact: true });
const told = (said: string) => inside(thread, role("button", new RegExp(`${said}$`)));

const sessionAfterAnother = walk("session-after-another", [
  { key: "Alt+a", on: cockpit, say: "⌥A opens Dispatch on a Session, with the focus on After" },
  { type: "after-audit", into: after, say: "After Rotate the audit log. It starts when that Session ends, from main with its handoff" },
  { type: "Add the rotation page to the docs\n", into: request, say: "It is a row of its own in Work, and opens on its brief, what it waits on and when it starts" },
  { press: inside(waiting, button("Rotate the audit log")), say: "The Session it waits on is told, with the brief" },
  { later: told("waits on you"), say: "Its agent sets the waiting Session to start when the pull request is pushed" },
  { later: told("condition changed"), say: "The pull request is pushed and the waiting Session starts, from its branch" },
  { press: told("started"), say: "Its row is a live Session, and its first message carries the handoff" },
]);

export { sessionAfterAnother };
