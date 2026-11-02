// The Logbook and Retros pages on the common look, drawn for real through `App` (`docs/concepts/logbook.md`): the panels share one
// top and one bottom, 12px above the window's foot, and the key caps on the entry's acts are not drawn until ⌘ is held. Geometry
// and display only: no colour is asserted, since the look is the owner's to change.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toPlace, toSystemPage, unmountAfterEach } from "./testing";

unmountAfterEach();

const FOOT = 12;
const SLICES = { slices: ["core", "lessons"] } as const;

const px = (n: number) => Math.round(n);
/** The window the app is drawn in, which the stylesheet sizes. */
const windowRect = () => document.getElementById("root")!.getBoundingClientRect();

/** The panels of the open page: the list and the open item or entry beside it. */
function panels(): { list: DOMRect; detail: DOMRect } {
  const list = document.querySelector(".armada-logbook__log");
  const detail = document.querySelector(".armada-logbook__detail");
  expect(list, "the list's panel").not.toBeNull();
  expect(detail, "the open one's panel").not.toBeNull();
  return { list: list!.getBoundingClientRect(), detail: detail!.getBoundingClientRect() };
}

function sharedEdges() {
  const { list, detail } = panels();
  expect(list.width, "the panels sit side by side").toBeLessThan(windowRect().width);
  expect(px(list.top)).toBe(px(detail.top));
  expect(px(list.bottom)).toBe(px(detail.bottom));
  expect(px(windowRect().bottom - list.bottom)).toBe(FOOT);
}

test("the Logbook's list and entry share one top and one bottom, 12px from the window's foot", async () => {
  mount("logbook");
  await onScreen();
  await toPlace("Logbook");
  await expect.element(page.getByRole("grid", { name: "Entries" })).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Entry" })).toBeVisible();
  sharedEdges();
});

test("Retros' list and item share one top and one bottom, 12px from the window's foot, as the Logbook's do", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();
  await toSystemPage("Retros");
  await expect.element(page.getByRole("grid", { name: "Retros" })).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Retro item" })).toBeVisible();
  sharedEdges();
});

test("the key caps on an entry's acts are not drawn until ⌘ is held", async () => {
  mount("logbook");
  await onScreen();
  await toPlace("Logbook");
  await expect.element(page.getByRole("region", { name: "Entry" }).getByRole("button", { name: /^Edit/ })).toBeVisible();
  const caps = () => [...document.querySelectorAll<HTMLElement>(".armada-logbook__detail .armada-kbd")];
  const drawn = (cap: HTMLElement) => getComputedStyle(cap).display !== "none";
  // Edit, Lock and Forget each hold one.
  expect(caps().length).toBeGreaterThanOrEqual(3);
  expect(caps().filter(drawn)).toHaveLength(0);

  await userEvent.keyboard("{Meta>}");
  try {
    await expect.poll(() => caps().filter((cap) => !drawn(cap)).length).toBe(0);
  } finally {
    await userEvent.keyboard("{/Meta}");
  }
  await expect.poll(() => caps().filter(drawn).length).toBe(0);
});
