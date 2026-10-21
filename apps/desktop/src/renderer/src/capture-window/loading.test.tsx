// The walk window's bar while the page loads: Loading in the status row, so a page not drawn yet does not read as broken.
// `docs/practices/capture-window.md`, *A page the window cannot show*.

import { createRoot } from "react-dom/client";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import type { CaptureWindowState } from "../../../shared/capture-window";
import "../styles/index.css";
import { CaptureWindow, type CaptureWindowApi } from "./CaptureWindow";

const state = (loading: boolean, queued: boolean): CaptureWindowState => ({
  served: { run: "session:a", name: "Findings", address: "http://localhost:5173" },
  shown: { name: "Findings", address: "http://localhost:5173" },
  studio: null,
  session: { id: "a", title: "Store clock findings" },
  serving: true,
  armed: false,
  framesRefused: 0,
  ...(queued ? { queue: { entries: [{ id: "a", name: "Store clock findings", title: "Findings", address: "http://localhost:5173" }], at: 0 } } : {}),
  ...(loading ? { loading: true as const } : {}),
});

function drawn(queued: boolean) {
  let push: (next: CaptureWindowState) => void = () => {};
  const none = async () => null;
  const api = {
    read: (onChanged: (next: CaptureWindowState) => void) => {
      push = onChanged;
      onChanged(state(true, queued));
      return () => {};
    },
    arm: none,
    aim: none,
    hold: none,
    release: async () => {},
    save: async () => ({ ok: true }),
    reload: async () => {},
    followRefused: async () => {},
    approve: async () => ({ ok: true }),
    sendNotes: async () => ({ ok: true }),
    scroll: () => {},
    showAsk: async () => {},
    answerAsk: async () => ({ ok: true }),
    pick: async () => {},
    panel: async (_open: boolean) => {},
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
  return { push: (next: CaptureWindowState) => push(next), done: () => (root.unmount(), host.remove()) };
}

for (const queued of [true, false]) {
  test(`the ${queued ? "queued" : "one walk's"} bar says Loading while the page loads, and stops once it has`, async () => {
    const bar = drawn(queued);
    await expect.element(page.getByText("Loading", { exact: true })).toBeVisible();
    bar.push(state(false, queued));
    await expect.element(page.getByText("Loading", { exact: true })).not.toBeInTheDocument();
    bar.done();
  });
}
