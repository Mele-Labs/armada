// What a chosen Job, pull request, issue, task, command or page row does, and what its actions do. **Each act is the
// code the Cockpit's row menu, the flight deck and the Projects page already run** (`cockpit/deck/acts.ts`, the Projects
// store); nothing here is a second copy. A command's shell line and chain have no runner yet. Apart from `palette.ts`
// because these rows are the palette's own, not the registry's.

import { SURFACE, type PaletteChoice, type PaletteCommand } from "@armada/shell";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { mergeSession, stopSession, toggleSessionPause } from "./cockpit/deck/acts";
import { setView } from "./cockpit/deck/store";
import type { PaletteFixtures, PaletteLinked } from "./palette-fixtures";
import { JOB_PAGE } from "./palette-page";
import { togglePin } from "./palette-memory";
import { focusAddTask } from "./projects-tab/keys";
import type { ProjectsStore } from "./projects-tab/store";

export type RowWorld = {
  fixtures: PaletteFixtures;
  jobs: readonly Session[];
  draft: SessionsDraft | undefined;
  openSession: (id: string) => void;
  compose: () => void;
  surface: (id: string) => void;
  /** A pull request's or an issue's address, opened where Bridge opens it. */
  openLink: (address: string) => boolean;
  /** Words moved into the dispatch sheet, as a Job. */
  startJob: (words: string) => void;
  /** The Job on the Cockpit's deck, or none. */
  job: Session | undefined;
  /** The Projects tab with a task open and in view, by its row id. Absent where the window keeps no projects. */
  task?: (rowId: string) => void;
  /** His projects, which a task is ticked and a project named in. Absent where the window keeps none. */
  projects?: ProjectsStore | undefined;
  /** The title a task row went by, for the Job that starts on it. */
  taskTitle: (rowId: string) => string | undefined;
};

const pullOf = (world: RowWorld, id: string): PaletteLinked | undefined => world.fixtures.pulls.find((one) => `${one.repo}:${one.number}` === id);
const issueOf = (world: RowWorld, id: string): PaletteLinked | undefined => world.fixtures.issues.find((one) => `${one.repo}:${one.number}` === id);

/** The pull request a Job holds, with the address it opens from. */
const heldBy = (job: Session): { number: number; address: string } | undefined => {
  const held = job.attachments.find((one) => one.kind === "pull_request");
  return held === undefined || held.kind !== "pull_request" ? undefined : { number: held.number, address: held.address };
};

export function carryOutRow(choice: PaletteChoice, world: RowWorld): void {
  switch (choice.of) {
    case "pull": {
      const one = pullOf(world, choice.id);
      if (one !== undefined) world.openLink(one.url);
      return;
    }
    case "issue": {
      const one = issueOf(world, choice.id);
      if (one !== undefined) world.openLink(one.url);
      return;
    }
    case "task":
      world.task?.(choice.id);
      return;
    case "command":
      runCommand(world.fixtures.commands.find((one) => one.id === choice.id), world);
      return;
    case "page":
      onPage(choice.id, world);
      return;
    case "on":
      on(choice.target, choice.action, world);
  }
}

function runCommand(command: PaletteCommand | undefined, world: RowWorld): void {
  if (command === undefined) return;
  // A URL opens in the browser. A shell line runs in the checkout, and a chain steps through the palette; neither has a runner here yet.
  if (command.kind === "url" && command.value !== undefined) window.open(`https://${command.value}`, "_blank", "noopener");
  if (command.kind === "chain") world.surface(SURFACE.projects);
}

function onPage(id: string, world: RowWorld): void {
  if (id.startsWith(JOB_PAGE) && world.job !== undefined) return on(`session:${world.job.id}`, id.slice(JOB_PAGE.length), world);
  if (id === "dispatch") world.compose();
  else if (id === "map" || id === "timeline") setView(id);
  else if (id === "needing-you") world.surface(SURFACE.overview);
  // Projects' own field and name box: the page's key runs the same two.
  else if (id === "add-task") window.requestAnimationFrame(focusAddTask);
  else if (id === "new-project") world.projects?.name(true);
}

function on(target: string, action: string, world: RowWorld): void {
  if (action === "pin" || action === "unpin") return togglePin(target);
  const [kind, ...rest] = target.split(":");
  const id = rest.join(":");
  if (kind === "session") {
    const job = world.jobs.find((one) => one.id === id);
    const held = job === undefined ? undefined : heldBy(job);
    if (action === "open") world.openSession(id);
    else if (action === "pause" || action === "resume") void toggleSessionPause(world.draft, id, action === "resume");
    else if (action === "stop") stopSession(world.draft, id);
    else if (action === "fork") void Promise.resolve(world.draft?.fork?.(id)).then((forked) => forked !== undefined && world.openSession(forked));
    else if (action === "open-pull") {
      if (held !== undefined) world.openLink(held.address);
    } else if (action === "merge-pull") {
      if (held !== undefined) void mergeSession(world.draft, id, held.number);
    } else if (action === "copy-id") void navigator.clipboard?.writeText(id);
    return;
  }
  if (kind === "pull" || kind === "issue") {
    const one = kind === "pull" ? pullOf(world, id) : issueOf(world, id);
    if (one === undefined) return;
    if (action === "open") world.openLink(one.url);
    else if (action === "github") window.open(one.url, "_blank", "noopener");
    else if (action === "start-job") world.startJob(`#${one.number} ${one.title}`);
    return;
  }
  if (kind === "task") {
    const title = world.taskTitle(id);
    if (action === "open") world.task?.(id);
    else if (action === "dispatch" && title !== undefined) world.startJob(title);
    else if (action === "tick") world.projects?.tick(id);
    return;
  }
  if (kind === "cmd" && action === "run") runCommand(world.fixtures.commands.find((one) => one.id === id), world);
}
