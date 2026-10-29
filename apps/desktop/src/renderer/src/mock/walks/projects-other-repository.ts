// A task dispatched from Projects to another repository than its own: the Job opens there and holds the task all the same. Then a
// start Fleet refuses: the reason shows in the sheet, which keeps its words. Over the scenario of the same name.

import { button, inside, place, role, text, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const start = inside(frame, button("Start", { exact: true }));

const projectsOtherRepository = walk("projects-other-repository", [
  { press: place("Projects"), say: "Projects" },
  { press: button("Dispatch Update the changelog"), say: "Dispatch on a task of armada's" },
  { look: request, say: "The sheet opens on armada, the task's repository, with the task as the words" },
  { press: inside(frame, role("radio", "Repository site", { exact: true })), say: "Another repository picked" },
  { press: start, say: "Start in site. Fleet takes it, with the task's words and its row" },
  { look: text(/Update the changelog/), say: "The Job opens in site, on the task's words" },
  { press: place("Projects"), say: "Back to Projects" },
  { look: role("checkbox", "In a Job: Update the changelog", { exact: true }), say: "The task reads In a Job, held by the Job that started in site" },
  { press: button("Dispatch Raise a call on all done"), say: "Dispatch on a task Fleet has since lost" },
  { press: start, say: "Start" },
  { look: inside(frame, role("alert")), say: "Fleet refused it. The reason is in the sheet" },
  { look: request, say: "The sheet stays open with its words, and no Job's thread shows the refusal" },
]);

export { projectsOtherRepository as "projects-other-repository" };
