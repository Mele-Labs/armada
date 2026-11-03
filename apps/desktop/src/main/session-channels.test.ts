// Every answer a card offers reaches Fleet through main. A list kept here missed `always_allow`
// and `always_block`, and the press came back as Fleet not connected (16 Oct 2026).

import { HELM_CALL_ANSWERS, NOTICES, type AnswerSessionAsk } from "@armada/protocol";
import { describe, expect, it, vi } from "vitest";

import { CHANNELS } from "../shared/bridge";
import { handleSessions } from "./session-channels";

type Handler = (event: unknown, ...args: unknown[]) => unknown;

function wired() {
  const handlers = new Map<string, Handler>();
  const answer = vi.fn(async () => ({
    ok: true,
  }));
  const muteNotice = vi.fn(async () => ({
    ok: true,
  }));
  const ipc = {
    handle: (channel: string, handler: Handler) => handlers.set(channel, handler),
  };
  const settleLater = vi.fn(async () => ({
    ok: true,
  }));
  const confirmLater = vi.fn(async () => ({
    ok: true,
  }));
  const connection = () => ({
    sessions: {
      answer,
      muteNotice,
      settleLater,
      confirmLater,
    },
  });
  handleSessions({
    ipc,
    connection,
    windowIdOf: () => undefined,
    pages: {},
  } as unknown as Parameters<typeof handleSessions>[0]);
  const send = (body: AnswerSessionAsk) => handlers.get(CHANNELS.answerSessionAsk)?.({}, body);
  const mute = (body: unknown) => handlers.get(CHANNELS.muteNotice)?.({}, body);
  const drop = (body: unknown) => handlers.get(CHANNELS.settleLater)?.({}, body);
  const confirm = (body: unknown) => handlers.get(CHANNELS.confirmLater)?.({}, body);
  return {
    send,
    answer,
    mute,
    muteNotice,
    drop,
    confirm,
    settleLater,
    confirmLater,
  };
}

describe("answering a session's ask", () => {
  it("sends every answer the protocol names, the folder with it", async () => {
    const { send, answer } = wired();
    for (const said of HELM_CALL_ANSWERS) {
      await send({
        session_id: "s",
        call: "helm-1",
        answer: said,
        folder: "/Users/user/Documents",
      } as AnswerSessionAsk);
    }
    expect(answer).toHaveBeenCalledTimes(HELM_CALL_ANSWERS.length);
    expect(answer).toHaveBeenLastCalledWith(
      expect.objectContaining({
        answer: "always_block",
        folder: "/Users/user/Documents",
      }),
    );
  });

  it("sends the kind a like-this answer chose, and refuses a malformed one, sending nothing", async () => {
    const { send, answer } = wired();
    const like = {
      session_id: "s",
      call: "helm-1",
      answer: "allow_like_this_for_session",
    } as const;
    await send({
      ...like,
      kind: {
        program: "node",
        script: "drive.mjs",
      },
    });
    expect(answer).toHaveBeenLastCalledWith({
      ...like,
      kind: {
        program: "node",
        script: "drive.mjs",
      },
    });
    await send({
      ...like,
      kind: {
        program: "node",
      },
      folder: "/Users/user/Desktop",
    });
    expect(answer).toHaveBeenLastCalledWith({
      ...like,
      kind: {
        program: "node",
      },
      folder: "/Users/user/Desktop",
    });
    answer.mockClear();
    for (const kind of [
      "node",
      null,
      {
        script: "drive.mjs",
      },
      {
        program: 3,
      },
      {
        program: "node",
        script: 5,
      },
    ]) {
      const said = await send({
        ...like,
        kind,
      } as unknown as AnswerSessionAsk);
      expect(said).toEqual({
        ok: false,
        outcome: {
          ok: false,
          why: "not_understood",
        },
      });
    }
    expect(answer).not.toHaveBeenCalled();
  });

  it("sends nothing for a word that is no answer, and says so rather than Fleet not connected", async () => {
    const { send, answer } = wired();
    const said = await send({
      session_id: "s",
      call: "helm-1",
      answer: "maybe",
    } as unknown as AnswerSessionAsk);
    expect(answer).not.toHaveBeenCalled();
    expect(said).toEqual({
      ok: false,
      outcome: {
        ok: false,
        why: "not_understood",
      },
    });
  });
});

