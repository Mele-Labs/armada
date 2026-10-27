// What a chosen Job, pull request, issue, task, command or page row does, and what its actions do. **Each act is the
// code the Cockpit's row menu, the flight deck and the Projects page already run** (`cockpit/deck/acts.ts`, the Projects
// store); nothing here is a second copy. A Commands mod's rows run here too (`runModRow`): a prompt starts a Job, an address
// opens, a chain runs the rows it names. A shell line has no terminal to run in yet, so its row is drawn dormant. Apart from
// `palette.ts` because these rows are the palette's own, not the registry's.

import { SURFACE, type PaletteChoice, type PaletteCommand } from "@armada/shell";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { mergeSession, stopSession, toggleSessionPause } from "./cockpit/deck/acts";
import { fill, type ModRow } from "./mod-commands";
import { knownRows } from "./palette-known";
import { askedOf } from "./palette-blanks";
import { setView } from "./cockpit/deck/store";
import type { PaletteFixtures, PaletteLinked } from "./palette-fixtures";
import { JOB_PAGE } from "./palette-page";
import { setAlias, setHotkey, togglePin } from "./palette-memory";
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
  /** The rows of every Commands mod that is on. */
  modRows: readonly ModRow[];
  /** Says a sentence as the window's toast. */
  tell: (sentence: string) => void;
  /** Runs a palette row by its id, as pressing ↵ on it would. */
  runRow: (id: string) => void;
  /** Opens the palette on the row's first blank, for a row that was run without them. */
  openAt: (row: ModRow) => void;
  /** The repositories the title row lists, and the one picked by root. */
  repositories: readonly { root: string; label: string }[];
  picked: string | null;
  /** Runs a line in a repository's checkout, showing its output in the side panel. False where no panel is hosted. */
  runShell: (command: string, dir: string, repository: string) => boolean;
};

/** What a blank was filled with: as a prompt reads it, and as an address does (`#1899` and `1899`). */
export type Filled = Readonly<Record<string, { text: string; bare: string; /** A repository's root, where the blank named one. */ root?: string }>>;

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
      runCommandId(choice.id, world);
      return;
    case "page":
      onPage(choice.id, world);
      return;
    case "on":
      on(choice.target, choice.action, world, choice.value);
  }
}

/** A command by its id: a mod's row, or else one a scenario handed the palette. */
function runCommandId(id: string, world: RowWorld): void {
  const mod = world.modRows.find((one) => one.id === id);
  if (mod !== undefined) return runModRow(mod, {}, world);
  runCommand(world.fixtures.commands.find((one) => one.id === id), world);
}

const blanksOf = (row: ModRow): readonly string[] => row.command.blanks ?? [];
const view = (filled: Filled, key: "text" | "bare", encode = (word: string) => word): Record<string, string> =>
  Object.fromEntries(Object.entries(filled).map(([name, one]) => [name, encode(one[key])]));

/** Rows being chained now, so a chain that comes back to a row it is running stops there. */
const chaining = new Set<string>();

/**
 * A Commands mod's row, with the blanks it was asked. A row run without them (a key, a chain) opens the palette on the first it
 * needs. A shell line also needs a repository: the one picked, or the one a blank named.
 */
export function runModRow(row: ModRow, filled: Filled, world: RowWorld): void {
  const { command } = row;
  const dir = Object.values(filled).find((one) => one.root !== undefined)?.root ?? world.picked;
  if (blanksOf(row).some((name) => filled[name] === undefined) || (command.shell !== undefined && dir === null)) return world.openAt(row);
  if (command.prompt !== undefined) return world.startJob(fill(command.prompt, view(filled, "text")));
  if (command.url !== undefined) {
    const address = fill(command.url, view(filled, "bare", encodeURIComponent));
    // A forge address opens where Bridge opens it; any other goes to the browser.
    if (!world.openLink(address)) void window.armada.openLink(address).then((followed) => followed.ok || world.tell(`${command.name}: ${address} did not open`));
    return;
  }
  if (command.shell !== undefined && dir !== null) {
    const repository = world.repositories.find((one) => one.root === dir)?.label ?? dir;
    // Typed as it was, with a pull request or an issue as its number: a `#` would start a comment.
    if (!world.runShell(fill(command.shell, view(filled, "bare")), dir, repository)) world.tell(`${command.name}: no side panel to show it in`);
    return;
  }
  if (command.chain !== undefined) runChain(row, command.chain, world);
}

/** The rows named, in order. A row that is missing or cannot run stops it, and the sentence names which. One that asks for a blank opens the palette there, and the chain ends. */
function runChain(row: ModRow, chain: readonly string[], world: RowWorld): void {
  const name = row.command.name;
  if (chaining.has(row.id)) return world.tell(`${name} chains back to itself`);
  chaining.add(row.id);
  try {
    for (const id of chain) {
      const known = knownRows().find((one) => one.id === id);
      if (known === undefined) return world.tell(`${name} stopped: no row ${id}`);
      if (known.dormant) return world.tell(`${name} stopped at ${known.label}: it cannot run`);
      const nested = world.modRows.find((one) => `cmd:${one.id}` === id);
      if (nested !== undefined && askedOf(nested, world.picked).length > 0) return world.openAt(nested);
      world.runRow(id);
    }
  } finally {
    chaining.delete(row.id);
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

function on(target: string, action: string, world: RowWorld, value = ""): void {
  if (action === "pin" || action === "unpin") return togglePin(target);
  // The word or key he gave a row, or its removal: kept by the palette, and no row of anyone's to open.
  if (action === "alias" || action === "remove-alias") return setAlias(target, action === "alias" ? value : "");
  if (action === "hotkey" || action === "remove-hotkey") return setHotkey(target, action === "hotkey" ? value : "");
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
    else if (action === "forge") window.open(one.url, "_blank", "noopener");
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
  if (kind === "cmd" && action === "run") runCommandId(id, world);
}
