// Three arrangements of the review gate's The Job's record, for the owner to
// pick one from (#1680, 2 Oct 2026). Each is Job 2 just before it landed, at
// its handoff step, opened from the Board in turn; the record arrives open so
// it can be read without a press. The fold itself is unchanged.

import { button, role, walk } from "../walk";

const RECORD = button("The Job's record");
const BOARD = button("Overview", { exact: true });
/** A copy of Job 2 on the Board, by the handle that names its arrangement. */
const copy = (handle: string) => role("option", handle);

export const threeRecords = walk("review-gate/record-a-cards", [
  {
    look: RECORD,
    say: "A · Settings-board cards: a card per section, in balanced columns. Every section has a head to find; it is the tallest open, and the asked-for card runs as long as the issue",
  },
  { press: BOARD, say: "Back to the Board for the next one" },
  { press: copy("2-b-glass-rows"), say: "Job 2 again, its record as B" },
  {
    look: RECORD,
    say: "B · One glass card, rows: each value at full weight, its name trailing quietly. One continuous read, nothing boxed; the names are the easiest thing to miss",
  },
  { press: BOARD, say: "Back to the Board for the last one" },
  { press: copy("2-c-work-first"), say: "Job 2 again, its record as C" },
  {
    look: RECORD,
    say: "C · The work leads: the pull request, its figures, then what proves it beside what nothing checked. The ask and the Drone's account fold inside, one press more each",
  },
]);
