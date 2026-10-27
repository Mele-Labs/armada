import type { CodePhase, CodeRunEvent, CodeRunner, TerminalSize } from "./RunnableBlock";

/** What a run shows: replaced, never edited, so a subscriber reads it by identity. */
export type RunView = { phase: CodePhase; out: string; runs: number; open: boolean };

/** One run held outside any card: its state, the handle that stops it, and who is watching. */
export type RunEntry = {
  view: RunView;
  handle: ReturnType<CodeRunner> | null;
  listeners: Set<() => void>;
};

const IDLE: RunView = { phase: { state: "idle" }, out: "", runs: 0, open: true };

export const newEntry = (): RunEntry => ({ view: IDLE, handle: null, listeners: new Set() });

/**
 * Runs a card started by something that outlives the card, by what started them. A card that leaves the
 * screen leaves its run going, and one that comes back finds it, still going or ended.
 */
const kept = new Map<string, RunEntry>();

/** What a view's docked pane holds beside its run: which command, whether it is showing, whether it waits for its size to start, and which block draws it in place, if one does. */
export type PaneView = { command: string | null; shown: boolean; waiting: boolean; inline: string | null };

/** `blocks` are the runnable blocks drawn now, each with its command, in the order they mounted. */
export type PaneEntry = { view: PaneView; listeners: Set<() => void>; blocks: Map<string, string> };

const panes = new Map<string, PaneEntry>();

/** The pane a view keeps under `key`, so a view that is left and drawn again finds its pane as it was. */
export const paneFor = (key: string): PaneEntry => {
  let entry = panes.get(key);
  if (entry === undefined) panes.set(key, (entry = { view: { command: null, shown: false, waiting: false, inline: null }, listeners: new Set(), blocks: new Map() }));
  return entry;
};

export function changePane(entry: PaneEntry, next: Partial<PaneView>): void {
  entry.view = { ...entry.view, ...next };
  for (const listener of [...entry.listeners]) listener();
}

export const subscribeToPane = (entry: PaneEntry) => (listener: () => void) => {
  entry.listeners.add(listener);
  return () => void entry.listeners.delete(listener);
};

/** Lets go of every kept run and pane, stopping none: a new Fleet has none of the old one's. A mock window starts here. */
export const forgetKeptRuns = (): void => {
  kept.clear();
  panes.clear();
};

export const entryFor = (key: string): RunEntry => {
  let entry = kept.get(key);
  if (entry === undefined) kept.set(key, (entry = newEntry()));
  return entry;
};

export const subscribeTo = (entry: RunEntry) => (listener: () => void) => {
  entry.listeners.add(listener);
  return () => void entry.listeners.delete(listener);
};

export function change(entry: RunEntry, next: Partial<RunView>): void {
  entry.view = { ...entry.view, ...next };
  for (const listener of [...entry.listeners]) listener();
}

/** Starts `command`, stopping one already going. What the old run says after that is not the new one's. */
export function begin(entry: RunEntry, runner: CodeRunner, command: string, size?: TerminalSize): void {
  entry.handle?.stop();
  const run = entry.view.runs + 1;
  change(entry, { out: "", runs: run, open: true, phase: { state: "running" } });
  entry.handle = runner(
    command,
    (event: CodeRunEvent) => {
      if (entry.view.runs !== run) return;
      if (event.kind === "out") change(entry, { out: entry.view.out + event.text });
      else change(entry, { phase: event.code === null ? { state: "ended", code: null, stopped: event.stopped } : { state: "ended", code: event.code } });
    },
    size,
  );
}
