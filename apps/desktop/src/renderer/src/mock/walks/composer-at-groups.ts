// A Session's `@` offers the Sessions. Once letters follow it the files that match
// them show under those, at most five, and choosing one writes `@path` into the line as plain text.
// Pull requests and branches are not under `@`: pull requests are `#`'s. Over `composer-at-groups`.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { message } = kit(false);

const composerAtGroups = walk("composer-at-groups", [
  toSessions,
  { press: inside(region("Work", { exact: true }), button("Store migration spike")), say: "A Session" },
  { type: "@", into: message, say: "An at sign opens what can be tagged" },
  { look: role("group", "Sessions"), say: "The other Sessions come first" },
  { type: "@re", into: message, say: "A few letters after it" },
  { look: role("group", "Files"), say: "Files that match them show under the Sessions" },
  { look: role("option", /sessions\.tsx/), say: "Each is a name and the folder it is in" },
  { press: role("option", /sessions\.tsx/), say: "One chosen" },
  { look: text("@apps/desktop/src/renderer/src/sessions.tsx"), say: "It is written into the line as plain text" },
]);

export { composerAtGroups as "composer-at-groups" };
