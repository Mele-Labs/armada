// Mod, from a spot pointed at: one operation on Fleet carrying the note a Report would carry, then the Job opened.

import { afterEach, expect, test, vi } from "vitest";

import { forgetAllKept, readKept } from "@armada/components";

import type { ModSessionStarted, StartModSession } from "@armada/protocol";

import type { BridgeApi } from "../../shared/api";
import { startEmptyModJob, startModJob } from "./mod-job";
import { askComposerFocus, composerFocusAsked } from "./session-focus";
import { aimOf } from "./session-view/mod-aim";

type Started = Awaited<ReturnType<BridgeApi["startModSession"]>>;

const heard: string[] = [];
const onOpen = (event: Event): void => void heard.push(`open ${(event as CustomEvent<string>).detail}`);
const onTell = (event: Event): void => void heard.push(`tell ${(event as CustomEvent<{ sentence: string }>).detail.sentence}`);

afterEach(() => {
  heard.length = 0;
  window.removeEventListener("armada:open-session-in-cockpit", onOpen);
  window.removeEventListener("armada:tell", onTell);
  document.querySelectorAll("[data-pointed]").forEach((one) => one.remove());
  forgetAllKept();
  Object.defineProperty(window, "armada", { value: undefined, configurable: true });
});

function fleetAnswering(answer: Started) {
  const startModSession = vi.fn(async (_start: StartModSession): Promise<Started> => answer);
  Object.defineProperty(window, "armada", { value: { startModSession }, configurable: true });
  window.addEventListener("armada:open-session-in-cockpit", onOpen);
  window.addEventListener("armada:tell", onTell);
  return startModSession;
}

function pointed(): HTMLElement {
  const one = document.createElement("button");
  one.setAttribute("data-pointed", "");
  one.setAttribute("aria-label", "Usage meter");
  one.textContent = "Usage";
  document.body.append(one);
  return one;
}

const started = (extra: Partial<ModSessionStarted> = {}): Started => ({ ok: true, value: { session_id: "01MOD", title: "Usage meter", ...extra } });

test("Mod posts the note a Report would carry, with no Session, titled Mod and the name, and opens the Job", async () => {
  const start = fleetAnswering(started());
  await startModJob(pointed());

  const body = start.mock.calls[0]![0];
  expect(body.title).toBe("Mod Usage meter");
  expect(body.note!.said).toBe("");
  expect(body.note!.address).toBe(window.location.href);
  expect(body.note!.place.element).toMatchObject({ tag: "button", label: "Usage meter" });
  expect(body.note!.place.selector).not.toBe("");
  expect(body.note).not.toHaveProperty("session_id");
  expect(heard).toEqual(["open 01MOD"]);
});

test("a Job row passes the name it already has", async () => {
  const start = fleetAnswering(started());
  await startModJob(pointed(), "Fix the flaky store test");
  expect(start.mock.calls[0]![0].title).toBe("Mod Fix the flaky store test");
});

test("a refusal is told and no Job is opened", async () => {
  fleetAnswering({ ok: false, outcome: { ok: false, why: "not_connected" } });
  await startModJob(pointed());
  expect(heard).toHaveLength(1);
  expect(heard[0]).toMatch(/^tell Mod Job not started: /);
});

test("a first message Fleet could not send is told, and the Job is still opened", async () => {
  fleetAnswering(started({ send_failed: "the Job was refused" }));
  await startModJob(pointed());
  expect(heard).toEqual(["tell Mod Job not told what was pointed at: the Job was refused", "open 01MOD"]);
});

test("Create mod posts an empty start, with no note and no words, focuses the box and opens the Job", async () => {
  const start = fleetAnswering(started());
  await startEmptyModJob();

  expect(start).toHaveBeenCalledTimes(1);
  expect(start.mock.calls[0]![0]).toEqual({});
  expect(composerFocusAsked("01MOD")).toBe(true);
  expect(heard).toEqual(["open 01MOD"]);
  expect(aimOf("01MOD")).toBeUndefined();
  expect(readKept("session:01MOD")).toBeUndefined();
});

test("Change it seeds the Job's aim and the words its box opens on", async () => {
  fleetAnswering(started());
  await startEmptyModJob({ aim: "layout", said: "Change the Dusk layout: " });

  expect(aimOf("01MOD")).toBe("layout");
  expect(readKept<{ text: string }>("session:01MOD")?.text).toBe("Change the Dusk layout: ");
  expect(composerFocusAsked("01MOD")).toBe(true);
  expect(heard).toEqual(["open 01MOD"]);
});

test("a refused Create mod is told, and no Job is opened, focused or seeded", async () => {
  // The ask is held for a moment and by one id, so an earlier test's is moved off this one's.
  askComposerFocus("elsewhere");
  fleetAnswering({ ok: false, outcome: { ok: false, why: "not_connected" } });
  await startEmptyModJob({ aim: "theme", said: "x" });

  expect(heard).toHaveLength(1);
  expect(heard[0]).toMatch(/^tell Mod Job not started: /);
  expect(aimOf("01MOD")).toBeUndefined();
  expect(readKept("session:01MOD")).toBeUndefined();
  expect(composerFocusAsked("01MOD")).toBe(false);
});

test("Create mod with no Fleet connected does nothing", async () => {
  await startEmptyModJob();
  expect(heard).toEqual([]);
});
