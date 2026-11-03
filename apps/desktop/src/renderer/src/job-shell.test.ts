import type { SessionRunMessage } from "@armada/protocol";
import { describe, expect, it, vi } from "vitest";

import type { JobTerminals } from "@armada/protocol";
import type { JobTerminalChange, SessionActed, TerminalWindowId } from "../../shared/api/sessions";
import { JobShells } from "./job-shell";

/** A promise the test settles when it chooses, so a message can land before or after an answer. */
function later<T>() {
  let settle!: (value: T) => void;
  const promise = new Promise<T>((done) => (settle = done));
  return {
    promise,
    settle,
  };
}

/** A window's api with Fleet's answers held back: the open's, and main's read of what the shell printed so far. */
function fakeApi() {
  const opened =
    later<
      SessionActed<{
        run_id: string;
      }>
    >();
  const soFar = later<string | null>();
  let ran: (runId: string, message: SessionRunMessage) => void = () => undefined;
  let told: (terminal: JobTerminalChange) => void = () => undefined;
  const api = {
    openTerminal: vi.fn(() => opened.promise),
    readTerminalSoFar: vi.fn(() => soFar.promise),
    writeSessionRun: vi.fn(async () => ({
      ok: true as const,
      value: {
        run_id: "r1",
      },
    })),
    resizeSessionRun: vi.fn(async () => ({
      ok: true as const,
      value: {
        run_id: "r1",
      },
    })),
    stopSessionRun: vi.fn(async () => ({
      ok: true as const,
      value: {
        run_id: "r1",
      },
    })),
    onSessionRunMessage: (on: typeof ran) => ((ran = on), () => undefined),
    onSessionTerminal: (on: typeof told) => ((told = on), () => undefined),
  };
  const print = (text: string) =>
    ran("r1", {
      kind: "output",
      text,
    });
  return {
    api,
    opened,
    soFar,
    print,
    end: () =>
      ran("r1", {
        kind: "exited",
        code: 0,
        stopped: false,
      }),
    tell: (terminal: JobTerminalChange) => told(terminal),
  };
}

const settled = () => new Promise((done) => setTimeout(done, 0));

describe("a window that comes to a running shell", () => {
  it("draws what it printed before, then what comes after, nothing twice", async () => {
    const { api, opened, soFar, print } = fakeApi();
    const shells = new JobShells(api);
    shells.want("page", "a");
    shells.sized("a", 80, 24);
    print("early ");
    opened.settle({
      ok: true,
      value: {
        run_id: "r1",
      },
    });
    await settled();
    expect(api.readTerminalSoFar).toHaveBeenCalledWith("r1");
    print("meanwhile ");
    soFar.settle("before early meanwhile ");
    await settled();
    print("after");
    expect(shells.shell("a")?.out).toBe("before early meanwhile after");
  });

  it("ends a shell whose end arrived while main's read was unanswered", async () => {
    const { api, opened, soFar, print, end } = fakeApi();
    const shells = new JobShells(api);
    shells.want("page", "a");
    shells.sized("a", 80, 24);
    opened.settle({
      ok: true,
      value: {
        run_id: "r1",
      },
    });
    await settled();
    print("bye\r\n");
    end();
    soFar.settle("bye\r\n");
    await settled();
    expect(shells.shell("a")).toMatchObject({
      out: "bye\r\n",
      ended: true,
    });
  });

  it("draws what it heard where main holds nothing for the run", async () => {
    const { api, opened, soFar, print } = fakeApi();
    const shells = new JobShells(api);
    shells.want("page", "a");
    shells.sized("a", 80, 24);
    print("early ");
    opened.settle({
      ok: true,
      value: {
        run_id: "r1",
      },
    });
    await settled();
    print("meanwhile");
    soFar.settle(null);
    await settled();
    expect(shells.shell("a")?.out).toBe("early meanwhile");
  });

  it("asks main for nothing where Fleet's opening reached the window before the answer", async () => {
    const { api, opened, print, tell } = fakeApi();
    const shells = new JobShells(api);
    shells.want("page", "a");
    shells.sized("a", 80, 24);
    tell({
      session_id: "a",
      terminal: "t1",
      run_id: "r1",
      change: "opened",
    });
    print("ls\r\n");
    opened.settle({
      ok: true,
      value: {
        run_id: "r1",
      },
    });
    await settled();
    expect(api.readTerminalSoFar).not.toHaveBeenCalled();
    expect(shells.shell("a")?.out).toBe("ls\r\n");
  });
});

