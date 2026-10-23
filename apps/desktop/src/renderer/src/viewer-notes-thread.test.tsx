// A sent notes message as the thread draws it: each note's heading is a press that opens what the note named again.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { NotedMessage, notedOf, notesMessage } from "@armada/components";
import type { HeldNote, NotedRef } from "@armada/components";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const NOTES: HeldNote[] = [
  { about: { kind: "file", path: "/Users/me/repo/crates/fleet/src/retry.rs", from: 12, to: 14, quote: "a\nb\nc" }, text: "Too many" },
  {
    about: {
      kind: "sketch",
      title: "Retry flow",
      parts: [
        { id: "n3", label: "Backoff" },
        { id: "n4", label: "Cap" },
      ],
    },
    text: "Cap is low",
  },
  { about: { kind: "image", name: "shot.png", path: "/tmp/shot.png", region: { x: 0.1, y: 0.2, w: 0.5, h: 0.25 }, crop: "data:image/png;base64,AAAA" }, text: "Misaligned" },
];

function thread(open: (ref: NotedRef) => void) {
  const blocks = notedOf(notesMessage(NOTES).text)!;
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(<NotedMessage blocks={blocks} opens={{ open, root: "/Users/me/repo" }} />);
}

test("a file's heading opens the file at its lines", async () => {
  const open = vi.fn();
  thread(open);
  await userEvent.click(page.getByRole("button", { name: "crates/fleet/src/retry.rs:12-14" }));

  expect(open).toHaveBeenCalledOnce();
  expect(open).toHaveBeenCalledWith({ kind: "file", path: "/Users/me/repo/crates/fleet/src/retry.rs", from: 12, to: 14 });
});

test("a sketch's heading opens the sketch with the parts the note named", async () => {
  const open = vi.fn();
  thread(open);
  await userEvent.click(page.getByRole("button", { name: "Retry flow: Backoff, Cap" }));

  expect(open).toHaveBeenCalledOnce();
  expect(open).toHaveBeenCalledWith({
    kind: "sketch",
    title: "Retry flow",
    parts: [
      { id: "n3", label: "Backoff" },
      { id: "n4", label: "Cap" },
    ],
  });
});

test("a picture's heading opens the picture with the region the note marked", async () => {
  const open = vi.fn();
  thread(open);
  await userEvent.click(page.getByRole("button", { name: "shot.png, region" }));

  expect(open).toHaveBeenCalledOnce();
  const ref = open.mock.calls[0]![0] as NotedRef;
  expect(ref.kind).toBe("image");
  if (ref.kind !== "image") return;
  expect(ref.path).toBe("/tmp/shot.png");
  expect(ref.region!.x).toBeCloseTo(0.1);
  expect(ref.region!.y).toBeCloseTo(0.2);
  expect(ref.region!.w).toBeCloseTo(0.5);
  expect(ref.region!.h).toBeCloseTo(0.25);
});

test("each note is listed under its number with its words, and the headings are the only presses", async () => {
  thread(vi.fn());
  await expect.element(page.getByRole("list", { name: "Notes" })).toBeVisible();
  await expect.element(page.getByText("Too many")).toBeVisible();
  await expect.element(page.getByText("Cap is low")).toBeVisible();
  await expect.element(page.getByText("Misaligned")).toBeVisible();
  expect(page.getByRole("button").elements()).toHaveLength(3);
});
