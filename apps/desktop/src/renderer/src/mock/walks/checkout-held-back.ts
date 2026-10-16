// The served checkout is behind main and Fleet would not catch it up, so main's light on the merge queue carries an amber mark
// and its card a section, "Your checkout": what holds it back, and the one command that is safe to press, where there is one.
// Four holds in turn on one repository. The safe one runs from the card and the checkout is level. The last is a long list of
// uncommitted files, in a box that scrolls inside the card. Over `checkout-held-back`.

import { button, dialog, inside, region, role, text, walk, type Step } from "../walk";

const queue = region("Merge queue");
const places = role("navigation", "Places");
const held = (title: string) => `Main is green, checkout ${title}`;
const light = (title: string) => inside(queue, button(held(title)));
const card = (title: string) => dialog(held(title));
const section = (title: string) => inside(card(title), region("Your checkout"));

const safe = "880 commits behind main";
const clashing = "12 commits behind main";
const diverged = "5 commits behind main";
const loose = "7 commits behind main";
const firstClashing = "apps/desktop/src/renderer/src/Dashboard.tsx";

const steps: readonly Step[] = [
  { look: inside(queue, role("img", `Checkout ${safe}`)), say: "An amber mark on main: the checkout is 880 commits behind main" },
  { press: light(safe), say: "A press pins main's card" },
  { look: section(safe), say: "Under main's state, your checkout" },
  { look: inside(section(safe), text("armada.yml")), say: "One file is uncommitted, and main never changed it" },
  { look: inside(section(safe), text("pull --ff-only --autostash")), say: "A pull that keeps the file is safe" },
  { look: inside(section(safe), button("Copy")), say: "Copy puts it on the clipboard" },
  { press: inside(section(safe), button("Run")), say: "Run plays it and exits 0" },
  { later: places, say: "Fleet has the checkout level" },
  { look: inside(queue, role("img", "Main is green")), say: "The mark is gone, and a green main has no card" },
  { later: places, say: "Main moves on and the checkout is held again" },
  { press: light(clashing), say: "A press pins the card: two files are uncommitted" },
  { hover: inside(section(clashing), role("img", "Also changed on main")), say: "A warning marks the file main also changed" },
  { look: section(clashing), say: "There is no command, because the pull could stop on a conflict" },
  { later: places, say: "On main, with 2 commits main lacks" },
  { look: inside(section(diverged), text("2 commits not on main")), say: "The history diverged, so there is no command" },
];

/** The four holds in turn, the last a long list held by scrolling. */
export const checkoutHeldBack = walk("checkout-held-back", [
  ...steps,
  { later: places, say: "An agent left a long list of files uncommitted" },
  { look: inside(section(loose), role("group", "Uncommitted files")), say: "The list stays inside the card and scrolls there" },
  { look: inside(section(loose), text(firstClashing)), say: "The files main also changed come first" },
  { look: inside(section(loose), text("7 commits behind main")), say: "There is no command, because the pull could stop on a conflict" },
]);
