import { describe, expect, it, vi } from "vitest";

import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { bangOf, hostedCommands, opensMcp, runnerOf } from "./session-runner";

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

describe("opensMcp", () => {
  const hosted = { id: "h" } as unknown as Session;

  it("reads /mcp alone, sent with nothing attached, in a hosted Job as opening its MCP servers", () => {
    expect(opensMcp(hosted, "/mcp", 0)).toBe(true);
    expect(opensMcp(hosted, "  /mcp \n", 0)).toBe(true);
  });

  it("leaves it a message in a terminal Job, with an attachment, or with more words", () => {
    expect(opensMcp(session, "/mcp", 0)).toBe(false);
    expect(opensMcp(hosted, "/mcp", 1)).toBe(false);
    expect(opensMcp(hosted, "/mcp list", 0)).toBe(false);
    expect(opensMcp(hosted, "/mcpx", 0)).toBe(false);
    expect(opensMcp(hosted, "use /mcp", 0)).toBe(false);
  });
});

describe("hostedCommands", () => {
  it("lists /mcp once beside the CLI's own, as the same list on every render", () => {
    const listed = [{ name: "review", says: "" }];
    expect(hostedCommands(listed).map((one) => one.name)).toEqual(["review", "mcp"]);
    expect(hostedCommands(listed)).toBe(hostedCommands(listed));
    expect(hostedCommands([{ name: "mcp", says: "" }]).filter((one) => one.name === "mcp")).toHaveLength(1);
  });
});
