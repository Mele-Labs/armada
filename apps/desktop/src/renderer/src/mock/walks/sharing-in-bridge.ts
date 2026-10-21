// Share, import and commit, in acme/widgets.
// A tile of yours that hides the repository's carries a mark naming it. Its sheet in Edit commits it by a Job of its own, saves it as a package and copies the same text.
// A package a teammate sent is pasted on the window and added with the one ask, and its piece for a repository this Fleet does not serve is given one.
// The teammate's committed Lint asks once before it runs, showing its command, and the repository is kept on this Mac.
// `docs/design/who-sees-each-piece-of-configuration.md`.

import { PACKAGE } from "../sharing-package";
import { button, dialog, inside, region, role, text, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const tile = (name: string) => inside(controls, button(name));
const editing = inside(controls, role("listbox", "Flight controls, editing"));
const sheet = dialog("Change Run tests");
const importing = dialog("Import");

const sharingInBridge = walk("sharing-in-bridge", [
  { press: inside(list, button("Speed up the widget tests")), say: "A Job in acme/widgets" },
  { hover: inside(controls, role("img", "Hides the repository's Run tests")), say: "Your Run tests runs make test-fast, and hides the one acme/widgets committed" },
  { press: tile("Edit"), say: "Edit" },
  { press: inside(editing, role("option", "Run tests")), say: "Your tile kept for acme/widgets" },
  { press: inside(sheet, button("Commit to repository")), say: "Commit to repository starts a Job of its own that commits this tile alone" },
  { look: inside(sheet, button("Open the pull request")), say: "Its pull request is open, and Open the Job goes to the Job" },
  { press: inside(sheet, button("Share")), say: "Share saves it as a package to send" },
  { look: inside(sheet, text("Saved run-tests.armada.toml")), say: "One file, sent any way you like" },
  { press: inside(sheet, button("Copy")), say: "Copy puts the same text on the clipboard" },
  { key: "Escape", on: sheet, say: "Close the sheet" },
  { press: tile("Cancel"), say: "Leave Edit" },
  { paste: controls, text: PACKAGE, say: "A teammate's package, pasted on the window" },
  { look: inside(importing, text("make preview")), say: "Every piece, where it lands, and every command it would run" },
  { press: inside(importing, button("Add")), say: "Add is the one ask" },
  { look: inside(importing, text("Added")), say: "Deploy preview is yours now, for acme/widgets" },
  { press: inside(importing, button("Choose")), say: "Migration safety names a repository this Fleet does not serve: choose one for it" },
  { press: inside(importing, button("Close")), say: "Close" },
  { press: tile("Lint"), say: "Lint, committed by a teammate, has not run on this Mac" },
  { look: inside(dialog("Lint"), text("make lint")), say: "It asks once, showing its command" },
  { press: inside(dialog("Lint"), button("Run")), say: "Run approves this command and runs it" },
  { look: region("Shell output"), say: "It ran in the Job's bay" },
  { press: button("Armada configuration kept on this Mac"), say: "Where acme/widgets keeps its configuration" },
  { press: role("checkbox", "Keep on this Mac"), say: "Keep on this Mac" },
  { look: button("On this Mac"), say: "Every piece of yours for it, and its run memory, stays on this Mac, the only place offered" },
]);

export { sharingInBridge as "sharing-in-bridge" };