describe("a shell moved back out of its window", () => {
  it("is held ended with what main kept of it where it ended there, and no shell is opened", async () => {
    const { api, soFar } = fakeApi();
    const shells = new JobShells(api);
    shells.movedIn("a", {
      runId: "r9",
      ended: true,
    });
    expect(shells.shell("a")).toMatchObject({
      runId: "r9",
      ended: true,
      waiting: false,
    });
    soFar.settle("~ $ exit\r\n");
    await settled();
    expect(api.readTerminalSoFar).toHaveBeenCalledWith("r9");
    expect(shells.shell("a")?.out).toBe("~ $ exit\r\n");
    shells.sized("a", 80, 24);
    expect(api.openTerminal).not.toHaveBeenCalled();
  });

  it("waits for its pane's size and is handed the running one where it still runs", () => {
    const { api } = fakeApi();
    const shells = new JobShells(api);
    shells.movedIn("a", {
      runId: "r1",
      ended: false,
    });
    expect(shells.shell("a")).toMatchObject({
      runId: null,
      waiting: true,
    });
    shells.sized("a", 80, 24);
    expect(api.openTerminal).toHaveBeenCalledWith("a", 80, 24, undefined);
  });
});

describe("a shell's activity", () => {
  it("stamps each output and nothing else, so Flight control can tell a shell is running something", async () => {
    const { api, opened, soFar, print } = fakeApi();
    const shells = new JobShells(api);
    shells.want("page", "a");
    shells.sized("a", 80, 24);
    opened.settle({
      ok: true,
      value: {
        run_id: "r1",
      },
    });
    await settled();
    soFar.settle(null);
    await settled();
    expect(shells.shell("a")?.printedAt).toBeUndefined();
    const before = Date.now();
    print("ls\r\n");
    expect(shells.shell("a")?.printedAt).toBeGreaterThanOrEqual(before);
  });
});

/** A Fleet with several shells: an open names its terminal or mints the next, each run prints on its own, and its answers are held until the test settles them. */
function severalApi(
  windows: readonly TerminalWindowId[] = [],
  listed: SessionActed<JobTerminals> = {
    ok: true,
    value: {
      terminals: [],
    },
  },
) {
  let ran: (runId: string, message: SessionRunMessage) => void = () => undefined;
  let told: (terminal: JobTerminalChange) => void = () => undefined;
  let minted = 0;
  const held: (() => void)[] = [];
  let holding = false;
  const api = {
    openTerminal: vi.fn(
      async (
        _session: string,
        _cols?: number,
        _rows?: number,
        which?: {
          terminal?: string;
          new?: boolean;
        },
      ) => {
        const terminal = which?.new === true ? `t${(minted += 1)}` : (which?.terminal ?? "t1");
        if (holding) await new Promise<void>((done) => held.push(done));
        return {
          ok: true as const,
          value: {
            run_id: `r${terminal.slice(1)}`,
            terminal,
          },
        };
      },
    ),
    readTerminalSoFar: vi.fn(async (_runId: string): Promise<string | null> => null),
    writeSessionRun: vi.fn(async () => ({
      ok: true as const,
      value: {
        run_id: "r1",
      },
    })),
    resizeSessionRun: vi.fn(async () => ({
      ok: true as const,
      value: {
        run_id: "r1",
      },
    })),
    stopSessionRun: vi.fn(async (runId: string) => ({
      ok: true as const,
      value: {
        run_id: runId,
      },
    })),
    listTerminals: vi.fn(async () => listed),
    renameTerminal: vi.fn(async () => ({
      ok: false as const,
      outcome: {
        ok: false as const,
        kind: "refused" as const,
        code: "fleet.not_found",
        words: "no terminal",
      } as never,
    })),
    onSessionRunMessage: (on: typeof ran) => ((ran = on), () => undefined),
    onSessionTerminal: (on: typeof told) => ((told = on), () => undefined),
    getTerminalWindows: vi.fn(async () => windows),
    openTerminalWindow: vi.fn(async () => undefined),
    moveTerminalBack: vi.fn(async () => undefined),
  };
  return {
    api,
    /** Holds every open's answer until `answer`. */
    hold: () => void (holding = true),
    answer: () => {
      holding = false;
      held.splice(0).forEach((done) => done());
    },
    print: (runId: string, text: string) =>
      ran(runId, {
        kind: "output",
        text,
      }),
    tell: (terminal: JobTerminalChange) => told(terminal),
  };
}

