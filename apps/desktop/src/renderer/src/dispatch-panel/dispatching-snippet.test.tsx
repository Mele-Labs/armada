// A response too big to send as words, pasted into the sheet, waits as a snippet file and is attached to the Session it starts. The
// words stay the person's own, so Fleet reads a sentence and not a megabyte, and the agent reads the file as far as it needs.

import { forgetAllKept } from "@armada/components";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "../styles/index.css";
import { connected } from "../mock/moment";
import { SessionsFrom } from "../sessions-draft";
import { DispatchSheet } from "./entries";
import { DispatchingProvider, useDispatch, useDispatching, type Dispatching } from "./dispatching";

let held: Dispatching | undefined;
let mounted: { root: Root; host: HTMLElement } | undefined;
const state = connected([], []);
const armada = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const start = vi.fn(async (_tag?: unknown, _root?: string, _request?: string, _tuning?: unknown) => "01SESSION");
const send = vi.fn();
const NO_SESSIONS: Session[] = [];
const draft = { get: () => NO_SESSIONS, subscribe: () => () => undefined, start, send, models: [], efforts: [] } as unknown as SessionsDraft;

function Outside() {
  held = useDispatching();
  return null;
}

function Window() {
  const dispatching = useDispatch({ state, repositories: [armada], openSession: () => undefined, openAutopilot: () => undefined });
  return (
    <DispatchingProvider value={dispatching}>
      <Outside />
      <DispatchSheet />
    </DispatchingProvider>
  );
}

beforeEach(() => {
  forgetAllKept();
  start.mockClear();
  send.mockClear();
  (window as unknown as { armada: unknown }).armada = { readRequestRepository: async () => ({ manifest_id: null, likely: [] }), searchFiles: async () => [] };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  root.render(
    <SessionsFrom held={draft}>
      <Window />
    </SessionsFrom>,
  );
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
  held = undefined;
  (window as unknown as { armada?: unknown }).armada = undefined;
});

/** `words` pasted into the request, from a clipboard of its own, as a person's copy would arrive. */
function paste(words: string): boolean {
  const clipboard = new DataTransfer();
  clipboard.setData("text/plain", words);
  return document.querySelector<HTMLTextAreaElement>('textarea[aria-label="Request"]')!.dispatchEvent(new ClipboardEvent("paste", { clipboardData: clipboard, bubbles: true, cancelable: true }));
}

const HUGE = "x".repeat(32_001);

test("a paste too big to send as words waits as a snippet and leaves the request as it was", async () => {
  await expect.poll(() => held).toBeDefined();
  held?.openNew(armada.root);
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toBeVisible();
  await userEvent.keyboard("Why does this fail");
  paste(HUGE);
  await expect.element(page.getByText("pasted-snippet-1.txt")).toBeVisible();
  await expect.element(request).toHaveValue("Why does this fail");
});

test("a paste that is short enough is left to the box, as words", async () => {
  await expect.poll(() => held).toBeDefined();
  held?.openNew(armada.root);
  await expect.element(page.getByRole("textbox", { name: "Request" })).toBeVisible();
  // Not cancelled, so the browser inserts it; a synthetic one inserts nothing to read back.
  expect(paste("a short note")).toBe(true);
  await expect.element(page.getByText("pasted-snippet-1.txt")).not.toBeInTheDocument();
});

test("the Session starts on the typed words, with the snippet sent as a file", async () => {
  await expect.poll(() => held).toBeDefined();
  held?.openNew(armada.root);
  await expect.element(page.getByRole("textbox", { name: "Request" })).toBeVisible();
  await userEvent.keyboard("Why does this fail");
  paste(HUGE);
  await expect.element(page.getByText("pasted-snippet-1.txt")).toBeVisible();
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => start.mock.calls.length).toBe(1);
  expect(start).toHaveBeenCalledWith(undefined, armada.root, "Why does this fail", {}, undefined, true, undefined, true);
  await expect.poll(() => send.mock.calls.length).toBe(1);
  const [, sent] = send.mock.calls[0]!;
  expect(sent.text).toBe("Why does this fail");
  expect(sent.files.map((one: { name: string }) => one.name)).toEqual(["pasted-snippet-1.txt"]);
  expect(await sent.files[0].file.text()).toBe(HUGE);
});

test("a snippet alone starts a Session, and a removed one is not sent", async () => {
  await expect.poll(() => held).toBeDefined();
  held?.openNew(armada.root);
  await expect.element(page.getByRole("textbox", { name: "Request" })).toBeVisible();
  paste(HUGE);
  await expect.element(page.getByText("pasted-snippet-1.txt")).toBeVisible();
  await page.getByRole("button", { name: /Remove/ }).click();
  await expect.element(page.getByText("pasted-snippet-1.txt")).not.toBeInTheDocument();
  paste(HUGE);
  await expect.element(page.getByText("pasted-snippet-1.txt")).toBeVisible();
  await page.getByRole("button", { name: "Start" }).click();
  await expect.poll(() => start.mock.calls.length).toBe(1);
  const request = start.mock.calls[0]![2];
  expect(request).not.toBe("");
  expect(request).not.toContain("xxxx");
  await expect.poll(() => send.mock.calls.length).toBe(1);
  expect(send.mock.calls[0]![1].files).toHaveLength(1);
});

const PICTURE = () => new File(["png"], "Screenshot.png", { type: "image/png" });
const field = () => document.querySelector<HTMLTextAreaElement>('textarea[aria-label="Request"]')!;

test("a screenshot pasted into the request waits as a chip and is sent with the Session", async () => {
  await expect.poll(() => held).toBeDefined();
  held?.openNew(armada.root);
  await expect.element(page.getByRole("textbox", { name: "Request" })).toBeVisible();
  await userEvent.keyboard("The button is hidden");
  const clipboard = new DataTransfer();
  clipboard.items.add(PICTURE());
  field().dispatchEvent(new ClipboardEvent("paste", { clipboardData: clipboard, bubbles: true, cancelable: true }));
  await expect.element(page.getByText("Screenshot.png")).toBeVisible();
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => send.mock.calls.length).toBe(1);
  expect(send.mock.calls[0]![1].files.map((one: { name: string }) => one.name)).toEqual(["Screenshot.png"]);
});

test("a file dropped on the request waits as a chip", async () => {
  await expect.poll(() => held).toBeDefined();
  held?.openNew(armada.root);
  await expect.element(page.getByRole("textbox", { name: "Request" })).toBeVisible();
  const transfer = new DataTransfer();
  transfer.items.add(PICTURE());
  field().dispatchEvent(new DragEvent("drop", { dataTransfer: transfer, bubbles: true, cancelable: true }));
  await expect.element(page.getByText("Screenshot.png")).toBeVisible();
});
