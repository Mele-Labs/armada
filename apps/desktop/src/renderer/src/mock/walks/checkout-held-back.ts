// The served checkout is behind main and Fleet would not catch it up, so it is a call in the Cockpit: what holds it, and
// the one command that is safe to press, where there is one. Four holds in turn on one repository, then main's tip reached.
// The safe one runs from the card and the card goes. Over `checkout-held-back`.

import { button, inside, region, role, text, walk } from "../walk";

const incoming = region("Incoming calls");
const places = role("navigation", "Places");
const card = (title: string) => region(`Call: ${title}`);

const clashing = card("12 commits behind main");
const branch = card("40 commits behind main");
const diverged = card("5 commits behind main");
const safe = card("880 commits behind main");

export const checkoutHeldBack = walk("checkout-held-back", [
  { press: inside(incoming, button("Checkout: 12 commits behind main")), say: "The checkout is a call in Incoming calls, 12 commits behind main" },
  { look: inside(clashing, text("Uncommitted changes main also changed")), say: "Two files are uncommitted, and main changed one of them" },
  { hover: inside(clashing, role("img", "Also changed on main")), say: "A warning marks the file main also changed" },
  { look: clashing, say: "There is no command, because the pull could stop on a conflict" },
  { later: places, say: "Another branch is checked out, with nothing uncommitted" },
  { look: inside(branch, text("fix/slot-lease")), say: "It is on fix/slot-lease, with 3 commits that are not on main" },
  { look: inside(branch, text("switch main")), say: "One command is safe: switch back to main" },
  { look: inside(branch, button("Run")), say: "Run plays it in the repository" },
  { look: inside(branch, button("Copy")), say: "Copy puts it on the clipboard" },
  { later: places, say: "On main, with 2 commits main lacks" },
  { look: inside(diverged, text("2 commits not on main")), say: "The history diverged, so there is no command" },
  { later: places, say: "On main, with one uncommitted file that main never changed" },
  { look: inside(safe, text("pull --ff-only --autostash")), say: "A pull that keeps the file is safe" },
  { press: inside(safe, button("Run")), say: "Run plays it, exits 0, and the card goes" },
  { later: places, say: "The checkout has main's tip" },
]);
