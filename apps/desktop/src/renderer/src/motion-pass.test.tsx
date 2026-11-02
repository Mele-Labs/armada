// What the motion pass holds: a closed sheet eases away before it leaves, a working Job and a waiting place draw the working
// mark, and a row arriving in a thread rises once while the history it opened with does not.

import { createRoot, type Root } from "react-dom/client";
import { act } from "react";
import { afterEach, expect, onTestFinished, test } from "vitest";

import "./styles/index.css";
import {
  SessionMark,
  SessionThread,
  Sheet,
  SkeletonText,
  type SessionThreadRow,
} from "@armada/components";

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];

function mount(): {
  root: Root;
  host: HTMLElement;
} {
  const host = document.createElement("div");
  document.body.append(host);
  const one = {
    root: createRoot(host),
    host,
  };
  roots.push(one);
  return one;
}

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

test("a closed sheet stays drawn as leaving for its exit, then goes", async () => {
  // Reduced motion zeroes the exit, and a headless browser may ask for it: the test names its own.
  document.documentElement.style.setProperty("--duration-exit", "300ms");
  onTestFinished(() => void document.documentElement.style.removeProperty("--duration-exit"));
  const { root } = mount();
  const sheet = (open: boolean) => (
    <Sheet open={open} title="Checks" width={undefined} onResize={() => {}} onClose={() => {}}>
      body
    </Sheet>
  );
  await act(async () => root.render(sheet(true)));
  expect(document.querySelector(".armada-sheet-scrim")).not.toBeNull();
  expect(document.querySelector(".armada-sheet-scrim[data-leaving]")).toBeNull();

  await act(async () => root.render(sheet(false)));
  expect(document.querySelector(".armada-sheet-scrim[data-leaving]")).not.toBeNull();

  await expect
    .poll(() => document.querySelector(".armada-sheet-scrim"), {
      timeout: 2000,
    })
    .toBeNull();
});

test("a loading placeholder and a working Job draw the working mark", async () => {
  const { root, host } = mount();
  await act(async () =>
    root.render(
      <>
        <SkeletonText label="Loading checks" />
        <SessionMark state="working" said="Working" />
        <SessionMark state="idle" said="Idle" />
      </>,
    ),
  );
  expect(host.querySelector(".armada-skeleton-text .armada-working-mark")).not.toBeNull();
  expect(host.querySelectorAll(".armada-session-mark .armada-working-mark")).toHaveLength(1);
});

test("a row arriving in a thread rises once, and the history it opened with does not", async () => {
  const { root, host } = mount();
  const you = (id: string): SessionThreadRow => ({
    kind: "message",
    from: "you",
    id,
    text: id,
    at: "now",
  });
  const thread = (ids: string[]) => (
    <SessionThread rows={ids.map(you)} onAnswer={() => {}} onOpenSession={() => {}} />
  );
  await act(async () =>
    root.render(
      thread([
        "a",
        "b",
      ]),
    ),
  );
  expect(host.querySelectorAll("[data-arrived]")).toHaveLength(0);

  await act(async () =>
    root.render(
      thread([
        "a",
        "b",
        "c",
      ]),
    ),
  );
  expect(host.querySelectorAll("[data-arrived]")).toHaveLength(1);
});
