// A long Artifacts list made short: pictures start out, the newest five show, More reveals the rest and
// Less folds back, and a kind toggle brings a hidden kind in. Over the `session-ledger` scenario, whose
// "Walk the ledger" Session holds fifteen pictures, three windows, three files and a Doc.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { kit, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const { ledger, opened } = kit(false);
  const sessions = region("Sessions");
  const artifacts = inside(ledger, region("Artifacts"));
  return [
    { press: inside(sessions, button("Walk the ledger")), say: "A Session with a long Artifacts list" },
    ...opened([
      { look: inside(artifacts, button("Windows")), say: "A toggle per kind it holds, Pictures the only one off" },
      { look: inside(artifacts, button("Open Shown in a window Window 1")), say: "Windows, files and the Doc show" },
      { press: inside(artifacts, button("More")), say: "The rest of what the filter lets through" },
      { look: inside(artifacts, button("Less")), say: "Less folds back to the newest five" },
      { press: inside(artifacts, button("Pictures")), say: "Pictures, all fifteen behind More" },
      { look: inside(artifacts, button("Open Looked at 01-shot.png")), say: "The first of them" },
      { press: inside(artifacts, button("Windows")), say: "Windows out of the list" },
      { press: inside(artifacts, role("button", "Less")), say: "Fold back" },
    ]),
  ];
}

const filter = walk("session-ledger", [toSessions, ...steps()]);

export { filter as "session-ledger-filter" };
