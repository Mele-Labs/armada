// @vitest-environment happy-dom
// The kit's script on a sealed page (`crates/fleet/src/session_host/page_kit.js`), run against a fake DOM: which
// way it talks to its host, and that both ways in draw the same answer. Pocket has the one test runner here that
// carries a DOM; the script is read from the crate, so this is the file Fleet serves, not a copy.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const KIT = readFileSync(
  resolve(process.cwd(), "../../crates/fleet/src/session_host/page_kit.js"),
  "utf8",
);

const RECORD = {
  artifact: "/r/a.html",
  revision: 2,
  presses: [],
  votes: [],
};

type Host = {
  webkit?: {
    messageHandlers: {
      armada: {
        postMessage: (message: unknown) => void;
      };
    };
  };
  armadaHost?: (message: unknown) => void;
};
const host = window as unknown as Host;

/** What the host would send for a press Fleet kept. */
const pressed = {
  armada: 1,
  kind: "pressed",
  press: {
    revision: 2,
    action: "go",
    act: "start",
    at: "2026-11-14T09:00:00.000Z",
    did: {
      kind: "sent",
    },
  },
};

/** Loads the page as Fleet serves it: the record, one action, then the script. */
function load() {
  document.body.innerHTML = `<main><armada-action id="go" label="Go" act="start"></armada-action></main>
    <script type="application/json" id="armada-page-record">${JSON.stringify(RECORD)}</script>`;
  new Function(KIT)();
}

const sent = () => document.querySelector(".armada-sent")?.textContent;

/** Whether an event is his own: happy-dom carries no `isTrusted`, so every event here is his unless a test says not. */
let trusted = true;

beforeEach(() => {
  trusted = true;
  Object.defineProperty(Event.prototype, "isTrusted", {
    get: () => trusted,
    configurable: true,
  });
  vi.spyOn(window.parent, "postMessage").mockImplementation(() => {});
});

afterEach(() => {
  delete host.webkit;
  delete host.armadaHost;
  delete (
    Event.prototype as {
      isTrusted?: boolean;
    }
  ).isTrusted;
  vi.restoreAllMocks();
});

describe("the kit's way to its host", () => {
  it("posts a press to the webkit handler, and not to the parent, where the host has one", () => {
    const handler = vi.fn();
    host.webkit = {
      messageHandlers: {
        armada: {
          postMessage: handler,
        },
      },
    };
    load();
    document.querySelector<HTMLButtonElement>("armada-action button")?.click();
    expect(handler).toHaveBeenCalledExactlyOnceWith({
      armada: 1,
      kind: "press",
      action: "go",
      artifact: "/r/a.html",
      revision: 2,
    });
    expect(window.parent.postMessage).not.toHaveBeenCalled();
  });

  it("posts nothing for a press a page's own script makes", () => {
    const handler = vi.fn();
    host.webkit = {
      messageHandlers: {
        armada: {
          postMessage: handler,
        },
      },
    };
    load();
    trusted = false;
    document.querySelector<HTMLButtonElement>("armada-action button")?.click();
    expect(handler).not.toHaveBeenCalled();
  });

  it("posts a press to the parent, with target origin *, where there is no webkit handler", () => {
    load();
    document.querySelector<HTMLButtonElement>("armada-action button")?.click();
    expect(window.parent.postMessage).toHaveBeenCalledExactlyOnceWith(
      {
        armada: 1,
        kind: "press",
        action: "go",
        artifact: "/r/a.html",
        revision: 2,
      },
      "*",
    );
  });
});

describe("the kit's ways in", () => {
  it("draws a press the host passes to window.armadaHost", () => {
    load();
    host.armadaHost?.(pressed);
    expect(sent()).toBe("Sent to the Job: Go");
  });

  it("draws the same press from a message event from the parent, and ignores one from anywhere else", () => {
    load();
    window.dispatchEvent(
      new MessageEvent("message", {
        data: pressed,
        source: {} as MessageEventSource,
      }),
    );
    expect(sent()).toBeUndefined();
    window.dispatchEvent(
      new MessageEvent("message", {
        data: pressed,
        source: window.parent,
      }),
    );
    expect(sent()).toBe("Sent to the Job: Go");
  });

  it("ignores a message that does not say armada: 1", () => {
    load();
    host.armadaHost?.({
      ...pressed,
      armada: 2,
    });
    expect(sent()).toBeUndefined();
  });
});

