// The account as a page: Sign in, leave mid-sign-in, find the way back by the mark, finish set-up, add a
// second profile, sign it out, and one sign-in that fails (`account-page`); and two profiles with the
// repositories bound to each, one waiting in Not matched, chosen, and one moved between profiles
// (`account-repositories`). The page is reached from the title menu's one row.

import { button, dialog, inside, role, text, walk } from "../walk";

const MENU = dialog("Menu");
const DEVICE_PAGE = button(/login\/device/);
const ENDED = role("listitem", "Sign-in ended", { exact: true });
const CODE = role("group", "Device code", { exact: true });
const WAITING = role("img", /^Waiting for/);
const TURNED_DOWN = role("img", "Turned down", { exact: true });
const SET_UP = (row: string) => inside(role("listitem", row, { exact: true }), role("img", "Done", { exact: true }));
const CLOSE = button(/Close panel/);
const MENU_BUTTON = button("Menu", { exact: true });
const ROW = (repository: string) => role("listitem", repository, { exact: true });
const MARK = (repository: string, word: string) => inside(ROW(repository), role("img", word, { exact: true }));

const accountPage = walk("account-page", [
  { look: MENU_BUTTON, say: "Signed out: the menu button carries a dot" },
  { press: MENU_BUTTON, say: "Open the menu" },
  { press: inside(MENU, button("Sign in", { exact: true })), say: "Sign in opens the account page" },
  { look: role("region", "Turns on", { exact: true }), say: "Signed out: what signing in turns on, sets up and never does" },
  { press: button(/^Sign in with/), say: "Start the sign-in" },
  { look: CODE, say: "The code" },
  { press: button("Kit", { exact: true }), say: "Leave the page mid-sign-in" },
  { press: MENU_BUTTON, say: "Open the menu" },
  { look: inside(MENU, WAITING), say: "The account row turns while it waits" },
  { hover: inside(MENU, WAITING), say: "Its tooltip" },
  { press: inside(MENU, button("Sign in", { exact: true })), say: "Back to the account" },
  { look: CODE, say: "The code is still there" },
  { press: DEVICE_PAGE, say: "The code is typed" },
  { look: SET_UP("Store"), say: "Identity and store set up" },
  { look: SET_UP("Shared home"), say: "Shared home set up" },
  { press: button("Add profile", { exact: true }), say: "Add profile" },
  { press: DEVICE_PAGE, say: "The work login signs in" },
  { press: inside(dialog("What syncs"), CLOSE), say: "What syncs opens, every kind off" },
  { press: button("Sign out mele-work", { exact: true }), say: "Sign the work profile out" },
  { look: text("Store still lists this device"), say: "The store kept its row" },
  { press: button("Add profile", { exact: true }), say: "Add profile" },
  { press: DEVICE_PAGE, say: "The code is typed" },
  { look: inside(ENDED, text("The sign-in was turned down.")), say: "Turned down, and Try again" },
  { press: MENU_BUTTON, say: "Open the menu" },
  { look: inside(MENU, TURNED_DOWN), say: "The account row carries the mark" },
]);

const accountRepositories = walk("account-repositories", [
  { press: MENU_BUTTON, say: "Open the menu" },
  { press: inside(MENU, button("Account", { exact: true })), say: "The account page" },
  { look: role("list", "Repositories of nickmele", { exact: true }), say: "Each profile lists the repositories bound to it" },
  { hover: MARK("nickmele/recipe-scaler", "Matched"), say: "Matched: its owner is the account" },
  { hover: MARK("mele-labs/armada", "Matched"), say: "Matched through an organisation" },
  { look: role("region", "Not matched", { exact: true }), say: "Two repositories no single profile matches" },
  { hover: MARK("shared-org/tools", "Waiting for a choice"), say: "Both profiles match shared-org, so it waits" },
  { press: button("Choose for acme-co/vendor-lib", { exact: true }), say: "Choose a profile for it" },
  { press: role("menuitem", "nickmele", { exact: true }), say: "nickmele" },
  { hover: MARK("acme-co/vendor-lib", "Chosen"), say: "It moves under nickmele, marked chosen" },
  { press: button("Move mele-labs/billing", { exact: true }), say: "Move a matched repository" },
  { press: role("menuitem", "nickmele", { exact: true }), say: "To the other profile" },
  { look: MARK("mele-labs/billing", "Chosen"), say: "Now chosen, under nickmele" },
  { press: button("Choose for shared-org/tools", { exact: true }), say: "Choose for the last one" },
  { press: role("menuitem", "None", { exact: true }), say: "None" },
  { hover: MARK("shared-org/tools", "None"), say: "It claims and syncs nothing" },
  { look: role("region", "Never", { exact: true }), say: "What signing in never does, at the foot" },
]);

export { accountPage as "account-page", accountRepositories as "account-repositories" };
