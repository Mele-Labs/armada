// The Fleet panel's build section on a preview that left branches out: the list under the build line, and the
// full reason on hover. Over `scenarios/fleet-build.ts`, scenario `fleet/preview-left-out`.

import { button, role, text, walk } from "../walk";

const previewLeftOut = walk("fleet/preview-left-out", [
  { press: button(/^Fleet — /), say: "The Fleet dot opens its panel" },
  { look: role("list", "Left out of the preview"), say: "Each branch the preview was built without, with its reason under it" },
  { hover: text(/^conflict:/), say: "The reason is cut to one line, and the full text is here" },
]);

export { previewLeftOut as "preview-left-out" };