describe("starting a Mod Job", () => {
  const note = {
    said: "",
    address: "file:///index.html",
    place: {
      selector: "li",
      element: {
        tag: "li",
        text: "x",
      },
      location: "/usage",
    },
    state: {
      selected: [],
    },
  };

  it("passes the note and the words Bridge gave to Fleet, and drops a blank model", async () => {
    const handlers = new Map<string, Handler>();
    const startMod = vi.fn(async () => ({
      ok: true,
    }));
    const ipc = {
      handle: (channel: string, handler: Handler) => handlers.set(channel, handler),
    };
    handleSessions({
      ipc,
      connection: () => ({
        sessions: {
          startMod,
        },
      }),
      windowIdOf: () => undefined,
      pages: {},
    } as unknown as Parameters<typeof handleSessions>[0]);
    await handlers.get(CHANNELS.startModSession)?.(
      {},
      {
        title: "Usage meter",
        model: "",
        note,
      },
    );
    expect(startMod).toHaveBeenCalledWith({
      title: "Usage meter",
      note,
    });
  });

  it("passes an empty body through as an empty Mod Job", async () => {
    const handlers = new Map<string, Handler>();
    const startMod = vi.fn(async () => ({
      ok: true,
    }));
    const ipc = {
      handle: (channel: string, handler: Handler) => handlers.set(channel, handler),
    };
    handleSessions({
      ipc,
      connection: () => ({
        sessions: {
          startMod,
        },
      }),
      windowIdOf: () => undefined,
      pages: {},
    } as unknown as Parameters<typeof handleSessions>[0]);
    await handlers.get(CHANNELS.startModSession)?.({}, {});
    expect(startMod).toHaveBeenCalledWith({});
  });

  it("passes the Job and tab a Custom tab started it from, and refuses either that is not a string", async () => {
    const handlers = new Map<string, Handler>();
    const startMod = vi.fn(async () => ({
      ok: true,
    }));
    const ipc = {
      handle: (channel: string, handler: Handler) => handlers.set(channel, handler),
    };
    handleSessions({
      ipc,
      connection: () => ({
        sessions: {
          startMod,
        },
      }),
      windowIdOf: () => undefined,
      pages: {},
    } as unknown as Parameters<typeof handleSessions>[0]);
    const start = handlers.get(CHANNELS.startModSession);
    await start?.(
      {},
      {
        said: "a list",
        from_job: "job-9",
        from_tab: "t4",
      },
    );
    expect(startMod).toHaveBeenCalledWith({
      said: "a list",
      from_job: "job-9",
      from_tab: "t4",
    });
    startMod.mockClear();
    const notUnderstood = {
      ok: false,
      outcome: {
        ok: false,
        why: "not_understood",
      },
    };
    expect(
      await start?.(
        {},
        {
          said: "a list",
          from_job: 9,
          from_tab: "t4",
        },
      ),
    ).toEqual(notUnderstood);
    expect(
      await start?.(
        {},
        {
          said: "a list",
          from_job: "job-9",
          from_tab: {},
        },
      ),
    ).toEqual(notUnderstood);
    expect(startMod).not.toHaveBeenCalled();
  });

  it("passes his words as said, and answers not understood to a malformed note or words, sending nothing", async () => {
    const handlers = new Map<string, Handler>();
    const startMod = vi.fn(async () => ({
      ok: true,
    }));
    const ipc = {
      handle: (channel: string, handler: Handler) => handlers.set(channel, handler),
    };
    handleSessions({
      ipc,
      connection: () => ({
        sessions: {
          startMod,
        },
      }),
      windowIdOf: () => undefined,
      pages: {},
    } as unknown as Parameters<typeof handleSessions>[0]);
    const start = handlers.get(CHANNELS.startModSession);
    await start?.(
      {},
      {
        said: "a dark theme",
      },
    );
    expect(startMod).toHaveBeenCalledWith({
      said: "a dark theme",
    });
    startMod.mockClear();
    const notUnderstood = {
      ok: false,
      outcome: {
        ok: false,
        why: "not_understood",
      },
    };
    expect(
      await start?.(
        {},
        {
          note: {
            said: "x",
          },
        },
      ),
    ).toEqual(notUnderstood);
    expect(
      await start?.(
        {},
        {
          said: 3,
        },
      ),
    ).toEqual(notUnderstood);
    expect(await start?.({}, null)).toEqual(notUnderstood);
    expect(startMod).not.toHaveBeenCalled();
  });
});

