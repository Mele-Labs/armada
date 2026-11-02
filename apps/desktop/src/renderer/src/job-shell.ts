// Each of a Job's shells as this window holds it: the run it is, everything it printed, whether its pane is showing
// in each place a Job's dock is drawn, and whether it has a window of its own. **Kept here for the window's life, by run**, so a pane
// mounted late draws the whole of it: main follows a shell once and Fleet replays only to a new
// follower, so a pane put away, or a Job switched away from, would otherwise come back to an empty screen.
// **A shell is kept under a key**: the Job's id for its first shell, which every caller without a tab means, and the Job's id with the
// shell's ID for any other. `keyOf` makes it.

import { useCallback, useMemo, useSyncExternalStore } from "react";
import type { SessionRunMessage } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import { NEW_TERMINAL, type SessionsApi, type TerminalMovedBack } from "../../shared/api/sessions";

type ShellApi = Pick<
  SessionsApi,
  | "openTerminal"
  | "writeSessionRun"
  | "resizeSessionRun"
  | "stopSessionRun"
  | "onSessionRunMessage"
  | "onSessionTerminal"
> &
  // A test's fake may serve no terminal windows, and no output kept in main.
  Partial<
    Pick<
      SessionsApi,
      | "readTerminalSoFar"
      | "openTerminalWindow"
      | "getTerminalWindows"
      | "onTerminalWindows"
      | "moveTerminalBack"
      | "onTerminalMovedBack"
      | "terminalRunning"
    >
  >;

/** Where a shell moved back out of its window goes. */
export type MovedBackTo = TerminalMovedBack["to"];

/** Where a Job's dock is drawn: its page, or a call card's box. Each shows or hides the shell on its own. */
export type ShellPlace = "page" | "card";

export type JobShell = {
  /** Null until Fleet answers with it. */
  runId: string | null;
  /** Everything it printed, as the bytes it printed them. */
  out: string;
  /** Which shell this is: a new one starts the screen over. */
  screen: number;
  ended: boolean;
  /** When it last printed, in ms; absent until it has. Flight control's Terminal tile pulses while this is recent. */
  printedAt?: number;
  /** Opened once the pane has fitted its screen, at that size. */
  waiting: boolean;
};

/** What a window keeps of a shell that prints for hours: the last of it, cut at a line. */
const KEEP = 1 << 20;

const trimmed = (out: string) => {
  if (out.length <= 2 * KEEP) return out;
  const cut = out.indexOf("\n", out.length - KEEP);
  return cut === -1 ? out.slice(-KEEP) : out.slice(cut + 1);
};

/** A shell a pane should be drawing: its ID, null until Fleet has said, and whether it is the Job's first. */
export type ShownShell = {
  terminal: string | null;
  first: boolean;
};

export class JobShells {
  private readonly shells = new Map<string, JobShell>();
  private readonly ofRun = new Map<string, string>();
  private readonly shown = new Set<string>();
  /** Messages for a run nobody has claimed, held while an open is unanswered: its answer may name it. */
  private held: {
    runId: string;
    message: SessionRunMessage;
  }[] = [];
  private opening = 0;
  /** A run this window came to late, with its messages held until main says what it printed so far. */
  private readonly catching = new Map<string, SessionRunMessage[]>();
  private readonly typed = new Map<string, string>();
  private readonly sizes = new Map<
    string,
    {
      cols: number;
      rows: number;
    }
  >();
  private readonly listeners = new Set<() => void>();
  private readonly opened = new Set<(sessionId: string, key: string) => void>();
  private screens = 0;
  /** The Jobs whose shell has a window of its own, as main last said. */
  private windowed: ReadonlySet<string> = new Set();
  private readonly movedBack = new Set<(move: TerminalMovedBack) => void>();
  /** Which Job and which of its shells a key is. The Job's first has no terminal until Fleet has said which it is. */
  private readonly ids = new Map<
    string,
    {
      session: string;
      terminal?: string;
    }
  >();
  /** The ID of the shell each Job's own key holds. */
  private readonly firsts = new Map<string, string>();
  /** The Jobs with a new shell being asked for, whose tab the asker is about to open. */
  private readonly minting = new Map<string, number>();

