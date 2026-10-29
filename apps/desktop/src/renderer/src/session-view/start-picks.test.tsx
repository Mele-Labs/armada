// A held Job's start strip: Fleet's workflow and landing, shown as it gave them, and a Start that sends the first message as he edited it.

import type { SessionsDraft, Session } from "@armada/screens/src/draft/sessions";
import { createRoot, type Root } from "react-dom/client";
import { flushSync } from "react-dom";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import "../styles/index.css";
import { SessionsFrom } from "../sessions-draft";
import { useStartPicks } from "./StartPicks";

// `workflows.off` as Fleet's settings list holds it; a test sets it before it mounts.
const settings = vi.hoisted(() => ({ off: undefined as unknown }));
vi.mock("../use-setting", () => ({ useSetting: () => settings.off }));

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  settings.off = undefined;
  draft.work!.workflows = workflows;
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const NONE: Session[] = [];
const workflows = [{ id: "feature", name: "Feature" }];
const begin = vi.fn(async (..._args: unknown[]) => true);
const choose = vi.fn();
const draft = { get: () => NONE, subscribe: () => () => undefined, begin, work: { workflows, choose } } as unknown as SessionsDraft;

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

test("sends what he typed for a Job whose Fleet held no message, and nothing for an empty box", async () => {
  begin.mockClear();
  mount(held({ asksDelivery: false }));
  await expect.poll(() => pressed).toBeDefined();
  await pressed!("typed");
  expect(begin).toHaveBeenLastCalledWith("new1", undefined, "typed");
  await pressed!("  ");
  expect(begin).toHaveBeenLastCalledWith("new1", undefined, undefined);
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

test("None is his pick: the Job goes custom, stays custom, and has no delivery to settle", async () => {
  choose.mockClear();
  begin.mockClear();
  mount(held({ asksDelivery: true, picked: { choice: "auto_land", from: "default" } }));
  await page.getByRole("combobox", { name: "Workflow" }).selectOptions("none");
  expect(choose).toHaveBeenCalledWith("new1", null);
  await expect.element(page.getByRole("combobox", { name: "Workflow" })).toHaveValue("none");
  expect(document.querySelector('select[aria-label="Delivery"]')).toBeNull();
  await pressed!("typed");
  expect(begin).toHaveBeenLastCalledWith("new1", undefined, "typed");
});

const reading = (start: NonNullable<Session["start"]>): Session =>
  ({ id: "new1", attachments: [], rows: [], turn: { state: "idle" }, work: { proposal: { workflow: null, reading: true } }, start }) as unknown as Session;

test("Start while the request is read goes with no landing and no workflow chosen, for Fleet to take the reading's", async () => {
  begin.mockClear();
  choose.mockClear();
  mount(reading({ asksDelivery: true, picked: { choice: "keep_local", from: "usual" }, message: "Port the clock" }));
  await expect.poll(() => pressed).toBeDefined();
  expect(await pressed!("Port the clock")).toBe(true);
  expect(begin).toHaveBeenCalledExactlyOnceWith("new1", undefined, "Port the clock");
  expect(choose).not.toHaveBeenCalled();
});

test("a second Start while the first is queued sends nothing, and the card says Starting", async () => {
  begin.mockClear();
  mount(reading({ asksDelivery: true, queued: true }));
  await expect.poll(() => pressed).toBeDefined();
  expect(await pressed!("Port the clock")).toBe(false);
  expect(begin).not.toHaveBeenCalled();
  await expect.element(page.getByRole("img", { name: "Starting" })).toBeVisible();
});

test("a card not yet queued does not say Starting", async () => {
  mount(held({ asksDelivery: true }));
  await expect.element(page.getByRole("combobox", { name: "Workflow" })).toBeVisible();
  expect(document.querySelector('[aria-label="Starting"]')).toBeNull();
});

const MANY = [{ id: "feature", name: "Feature" }, { id: "bug", name: "Bug" }, { id: "prototype", name: "Prototype" }];
const heldIn = (manifestId: string): Session => ({ ...held({ asksDelivery: true }), manifestId }) as unknown as Session;
const offered = () => [...document.querySelectorAll<HTMLOptionElement>('select[aria-label="Workflow"] option')].map((one) => one.textContent);

test("the Workflow picker leaves out a workflow that is off for the machine, and keeps None", async () => {
  draft.work!.workflows = MANY;
  settings.off = { machine: ["prototype"], repositories: {} };
  mount(heldIn("armada"));
  await expect.poll(offered).toEqual(["None", "Feature", "Bug"]);
});

test("the Workflow picker leaves out a workflow that is off for the Job's repository, and not one that is off for another", async () => {
  draft.work!.workflows = MANY;
  settings.off = { machine: [], repositories: { armada: ["bug"], other: ["feature"] } };
  mount(heldIn("armada"));
  await expect.poll(offered).toEqual(["None", "Feature", "Prototype"]);
});

test("with nothing off the picker lists every workflow, and a value that does not read leaves all of them on", async () => {
  draft.work!.workflows = MANY;
  settings.off = "bug";
  mount(heldIn("armada"));
  await expect.poll(offered).toEqual(["None", "Feature", "Bug", "Prototype"]);
});
