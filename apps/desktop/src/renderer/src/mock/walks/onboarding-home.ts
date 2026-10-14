// The home machine, first run: one button in the title bar, the device code and the link it is typed
// at, what signing in set up, and the account chip on the Kit page. Over `onboarding-home`.

import { button, inside, role, text, walk } from "../walk";

const SET_UP = (row: string) => inside(role("listitem", row, { exact: true }), role("img", "Done", { exact: true }));

const onboardingHome = walk("onboarding-home", [
  { look: button("Sign in with GitHub", { exact: true }), say: "Signed out: one button" },
  { press: button("Sign in with GitHub", { exact: true }), say: "Sign in" },
  { look: text("WDJB-MJHT"), say: "The code" },
  { hover: role("img", "Waiting for GitHub", { exact: true }), say: "Fleet is waiting" },
  { press: button("Open github.com/login/device"), say: "The page the code is typed at" },
  { look: SET_UP("Shared home"), say: "Identity, store and shared home, done" },
  { press: button("Kit", { exact: true }), say: "Kit" },
  { look: button("Account nickmele", { exact: true }), say: "The account chip, and the shared home beside Sync" },
]);

export { onboardingHome as "onboarding-home" };