describe("a Mod Job's aim", () => {
  it("passes the aim Bridge named to Fleet, and refuses a word that is no aim, sending nothing", async () => {
    const handlers = new Map<string, Handler>();
    const startMod = vi.fn(async () => ({
      ok: true,
    }));
    const ipc = {
      handle: (channel: string, handler: Handler) => handlers.set(channel, handler),
    };
    handleSessions({
      ipc,
      connection: () => ({
        sessions: {
          startMod,
        },
      }),
      windowIdOf: () => undefined,
      pages: {},
    } as unknown as Parameters<typeof handleSessions>[0]);
    const start = handlers.get(CHANNELS.startModSession);
    await start?.(
      {},
      {
        said: "a track view",
        aim: "tab",
      },
    );
    expect(startMod).toHaveBeenCalledWith({
      said: "a track view",
      aim: "tab",
    });
    startMod.mockClear();
    const notUnderstood = {
      ok: false,
      outcome: {
        ok: false,
        why: "not_understood",
      },
    };
    expect(
      await start?.(
        {},
        {
          said: "x",
          aim: "panel",
        },
      ),
    ).toEqual(notUnderstood);
    expect(
      await start?.(
        {},
        {
          said: "x",
          aim: 3,
        },
      ),
    ).toEqual(notUnderstood);
    expect(startMod).not.toHaveBeenCalled();
  });
});

describe("sending a session message", () => {
  it("passes the aim of an empty Mod Job's first message to Fleet unchanged", async () => {
    const handlers = new Map<string, Handler>();
    const send = vi.fn(async () => ({
      ok: true,
    }));
    const ipc = {
      handle: (channel: string, handler: Handler) => handlers.set(channel, handler),
    };
    handleSessions({
      ipc,
      connection: () => ({
        sessions: {
          send,
        },
      }),
      windowIdOf: () => undefined,
      pages: {},
    } as unknown as Parameters<typeof handleSessions>[0]);
    await handlers.get(CHANNELS.sendSessionMessage)?.(
      {},
      {
        session_id: "m",
        text: "a dark theme",
        aim: "theme",
      },
    );
    expect(send).toHaveBeenCalledWith({
      session_id: "m",
      text: "a dark theme",
      aim: "theme",
    });
  });
});

function terminalWired() {
  const handlers = new Map<string, Handler>();
  const ipc = {
    handle: (channel: string, handler: Handler) => handlers.set(channel, handler),
  };
  const soFar = vi.fn((runId: string) => (runId === "r1" ? "~ $ ls\r\n" : null));
  const windows = {
    show: vi.fn(),
    close: vi.fn(),
    ids: () => [
      {
        sessionId: "a",
      },
      {
        sessionId: "a",
        terminal: "t2",
      },
    ],
  };
  const movedBack = vi.fn();
  const shellNow = vi.fn((sessionId: string, terminal?: string) =>
    sessionId === "a" && (terminal === undefined || terminal === "t2")
      ? {
          runId: terminal === undefined ? "r1" : "r2",
          ended: true,
        }
      : null,
  );
  const open = vi.fn(async () => ({
    ok: true,
  }));
  const listTerminals = vi.fn(async () => ({
    ok: true,
  }));
  const renameTerminal = vi.fn(async () => ({
    ok: true,
  }));
  handleSessions({
    ipc,
    connection: () => ({
      sessions: {
        soFar,
        shellNow,
        openTerminal: open,
        listTerminals,
        renameTerminal,
      },
    }),
    windowIdOf: () => undefined,
    pages: {},
    terminals: {
      windows,
      movedBack,
    },
  } as unknown as Parameters<typeof handleSessions>[0]);
  const call = (channel: string, ...args: unknown[]) => handlers.get(channel)?.({}, ...args);
  return {
    call,
    windows,
    movedBack,
    soFar,
    shellNow,
    open,
    listTerminals,
    renameTerminal,
  };
}

