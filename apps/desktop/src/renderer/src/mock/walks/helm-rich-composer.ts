// Helm's message box is the Sessions box. ↵ sends and ⇧↵ breaks the line; the box brings a picture
// pasted or attached, sketches, `/` for the skills and commands, `@` for the other Sessions, and `#`
// for the repository's issues and pull requests by number. A number that is not open (a merged pull
// request) is looked up and still tags. What was sent shows in the thread, and a Session in another
// repository reads that repository's numbers. Over `helm-rich-composer`.

import { button, inside, place, region, role, text, walk } from "../walk";

const helm = role("complementary", "Helm");
const box = inside(helm, role("textbox", "Ask Helm"));
const sessions = region("Jobs", { exact: true });
const message = role("textbox", "Message");

const helmRichComposer = walk("helm-rich-composer", [
  { press: button("Helm"), say: "Helm's dock" },
  { look: inside(helm, button("Attach file")), say: "The paperclip, beside the pencil for a sketch" },
  { look: inside(helm, role("combobox", "Model")), say: "The model, and the effort beside it" },
  { look: inside(helm, role("button", /^Point Helm at a different repository/)), say: "The repository switch stays at the head, with no permission mode in the box" },

  { paste: box, say: "A screenshot pasted in" },
  { look: inside(inside(helm, role("group", "Attached")), text("Screenshot")), say: "It waits in the row, as a chip with a preview" },

  { type: "/", into: box, say: "A slash opens the skills and commands" },
  { look: inside(helm, role("listbox", "Skills and commands")), say: "The same list a Job offers" },
  { key: "Escape", on: box, say: "Esc puts it away" },

  { type: "@", into: box, say: "An at sign opens what can be tagged" },
  { look: inside(helm, role("group", "Jobs")), say: "Jobs of every repository" },
  { press: inside(helm, role("option", "Sessions list views")), say: "Tagged" },

  { type: "#22", into: box, say: "A hash and the start of a number" },
  { look: inside(helm, role("listbox", "Tag an issue or pull request")), say: "The open issues and pull requests of this repository that start with 22" },
  { hover: inside(helm, role("img", "Pull request")), say: "The kind, named on hover" },
  { press: inside(helm, role("option", /Pin the Bridge store clock/)), say: "A pull request, chosen" },

  { type: "#2207", into: box, say: "A number that is not open" },
  { look: inside(helm, role("option", /Stop the Drone count drifting/)), say: "It is looked up, and offered with where it stands" },
  { press: inside(helm, role("option", /Stop the Drone count drifting/)), say: "A merged pull request still tags" },

  { type: "do these explain the stale clock?", into: box, say: "Words after the tags" },
  { key: "Enter", on: box, say: "↵ sends" },
  { look: inside(helm, role("img", /Screenshot/)), say: "The thread row shows the picture it went with" },
  { look: inside(helm, role("link", /Stop the Drone count drifting/)), say: "And the tags, in the line, as links" },
  { look: inside(helm, text("Neither touches the Sessions page")), say: "Helm's reply" },

  { hover: inside(helm, role("link", "#2219")), say: "A pull request in the reply is a link, named on hover" },
  { press: inside(helm, role("link", "#2219")), say: "It opens in the side panel" },
  { look: role("dialog", "Pull request #2219"), say: "Where Bridge shows that pull request" },
  { key: "Escape", on: role("dialog", "Pull request #2219"), say: "Esc" },
  { hover: inside(helm, role("link", "#2207")), say: "A merged one links too, to where it landed" },
  { press: inside(helm, role("link", "Sessions list views")), say: "A Job named by its title opens" },
  { look: region("Thread"), say: "The Job" },
  { key: "Meta+[", on: helm, say: "Back" },
  { hover: inside(helm, role("link", /^#2219 Pin the Bridge store clock/)), say: "A tag in the person's own row links the same way" },
  { press: inside(helm, role("link", /^#2219 Pin the Bridge store clock/)), say: "It opens the same panel" },
  { look: role("dialog", "Pull request #2219"), say: "The pull request" },
  { key: "Escape", on: role("dialog", "Pull request #2219"), say: "Esc" },
  { press: place("Cockpit"), say: "A Job in another repository" },
  { press: inside(sessions, button("Cart totals rounding")), say: "Cart totals rounding, in storefront" },
  { type: "#41", into: message, say: "A hash and the start of a number" },
  { look: role("listbox", "Tag an issue or pull request"), say: "That repository's issues and pull requests, not armada's" },
  { look: role("option", /Round cart totals once, at the end/), say: "storefront's own" },
]);

export { helmRichComposer as "helm-rich-composer" };