  constructor(private readonly api: ShellApi) {
    api.onSessionRunMessage((runId, message) => this.message(runId, message));
    api.onSessionTerminal(({ session_id, run_id, change, terminal }) =>
      change === "opened"
        ? this.adopt(this.route(session_id, terminal), run_id, true)
        : this.end(run_id),
    );
    let heard = false;
    api.onTerminalWindows?.((sessionIds) => {
      heard = true;
      this.windowsAre(sessionIds);
    });
    // A push that came first is newer than this answer. A fake that left Sessions out answers a refusal, not a list.
    void api.getTerminalWindows?.().then((sessionIds) => {
      if (!heard && Array.isArray(sessionIds)) this.windowsAre(sessionIds);
    });
    api.onTerminalMovedBack?.((move) => this.movedBack.forEach((one) => one(move)));
  }

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };

  /** Hears each shell Fleet says opened, the agent's among them, by its Job and its key. */
  onOpened(listener: (sessionId: string, key: string) => void): () => void {
    this.opened.add(listener);
    return () => void this.opened.delete(listener);
  }

  /** The key of the Job's shell `terminal`; without one, of the Job's first. */
  keyOf(sessionId: string, terminal?: string): string {
    if (terminal === undefined || this.firsts.get(sessionId) === terminal) return sessionId;
    const key = `${sessionId}#${terminal}`;
    if (!this.ids.has(key))
      this.ids.set(key, {
        session: sessionId,
        terminal,
      });
    return key;
  }

  /** The key a shell Fleet named belongs under: one that is the Job's first, or is opening it, takes the Job's own key. */
  private route(sessionId: string, terminal: string | undefined): string {
    if (terminal === undefined) return sessionId;
    if (this.firsts.get(sessionId) === terminal) return sessionId;
    const own = `${sessionId}#${terminal}`;
    if (this.shells.has(own)) return own;
    const first = this.shells.get(sessionId);
    if (first === undefined || (first.runId === null && !first.ended)) {
      this.learn(sessionId, terminal);
      return sessionId;
    }
    return this.keyOf(sessionId, terminal);
  }

  /** The Job's own key is the shell Fleet says it is. */
  private learn(sessionId: string, terminal: string): void {
    this.firsts.set(sessionId, terminal);
    this.ids.set(sessionId, {
      session: sessionId,
      terminal,
    });
  }

  shell(sessionId: string): JobShell | undefined {
    return this.shells.get(sessionId);
  }

  /** The Job's shells shown in `place` and not ended, by their IDs, for the tabs that should draw them. None while a new shell is asked for, whose tab its asker opens. */
  shownOf(place: ShellPlace, sessionId: string): ShownShell[] {
    if ((this.minting.get(sessionId) ?? 0) > 0) return [];
    const keys = [
      ...this.shells.keys(),
    ].filter(
      (key) =>
        (this.ids.get(key)?.session ?? key) === sessionId &&
        this.shown.has(`${place}:${key}`) &&
        this.shells.get(key)?.ended === false,
    );
    return keys.map((key) => ({
      terminal: this.ids.get(key)?.terminal ?? null,
      first: key === sessionId,
    }));
  }

  /**
   * A new shell in the Job, which Fleet gives an ID. `opened` hears it before the shell is heard of anywhere else, and
   * opens its tab; the Job's agent opening one at the same time has its tab opened after.
   */
  async openNew(sessionId: string, opened: (terminal: string | undefined) => void): Promise<void> {
    this.minting.set(sessionId, (this.minting.get(sessionId) ?? 0) + 1);
    this.opening += 1;
    try {
      const answer = await this.api.openTerminal(sessionId, 80, 24, NEW_TERMINAL);
      // A Fleet that names no shell serves one: its tab draws the Job's first, and so does a refusal, which it then says.
      const terminal = answer.ok ? answer.value.terminal : undefined;
      if (answer.ok) {
        const key = this.route(sessionId, terminal);
        if (this.shells.get(key)?.runId !== answer.value.run_id)
          this.catchUp(key, answer.value.run_id);
      }
      opened(terminal);
    } finally {
      this.opening -= 1;
      if (this.opening === 0) this.held = [];
      this.minting.set(sessionId, (this.minting.get(sessionId) ?? 1) - 1);
      this.changed();
    }
  }

  /** Whether a program is running in the shell, as against its prompt. False for one that is not open, and where Fleet does not say. */
  async busy(key: string): Promise<boolean> {
    const now = this.shells.get(key);
    if (now === undefined || now.ended || this.api.terminalRunning === undefined) return false;
    const id = this.ids.get(key);
    return (await this.api.terminalRunning(id?.session ?? key, id?.terminal)) === true;
  }

  /** Whether the Job's shell is in a window of its own now. */
  hasWindow(sessionId: string): boolean {
    return this.windowed.has(sessionId);
  }

  /** Opens the Job's shell in its window, or brings the window to the front. False where the window serves none. */
  toWindow(sessionId: string): boolean {
    if (this.api.openTerminalWindow === undefined) return false;
    void this.api.openTerminalWindow(sessionId);
    return true;
  }

  /** From the Job's window: closes it, and the document drawing the Job's dock opens the shell `to` there. */
  moveBack(sessionId: string, to: MovedBackTo): void {
    void this.api.moveTerminalBack?.(sessionId, to);
  }

  /** Hears each shell moved back out of its window, for the document that draws that Job's dock. */
  onMovedBack(listener: (move: TerminalMovedBack) => void): () => void {
    this.movedBack.add(listener);
    return () => void this.movedBack.delete(listener);
  }

  isShown(place: ShellPlace, sessionId: string): boolean {
    return this.shown.has(`${place}:${sessionId}`);
  }

  show(place: ShellPlace, sessionId: string, on: boolean): void {
    const key = `${place}:${sessionId}`;
    if (this.shown.has(key) === on) return;
    if (on) this.shown.add(key);
    else this.shown.delete(key);
    this.changed();
  }

  /** A new shell where none is running: it opens once a pane has its size. */
  fresh(sessionId: string): void {
    const now = this.shells.get(sessionId);
    if (now !== undefined && !now.ended) return;
    this.typed.delete(sessionId);
    this.set(sessionId, {
      runId: null,
      out: "",
      screen: (this.screens += 1),
      ended: false,
      waiting: true,
    });
  }

  /**
   * A shell moved back out of its window, held as main says it stands. One that ended there is shown ended, with what main kept
   * of it, rather than opened again; any other is `fresh`'s, handed the running one where there is one.
   */
  movedIn(sessionId: string, shell: TerminalMovedBack["shell"]): void {
    if (shell === undefined || !shell.ended) return this.fresh(sessionId);
    // Held already: ended, or its end is on its way.
    if (this.shells.get(sessionId)?.runId === shell.runId) return;
    const { runId } = shell;
    this.set(sessionId, {
      runId,
      out: "",
      screen: (this.screens += 1),
      ended: true,
      waiting: false,
    });
    this.ofRun.set(runId, sessionId);
    void (this.api.readTerminalSoFar?.(runId) ?? Promise.resolve(null)).then((soFar) => {
      const now = this.shells.get(sessionId);
      if (soFar !== null && now?.runId === runId)
        this.set(sessionId, {
          ...now,
          out: trimmed(soFar),
        });
    });
  }

  /** Shows the pane, with a new shell where none is running. */
  want(place: ShellPlace, sessionId: string): void {
    this.fresh(sessionId);
    this.show(place, sessionId, true);
  }

  /** The pane fitted its screen: the size a waiting shell opens at, else the size a running one is told. */
  sized(sessionId: string, cols: number, rows: number): void {
    this.sizes.set(sessionId, {
      cols,
      rows,
    });
    const now = this.shells.get(sessionId);
    if (now === undefined || now.ended) return;
    if (now.waiting) void this.open(sessionId, cols, rows);
    else if (now.runId !== null) void this.api.resizeSessionRun(now.runId, cols, rows);
  }

  write(sessionId: string, data: string): void {
    const now = this.shells.get(sessionId);
    if (now === undefined || now.ended) return;
    if (now.runId === null) this.typed.set(sessionId, (this.typed.get(sessionId) ?? "") + data);
    else void this.api.writeSessionRun(now.runId, data);
  }

  stop(sessionId: string): void {
    const runId = this.shells.get(sessionId)?.runId;
    if (runId !== null && runId !== undefined) void this.api.stopSessionRun(runId);
  }

  /** Its tab is closed: the shell is put away first, so nothing opens a tab for it while it ends. */
  closeShell(key: string): void {
    this.show("page", key, false);
    this.stop(key);
  }

  private async open(sessionId: string, cols: number, rows: number): Promise<void> {
    const asked = this.shells.get(sessionId)!;
    this.set(sessionId, {
      ...asked,
      waiting: false,
    });
    this.opening += 1;
    const id = this.ids.get(sessionId);
    const answer = await this.api.openTerminal(id?.session ?? sessionId, cols, rows, id?.terminal);
    this.opening -= 1;
    const now = this.shells.get(sessionId);
    // Fleet's opening, heard first, already holds it; or a newer shell took its place.
    if (now?.screen === asked.screen && now.runId === null) {
      if (answer.ok) {
        if (answer.value.terminal !== undefined && id?.terminal === undefined)
          this.learn(sessionId, answer.value.terminal);
        this.catchUp(sessionId, answer.value.run_id);
      } else
        this.set(sessionId, {
          ...now,
          out: `${refusalWords(answer.outcome)}\r\n`,
          ended: true,
        });
    }
    if (this.opening === 0) this.held = [];
  }

  /**
   * Fleet handed this window a run it was not following: another window opened it, and Fleet replays only to main.
   * Main keeps each message before it sends it, so what it answers holds every output held here; only the rest is applied.
   */
  private catchUp(sessionId: string, runId: string): void {
    const early = this.held.filter((it) => it.runId === runId).map((it) => it.message);
    this.held = this.held.filter((it) => it.runId !== runId);
    this.catching.set(runId, early);
    this.adopt(sessionId, runId, false);
    const read = this.api.readTerminalSoFar?.(runId) ?? Promise.resolve(null);
    void read.then((soFar) => {
      const caught = this.catching.get(runId) ?? [];
      this.catching.delete(runId);
      const now = this.shells.get(sessionId);
      if (now?.runId !== runId) return;
      if (soFar !== null)
        this.set(sessionId, {
          ...now,
          out: trimmed(soFar),
        });
      for (const message of caught)
        if (soFar === null || message.kind !== "output") this.message(runId, message);
    });
  }

  /** A shell's run, from Fleet's answer or its event: the one opening takes it, else it is a new shell. */
  private adopt(sessionId: string, runId: string, told: boolean): void {
    const now = this.shells.get(sessionId);
    if (now?.runId !== runId) {
      const opening = now !== undefined && now.runId === null && !now.ended;
      this.set(
        sessionId,
        opening
          ? {
              ...now,
              runId,
              waiting: false,
            }
          : {
              runId,
              out: "",
              screen: (this.screens += 1),
              ended: false,
              waiting: false,
            },
      );
      this.ofRun.set(runId, sessionId);
      for (const one of this.held.filter((it) => it.runId === runId))
        this.message(one.runId, one.message);
      this.held = this.held.filter((it) => it.runId !== runId);
      const typed = this.typed.get(sessionId);
      this.typed.delete(sessionId);
      if (typed !== undefined) void this.api.writeSessionRun(runId, typed);
      const size = this.sizes.get(sessionId);
      if (opening && size !== undefined)
        void this.api.resizeSessionRun(runId, size.cols, size.rows);
    }
    if (told)
      this.opened.forEach((one) => one(this.ids.get(sessionId)?.session ?? sessionId, sessionId));
  }

  private message(runId: string, message: SessionRunMessage): void {
    const caught = this.catching.get(runId);
    if (caught !== undefined) {
      caught.push(message);
      return;
    }
    const sessionId = this.ofRun.get(runId);
    if (sessionId === undefined) {
      if (this.opening > 0)
        this.held.push({
          runId,
          message,
        });
      return;
    }
    const now = this.shells.get(sessionId);
    if (now?.runId !== runId) return;
    if (message.kind === "output" && message.text !== "")
      this.set(sessionId, {
        ...now,
        out: trimmed(now.out + message.text),
        printedAt: Date.now(),
      });
    else if (message.kind === "exited") this.end(runId);
  }

  private end(runId: string): void {
    const sessionId = this.ofRun.get(runId);
    const now = sessionId === undefined ? undefined : this.shells.get(sessionId);
    if (sessionId === undefined || now?.runId !== runId || now.ended) return;
    this.set(sessionId, {
      ...now,
      ended: true,
    });
  }

  private windowsAre(sessionIds: readonly string[]): void {
    this.windowed = new Set(sessionIds);
    this.changed();
  }

  private set(sessionId: string, shell: JobShell): void {
    this.shells.set(sessionId, shell);
    this.changed();
  }

  private changed(): void {
    this.listeners.forEach((one) => one());
  }
}