describe("a Job's terminals", () => {
  it("opens the terminal it names, a new one, or the first, and passes Fleet's refusal of both at once on", async () => {
    const { call, open } = terminalWired();
    await call(CHANNELS.openTerminal, "a", 80, 24, {
      terminal: "t2",
    });
    await call(CHANNELS.openTerminal, "a", undefined, undefined, {
      new: true,
    });
    await call(CHANNELS.openTerminal, "a");
    await call(CHANNELS.openTerminal, "a", undefined, undefined, {
      terminal: "t1",
      new: true,
    });
    expect(open.mock.calls).toEqual([
      [
        {
          session_id: "a",
          cols: 80,
          rows: 24,
          terminal: "t2",
        },
      ],
      [
        {
          session_id: "a",
          new: true,
        },
      ],
      [
        {
          session_id: "a",
        },
      ],
      [
        {
          session_id: "a",
          terminal: "t1",
          new: true,
        },
      ],
    ]);
  });

  it("opens nothing for a terminal that is not an ID or a new that is not a flag", async () => {
    const { call, open } = terminalWired();
    await call(CHANNELS.openTerminal, "a", undefined, undefined, {
      terminal: 2,
    });
    await call(CHANNELS.openTerminal, "a", undefined, undefined, {
      terminal: "",
    });
    await call(CHANNELS.openTerminal, "a", undefined, undefined, {
      new: "yes",
    });
    await call(CHANNELS.openTerminal, "", undefined, undefined, undefined);
    expect(open).not.toHaveBeenCalled();
  });

  it("lists a Job's terminals and names one, each by the Job's id", async () => {
    const { call, listTerminals, renameTerminal } = terminalWired();
    await call(CHANNELS.listTerminals, "a");
    await call(CHANNELS.listTerminals, 3);
    await call(CHANNELS.renameTerminal, "a", "t2", "server");
    await call(CHANNELS.renameTerminal, "a", "t2", "");
    await call(CHANNELS.renameTerminal, "a", "", "x");
    expect(listTerminals.mock.calls).toEqual([
      [
        "a",
      ],
    ]);
    expect(renameTerminal.mock.calls).toEqual([
      [
        {
          session_id: "a",
          terminal: "t2",
          name: "server",
        },
      ],
      [
        {
          session_id: "a",
          terminal: "t2",
          name: "",
        },
      ],
    ]);
  });
});

