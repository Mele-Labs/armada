// The walk's controls sit on the walk window's bar: the row draws from the state main pushes, and
// its presses call the bar's api — nothing else is reached from here.

import { createRoot } from "react-dom/client";
import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { CaptureWindowState } from "../../../shared/capture-window";
import "../styles/index.css";
import { CaptureWindow, type CaptureWindowApi } from "./CaptureWindow";

const state = (walk?: CaptureWindowState["walk"]): CaptureWindowState => ({
  served: { run: "session:a", name: "Findings", address: "http://localhost:5173" },
  shown: { name: "Findings", address: "http://localhost:5173" },
  studio: null,
  session: { id: "a", title: "Store clock findings" },
  serving: true,
  armed: false,
  framesRefused: 0,
  ...(walk === undefined ? {} : { walk }),
});

const step = { name: "walk-variations", at: 2, of: 4, said: "Open the menu", autoplay: false, ready: true, done: false };

test("the bar draws a pushed walk and its presses call the api", async () => {
  let push: (next: CaptureWindowState) => void = () => {};
  const none = async () => null;
  const api = {
    read: (onChanged: (next: CaptureWindowState) => void) => {
      push = onChanged;
      onChanged(state());
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
    scroll: () => {},
    showAsk: async () => {},
    answerAsk: async () => ({ ok: true }),
    pick: async () => {},
    panel: async () => {},
    openOwner: async () => {},
    walkNext: vi.fn(async () => {}),
    walkBack: vi.fn(async () => {}),
    walkAutoplay: vi.fn(async (_on: boolean) => {}),
    walkReplay: vi.fn(async () => {}),
    walkGuide: async (_shown: boolean) => {},
  } satisfies CaptureWindowApi;
  const host = document.body.appendChild(document.createElement("div"));
  createRoot(host).render(<CaptureWindow api={api} />);

  // No walk on the page, no row.
  await expect.element(page.getByRole("button", { name: "Reload" })).toBeVisible();
  await expect.element(page.getByText("Open the menu")).not.toBeInTheDocument();

  push(state(step));
  await expect.element(page.getByText("Step 2 of 4")).toBeVisible();
  await expect.element(page.getByText("Open the menu")).toBeVisible();

  await page.getByRole("button", { name: "Next" }).click();
  expect(api.walkNext).toHaveBeenCalledTimes(1);
  await page.getByRole("button", { name: "Back" }).click();
  expect(api.walkBack).toHaveBeenCalledTimes(1);
  await page.getByRole("button", { name: "Replay" }).click();
  expect(api.walkReplay).toHaveBeenCalledTimes(1);
  await page.getByText("Autoplay", { exact: true }).click();
  expect(api.walkAutoplay).toHaveBeenCalledWith(true);

  // A key pressed while the bar has focus acts on the walk, as it does in the page.
  await page.getByRole("button", { name: "Replay" }).click();
  await userEvent.keyboard("{ArrowRight}");
  expect(api.walkNext).toHaveBeenCalledTimes(2);

  // The page that stops being a walk takes the row away.
  push(state());
  await expect.element(page.getByText("Open the menu")).not.toBeInTheDocument();
});

test("in the walk window, Next and Back stay where they are whatever else the row says", async () => {
  let push: (next: CaptureWindowState) => void = () => {};
  const none = async () => null;
  const api = {
    read: (onChanged: (next: CaptureWindowState) => void) => {
      push = onChanged;
      onChanged(state());
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
    scroll: () => {},
    showAsk: async () => {},
    answerAsk: async () => ({ ok: true }),
    pick: async () => {},
    panel: async () => {},
    openOwner: async () => {},
    walkNext: async () => {},
    walkBack: async () => {},
    walkAutoplay: async (_on: boolean) => {},
    walkReplay: async () => {},
    walkGuide: async (_shown: boolean) => {},
  } satisfies CaptureWindowApi;
  // The window above is still mounted: this one is the only bar on the page.
  document.body.replaceChildren();
  const host = document.body.appendChild(document.createElement("div"));
  createRoot(host).render(<CaptureWindow api={api} />);

  const queued = (walk: CaptureWindowState["walk"], ask?: CaptureWindowState["ask"]): CaptureWindowState => ({
    ...state(walk),
    queue: { at: 0, entries: [{ id: "a", name: "Findings", title: "Findings", address: "http://localhost:5173" }] },
    ...(ask === undefined ? {} : { ask }),
  });
  const folded = { id: "q", question: "Which of the two reads better here?", options: [], from: "Store clock findings", shown: false };
  const where = () => ({
    next: document.querySelector<HTMLElement>("button:has(svg.lucide-chevron-right)")!.getBoundingClientRect().left,
    back: document.querySelector<HTMLElement>("button:has(svg.lucide-chevron-left)")!.getBoundingClientRect().left,
  });

  await expect.element(page.getByRole("button", { name: "Reload" })).toBeVisible();
  push(queued(step));
  await expect.element(page.getByText("Open the menu")).toBeVisible();
  const bare = where();

  push(queued(step, folded));
  await expect.element(page.getByRole("button", { name: "Show question" })).toBeVisible();
  expect(where()).toEqual(bare);

  push(queued({ ...step, said: "Open the menu and read the long line of words that this caption is made of, which runs on" }, folded));
  await expect.element(page.getByRole("button", { name: "Show question" })).toBeVisible();
  expect(where()).toEqual(bare);
});
