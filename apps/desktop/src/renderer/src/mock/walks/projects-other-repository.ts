// A task dispatched from Projects to another repository than its own: a new Job opens there and holds the task all the same, its
// rail names the task under the brief, and a press there opens it on Projects. Then a start Fleet refuses: the reason is the
// window's toast, and the sheet keeps its words. Over the scenario of the same name.

import { button, inside, place, region, role, text, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const start = inside(frame, button("Start", { exact: true }));
const held = role("list", "Task", { exact: true });
const line = inside(held, button("Open Update the changelog on Projects", { exact: true }));

const projectsOtherRepository = walk("projects-other-repository", [
  { press: place("Projects"), say: "Projects" },
  { press: button("Dispatch Update the changelog"), say: "Dispatch on a task of armada's" },
  { look: request, say: "The sheet opens on armada, the task's repository, with the task as the words" },
  { press: inside(frame, role("radio", "Repository site", { exact: true })), say: "Another repository picked" },
  { press: start, say: "Start in site. Fleet takes it, with the task's words and its row" },
  { look: region("Job: Update the changelog"), say: "A new Job opens in site, named after the task's words" },
  { look: line, say: "Under the brief, the Task line names the task the Job holds" },
  { look: inside(held, text("No project")), say: "And its project. This task is in none" },
  { press: line, say: "A press opens it on Projects" },
  { look: role("checkbox", "In a Job: Update the changelog", { exact: true }), say: "Projects opens on the task, which reads In a Job" },
  { press: button("Dispatch Raise a call on all done"), say: "Dispatch on a task Fleet has since lost" },
  { press: start, say: "Start" },
  { look: text(/no row is named task-all-done/), say: "Fleet refused it. The reason is in the error toast" },
  { look: text("What Fleet refused"), say: "The toast holds what Fleet refused, and Dispatch job to look into it" },
  { look: request, say: "The sheet stays open with its words" },
]);

export { projectsOtherRepository as "projects-other-repository" };
