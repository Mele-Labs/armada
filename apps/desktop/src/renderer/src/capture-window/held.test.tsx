// The walk window's bar while the Job on screen holds notes: it says they wait in the Job panel, and Approve asks first
// whether to send them. `docs/practices/capture-window.md`, *Held notes*.

import { createRoot } from "react-dom/client";
import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { CaptureWindowState } from "../../../shared/capture-window";
import "../styles/index.css";
import { CaptureWindow, type CaptureWindowApi } from "./CaptureWindow";

const state = (held?: "one" | "several"): CaptureWindowState => ({
  served: { run: "session:a", name: "Findings", address: "http://localhost:5173" },
  shown: { name: "Findings", address: "http://localhost:5173" },
  studio: null,
  session: { id: "a", title: "Store clock findings" },
  serving: true,
  armed: false,
  framesRefused: 0,
  readiness: "ready",
  queue: { entries: [{ id: "a", name: "Store clock findings", title: "Findings", address: "http://localhost:5173" }], at: 0 },
  ...(held === undefined ? {} : { held }),
});

function drawn(held: "one" | "several" | undefined, sent: { ok: boolean; why?: string } = { ok: true }) {
  let push: (next: CaptureWindowState) => void = () => {};
  const none = async () => null;
  const order: string[] = [];
  const api = {
    read: (onChanged: (next: CaptureWindowState) => void) => {
      push = onChanged;
      onChanged(state(held));
      return () => {};
    },
    arm: none,
    aim: none,
    hold: none,
    release: async () => {},
    save: async () => ({ ok: true }),
    reload: async () => {},
    followRefused: async () => {},
    approve: vi.fn(async () => (order.push("approve"), { ok: true })),
    sendNotes: vi.fn(async () => (order.push("send"), sent)),
    scroll: () => {},
    showAsk: async () => {},
    answerAsk: async () => ({ ok: true }),
    pick: async () => {},
    panel: async () => {},
    openOwner: async () => {},
    walkNext: async () => {},
    walkBack: async () => {},
    walkAutoplay: async () => {},
    walkReplay: async () => {},
    walkGuide: async () => {},
    walkTo: async () => {},
  } satisfies CaptureWindowApi;
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  root.render(<CaptureWindow api={api} />);
  return { api, order, push: (next: CaptureWindowState) => push(next), done: () => (root.unmount(), host.remove()) };
}

test("a save that held says so on the bar, and Approve with nothing held approves at once", async () => {
  const bar = drawn("one");
  await expect.element(page.getByText("Note held in the Job panel")).toBeVisible();
  bar.push(state(undefined));
  await expect.element(page.getByText("Note held in the Job panel")).not.toBeInTheDocument();
  await userEvent.click(page.getByRole("button", { name: "Approve" }));
  await expect.poll(() => bar.order).toEqual(["approve"]);
  bar.done();
});

test("Approve asks while notes are held: going back does nothing, and approving without them sends nothing", async () => {
  const bar = drawn("several");
  await expect.element(page.getByText("Notes held in the Job panel")).toBeVisible();
  await userEvent.click(page.getByRole("button", { name: /^Approve/ }));
  await userEvent.click(page.getByRole("menuitem", { name: "Go back" }));
  expect(bar.order).toEqual([]);
  await userEvent.click(page.getByRole("button", { name: /^Approve/ }));
  await userEvent.click(page.getByRole("menuitem", { name: "Approve without them" }));
  await expect.poll(() => bar.order).toEqual(["approve"]);
  bar.done();
});

test("Send notes and approve sends them first and then approves; a refused send approves nothing and says why", async () => {
  const bar = drawn("one");
  await userEvent.click(page.getByRole("button", { name: /^Approve/ }));
  await userEvent.click(page.getByRole("menuitem", { name: "Send notes and approve" }));
  await expect.poll(() => bar.order).toEqual(["send", "approve"]);
  bar.done();

  const refused = drawn("one", { ok: false, why: "not_connected" });
  await userEvent.click(page.getByRole("button", { name: /^Approve/ }));
  await userEvent.click(page.getByRole("menuitem", { name: "Send notes and approve" }));
  await expect.element(page.getByText("Notes not sent")).toBeVisible();
  expect(refused.order).toEqual(["send"]);
  refused.done();
});
