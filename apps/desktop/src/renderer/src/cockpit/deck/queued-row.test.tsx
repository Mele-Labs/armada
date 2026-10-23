// A queued idea's row in the deck's list. A mouse click on its words starts nothing, because a start spends the plan: only the
// Start press, and ↵ with the row focused, start it. Remove lets it go.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { QueuedIdea, Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import "../../styles/index.css";
import { SessionsFrom } from "../../sessions-draft";
import { QueuedRow } from "./Deck";

const IDEA: QueuedIdea = { id: "q1", text: "Make the Cleanup grid sortable by size\nThen the second line", root: "/code/armada", repository: "armada", model: "opus" };

// One array for every read: the viewer notes' `useSyncExternalStore` loops on a `get` that answers a new one each time.
const NO_SESSIONS: readonly Session[] = [];

let mounted: { root: Root; host: HTMLElement } | undefined;
const startQueued = vi.fn(async (_id: string) => "s-new");
const dropQueued = vi.fn();

beforeEach(() => {
  startQueued.mockClear();
  dropQueued.mockClear();
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const draft = { get: () => NO_SESSIONS, subscribe: () => () => undefined, startQueued, dropQueued } as unknown as SessionsDraft;
  root.render(
    <SessionsFrom held={draft}>
      <ul>
        <QueuedRow idea={IDEA} />
      </ul>
    </SessionsFrom>,
  );
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
});

const row = () => page.getByRole("button", { name: "Make the Cleanup grid sortable by size", exact: true });

test("shows the first line, the repository and the model, and marks the row as queued", async () => {
  await expect.element(row()).toBeVisible();
  await expect.element(page.getByRole("img", { name: "Queued" })).toBeInTheDocument();
  await expect.element(page.getByRole("img", { name: "Repository armada" })).toBeInTheDocument();
  await expect.element(page.getByRole("img", { name: "Model opus" })).toBeInTheDocument();
  await expect.element(page.getByText("Then the second line")).not.toBeInTheDocument();
});

test("a mouse click on its words starts nothing and puts the cursor on the row", async () => {
  await userEvent.click(row());
  await new Promise((done) => setTimeout(done, 100));
  expect(startQueued).not.toHaveBeenCalled();
  await expect.element(row()).toHaveFocus();
});

test("the Start press starts the idea, and so does ↵ with the row focused", async () => {
  await userEvent.click(page.getByRole("button", { name: /^Start Make the Cleanup grid/ }));
  await expect.poll(() => startQueued.mock.calls.length).toBe(1);
  expect(startQueued).toHaveBeenLastCalledWith("q1");

  await expect.element(row()).toBeVisible();
  row().element().focus();
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => startQueued.mock.calls.length).toBe(2);
});

test("Remove drops the idea and starts nothing", async () => {
  await userEvent.click(page.getByRole("button", { name: /^Remove Make the Cleanup grid/ }));
  expect(dropQueued).toHaveBeenCalledWith("q1");
  expect(startQueued).not.toHaveBeenCalled();
});

test("the row and its two presses are Tab stops, in that order", async () => {
  await expect.element(row()).toBeVisible();
  row().element().focus();
  await userEvent.tab();
  await expect.element(page.getByRole("button", { name: /^Start Make/ })).toHaveFocus();
  await userEvent.tab();
  await expect.element(page.getByRole("button", { name: /^Remove Make/ })).toHaveFocus();
});