/** A tab drawn: its shell is wanted and its pane has fitted. */
async function drawn(shells: JobShells, key: string) {
  shells.want("page", key);
  shells.sized(key, 80, 24);
  await settled();
}

describe("a Job's several terminals", () => {
  it("keeps what each printed apart, one store for each", async () => {
    const { api, print } = severalApi();
    const shells = new JobShells(api);
    const [t1, t2] = [
      shells.keyOf("a", "t1"),
      shells.keyOf("a", "t2"),
    ];
    await drawn(shells, t1);
    await drawn(shells, t2);
    print("r1", "one");
    print("r2", "two");
    print("r1", " more");
    expect(shells.shell(t1)?.out).toBe("one more");
    expect(shells.shell(t2)?.out).toBe("two");
    expect(shells.shell(t1)?.runId).toBe("r1");
    expect(shells.shell(t2)?.runId).toBe("r2");
  });

  it("opens a tab's own terminal by its ID after a reload, and is replayed what main kept of it", async () => {
    const { api } = severalApi();
    api.readTerminalSoFar.mockResolvedValueOnce("before the reload");
    // A window just loaded: the store holds nothing, and the tab names the terminal it held.
    const shells = new JobShells(api);
    const key = shells.keyOf("a", "t2");
    await drawn(shells, key);
    expect(api.openTerminal).toHaveBeenCalledTimes(1);
    expect(api.openTerminal).toHaveBeenCalledWith("a", 80, 24, {
      terminal: "t2",
    });
    expect(api.readTerminalSoFar).toHaveBeenCalledWith("r2");
    expect(shells.shell(key)).toMatchObject({
      runId: "r2",
      out: "before the reload",
      ended: false,
    });
    expect(shells.terminalOf(key)).toBe("t2");
  });

  it("opens a tab with no terminal yet as the Job's first, and learns which it is", async () => {
    const { api } = severalApi();
    const shells = new JobShells(api);
    await drawn(shells, "a");
    expect(api.openTerminal).toHaveBeenCalledWith("a", 80, 24, undefined);
    expect(shells.terminalOf("a")).toBe("t1");
    expect(shells.keyOf("a", "t1")).toBe("a");
  });

  it("folds a tab that held the Job's first by its ID into the Job's own key once the first is known, and draws it still", async () => {
    const { api, print } = severalApi();
    const shells = new JobShells(api);
    const tab = shells.keyOf("a", "t1");
    expect(tab).toBe("a#t1");
    await drawn(shells, tab);
    print("r1", "ls\r\n");
    await drawn(shells, "a");
    expect(shells.keyOf("a", "t1")).toBe("a");
    expect(shells.shell("a")).toMatchObject({
      runId: "r1",
      ended: false,
    });
    print("r1", "more");
    expect(shells.shell("a")?.out).toContain("more");
  });

  it("opens a new one by asking Fleet for the next, which names it", async () => {
    const { api } = severalApi();
    const shells = new JobShells(api);
    const opened = vi.fn();
    await shells.openNew("a", opened);
    await shells.openNew("a", opened);
    expect(api.openTerminal.mock.calls.map((call) => call[3])).toEqual([
      {
        new: true,
      },
      {
        new: true,
      },
    ]);
    expect(opened.mock.calls).toEqual([
      [
        "t1",
      ],
      [
        "t2",
      ],
    ]);
  });
});

