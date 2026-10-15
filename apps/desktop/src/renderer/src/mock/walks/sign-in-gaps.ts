// The states the Account view was missing, over the scenarios of the same names: a sign-in that ends
// without signing in four ways and is then refused for blank settings (`sign-in-ended`), a profile
// signing out with the store down (`sign-out`), and a sign-in where no shared home exists yet, with What
// syncs greyed (`sign-in-no-home`) or not drawn (`sign-in-no-home-hidden`, the second take).

import { button, dialog, inside, role, text, walk } from "../walk";

const MENU = dialog("Menu");
const SIGN_IN = inside(MENU, button(/^Sign in with/));
const DEVICE_PAGE = button(/login\/device/);
const TRY_AGAIN = button("Try again", { exact: true });
const ENDED = role("region", "Sign-in ended", { exact: true });

const signInEnded = walk("sign-in-ended", [
  { press: button("Menu", { exact: true }), say: "Open the menu" },
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
  { press: inside(MENU, button("Account nickmele", { exact: true })), say: "The account" },
  { look: button("Sign out mele-work", { exact: true }), say: "Sign out is on each profile" },
  { press: button("Sign out mele-work", { exact: true }), say: "Sign the work profile out" },
  { look: role("listitem", "mele-work", { exact: true }), say: "mele-work signed out" },
  { press: button("Sign out nickmele", { exact: true }), say: "Sign the last profile out, the store down" },
  { look: text("Store still lists this device"), say: "The store kept its row" },
  { hover: role("img", "Store not reached", { exact: true }), say: "What the store said" },
  { press: button("Back to the menu", { exact: true }), say: "Back" },
  { look: SIGN_IN, say: "Signed out" },
]);

/** The steps both takes share: the sign-in, the set-up rows, and a second login added. */
const NO_HOME = [
  { press: button("Menu", { exact: true }), say: "Open the menu" },
  { press: SIGN_IN, say: "Sign in" },
  { press: DEVICE_PAGE, say: "The code is typed" },
  { look: inside(role("listitem", "Store", { exact: true }), role("img", "Done", { exact: true })), say: "Identity and store done" },
  {
    look: inside(role("listitem", "Shared home", { exact: true }), role("img", "Not set up yet", { exact: true })),
    say: "The shared home is not set up yet",
  },
  { hover: role("img", "Not set up yet", { exact: true }), say: "Its tooltip" },
  { press: button("Menu", { exact: true }), say: "Close the menu" },
  { press: button("Menu", { exact: true }), say: "Open it again" },
  { press: inside(MENU, button("Account nickmele", { exact: true })), say: "The account" },
  { press: button("Add profile", { exact: true }), say: "Add profile" },
  { press: DEVICE_PAGE, say: "The work login signs in" },
] as const;

const signInNoHome = walk("sign-in-no-home", [
  ...NO_HOME,
  { look: button("What syncs", { exact: true }), say: "What syncs stays, greyed" },
  { hover: button("What syncs", { exact: true }), say: "Why" },
]);

const signInNoHomeHidden = walk("sign-in-no-home-hidden", [
  ...NO_HOME,
  { look: button("mele-work", { exact: true }), say: "What syncs is not drawn" },
]);

export {
  signInEnded as "sign-in-ended",
  signOut as "sign-out",
  signInNoHome as "sign-in-no-home",
  signInNoHomeHidden as "sign-in-no-home-hidden",
};
