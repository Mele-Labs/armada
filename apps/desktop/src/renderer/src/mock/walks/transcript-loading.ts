// A Job whose thread has not been read opens on a transcript's shape and no Armada configuration card; one read before opens on that
// copy, with a mark at its foot until the newer rows land. Over the `transcript-loading` scenario.

import { deckRow, inside, region, role, text, walk } from "../walk";

const thread = region("Thread");

const transcriptLoading = walk("transcript-loading", [
  { press: deckRow("Fix the flaky store clock test"), say: "This Job's thread has not been read yet" },
  { look: thread, say: "Bars in the shape of a thread until Fleet answers. No Armada configuration card" },
  { later: inside(thread, text("passes 200 runs in a row")), say: "The thread replaces the bars" },
  { press: deckRow("Fix the retry backoff reset"), say: "This Job's thread was read before, and that copy shows at once" },
  { hover: inside(thread, role("img", "Loading new messages")), say: "A turning mark at the foot while newer messages are read" },
  { later: inside(thread, text("fails on main and passes")), say: "The two newer messages land under the others and the mark goes" },
]);

export { transcriptLoading as "transcript-loading" };