describe("closing a terminal's tab", () => {
  it("ends only that shell", async () => {
    const { api } = severalApi();
    const shells = new JobShells(api);
    const [t1, t2] = [
      shells.keyOf("a", "t1"),
      shells.keyOf("a", "t2"),
    ];
    await drawn(shells, t1);
    await drawn(shells, t2);
    shells.closeShell(t2);
    expect(api.stopSessionRun.mock.calls).toEqual([
      [
        "r2",
      ],
    ]);
    expect(shells.isShown("page", t1)).toBe(true);
    expect(shells.isShown("page", t2)).toBe(false);
  });

  it("stops a shell that was still opening, as soon as Fleet says which run it is", async () => {
    const { api, hold, answer } = severalApi();
    const shells = new JobShells(api);
    const key = shells.keyOf("a", "t2");
    hold();
    shells.want("page", key);
    shells.sized(key, 80, 24);
    await settled();
    shells.closeShell(key);
    expect(api.stopSessionRun).not.toHaveBeenCalled();
    answer();
    await settled();
    expect(api.stopSessionRun.mock.calls).toEqual([
      [
        "r2",
      ],
    ]);
    expect(shells.shell(key)).toMatchObject({
      ended: true,
    });
  });

  it("stops it when Fleet's event naming the run comes before the answer, and opens no tab for it", async () => {
    const { api, hold, answer, tell } = severalApi();
    const shells = new JobShells(api);
    const heard = vi.fn();
    shells.onOpened(heard);
    const key = shells.keyOf("a", "t2");
    hold();
    shells.want("page", key);
    shells.sized(key, 80, 24);
    await settled();
    shells.closeShell(key);
    tell({
      session_id: "a",
      terminal: "t2",
      run_id: "r2",
      change: "opened",
    });
    answer();
    await settled();
    expect(api.stopSessionRun.mock.calls).toEqual([
      [
        "r2",
      ],
    ]);
    expect(heard).not.toHaveBeenCalled();
    expect(shells.isShown("page", key)).toBe(false);
    expect(shells.shell(key)).toMatchObject({
      ended: true,
    });
  });

  it("opens nothing for a shell whose pane never fitted before the tab closed", async () => {
    const { api } = severalApi();
    const shells = new JobShells(api);
    const key = shells.keyOf("a", "t2");
    shells.want("page", key);
    shells.closeShell(key);
    shells.sized(key, 80, 24);
    await settled();
    expect(api.openTerminal).not.toHaveBeenCalled();
    expect(api.stopSessionRun).not.toHaveBeenCalled();
  });

  const listing = (running: boolean): SessionActed<JobTerminals> => ({
    ok: true,
    value: {
      terminals: [
        {
          terminal: "t1",
          name: "Terminal 1",
          run_id: "r1",
          running: false,
        },
        {
          terminal: "t2",
          name: "Terminal 2",
          run_id: "r2",
          running,
        },
      ],
    },
  });

  it("asks only for a shell with a program running in it, as Fleet's list of the Job's terminals says", async () => {
    const idle = severalApi([], listing(false));
    const quiet = new JobShells(idle.api);
    await drawn(quiet, quiet.keyOf("a", "t1"));
    await drawn(quiet, quiet.keyOf("a", "t2"));
    expect(await quiet.busy(quiet.keyOf("a", "t2"))).toBe(false);
    const busy = severalApi([], listing(true));
    const loud = new JobShells(busy.api);
    await drawn(loud, loud.keyOf("a", "t1"));
    await drawn(loud, loud.keyOf("a", "t2"));
    expect(await loud.busy(loud.keyOf("a", "t2"))).toBe(true);
    // The other terminal's program is not this one's.
    expect(await loud.busy(loud.keyOf("a", "t1"))).toBe(false);
    expect(busy.api.listTerminals).toHaveBeenCalledWith("a");
  });

  it("asks where the list could not be read, rather than losing a program", async () => {
    const { api } = severalApi([], {
      ok: false,
      outcome: {
        ok: false,
        kind: "unreachable",
      } as never,
    });
    const shells = new JobShells(api);
    const key = shells.keyOf("a", "t2");
    await drawn(shells, key);
    expect(await shells.busy(key)).toBe(true);
  });

  it("does not ask for a shell that has ended or was never opened", async () => {
    const { api } = severalApi([], {
      ok: false,
      outcome: {
        ok: false,
        kind: "unreachable",
      } as never,
    });
    const shells = new JobShells(api);
    expect(await shells.busy(shells.keyOf("a", "t3"))).toBe(false);
    expect(api.listTerminals).not.toHaveBeenCalled();
  });
});

