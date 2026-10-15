// The account in its two homes, over `account-page` and `account-rail`: Sign in, leave mid-sign-in, find
// the way back by the mark, finish set-up, add a second profile, sign it out, and one sign-in that fails.
// The page is reached from the title menu's one row; the panel from a row in the rail.

import { button, dialog, inside, role, text, walk } from "../walk";
import type { Step } from "../walk";

const MENU = dialog("Menu");
const DEVICE_PAGE = button(/login\/device/);
const ENDED = role("listitem", "Sign-in ended", { exact: true });
const CODE = role("group", "Device code", { exact: true });
const WAITING = role("img", /^Waiting for/);
const TURNED_DOWN = role("img", "Turned down", { exact: true });
const SET_UP = (row: string) => inside(role("listitem", row, { exact: true }), role("img", "Done", { exact: true }));
const CLOSE = button(/Close panel/);

/** Both takes, step for step but for where the account is opened from. */
function account(place: "page" | "rail"): Step[] {
  const page = place === "page";
  const menu = button("Menu", { exact: true });
  const named = (label: string) => (page ? inside(MENU, button(label, { exact: true })) : button(label, { exact: true }));
  const signIn = named("Sign in");
  return [
    page
      ? { look: menu, say: "Signed out: the menu button carries a dot" }
      : { look: signIn, say: "Signed out: the account row in the rail" },
    ...(page ? [{ press: menu, say: "Open the menu" }] : []),
    { press: signIn, say: page ? "Sign in opens the account page" : "Sign in opens the account panel" },
    { look: role("region", "Turns on", { exact: true }), say: "Signed out: what signing in turns on, sets up and never does" },
    { press: button(/^Sign in with/), say: "Start the sign-in" },
    { look: CODE, say: "The code" },
    page ? { press: button("Kit", { exact: true }), say: "Leave the page mid-sign-in" } : { press: CLOSE, say: "Put the panel away mid-sign-in" },
    ...(page
      ? [
          { press: menu, say: "Open the menu" },
          { look: inside(MENU, WAITING), say: "The account row turns while it waits" },
          { hover: inside(MENU, WAITING), say: "Its tooltip" },
        ]
      : [
          { look: WAITING, say: "The rail row turns while it waits" },
          { hover: WAITING, say: "Its tooltip" },
        ]),
    { press: signIn, say: "Back to the account" },
    { look: CODE, say: "The code is still there" },
    { press: DEVICE_PAGE, say: "The code is typed" },
    { look: SET_UP("Store"), say: "Identity and store set up" },
    { look: SET_UP("Shared home"), say: "Shared home set up" },
    { press: button("Add profile", { exact: true }), say: "Add profile" },
    { press: DEVICE_PAGE, say: "The work login signs in" },
    { press: inside(dialog("What syncs"), CLOSE), say: "What syncs opens, every kind off" },
    ...(page ? [] : [{ press: button("Account nickmele", { exact: true }), say: "Open the account again" }]),
    { press: button("Sign out mele-work", { exact: true }), say: "Sign the work profile out" },
    { look: text("Store still lists this device"), say: "The store kept its row" },
    { press: button("Add profile", { exact: true }), say: "Add profile" },
    { press: DEVICE_PAGE, say: "The code is typed" },
    { look: inside(ENDED, text("The sign-in was turned down.")), say: "Turned down, and Try again" },
    ...(page ? [{ press: menu, say: "Open the menu" }] : []),
    page
      ? { look: inside(MENU, TURNED_DOWN), say: "The account row carries the mark" }
      : { look: TURNED_DOWN, say: "The rail row carries the mark" },
  ];
}

const accountPage = walk("account-page", account("page"));
const accountRail = walk("account-rail", account("rail"));

export { accountPage as "account-page", accountRail as "account-rail" };
