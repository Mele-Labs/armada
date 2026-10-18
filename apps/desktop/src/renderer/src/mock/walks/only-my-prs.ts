// Armada follows the pull requests its person opened and nobody else's. A teammate has #1901 in the merge queue and #1902 outside
// it, both with failing checks. #1901 is a dot with its place and no press, #1902 is not drawn, and only the person's own failing
// #1896 raises a call. Over `only-my-prs`.

import { button, inside, region, role, walk } from "../walk";

const queue = region("Merge queue");
const list = region("Work", { exact: true });

export const onlyMyPrs = walk("only-my-prs", [
  { look: inside(queue, button(/^#1890/)), say: "The person's #1890 is first in the merge queue" },
  { look: inside(queue, role("img", /^#1901/)), say: "A teammate's #1901 is behind it at place 2: a quiet dot, no press, though its checks fail" },
  { hover: inside(queue, role("img", /^#1901/)), say: "Its card names the author, mia, with its place and state, and offers nothing to press" },
  { look: inside(queue, button(/^#1896/)), say: "The person's own #1896 is open and pressable. A teammate's #1902, outside the queue, is not on the row" },
  { key: "w", on: list, say: "w brings the next call forward. #1896 raises one; neither of the teammate's pull requests does" },
  { look: region("Pull request #1896"), say: "The call opens #1896 on the stage" },
]);
