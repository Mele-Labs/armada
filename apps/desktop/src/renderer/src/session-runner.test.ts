import { describe, expect, it, vi } from "vitest";

import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { bangOf, runnerOf } from "./session-runner";

const asking = { id: "i", act: { kind: "run", target: "/update-armada" } };
const session = { id: "s", terminal: true, commands: [{ name: "update-armada" }], waitingFor: [asking] } as unknown as Session;

describe("runnerOf", () => {
  it("sends a command the terminal lists to the terminal, not to a shell, and settles its item", () => {
    const run = vi.fn();
    const command = vi.fn();
    const answerWaiting = vi.fn();
    const on = vi.fn();
    runnerOf({ run, command, answerWaiting } as unknown as SessionsDraft, session)!("/update-armada", on);
    expect(command).toHaveBeenCalledWith("s", "/update-armada");
    expect(run).not.toHaveBeenCalled();
    expect(on).toHaveBeenCalledWith({ kind: "end", code: 0 });
    expect(answerWaiting).toHaveBeenCalledWith("s", "i", { text: "Ran: exit 0" });
  });

  it("runs anything else in a shell", () => {
    const run = vi.fn(() => ({ stop: () => {} }));
    const command = vi.fn();
    runnerOf({ run, command } as unknown as SessionsDraft, session)!("/tmp/x", () => {});
    expect(run).toHaveBeenCalled();
    expect(command).not.toHaveBeenCalled();
  });

  it("leaves the item of a shell command that exits clean to Fleet", () => {
    const answerWaiting = vi.fn();
    const settle = vi.fn();
    const run = vi.fn((_id: string, _command: string, on: (event: unknown) => void) => {
      on({ kind: "end", code: 0 });
      return { stop: () => {} };
    });
    const on = vi.fn();
    const waiting = { ...session, waitingFor: [{ id: "i", act: { kind: "run", target: "make" } }] } as unknown as Session;
    runnerOf({ run, answerWaiting } as unknown as SessionsDraft, waiting, settle)!("make", on);
    expect(on).toHaveBeenCalledWith({ kind: "end", code: 0 });
    expect(answerWaiting).not.toHaveBeenCalled();
    expect(settle).not.toHaveBeenCalled();
  });
});

describe("bangOf", () => {
  it("reads one line after a ! as a shell command", () => {
    expect(bangOf("! rbenv install 4.0.6")).toBe("rbenv install 4.0.6");
    expect(bangOf("  !pwd ")).toBe("pwd");
  });

  it("leaves a bare !, a block and plain words to be sent as a message", () => {
    expect(bangOf("!")).toBeUndefined();
    expect(bangOf("!  ")).toBeUndefined();
    expect(bangOf("!ls\nrm x")).toBeUndefined();
    expect(bangOf("hello !")).toBeUndefined();
  });
});
