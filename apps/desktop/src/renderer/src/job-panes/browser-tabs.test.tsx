// The Job's Browser tabs, told to main as the panes change: a tab closed from behind another still reaches main, which destroys its page.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { browserTabIds, wireJobBrowsers } from "../job-browser/store";
import { BrowserTabsSync } from "./bodies";
import { jobOpens, rearrange, peek } from "./store";
import { closeTab } from "./tree";

const tabs = vi.fn(async () => undefined);
const api = {
  list: async () => [],
  onChanged: () => () => undefined,
  onShow: () => () => undefined,
  tabs,
};

let mounted:
  | {
      root: Root;
      host: HTMLElement;
    }
  | undefined;
beforeEach(() => {
  tabs.mockClear();
  wireJobBrowsers(api as unknown as Parameters<typeof wireJobBrowsers>[0]);
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
});

function mount(jobId: string): void {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<BrowserTabsSync jobId={jobId} />));
  mounted = {
    root,
    host,
  };
}

test("a Job with two Browser tabs tells main both, and tells it the one left when the tab behind the front is closed", () => {
  jobOpens("sync-job", "browser");
  jobOpens("sync-job", "browser");
  mount("sync-job");
  const [first, second] = browserTabIds(peek("sync-job"));
  expect(tabs).toHaveBeenLastCalledWith("sync-job", [
    first,
    second,
  ]);
  // The second tab is in front of its pane, so closing the first draws nothing that says so.
  act(() => rearrange("sync-job", (arrangement) => closeTab(arrangement, first!)));
  expect(tabs).toHaveBeenLastCalledWith("sync-job", [
    second,
  ]);
});
