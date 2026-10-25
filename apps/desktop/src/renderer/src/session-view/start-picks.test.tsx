// A held Job's start strip: Fleet's workflow and landing, shown as it gave them, and a Start that sends the first message as he edited it.

import type { SessionsDraft, Session } from "@armada/screens/src/draft/sessions";
import { createRoot, type Root } from "react-dom/client";
import { flushSync } from "react-dom";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import "../styles/index.css";
import { SessionsFrom } from "../sessions-draft";
import { useStartPicks } from "./StartPicks";

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const NONE: Session[] = [];
const workflows = [{ id: "feature", name: "Feature" }];
const begin = vi.fn(async (..._args: unknown[]) => true);
const draft = { get: () => NONE, subscribe: () => () => undefined, begin, work: { workflows, choose: () => undefined } } as unknown as SessionsDraft;

const held = (start: NonNullable<Session["start"]>): Session =>
  ({ id: "new1", attachments: [], rows: [], turn: { state: "idle" }, work: { proposal: { workflow: "feature" } }, start }) as unknown as Session;

let pressed: ((text: string) => Promise<boolean>) | undefined;
function Strip({ one }: { one: Session }) {
  const starting = useStartPicks(one, [], false);
  pressed = starting?.begin;
  return <div>{starting?.lead}</div>;
}

function mount(one: Session) {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({ root, host });
  flushSync(() => root.render(<SessionsFrom held={draft}><Strip one={one} /></SessionsFrom>));
}

test("Start sends the message as he edited it, with the landing on the card", async () => {
  begin.mockClear();
  mount(held({ asksDelivery: true, picked: { choice: "keep_local", from: "usual" }, message: "Port the clock" }));
  await expect.poll(() => pressed).toBeDefined();
  expect(await pressed!("Port the clock, then the loader")).toBe(true);
  expect(begin).toHaveBeenCalledWith("new1", "keep_local", "Port the clock, then the loader");
});

test("sends no message for a Job whose Fleet gave none to edit", async () => {
  begin.mockClear();
  mount(held({ asksDelivery: false }));
  await expect.poll(() => pressed).toBeDefined();
  await pressed!("typed");
  expect(begin).toHaveBeenCalledWith("new1", undefined, undefined);
});

test("marks a landing from his usual, and one his words named", async () => {
  mount(held({ asksDelivery: true, picked: { choice: "open_pr", from: "usual" } }));
  await expect.element(page.getByRole("img", { name: "Your usual" })).toBeVisible();
  mount(held({ asksDelivery: true, picked: { choice: "open_pr", from: "named" } }));
  await expect.element(page.getByRole("img", { name: "Named in your request" })).toBeVisible();
});

test("marks nothing on a landing Fleet fell back to", async () => {
  mount(held({ asksDelivery: true, picked: { choice: "auto_land", from: "default" } }));
  await expect.element(page.getByRole("combobox", { name: "Delivery" })).toHaveValue("auto_land");
  expect(document.querySelector('[aria-label^="Your usual"], [aria-label="Named in your request"]')).toBeNull();
});
