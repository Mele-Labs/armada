import type { SessionRunMessage } from "@armada/protocol";
import { describe, expect, it, vi } from "vitest";

import type { JobTerminalChange, SessionActed } from "../../shared/api/sessions";
import { JobShells } from "./job-shell";

/** A promise the test settles when it chooses, so a message can land before or after an answer. */
function later<T>() {
  let settle!: (value: T) => void;
  const promise = new Promise<T>((done) => (settle = done));
  return { promise, settle };
}

/** A window's api with Fleet's answers held back: the open's, and main's read of what the shell printed so far. */
function fakeApi() {
  const opened = later<SessionActed<{ run_id: string }>>();
  const soFar = later<string | null>();
  let ran: (runId: string, message: SessionRunMessage) => void = () => undefined;
  let told: (terminal: JobTerminalChange) => void = () => undefined;
  const api = {
    openTerminal: vi.fn(() => opened.promise),
    readTerminalSoFar: vi.fn(() => soFar.promise),
    writeSessionRun: vi.fn(async () => ({ ok: true as const, value: { run_id: "r1" } })),
    resizeSessionRun: vi.fn(async () => ({ ok: true as const, value: { run_id: "r1" } })),
    stopSessionRun: vi.fn(async () => ({ ok: true as const, value: { run_id: "r1" } })),
    onSessionRunMessage: (on: typeof ran) => ((ran = on), () => undefined),
    onSessionTerminal: (on: typeof told) => ((told = on), () => undefined),
  };
  const print = (text: string) => ran("r1", { kind: "output", text });
  return { api, opened, soFar, print, end: () => ran("r1", { kind: "exited", code: 0 }), tell: (terminal: JobTerminalChange) => told(terminal) };
}

const settled = () => new Promise((done) => setTimeout(done, 0));

describe("a window that comes to a running shell", () => {
  it("draws what it printed before, then what comes after, nothing twice", async () => {
    const { api, opened, soFar, print } = fakeApi();
    const shells = new JobShells(api);
    shells.want("page", "a");
    shells.sized("a", 80, 24);
    print("early ");
    opened.settle({ ok: true, value: { run_id: "r1" } });
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
    opened.settle({ ok: true, value: { run_id: "r1" } });
    await settled();
    print("bye\r\n");
    end();
    soFar.settle("bye\r\n");
    await settled();
    expect(shells.shell("a")).toMatchObject({ out: "bye\r\n", ended: true });
  });

  it("draws what it heard where main holds nothing for the run", async () => {
    const { api, opened, soFar, print } = fakeApi();
    const shells = new JobShells(api);
    shells.want("page", "a");
    shells.sized("a", 80, 24);
    print("early ");
    opened.settle({ ok: true, value: { run_id: "r1" } });
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
    tell({ session_id: "a", run_id: "r1", change: "opened" });
    print("ls\r\n");
    opened.settle({ ok: true, value: { run_id: "r1" } });
    await settled();
    expect(api.readTerminalSoFar).not.toHaveBeenCalled();
    expect(shells.shell("a")?.out).toBe("ls\r\n");
  });
});