/** A pick vote with two takes, as an agent writes it. */
const pickVote = (id: string) => `<armada-vote id="${id}" form="pick" question="Which ${id}?">
  <armada-take id="a" title="A"></armada-take><armada-take id="b" title="B"></armada-take></armada-vote>`;

/** Loads a page of votes the way Fleet serves it, and returns what the page posts to its host. */
function loadVotes(body: string, votes: unknown[] = []) {
  const posted = vi.fn();
  host.webkit = {
    messageHandlers: {
      armada: {
        postMessage: posted,
      },
    },
  };
  document.body.innerHTML = `<main>${body}</main>
    <script type="application/json" id="armada-page-record">${JSON.stringify({
      ...RECORD,
      votes,
    })}</script>`;
  new Function(KIT)();
  return posted;
}

const pickTake = (vote: string, take: string) => {
  const radio = document.querySelector<HTMLInputElement>(
    `input[name="armada-pick-${vote}"][value="${take}"]`,
  );
  radio?.click();
};
const bar = () => document.querySelector<HTMLButtonElement>(".armada-vote-bar button");
const statusOf = (vote: string) =>
  document.querySelector(`armada-vote[id="${vote}"] .armada-vote-status`)?.textContent;
const votesPosted = (posted: ReturnType<typeof vi.fn>) =>
  posted.mock.calls.map(
    ([message]) =>
      (
        message as {
          vote: string;
        }
      ).vote,
  );

describe("a page with several votes", () => {
  it("sends every vote with a result from one press, each as its own post", () => {
    const posted = loadVotes(`${pickVote("one")}${pickVote("two")}
      <armada-vote id="order" form="rank"><armada-take id="x" title="X"></armada-take><armada-take id="y" title="Y"></armada-take></armada-vote>`);
    expect(document.querySelectorAll("armada-vote .armada-send")).toHaveLength(0);
    expect(bar()?.textContent).toBe("Send picks");
    pickTake("one", "a");
    pickTake("two", "b");
    bar()?.click();
    expect(posted.mock.calls.map(([message]) => message)).toEqual([
      expect.objectContaining({
        kind: "vote",
        vote: "one",
        result: {
          form: "pick",
          take: "a",
        },
      }),
      expect.objectContaining({
        kind: "vote",
        vote: "two",
        result: {
          form: "pick",
          take: "b",
        },
      }),
      expect.objectContaining({
        kind: "vote",
        vote: "order",
        result: {
          form: "rank",
          order: [
            "x",
            "y",
          ],
        },
      }),
    ]);
  });

  it("leaves a vote with no pick open, saying so, and sends the rest", () => {
    const posted = loadVotes(`${pickVote("one")}${pickVote("two")}`);
    expect(bar()?.disabled).toBe(true);
    pickTake("one", "a");
    expect(bar()?.disabled).toBe(false);
    bar()?.click();
    expect(votesPosted(posted)).toEqual([
      "one",
    ]);
    expect(statusOf("two")).toBe("Pick a take first");
  });

  it("sends from the keyboard with the key the bar names", () => {
    const posted = loadVotes(`${pickVote("one")}${pickVote("two")}`);
    pickTake("two", "a");
    document.querySelector("armada-vote")?.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "Enter",
        metaKey: true,
        bubbles: true,
      }),
    );
    expect(votesPosted(posted)).toEqual([
      "two",
    ]);
  });

  it("keeps a vote already sent as sent, and does not send it again", () => {
    const posted = loadVotes(`${pickVote("one")}${pickVote("two")}`, [
      {
        vote: "one",
        result: {
          form: "pick",
          take: "a",
        },
      },
    ]);
    expect(statusOf("one")).toBe("Sent to the Job");
    pickTake("two", "b");
    bar()?.click();
    expect(votesPosted(posted)).toEqual([
      "two",
    ]);
    expect(statusOf("one")).toBe("Sent to the Job");
  });
});

describe("a page with one vote", () => {
  it("keeps the vote's own Send and draws no bar", () => {
    const posted = loadVotes(pickVote("one"));
    expect(bar()).toBeNull();
    pickTake("one", "b");
    document.querySelector<HTMLButtonElement>("armada-vote .armada-send")?.click();
    expect(votesPosted(posted)).toEqual([
      "one",
    ]);
  });
});
