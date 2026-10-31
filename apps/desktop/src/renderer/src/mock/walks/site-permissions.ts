// Site settings in the Job browser: the lock before the address opens what the page may use, for this site and as the default for
// every site, and a page that asks raises a bubble under the lock. While the agent drives the choices are fixed and a page's ask is
// refused unseen. Mock only: nothing is wired. Over the `site-permissions` scenario, with the person already driving.

import { button, deckRow, dialog, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const tile = inside(region("Flight controls"), button("Browser", { exact: true }));
const panel = dialog("Job browser");
const pane = inside(panel, role("group", "Job browser", { exact: true }));
const page = inside(pane, role("article", "Fernwood Supply checkout"));
const lock = inside(pane, button("Site settings", { exact: true }));
const settings = inside(pane, dialog("Site settings"));
const choose = (permission: string, choice: string) => inside(inside(settings, role("radiogroup", permission, { exact: true })), role("radio", choice, { exact: true }));
const view = (name: string) => inside(inside(settings, role("radiogroup", "Settings for", { exact: true })), role("radio", name, { exact: true }));
const okta = inside(page, button("Connect Okta Verify"));
const bubble = inside(pane, role("alertdialog", "Site request"));

const steps: Step[] = [
  toSessions,
  { press: deckRow("Fix the checkout total"), say: "A Job fixing the checkout total" },
  { press: tile, say: "Browser" },
  { look: pane, say: "The Job browser, with you driving" },
  { press: lock, say: "The lock before the address opens Site settings" },
  { look: inside(settings, role("radiogroup", "Local network", { exact: true })), say: "Seven permissions, each Ask, Allow or Block" },
  { press: choose("Notifications", "Block"), say: "Notifications, Block, for this site" },
  { press: view("All sites"), say: "All sites edits the defaults" },
  { press: choose("Camera", "Block"), say: "Camera, Block, everywhere" },
  { press: lock, say: "Close" },
  { press: okta, say: "The page asks for the local network" },
  { look: bubble, say: "The site and what it wants, with Block and Allow" },
  { press: inside(bubble, button("Allow", { exact: true })), say: "Allow" },
  { press: lock, say: "Reopen Site settings" },
  { look: choose("Local network", "Allow"), say: "Local network now shows Allow for this site" },
  { press: lock, say: "Close" },
  { press: inside(pane, button(/^You're driving/)), say: "Hand back to the agent" },
  { look: inside(pane, button("Take over to change site settings")), say: "The lock says why the choices are fixed" },
  { press: okta, say: "The page asks again" },
  { look: inside(pane, role("article", "Fernwood Supply checkout")), say: "No bubble appears while the agent drives" },
];

const sitePermissionsWalk = walk("site-permissions", steps);

export { sitePermissionsWalk as "site-permissions" };