describe("a terminal's window", () => {
  it("opens a window by the Job's id and the terminal's, and nothing for an id that is not one", async () => {
    const { call, windows } = terminalWired();
    await call(CHANNELS.openTerminalWindow, "a");
    await call(CHANNELS.openTerminalWindow, "a", "t2");
    await call(CHANNELS.openTerminalWindow, "");
    await call(CHANNELS.openTerminalWindow, 7);
    await call(CHANNELS.openTerminalWindow, "a", 2);
    expect(windows.show.mock.calls).toEqual([
      [
        {
          sessionId: "a",
        },
      ],
      [
        {
          sessionId: "a",
          terminal: "t2",
        },
      ],
    ]);
    expect(await call(CHANNELS.getTerminalWindows)).toEqual([
      {
        sessionId: "a",
      },
      {
        sessionId: "a",
        terminal: "t2",
      },
    ]);
  });

  it("moves a shell back into Bridge, closing its window and telling every window where it went and which shell it is", async () => {
    const { call, windows, movedBack, shellNow } = terminalWired();
    // The side panel is gone, and a window is where a shell moves from.
    await call(CHANNELS.moveTerminalBack, "a", "side panel");
    await call(CHANNELS.moveTerminalBack, "a", "window");
    await call(CHANNELS.moveTerminalBack, "", "docked");
    await call(CHANNELS.moveTerminalBack, "a", "docked");
    await call(CHANNELS.moveTerminalBack, "a", "docked", "t2");
    await call(CHANNELS.moveTerminalBack, "b", "docked");
    expect(windows.close.mock.calls).toEqual([
      [
        {
          sessionId: "a",
        },
      ],
      [
        {
          sessionId: "a",
          terminal: "t2",
        },
      ],
      [
        {
          sessionId: "b",
        },
      ],
    ]);
    expect(shellNow.mock.calls).toEqual([
      [
        "a",
        undefined,
      ],
      [
        "a",
        "t2",
      ],
      [
        "b",
        undefined,
      ],
    ]);
    expect(movedBack.mock.calls).toEqual([
      [
        {
          sessionId: "a",
          to: "docked",
          shell: {
            runId: "r1",
            ended: true,
          },
        },
      ],
      [
        {
          sessionId: "a",
          terminal: "t2",
          to: "docked",
          shell: {
            runId: "r2",
            ended: true,
          },
        },
      ],
      [
        {
          sessionId: "b",
          to: "docked",
        },
      ],
    ]);
  });

  it("reads what a shell has printed so far by its run id", async () => {
    const { call, soFar } = terminalWired();
    expect(await call(CHANNELS.readTerminalSoFar, "r1")).toBe("~ $ ls\r\n");
    expect(await call(CHANNELS.readTerminalSoFar, 1)).toBeNull();
    expect(soFar.mock.calls).toEqual([
      [
        "r1",
      ],
    ]);
  });
});

describe("muting a notice", () => {
  it("sends a mute for every word a notice goes by, not only the behind-base one", async () => {
    const { mute, muteNotice } = wired();
    for (const notice of NOTICES)
      await mute({
        notice,
        muted: true,
        scope: {
          kind: "job",
          session_id: "s",
        },
      });
    expect(muteNotice).toHaveBeenCalledTimes(NOTICES.length);
    expect(muteNotice).toHaveBeenCalledWith({
      notice: "pr_merged",
      muted: true,
      scope: {
        kind: "job",
        session_id: "s",
      },
    });
  });

  it("sends nothing for a word that is no notice", async () => {
    const { mute, muteNotice } = wired();
    await mute({
      notice: "something_else",
      muted: true,
      scope: {
        kind: "job",
        session_id: "s",
      },
    });
    expect(muteNotice).not.toHaveBeenCalled();
  });
});

describe("a Job held on its start card", () => {
  function started() {
    const handlers = new Map<string, Handler>();
    const ipc = {
      handle: (channel: string, handler: Handler) => handlers.set(channel, handler),
    };
    const begin = vi.fn(async () => ({
      ok: true,
    }));
    const startSession = vi.fn(async (..._args: unknown[]) => ({
      ok: true,
    }));
    const connection = () => ({
      startSession,
      repositories: {
        pickedByWindow: {
          of: () => ({}),
        },
      },
      sessions: {
        begin,
      },
    });
    handleSessions({
      ipc,
      connection,
      windowIdOf: () => undefined,
      pages: {},
    } as unknown as Parameters<typeof handleSessions>[0]);
    const call = (channel: string, ...args: unknown[]) => handlers.get(channel)?.({}, ...args);
    return {
      call,
      begin,
      startSession,
    };
  }

  it("passes `held` on to the start, and only when it is true", async () => {
    const { call, startSession } = started();
    await call(CHANNELS.startSession, undefined, "/r", "fix it", {}, undefined, true);
    await call(CHANNELS.startSession, undefined, "/r", "fix it", {});
    expect(startSession.mock.calls.map((one) => one[6])).toEqual([
      true,
      false,
    ]);
  });

  it("begins it with the delivery chosen, or none", async () => {
    const { call, begin } = started();
    await call(CHANNELS.beginSession, {
      session_id: "s",
      delivery: "open_pr",
    });
    await call(CHANNELS.beginSession, {
      session_id: "s",
    });
    expect(begin.mock.calls).toEqual([
      [
        {
          session_id: "s",
          delivery: "open_pr",
        },
      ],
      [
        {
          session_id: "s",
        },
      ],
    ]);
  });

  it("passes the first message as he left it on the card, and refuses one that is no text rather than begin on the unedited words", async () => {
    const { call, begin } = started();
    await call(CHANNELS.beginSession, {
      session_id: "s",
      delivery: "keep_local",
      message: "Port the clock, then the loader",
    });
    expect(begin.mock.calls).toEqual([
      [
        {
          session_id: "s",
          delivery: "keep_local",
          message: "Port the clock, then the loader",
        },
      ],
    ]);
    expect(
      await call(CHANNELS.beginSession, {
        session_id: "s",
        message: 4,
      }),
    ).toEqual({
      ok: false,
      outcome: {
        ok: false,
        why: "not_understood",
      },
    });
    expect(begin).toHaveBeenCalledTimes(1);
  });

  it("sends nothing for a Job that is no id or a delivery that is no choice, and says so", async () => {
    const { call, begin } = started();
    expect(
      await call(CHANNELS.beginSession, {
        session_id: "",
        delivery: "open_pr",
      }),
    ).toEqual({
      ok: false,
      outcome: {
        ok: false,
        why: "not_understood",
      },
    });
    expect(
      await call(CHANNELS.beginSession, {
        session_id: "s",
        delivery: "merge_now",
      }),
    ).toEqual({
      ok: false,
      outcome: {
        ok: false,
        why: "not_understood",
      },
    });
    expect(begin).not.toHaveBeenCalled();
  });
});