describe("naming a terminal", () => {
  it("tells Fleet the name, by the Job and the terminal's ID, and says nothing where Fleet refuses", async () => {
    const { api } = severalApi();
    const shells = new JobShells(api);
    const key = shells.keyOf("a", "t2");
    await drawn(shells, key);
    shells.rename(key, "dev");
    await settled();
    expect(api.renameTerminal).toHaveBeenCalledWith("a", "t2", "dev");
    shells.rename(key, "");
    expect(api.renameTerminal).toHaveBeenLastCalledWith("a", "t2", "");
  });

  it("tells Fleet nothing for a shell whose terminal is not yet known", () => {
    const { api } = severalApi();
    const shells = new JobShells(api);
    shells.rename("a", "dev");
    expect(api.renameTerminal).not.toHaveBeenCalled();
  });
});

describe("a terminal the Job's agent opens", () => {
  it("is heard with its own key and shown, not as the Job's first", async () => {
    const { api, tell } = severalApi();
    const shells = new JobShells(api);
    const heard = vi.fn();
    shells.onOpened(heard);
    await drawn(shells, "a");
    tell({
      session_id: "a",
      terminal: "t2",
      run_id: "r2",
      change: "opened",
    });
    expect(heard).toHaveBeenCalledWith("a", "a#t2");
    shells.show("page", "a#t2", true);
    expect(shells.shownOf("page", "a")).toEqual([
      {
        terminal: "t1",
        first: true,
      },
      {
        terminal: "t2",
        first: false,
      },
    ]);
  });

  it("is not offered a tab while the person's new one is being asked for, which opens its own", async () => {
    const { api, tell, hold, answer } = severalApi();
    const shells = new JobShells(api);
    hold();
    const opening = shells.openNew("a", () => undefined);
    await settled();
    tell({
      session_id: "a",
      terminal: "t1",
      run_id: "r1",
      change: "opened",
    });
    shells.show("page", "a", true);
    expect(shells.shownOf("page", "a")).toEqual([]);
    answer();
    await opening;
    expect(shells.shownOf("page", "a")).toEqual([
      {
        terminal: "t1",
        first: true,
      },
    ]);
  });
});

describe("a terminal's window", () => {
  const window2: TerminalWindowId = {
    sessionId: "a",
    terminal: "t2",
  };

  it("is one for each terminal: a window for the second leaves the first docked", async () => {
    const { api } = severalApi([
      window2,
    ]);
    const shells = new JobShells(api);
    const [t1, t2] = [
      shells.keyOf("a", "t1"),
      shells.keyOf("a", "t2"),
    ];
    await settled();
    expect(shells.hasWindow(t2)).toBe(true);
    expect(shells.hasWindow(t1)).toBe(false);
    expect(shells.hasWindow("a")).toBe(false);
  });

  it("opens and moves back by the terminal, and the Job's first with no ID by the Job", async () => {
    const { api } = severalApi();
    const shells = new JobShells(api);
    const t2 = shells.keyOf("a", "t2");
    expect(shells.toWindow(t2)).toBe(true);
    expect(api.openTerminalWindow).toHaveBeenLastCalledWith("a", "t2");
    shells.moveBack(t2, "docked");
    expect(api.moveTerminalBack).toHaveBeenLastCalledWith("a", "docked", "t2");
    shells.toWindow("a");
    expect(api.openTerminalWindow).toHaveBeenLastCalledWith("a", undefined);
  });

  it("is the Job's first terminal's when it has no ID", async () => {
    const { api } = severalApi([
      {
        sessionId: "a",
      },
    ]);
    const shells = new JobShells(api);
    await settled();
    expect(shells.hasWindow("a")).toBe(true);
    expect(shells.hasWindow(shells.keyOf("a", "t2"))).toBe(false);
  });
});
