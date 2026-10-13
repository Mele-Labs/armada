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
