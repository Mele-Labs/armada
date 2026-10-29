// A task dispatched from Projects to another repository than its own: the sheet says the Job will not hold it, and Start opens the
// Job without it. Then a start Fleet refuses: the reason shows in the sheet, which keeps its words. Over the scenario of the same name.

import { button, inside, place, role, text, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const start = inside(frame, button("Start", { exact: true }));

const projectsOtherRepository = walk("projects-other-repository", [
  { press: place("Projects"), say: "Projects" },
  { press: button("Dispatch Update the changelog"), say: "Dispatch on a task of armada's" },
  { look: request, say: "The sheet opens on armada, the task's repository, with the task as the words" },
  { press: inside(frame, role("radio", "Repository site", { exact: true })), say: "Another repository picked" },
  { look: inside(frame, text(/This task is in armada\. A Job started in site is not linked to it, so finishing the Job leaves it open\./)), say: "The sheet says the Job will not be linked to the task" },
  { press: start, say: "Start. Fleet takes it, with the task's words and no rows" },
  { look: text(/Update the changelog/), say: "The Job opens in site, on the task's words" },
  { press: place("Projects"), say: "Back to Projects" },
  { press: button("Dispatch Raise a call on all done"), say: "Dispatch on a task Fleet has since moved to site" },
  { press: start, say: "Start in armada" },
  { look: inside(frame, role("alert")), say: "Fleet refused it. The reason is in the sheet, by the task's title and repository" },
  { look: request, say: "The sheet stays open with its words, and no Job's thread shows the refusal" },
]);

export { projectsOtherRepository as "projects-other-repository" };
