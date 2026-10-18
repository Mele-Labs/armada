// The owner registered to a pull request, drawn as a chip: the kind, a short id and the machine. The
// repository that registers has two Sessions' pull requests on this machine, two on the
// other, and one pull request with no row; the repository that does not draws no chip anywhere. The
// switch that turns registering on is in the repository's Manifest, and its own walk. Over `pr-owners`.

import { button, inside, region, role, text, viaPalette, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const ARMADA = region("Merge line, armada");
const LEDGER = region("Attachments");
const PANEL = role("dialog", "Pull request #2209");
const SWITCH = role("switch", "Pull request owners");

const chip = (scope: ReturnType<typeof region>, who: RegExp) => inside(scope, role("group", who));
const machine = (scope: ReturnType<typeof region>, who: RegExp, name: string) => inside(chip(scope, who), role("img", name, { exact: true }));

const owners = walk("pr-owners", [
  ...viaPalette("Merge line", "The merge line"),
  { hover: machine(ARMADA, /^Job a3f9c21e/, "This machine"), say: "A Job on this machine" },
  { hover: machine(ARMADA, /^Job 9c74d82e/, "MacBook Air"), say: "A Job on the other machine" },
  { hover: inside(chip(ARMADA, /^Job 7be40d15/), text("7be40d15")), say: "Another Job on the other machine" },
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button("Store migration spike")), say: "A Job on this machine" },
  { press: inside(LEDGER, button("Open Pull request #2209")), say: "Its pull request" },
  { hover: machine(PANEL, /^Job a3f9c21e/, "This machine"), say: "The chip sits in the panel's head" },
]);

const optIn = walk("pr-owners", [
  { press: button(/^Open the/), say: "The Manifest" },
  { press: button("Choose a repository"), say: "Choose a repository" },
  { press: role("menuitem", "armada"), say: "armada" },
  { press: role("tab", "Edit"), say: "Edit" },
  { hover: SWITCH, say: "On for armada" },
  { press: button("armada", { exact: true }), say: "The picker" },
  { press: role("menuitem", "pocket"), say: "pocket" },
  { look: SWITCH, say: "Off for pocket" },
]);

export { owners as "pr-owners", optIn as "pr-owners-opt-in" };
