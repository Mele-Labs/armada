// A picker over many repositories: type to narrow it, and the next time the one picked leads under
// Recent. Told through the dispatcher's More menu, which lists the repositories that do not fit as
// chips. A repository counts as picked for Recent once a Job is started in it.

import { button, inside, region, role, text, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const more = inside(frame, button(/^(More repositories|Repository )/));

export const findingOneRepositoryOfMany = walk("many-repositories", [
  { key: "n", on: jobs, say: "N opens the dispatcher, with the repositories that fit as chips" },
  { press: more, say: "Twenty-one repositories, so the rest are a menu that opens on a filter field" },
  { type: "arm", into: role("searchbox", "Find a repository"), say: "Typing narrows it to what matches" },
  { look: role("menuitem", "armada", { exact: true }), say: "Nothing else is left to read past but a repository not set up, which cannot be picked" },
  { press: role("menuitem", "armada", { exact: true }), say: "Picking one chooses it for the request" },
  { type: "Check the migration order", into: request, say: "An idea for it" },
  { press: inside(frame, button("Start", { exact: true })), say: "Started, which is what makes it a recent pick" },
  { key: "n", on: jobs, say: "N opens the dispatcher again" },
  { press: more, say: "The menu, open again" },
  { look: text("Recent"), say: "The repository just picked leads, under Recent" },
]);
