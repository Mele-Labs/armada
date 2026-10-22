// Import, with the Requests a teammate sent.
// A package pasted on the window holds one piece, the Requests kept for acme/widgets: its Store API collection and its test environment.
// Its card names the kind, the repository and what it holds, and Add lands it.

import { button, dialog, inside, region, role, walk } from "../walk";

const PACKAGE = ["version = 1", "", "[[piece]]", 'kind = "requests"', 'remote = "acme/widgets"', ""].join("\n");

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const importing = dialog("Import");

const requestsInAPackage = walk("requests-in-a-package", [
  { press: inside(list, button("Check the Store API")), say: "A Job in acme/widgets" },
  { paste: controls, text: PACKAGE, say: "A teammate's package, pasted on the window" },
  { look: inside(importing, role("listitem", "Requests Store API, test")), say: "Requests for acme/widgets, holding the Store API collection and the test environment" },
  { press: inside(importing, button("Add")), say: "Add is the one ask" },
  { hover: inside(importing, role("img", "Added")), say: "The Requests are yours now, for acme/widgets" },
]);

export { requestsInAPackage as "requests-in-a-package" };
