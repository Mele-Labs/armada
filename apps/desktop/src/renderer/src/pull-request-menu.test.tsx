// A pull request chip's right press offers Open in browser and Copy link wherever the chip is drawn, with no Job or Fleet read behind it.
// Browser: a rendering is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import "./styles/index.css";
import { PullRequestMenuOn } from "./pull-request-menu";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ADDRESS = "https://forge.example/harbor-labs/tidewater/pull/418";
const openLink = vi.fn(async (_address: string) => ({ ok: true }) as const);
const writeText = vi.fn(async (_text: string) => {});

let root: Root | undefined;
let host: HTMLElement | undefined;

beforeEach(async () => {
  openLink.mockClear();
  writeText.mockClear();
  (window as unknown as { armada: unknown }).armada = { openLink };
  Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
  host = document.body.appendChild(document.createElement("div"));
  root = createRoot(host);
  await act(async () =>
    root!.render(
      <PullRequestMenuOn manifestId={undefined} number={418} title="Keep the tide tables a day" address={ADDRESS}>
        {(on) => (
          <button type="button" {...on}>
            #418
          </button>
        )}
      </PullRequestMenuOn>,
    ),
  );
});

afterEach(() => {
  act(() => root?.unmount());
  host?.remove();
});

const chip = () => host!.querySelector("button")!;
const rightPress = () => act(async () => void chip().dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 20, clientY: 20 })));
const item = (name: string) => [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find((one) => one.textContent === name);

test("a right press on the chip offers Open in browser and Copy link", async () => {
  await rightPress();
  expect(item("Open in browser")).toBeDefined();
  expect(item("Copy link")).toBeDefined();
});

test("Copy link puts the pull request's address on the clipboard, and Open in browser opens it outside Armada", async () => {
  await rightPress();
  await act(async () => item("Copy link")!.click());
  expect(writeText).toHaveBeenCalledWith(ADDRESS);
  await rightPress();
  await act(async () => item("Open in browser")!.click());
  expect(openLink).toHaveBeenCalledWith(ADDRESS);
});
