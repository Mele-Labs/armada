// The walk window's bar says whether the page is ready, from the state main pushes it.

import { createRoot } from "react-dom/client";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import type { CaptureWindowState } from "../../../shared/capture-window";
import "../styles/index.css";
import { CaptureWindow, type CaptureWindowApi } from "./CaptureWindow";

const state = (readiness?: "ready" | "working"): CaptureWindowState => ({
  served: { run: "session:a", name: "Findings", address: "http://localhost:5173" },
  shown: { name: "Findings", address: "http://localhost:5173" },
  studio: null,
  session: { id: "a", title: "Store clock findings" },
  serving: true,
  armed: false,
  framesRefused: 0,
  ...(readiness === undefined ? {} : { readiness }),
});

test("the bar names the page ready or still changing, and Approve waits while it changes", async () => {
  let push: (next: CaptureWindowState) => void = () => {};
  const none = async () => null;
  const api = {
    read: (onChanged: (next: CaptureWindowState) => void) => {
      push = onChanged;
      onChanged(state("ready"));
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
    walkAutoplay: async () => {},
    walkReplay: async () => {},
  } satisfies CaptureWindowApi;
  const host = document.body.appendChild(document.createElement("div"));
  createRoot(host).render(<CaptureWindow api={api} />);

  await expect.element(page.getByRole("img", { name: "Ready to walk" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Approve" })).toBeEnabled();

  push(state("working"));
  await expect.element(page.getByRole("img", { name: "Still changing" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Not ready" })).toBeDisabled();
});
