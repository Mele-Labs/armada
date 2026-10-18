// A restart that failed raises the failure toast once, with Dispatch job. Fleet reports the preview restart under way, then
// failed with the compiler's line; the toast names the build and the line, and its first control is Dispatch job. Over
// `build-failed-toast`.

import { button, text, walk } from "../walk";

const buildFailedToast = walk("build-failed-toast", [
  { later: button(/^Fleet — /), say: "Fleet's next read of the build says the preview restart failed" },
  { look: text("Preview build failed: error[E0432]: unresolved import crate::drones"), say: "The toast names the build and the line Fleet reports" },
  { look: button("Dispatch job"), say: "Dispatch job is the face of its control, and Dismiss is beside it" },
]);

export { buildFailedToast as "build-failed-toast" };
