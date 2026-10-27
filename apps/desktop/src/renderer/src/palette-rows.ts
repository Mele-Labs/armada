// What a chosen Job, pull request, issue, task, command or page row does, and what its actions do. **A stand-in for
// now**: each does the nearest thing the window already has, and the ones with nothing behind them yet do nothing, which
// the wiring step names and answers. Apart from `palette.ts` because these rows are the palette's own, not the registry's.

import { SURFACE, type PaletteChoice, type PaletteCommand } from "@armada/shell";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { setView } from "./cockpit/deck/store";
import type { PaletteFixtures, PaletteLinked } from "./palette-fixtures";
import { JOB_PAGE } from "./palette-page";
import { togglePin } from "./palette-memory";

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
};

const pullOf = (world: RowWorld, id: string): PaletteLinked | undefined => world.fixtures.pulls.find((one) => `${one.repo}:${one.number}` === id);
const issueOf = (world: RowWorld, id: string): PaletteLinked | undefined => world.fixtures.issues.find((one) => `${one.repo}:${one.number}` === id);

/** The pull request a Job holds, as the palette lists it. */
const heldBy = (world: RowWorld, job: Session): PaletteLinked | undefined => {
  const held = job.attachments.find((one) => one.kind === "pull_request");
  return held === undefined || held.kind !== "pull_request" ? undefined : world.fixtures.pulls.find((one) => one.number === held.number);
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
      world.surface(SURFACE.projects);
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
  // Projects' own field and button, which the page draws; the wiring step asks the page instead of reaching for them.
  else if (id === "add-task") window.requestAnimationFrame(() => document.querySelector<HTMLElement>('[aria-label="Add a task"]')?.focus());
  else if (id === "new-project") window.requestAnimationFrame(() => document.querySelector<HTMLElement>('[aria-label="New project"]')?.click());
}

function on(target: string, action: string, world: RowWorld): void {
  if (action === "pin" || action === "unpin") return togglePin(target);
  const [kind, ...rest] = target.split(":");
  const id = rest.join(":");
  if (kind === "session") {
    const job = world.jobs.find((one) => one.id === id);
    if (action === "open") world.openSession(id);
    else if (action === "pause") void world.draft?.pause?.(id);
    else if (action === "resume") void world.draft?.resume?.(id);
    else if (action === "stop") world.draft?.stop?.(id);
    else if (action === "fork") void Promise.resolve(world.draft?.fork?.(id)).then((forked) => forked !== undefined && world.openSession(forked));
    else if (action === "open-pull") {
      const held = job === undefined ? undefined : heldBy(world, job);
      if (held !== undefined) world.openLink(held.url);
    } else if (action === "copy-id") void navigator.clipboard?.writeText(id);
    // Merge pull request has nothing to merge through yet.
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
    const one = world.fixtures.tasks.find((held) => held.id === id);
    if (action === "open") world.surface(SURFACE.projects);
    else if (action === "dispatch" && one !== undefined) world.startJob(one.title);
    // Tick has no task to tick until the tasks are read from Projects.
    return;
  }
  if (kind === "cmd" && action === "run") runCommand(world.fixtures.commands.find((one) => one.id === id), world);
}
