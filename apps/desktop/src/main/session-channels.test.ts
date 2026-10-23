// Every answer a card offers reaches Fleet through main. A list kept here missed `always_allow`
// and `always_block`, and the press came back as Fleet not connected (16 Oct 2026).

import { HELM_CALL_ANSWERS, NOTICES, type AnswerSessionAsk } from "@armada/protocol";
import { describe, expect, it, vi } from "vitest";

import { CHANNELS } from "../shared/bridge";
import { handleSessions } from "./session-channels";

type Handler = (event: unknown, ...args: unknown[]) => unknown;

function wired() {
  const handlers = new Map<string, Handler>();
  const answer = vi.fn(async () => ({ ok: true }));
  const muteNotice = vi.fn(async () => ({ ok: true }));
  const ipc = { handle: (channel: string, handler: Handler) => handlers.set(channel, handler) };
  const connection = () => ({ sessions: { answer, muteNotice } });
  handleSessions({ ipc, connection, windowIdOf: () => undefined, pages: {} } as unknown as Parameters<typeof handleSessions>[0]);
  const send = (body: AnswerSessionAsk) => handlers.get(CHANNELS.answerSessionAsk)?.({}, body);
  const mute = (body: unknown) => handlers.get(CHANNELS.muteNotice)?.({}, body);
  return { send, answer, mute, muteNotice };
}

describe("answering a session's ask", () => {
  it("sends every answer the protocol names, the folder with it", async () => {
    const { send, answer } = wired();
    for (const said of HELM_CALL_ANSWERS) {
      await send({ session_id: "s", call: "helm-1", answer: said, folder: "/Users/user/Documents" } as AnswerSessionAsk);
    }
    expect(answer).toHaveBeenCalledTimes(HELM_CALL_ANSWERS.length);
    expect(answer).toHaveBeenLastCalledWith(expect.objectContaining({ answer: "always_block", folder: "/Users/user/Documents" }));
  });

  it("sends nothing for a word that is no answer, and says so rather than Fleet not connected", async () => {
    const { send, answer } = wired();
    const said = await send({ session_id: "s", call: "helm-1", answer: "maybe" } as unknown as AnswerSessionAsk);
    expect(answer).not.toHaveBeenCalled();
    expect(said).toEqual({ ok: false, outcome: { ok: false, why: "not_understood" } });
  });
});

function terminalWired() {
  const handlers = new Map<string, Handler>();
  const ipc = { handle: (channel: string, handler: Handler) => handlers.set(channel, handler) };
  const soFar = vi.fn((runId: string) => (runId === "r1" ? "~ $ ls\r\n" : null));
  const windows = { show: vi.fn(), close: vi.fn(), sessionIds: () => ["a"] };
  const movedBack = vi.fn();
  handleSessions({ ipc, connection: () => ({ sessions: { soFar } }), windowIdOf: () => undefined, pages: {}, terminals: { windows, movedBack } } as unknown as Parameters<typeof handleSessions>[0]);
  const call = (channel: string, ...args: unknown[]) => handlers.get(channel)?.({}, ...args);
  return { call, windows, movedBack, soFar };
}

describe("a Job's terminal window", () => {
  it("opens a Job's window by its id, and nothing for an id that is not one", async () => {
    const { call, windows } = terminalWired();
    await call(CHANNELS.openTerminalWindow, "a");
    await call(CHANNELS.openTerminalWindow, "");
    await call(CHANNELS.openTerminalWindow, 7);
    expect(windows.show.mock.calls).toEqual([["a"]]);
    expect(await call(CHANNELS.getTerminalWindows)).toEqual(["a"]);
  });

  it("moves a shell back into Bridge, closing its window and telling every window where it went", async () => {
    const { call, windows, movedBack } = terminalWired();
    await call(CHANNELS.moveTerminalBack, "a", "side panel");
    await call(CHANNELS.moveTerminalBack, "a", "window");
    await call(CHANNELS.moveTerminalBack, "", "docked");
    expect(windows.close.mock.calls).toEqual([["a"]]);
    expect(movedBack.mock.calls).toEqual([[{ sessionId: "a", to: "side panel" }]]);
  });

  it("reads what a shell has printed so far by its run id", async () => {
    const { call, soFar } = terminalWired();
    expect(await call(CHANNELS.readTerminalSoFar, "r1")).toBe("~ $ ls\r\n");
    expect(await call(CHANNELS.readTerminalSoFar, 1)).toBeNull();
    expect(soFar.mock.calls).toEqual([["r1"]]);
  });
});

describe("muting a notice", () => {
  it("sends a mute for every word a notice goes by, not only the behind-base one", async () => {
    const { mute, muteNotice } = wired();
    for (const notice of NOTICES) await mute({ notice, muted: true, scope: { kind: "job", session_id: "s" } });
    expect(muteNotice).toHaveBeenCalledTimes(NOTICES.length);
    expect(muteNotice).toHaveBeenCalledWith({ notice: "pr_merged", muted: true, scope: { kind: "job", session_id: "s" } });
  });

  it("sends nothing for a word that is no notice", async () => {
    const { mute, muteNotice } = wired();
    await mute({ notice: "something_else", muted: true, scope: { kind: "job", session_id: "s" } });
    expect(muteNotice).not.toHaveBeenCalled();
  });
});
