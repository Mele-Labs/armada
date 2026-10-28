// A question nobody answered in time waits on him with no waiting item behind it, so the Job's row in the Jobs list
// pulses as one that needs him does. Over the `lapsed-question` scenario.

import { toSessions } from "../sessions/walk-kit";
import { deckRow, walk } from "../walk";

const lapsedQuestionRail = walk("lapsed-question", [
  toSessions,
  { look: deckRow(/Book the table/), say: "Its question timed out: the row pulses, Waiting on you" },
  { hover: deckRow(/Book the table/), say: "The mark names it" },
  { press: deckRow(/Book the table/), say: "Opening it opens the question" },
]);

export { lapsedQuestionRail };
