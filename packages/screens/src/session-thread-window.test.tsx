// A Session that ran for days is thousands of rows. The thread draws the newest items and
// draws older ones as the person scrolls up to them, or on a press, so typing beside it stays quick.

import { useState } from "react";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { SessionThread } from "@armada/components";
import type { SessionThreadRow } from "@armada/components";
import { mount, unmount } from "./mounted";

afterEach(unmount);

const rows = (count: number): SessionThreadRow[] =>
  Array.from(
    {
      length: count,
    },
    (_, at) => ({
      id: `r${at}`,
      at: "10:00:00",
      kind: "message",
      from: "agent",
      text: `row ${at} said`,
    }),
  );

// A window's height, as the Job's pane gives it: a thread with no height of its own would be all on screen.
const thread = (sessionId: string, count: number) => (
  <div
    style={{
      height: 600,
      display: "flex",
      flexDirection: "column",
    }}
  >
    <SessionThread
      sessionId={sessionId}
      rows={rows(count)}
      onAnswer={() => {}}
      onOpenSession={() => {}}
    />
  </div>
);

const drawn = () => document.querySelectorAll(".armada-thread-rows > li").length;
const scroller = () => document.querySelector<HTMLElement>(".armada-session-thread__rows")!;

test("opens on the newest 40 rows, with the older ones not drawn", async () => {
  mount(thread("a", 400));

  await expect.element(page.getByText("row 399 said")).toBeVisible();
  expect(drawn()).toBe(41);
  expect(page.getByText("row 359 said").query()).toBeNull();
  expect(page.getByText("row 360 said").query()).not.toBeNull();
});

test("scrolling up to the top draws the next 40, and what he was reading stays where it was", async () => {
  mount(thread("a", 400));
  await expect.element(page.getByText("row 399 said")).toBeVisible();

  const one = scroller();
  one.scrollTop = 0;
  // Unpinned at once: a resize before the browser's own scroll event would put him back at the end.
  one.dispatchEvent(new Event("scroll"));
  const reading = page.getByText("row 360 said").element();
  const top = reading.getBoundingClientRect().top;

  await expect.poll(() => page.getByText("row 320 said").query()).not.toBeNull();
  expect(Math.abs(reading.getBoundingClientRect().top - top)).toBeLessThan(2);
});

test("a row arriving while he reads higher up keeps every row he can see drawn", async () => {
  let add: () => void = () => {};
  function Growing() {
    const [count, setCount] = useState(400);
    add = () => setCount((now) => now + 1);
    return thread("a", count);
  }
  mount(<Growing />);
  await expect.element(page.getByText("row 399 said")).toBeVisible();

  const one = scroller();
  one.scrollTop = one.scrollHeight / 2;
  one.dispatchEvent(new Event("scroll"));
  await new Promise((settled) => setTimeout(settled, 300));
  const firstDrawn = () =>
    document.querySelector<HTMLElement>(
      ".armada-thread-rows > li:not(.armada-session-thread__earlier)",
    )!;
  const said = firstDrawn().textContent;
  const top = firstDrawn().getBoundingClientRect().top;

  add();
  await expect.element(page.getByText("row 400 said")).toBeInTheDocument();
  expect(firstDrawn().textContent).toBe(said);
  expect(Math.abs(firstDrawn().getBoundingClientRect().top - top)).toBeLessThan(2);
});

test("Show earlier draws the next 40 from the keyboard", async () => {
  mount(thread("a", 400));
  await expect.element(page.getByText("row 399 said")).toBeVisible();

  // The element's own press: coming into view draws rows above it, so it never holds still for a pointer.
  (
    page
      .getByRole("button", {
        name: "Show earlier",
      })
      .element() as HTMLButtonElement
  ).click();
  await expect.poll(() => page.getByText("row 320 said").query()).not.toBeNull();
});

test("a thread that fits draws no Show earlier", async () => {
  mount(thread("a", 20));

  await expect.element(page.getByText("row 19 said")).toBeVisible();
  expect(
    page
      .getByRole("button", {
        name: "Show earlier",
      })
      .query(),
  ).toBeNull();
});
