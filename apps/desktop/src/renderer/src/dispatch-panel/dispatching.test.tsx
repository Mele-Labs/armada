// The words in the Dispatch sheet and Fleet's reading of them belong to the sheet. Whatever holds the
// dispatching outside it (in the app, the whole window) is not drawn again for a key typed there.

import { forgetAllKept } from "@armada/components";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "../styles/index.css";
import { connected } from "../mock/moment";
import { DispatchSheet } from "./entries";
import { DispatchingProvider, useDispatch, useDispatching, type Dispatching } from "./dispatching";

const counts = { window: 0, outside: 0 };
let held: Dispatching | undefined;
let mounted: { root: Root; host: HTMLElement } | undefined;
const state = connected([], [], []);
const readRequestRepository = vi.fn(async (_words: string) => ({ manifest_id: null, likely: [] }));

/** A part of the window outside the sheet that reads the dispatching, as the Deck and the Merge line do. */
function Outside() {
  held = useDispatching();
  counts.outside += 1;
  return null;
}

/** What the app is to the dispatching: the one that holds it. */
function Window() {
  counts.window += 1;
  const dispatching = useDispatch({
    state,
    repositories: [],
    proposeFrom: async () => null,
    openSession: () => undefined,
    openJob: () => undefined,
    openAutopilot: () => undefined,
    said: () => undefined,
  });
  return (
    <DispatchingProvider value={dispatching}>
      <Outside />
      <DispatchSheet />
    </DispatchingProvider>
  );
}

beforeEach(() => {
  forgetAllKept();
  counts.window = 0;
  counts.outside = 0;
  readRequestRepository.mockClear();
  (window as unknown as { armada: unknown }).armada = { readRequestRepository };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  root.render(<Window />);
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
  held = undefined;
  (window as unknown as { armada?: unknown }).armada = undefined;
});

test("typing in the sheet draws the sheet, and not the window that holds the dispatching", async () => {
  await expect.poll(() => held).toBeDefined();
  held?.openNew();
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toBeVisible();
  await expect.element(request).toHaveFocus();
  // The first key lands on a window that has just opened its sheet, and React runs the window's body once more
  // to find that the acts typing makes change nothing. That draws no part of it, and it is the one pass allowed.
  await userEvent.keyboard("F");
  await expect.element(request).toHaveValue("F");
  const before = { ...counts };

  await userEvent.keyboard("ix the login page");
  await expect.element(request).toHaveValue("Fix the login page");
  // The reading is made and answered, and that stays inside the sheet too.
  await expect.poll(() => readRequestRepository.mock.calls.at(-1)?.[0]).toBe("Fix the login page");
  await new Promise((done) => setTimeout(done, 100));

  expect(counts).toEqual(before);
});

test("the draft is still there when the sheet is put away and comes back", async () => {
  await expect.poll(() => held).toBeDefined();
  held?.openNew();
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toBeVisible();
  await userEvent.keyboard("Fix the login page");
  await expect.element(request).toHaveValue("Fix the login page");

  held?.closeSheet();
  await expect.element(request).not.toBeInTheDocument();
  held?.openNew();
  await expect.element(request).toHaveValue("Fix the login page");
});
