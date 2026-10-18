// A Session that waits on another and starts on its own. There is no form: the brief says it. An `@` in the Dispatch request opens the live Sessions,
// and a sentence over the request shows the proposer's reading of the wait, each part a button that cycles. The waiting Session is a row of its own in
// the Work list, and when the Session it waits on lands its pull request it starts with that Session's handoff. Over `session-after-another`.

import { button, inside, region, role, walk } from "../walk";

const cockpit = region("Jobs", { exact: true });
const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const chip = inside(frame, role("group", "Start condition"));
const base = inside(chip, button(/^Starts from /));
const waiting = region("Job: Let the Field expand into the overlay", { exact: true });

const sessionAfterAnother = walk("session-after-another", [
  { key: "Alt+a", on: cockpit, say: "⌥A opens Dispatch on a Job with an @ typed" },
  { type: "after @Coc\n", into: request, say: "Arrows move, ↵ picks. The Job stands in the words as a tag" },
  { type: "after @Cockpit Map lands its PR, let the Field expand into the overlay and show plan progress", into: request, say: "The rest of the brief says when and what" },
  { look: chip, say: "The proposer's reading. Tab moves between the parts, ↑ ↓ or ↵ change one" },
  { key: "ArrowDown", on: base, say: "↓ on the base: it starts from the pull request's branch. The part is marked as yours" },
  { key: "Enter", on: request, say: "A row of its own in Jobs. It opens on its brief and what it waits on" },
  { later: waiting, say: "Cockpit Map lands its PR and the Job starts, its first message the handoff" },
]);

export { sessionAfterAnother };
