// The states the Account view was missing, over the scenarios of the same names: a sign-in that ends
// without signing in four ways and is then refused for blank settings (`sign-in-ended`), a profile
// signing out with the store down (`sign-out`), and a sign-in where no shared home exists yet, with What
// syncs greyed (`sign-in-no-home`).

import { button, dialog, inside, role, text, walk } from "../walk";

const MENU = dialog("Menu");
// The account is a page: the menu's one row opens it, and its own button starts the sign-in.
const OPEN_ACCOUNT = inside(MENU, button("Sign in", { exact: true }));
const SIGN_IN = button(/^Sign in with/);
const DEVICE_PAGE = button(/login\/device/);
const TRY_AGAIN = button("Try again", { exact: true });
const ENDED = role("listitem", "Sign-in ended", { exact: true });

const signInEnded = walk("sign-in-ended", [
  { press: button("Menu", { exact: true }), say: "Open the menu" },
  { press: OPEN_ACCOUNT, say: "The account page" },
  { look: role("region", "Turns on", { exact: true }), say: "Signed out: what signing in turns on, sets up and never does" },
  { press: SIGN_IN, say: "Sign in" },
  { press: DEVICE_PAGE, say: "The code is typed" },
  { look: inside(ENDED, text("The sign-in was turned down.")), say: "Turned down, and Try again" },
  { hover: role("img", "Turned down", { exact: true }), say: "The mark for how it ended" },
  { press: TRY_AGAIN, say: "Try again" },
  { press: DEVICE_PAGE, say: "The code is typed" },
  { look: inside(ENDED, text("The code ran out before it was typed.")), say: "Expired" },
  { press: TRY_AGAIN, say: "Try again" },
  { press: DEVICE_PAGE, say: "The code is typed" },
  { look: inside(ENDED, text("The forge could not be reached")), say: "Not reached" },
  { press: TRY_AGAIN, say: "Try again" },
  { press: DEVICE_PAGE, say: "The code is typed" },
  { look: inside(ENDED, text("The forge refused the sign-in")), say: "Refused" },
  { press: TRY_AGAIN, say: "Try again" },
  { look: text("Sign-in is unavailable"), say: "Refused before it began: the settings that are blank" },
  { look: button("Store address", { exact: true }), say: "A button to each" },
  { press: button("Sign-in app", { exact: true }), say: "Settings opens at the sign-in app" },
  { look: text("Sign-in app"), say: "Settings" },
]);

const signOut = walk("sign-out", [
  { press: button("Menu", { exact: true }), say: "Open the menu" },
  { press: inside(MENU, button("Account", { exact: true })), say: "The account page" },
  { look: button("Sign out mele-work", { exact: true }), say: "Sign out is on each profile" },
  { press: button("Sign out mele-work", { exact: true }), say: "Sign the work profile out" },
  { look: role("listitem", "Signed out", { exact: true }), say: "mele-work signed out" },
  { press: button("Sign out nickmele", { exact: true }), say: "Sign the last profile out, the store down" },
  { look: text("Store still lists this device"), say: "The store kept its row" },
  { hover: role("img", "Store not reached", { exact: true }), say: "What the store said" },
  { press: button("Menu", { exact: true }), say: "Open the menu" },
  { look: OPEN_ACCOUNT, say: "Signed out" },
]);

/** The steps both takes share: the sign-in, the set-up rows, and a second login added. */
const NO_HOME = [
  { press: button("Menu", { exact: true }), say: "Open the menu" },
  { press: OPEN_ACCOUNT, say: "The account page" },
  { press: SIGN_IN, say: "Sign in" },
  { press: DEVICE_PAGE, say: "The code is typed" },
  { look: inside(role("listitem", "Store", { exact: true }), role("img", "Done", { exact: true })), say: "Identity and store done" },
  {
    look: inside(role("listitem", "Shared home", { exact: true }), role("img", "Not set up yet", { exact: true })),
    say: "The shared home is not set up yet",
  },
  { hover: role("img", "Not set up yet", { exact: true }), say: "Its tooltip" },
  { press: button("Add profile", { exact: true }), say: "Add profile" },
  { press: DEVICE_PAGE, say: "The work login signs in" },
] as const;

const signInNoHome = walk("sign-in-no-home", [
  ...NO_HOME,
  { look: button("What syncs", { exact: true }), say: "What syncs stays, greyed" },
  { hover: button("What syncs", { exact: true }), say: "Why" },
]);

// Straight to the point: the guide, why the home is blank, and the button that lands on Kit.
const syncYourKit = walk("sync-your-kit", [
  { press: button("Menu", { exact: true }), say: "Open the menu" },
  { press: OPEN_ACCOUNT, say: "The account page" },
  { look: role("region", "Sync your Kit", { exact: true }), say: "Signed out: the way to a second machine, before signing in" },
  { press: SIGN_IN, say: "Sign in" },
  { press: DEVICE_PAGE, say: "The code is typed" },
  { look: button("Open Kit", { exact: true }), say: "Step two is next, and carries the button" },
  { hover: role("img", "Not set up yet", { exact: true }), say: "The shared home: what it is and what makes it" },
  { press: button("Open Kit", { exact: true }), say: "Opens Kit, where Sync makes the home" },
]);

export {
  syncYourKit as "sync-your-kit",
  signInEnded as "sign-in-ended",
  signOut as "sign-out",
  signInNoHome as "sign-in-no-home",
};
