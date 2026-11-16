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

beforeEach(() => {
  vi.spyOn(window.parent, "postMessage").mockImplementation(() => {});
});

afterEach(() => {
  delete host.webkit;
  delete host.armadaHost;
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
