// The Dispatch request tags what it is about as the Job composer does. `@` offers the Jobs and, once something follows it, the repository's files;
// `#` offers its open issues and pull requests by number. Choosing one writes it into the words. Over `session-after-another`, whose repository is chosen.

import { inside, role, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const menu = inside(frame, role("listbox"));

const chip = inside(frame, role("group", "Start condition"));

const dispatchTagsFilesAndPullRequests = walk("session-after-another", [
  { key: "n", on: role("region", "Jobs", { exact: true }), say: "n opens Dispatch on the repository the rail is on" },
  { type: "look at @ses", into: request, say: "@ offers the Jobs, then the files that match what follows it" },
  { look: menu, say: "A file's row names it, then the folder it is in" },
  { key: "Enter", on: request, say: "↵ writes the path into the request" },
  { type: "and review #22", into: request, say: "# offers the open issues and pull requests, newest number first" },
  { look: menu, say: "The glyph says issue or pull request" },
  { key: "Enter", on: request, say: "↵ writes the reference and title. The pull request is sent with the first message" },
]);

export { dispatchTagsFilesAndPullRequests };

// A Job tagged with no condition in the words is only referred to: no sentence, and the new Job starts at once with the tag. Saying when changes that.
const dispatchPlainJobTag = walk("session-after-another", [
  { key: "n", on: role("region", "Jobs", { exact: true }), say: "n opens Dispatch" },
  { type: "look at what @Coc\n", into: request, say: "@ picks the Job and writes its tag into the request" },
  { type: "look at what @Cockpit Map did to the Field", into: request, say: "Nothing says to wait, so no sentence is drawn: the tag is a reference and the new Job starts at once" },
  { type: "look at what @Cockpit Map did to the Field and start once it lands", into: request, say: "Say when and the sentence appears: the new Job waits" },
  { look: chip, say: "The proposer's reading of the wait" },
]);

export { dispatchPlainJobTag };
