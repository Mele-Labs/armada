// Which agent-driven opens draw the side panel: the person's press, or a Job he has open. Any other waits behind its Browser tile.

import { beforeEach, expect, test, vi } from "vitest";

import { openInPanel } from "../shared-panel/store";
import { viewingNow } from "../cockpit/viewed";
import { openBrowser, wireJobBrowsers } from "./store";

vi.mock("../shared-panel/store", () => ({ openInPanel: vi.fn(() => true) }));
vi.mock("../cockpit/viewed", () => ({ viewingNow: vi.fn(() => false) }));

let show: (sessionId: string) => void;

beforeEach(() => {
  vi.mocked(openInPanel).mockClear();
  vi.mocked(viewingNow).mockReturnValue(false);
  const api = {
    list: async () => [],
    onChanged: () => () => undefined,
    onShow: (listener: (sessionId: string) => void) => {
      show = listener;
      return () => undefined;
    },
    show: async (sessionId: string) => show(sessionId),
  };
  wireJobBrowsers(api as unknown as Parameters<typeof wireJobBrowsers>[0]);
});

test("an agent's open for a Job that is not on screen draws no panel", () => {
  show("far");
  expect(openInPanel).not.toHaveBeenCalled();
});

test("an agent's open for a Job on screen draws the panel", () => {
  vi.mocked(viewingNow).mockImplementation((id) => id === "near");
  show("near");
  expect(openInPanel).toHaveBeenCalledWith({ kind: "browser", sessionId: "near" });
});

test("a press on the Browser tile draws the panel whatever is on screen, once", async () => {
  openBrowser("far");
  await Promise.resolve();
  expect(openInPanel).toHaveBeenCalledTimes(1);
  show("far");
  expect(openInPanel).toHaveBeenCalledTimes(1);
});
