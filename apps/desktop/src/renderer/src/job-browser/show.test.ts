// Which agent-driven opens draw a Browser tab: a Job he has open gets one, or has its first brought to the front. Any other waits for the tab he opens.

import { beforeEach, expect, test, vi } from "vitest";

import { jobOpens, look, peek } from "../job-panes/store";
import { build } from "../job-panes/tree";
import { viewingNow } from "../cockpit/viewed";
import { wireJobBrowsers } from "./store";

vi.mock("../job-panes/store", () => ({ jobOpens: vi.fn(), look: vi.fn(), peek: vi.fn() }));
vi.mock("../cockpit/viewed", () => ({ viewingNow: vi.fn(() => false) }));

let show: (sessionId: string) => void;

beforeEach(() => {
  vi.mocked(jobOpens).mockClear();
  vi.mocked(look).mockClear();
  vi.mocked(peek).mockReturnValue(build({ dir: "row", ratio: 0.5, a: ["thread"], b: ["done"] }));
  vi.mocked(viewingNow).mockReturnValue(false);
  const api = {
    list: async () => [],
    onChanged: () => () => undefined,
    onShow: (listener: (sessionId: string) => void) => {
      show = listener;
      return () => undefined;
    },
  };
  wireJobBrowsers(api as unknown as Parameters<typeof wireJobBrowsers>[0]);
});

test("an agent's open for a Job that is not on screen opens no tab", () => {
  show("far");
  expect(jobOpens).not.toHaveBeenCalled();
  expect(look).not.toHaveBeenCalled();
});

test("an agent's open for a Job on screen with no Browser tab opens one", () => {
  vi.mocked(viewingNow).mockImplementation((id) => id === "near");
  show("near");
  expect(jobOpens).toHaveBeenCalledWith("near", "browser");
});

test("an agent's open for a Job on screen with a Browser tab brings the first to the front, and opens no second", () => {
  vi.mocked(viewingNow).mockReturnValue(true);
  vi.mocked(peek).mockReturnValue(build({ dir: "row", ratio: 0.5, a: ["thread", "browser"], b: ["browser"] }));
  show("near");
  expect(jobOpens).not.toHaveBeenCalled();
  expect(look).toHaveBeenCalledTimes(1);
});