// Approve and the picks name only their own field. A list of fields here refused them as Bridge not reading the request (3 Nov 2026).
describe("a change to a waiting Job", () => {
  function waiting() {
    const handlers = new Map<string, Handler>();
    const ipc = {
      handle: (channel: string, handler: Handler) => handlers.set(channel, handler),
    };
    const changeWaiting = vi.fn(async () => ({
      ok: true,
    }));
    handleSessions({
      ipc,
      connection: () => ({
        sessions: {
          changeWaiting,
        },
      }),
      windowIdOf: () => undefined,
      pages: {},
    } as unknown as Parameters<typeof handleSessions>[0]);
    const change = (body: unknown) => handlers.get(CHANNELS.changeWaiting)?.({}, body);
    return {
      change,
      changeWaiting,
    };
  }

  it.each([
    {
      held: false,
    },
    {
      held: true,
    },
    {
      workflow: "feature",
    },
    {
      model: "opus",
    },
    {
      effort: "",
    },
    {
      condition: {
        kind: "pr_lands",
      },
    },
    {
      base: "branch",
    },
  ])("passes %j on to Fleet", async (field) => {
    const { change, changeWaiting } = waiting();
    const body = {
      waiting_session: "w",
      ...field,
    };
    expect(await change(body)).toEqual({
      ok: true,
    });
    expect(changeWaiting).toHaveBeenCalledWith(body);
  });

  it("sends nothing for a Job that is no id, and says so", async () => {
    const { change, changeWaiting } = waiting();
    expect(
      await change({
        waiting_session: "",
        held: false,
      }),
    ).toEqual({
      ok: false,
      outcome: {
        ok: false,
        why: "not_understood",
      },
    });
    expect(changeWaiting).not.toHaveBeenCalled();
  });
});

describe("the thumbs on a follow-up", () => {
  it("passes the misread of a thumbs down on to Fleet", async () => {
    const { drop, settleLater } = wired();
    await drop({
      id: "later-2",
      how: "dropped",
      misread: true,
      session_id: "a",
    });
    expect(settleLater).toHaveBeenCalledWith({
      id: "later-2",
      how: "dropped",
      misread: true,
      session_id: "a",
    });
  });

  it("sends a thumbs up by id, and refuses one with no id", async () => {
    const { confirm, confirmLater } = wired();
    await confirm({
      id: "later-2",
      session_id: "a",
    });
    expect(confirmLater).toHaveBeenCalledWith({
      id: "later-2",
      session_id: "a",
    });
    expect(
      await confirm({
        session_id: "a",
      }),
    ).toEqual({
      ok: false,
      outcome: {
        ok: false,
        why: "not_understood",
      },
    });
    expect(confirmLater).toHaveBeenCalledTimes(1);
  });
});