const SHELLS = new WeakMap<object, JobShells>();

/** The window's shells, over its `window.armada`. Made once, and listening from then on. */
export function shellsOf(api: ShellApi): JobShells | null {
  // A window whose Fleet serves no shells: a test's fake that left Sessions out.
  if (typeof api.onSessionTerminal !== "function" || typeof api.onSessionRunMessage !== "function")
    return null;
  let made = SHELLS.get(api);
  if (made === undefined) {
    made = new JobShells(api);
    SHELLS.set(api, made);
  }
  return made;
}

/** One Job's shell, drawn again as it changes: the side panel's, which shows it while it is open. */
export function useShellOf(shells: JobShells | null, sessionId: string): JobShell | undefined {
  const subscribe = useCallback(
    (listener: () => void) => shells?.subscribe(listener) ?? (() => undefined),
    [
      shells,
    ],
  );
  return useSyncExternalStore(subscribe, () => shells?.shell(sessionId));
}

/** One Job's shell, whether its pane shows in `place`, and whether it is in a window of its own, drawn again as any changes. */
export function useJobShell(
  shells: JobShells | null,
  place: ShellPlace,
  sessionId: string,
): {
  shell: JobShell | undefined;
  shown: boolean;
  windowed: boolean;
} {
  const subscribe = useCallback(
    (listener: () => void) => shells?.subscribe(listener) ?? (() => undefined),
    [
      shells,
    ],
  );
  const shell = useSyncExternalStore(subscribe, () => shells?.shell(sessionId));
  const shown = useSyncExternalStore(subscribe, () => shells?.isShown(place, sessionId) ?? false);
  const windowed = useSyncExternalStore(subscribe, () => shells?.hasWindow(sessionId) ?? false);
  return {
    shell,
    shown,
    windowed,
  };
}

/** The Job's shells shown in `place` and not ended, drawn again as they change. */
export function useShownShells(
  shells: JobShells | null,
  place: ShellPlace,
  sessionId: string,
): ShownShell[] {
  const subscribe = useCallback(
    (listener: () => void) => shells?.subscribe(listener) ?? (() => undefined),
    [
      shells,
    ],
  );
  const said = useSyncExternalStore(subscribe, () =>
    JSON.stringify(shells?.shownOf(place, sessionId) ?? []),
  );
  return useMemo(
    () => JSON.parse(said) as ShownShell[],
    [
      said,
    ],
  );
}
