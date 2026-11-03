// A terminal the Job's agent shows comes to the front of its Job's panes whichever Job is on screen, and one with a window of its own gets no tab. Mounted as App mounts it: once, over the window's shells, with no panes drawn.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { JobTerminalChange, TerminalWindowId } from "../../../shared/api/sessions";

import { useShownTerminals } from "./bodies";
import { forgetPanes, peek } from "./store";
import { panesOf } from "./tree";

// The mod list is Fleet's, read through `window.armada`; no Job here made a mod.
vi.mock("../mod-list", () => ({
  useModList: () => [],
}));

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

function Mounted() {
  useShownTerminals();
  return null;
}

let root: Root | null = null;
let host: HTMLElement | null = null;

/** The window's `armada` with a Fleet that can tell of a shell, and main having said which terminals have a window. */
function mount(windows: readonly TerminalWindowId[] = []) {
  let told: (change: JobTerminalChange) => void = () => undefined;
  (
    window as unknown as {
      armada: unknown;
    }
  ).armada = {
    onSessionRunMessage: () => () => undefined,
    onSessionTerminal: (on: typeof told) => ((told = on), () => undefined),
    getTerminalWindows: async () => windows,
    readTerminalSoFar: async () => null,
  };
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root!.render(<Mounted />));
  return {
    show: async (sessionId: string, terminal: string) => {
      // Main's list of windows is read once when the window starts.
      await act(async () => undefined);
      act(() =>
        told({
          session_id: sessionId,
          terminal,
          run_id: `run-${terminal}`,
          change: "shown",
        }),
      );
    },
  };
}

const tabsOf = (jobId: string) =>
  panesOf(peek(jobId).root).flatMap((pane) =>
    pane.tabs
      .filter((tab) => tab.kind === "terminal")
      .map((tab) => ({
        terminal: tab.terminal,
        front: pane.active === tab.id,
      })),
  );

beforeEach(() => forgetPanes());
afterEach(() => {
  act(() => root?.unmount());
  host?.remove();
  root = null;
  host = null;
  localStorage.clear();
});

test("a terminal shown in a Job nobody has open gets a tab in front in that Job", async () => {
  const { show } = mount();
  await show("other-job", "t2");
  expect(tabsOf("other-job")).toEqual([
    {
      terminal: "t2",
      front: true,
    },
  ]);
});

// Its own Job, so the Job above leaves nothing in this one.
test("a terminal that has a window of its own gets no tab", async () => {
  const { show } = mount([
    {
      sessionId: "windowed-job",
      terminal: "t2",
    },
  ]);
  await show("windowed-job", "t2");
  expect(tabsOf("windowed-job")).toEqual([]);
});
