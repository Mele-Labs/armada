import { describe, expect, it, vi } from "vitest";

import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { runnerOf } from "./session-runner";

const session = { id: "s", terminal: true, commands: [{ name: "update-armada" }], waitingFor: [] } as unknown as Session;

describe("runnerOf", () => {
  it("sends a command the terminal lists to the terminal, not to a shell", () => {
    const run = vi.fn();
    const command = vi.fn();
    const on = vi.fn();
    runnerOf({ run, command } as unknown as SessionsDraft, session)!("/update-armada", on);
    expect(command).toHaveBeenCalledWith("s", "/update-armada");
    expect(run).not.toHaveBeenCalled();
    expect(on).toHaveBeenCalledWith({ kind: "end", code: 0 });
  });

  it("runs anything else in a shell", () => {
    const run = vi.fn(() => ({ stop: () => {} }));
    const command = vi.fn();
    runnerOf({ run, command } as unknown as SessionsDraft, session)!("/tmp/x", () => {});
    expect(run).toHaveBeenCalled();
    expect(command).not.toHaveBeenCalled();
  });
});
