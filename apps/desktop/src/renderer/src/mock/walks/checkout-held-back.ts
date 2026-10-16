// The served checkout is behind main and Fleet would not catch it up, so the merge queue carries a light for it beside main's:
// what holds it back, and the one command that is safe to press, where there is one. Four holds in turn on one repository. The
// safe one runs from the card and the checkout is level. Over `checkout-held-back`.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const queue = region("Merge queue");
const places = role("navigation", "Places");
const light = (title: string) => inside(queue, button(`Checkout ${title}`));
const card = (title: string) => dialog(`Checkout ${title}`);

const safe = "880 commits behind main";
const clashing = "12 commits behind main";
const branch = "40 commits behind main";
const diverged = "5 commits behind main";

export const checkoutHeldBack = walk("checkout-held-back", [
  { look: light(safe), say: "Beside main on the merge queue, a light for the checkout" },
  { hover: light(safe), say: "Hovering it opens the card: the checkout is 880 commits behind main" },
  { look: inside(card(safe), text("armada.yml")), say: "One file is uncommitted, and main never changed it" },
  { look: inside(card(safe), text("pull --ff-only --autostash")), say: "A pull that keeps the file is safe" },
  { look: inside(card(safe), button("Copy")), say: "Copy puts it on the clipboard" },
  { press: inside(card(safe), button("Run")), say: "Run plays it, exits 0, and the light goes" },
  { later: places, say: "Fleet has the checkout level" },
  { later: places, say: "Main moves on and the checkout is held again" },
  { press: light(clashing), say: "A press pins the card: two files are uncommitted" },
  { hover: inside(card(clashing), role("img", "Also changed on main")), say: "A warning marks the file main also changed" },
  { look: card(clashing), say: "There is no command, because the pull could stop on a conflict" },
  { later: places, say: "Another branch is checked out, with nothing uncommitted" },
  { look: inside(card(branch), text("fix/slot-lease")), say: "It is on fix/slot-lease, with 3 commits that are not on main" },
  { look: inside(card(branch), text("switch main")), say: "One command is safe: switch back to main" },
  { look: inside(card(branch), button("Run")), say: "Run plays it in the repository" },
  { later: places, say: "On main, with 2 commits main lacks" },
  { look: inside(card(diverged), text("2 commits not on main")), say: "The history diverged, so there is no command" },
]);
