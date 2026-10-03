// Glyph candidates for five meanings the icon registry has no glyph for, each
// drawn where it would sit. SCRATCH, but committed on `icons/candidates` with
// `git add -f` so the owner can play it; `walks.test.tsx` does not run it. The
// sheet is `../../glyph-candidates/`.

import { region, role, walk } from "../../walk";

const candidate = (meaning: string, name: string) => role("article", `${meaning}: ${name}`);

export const glyphCandidates = walk("glyph-candidates", [
  { look: region("Origin: an issue"), say: "1. A criterion read out of an issue: ticket, hash, square-dot" },
  { look: region("Origin: your request"), say: "2. A criterion from the request typed at dispatch: quote, text-cursor-input, text-quote" },
  { look: region("Origin: a person, at the gate"), say: "3. A criterion a person added or reworded at the gate: user-pen, signature, pen-line" },
  { look: region("The issue has moved since"), say: "4. The issue was edited after Fleet read it: diff, git-compare, asterisk" },
  { look: candidate("A field the proposer is still settling", "text-cursor"), say: "5. A field still settling, blinking: text-cursor" },
  { look: candidate("A field the proposer is still settling", "ellipsis"), say: "5. Dots brightening in turn: ellipsis" },
  { look: candidate("A field the proposer is still settling", "loader"), say: "5. Turning: loader" },
  { look: region("Together"), say: "The recommended origin and moved marks, on one list" },
]);
