// Mod, from a spot pointed at: one operation on Fleet carrying the note a Report would carry, then the Job opened.

import { afterEach, expect, test, vi } from "vitest";

import type { ModSessionStarted, StartModSession } from "@armada/protocol";

import type { BridgeApi } from "../../shared/api";
import { startModJob } from "./mod-job";

type Started = Awaited<ReturnType<BridgeApi["startModSession"]>>;

const heard: string[] = [];
const onOpen = (event: Event): void => void heard.push(`open ${(event as CustomEvent<string>).detail}`);
const onTell = (event: Event): void => void heard.push(`tell ${(event as CustomEvent<{ sentence: string }>).detail.sentence}`);

afterEach(() => {
  heard.length = 0;
  window.removeEventListener("armada:open-session-in-cockpit", onOpen);
  window.removeEventListener("armada:tell", onTell);
  document.querySelectorAll("[data-pointed]").forEach((one) => one.remove());
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
  expect(body.note.said).toBe("");
  expect(body.note.address).toBe(window.location.href);
  expect(body.note.place.element).toMatchObject({ tag: "button", label: "Usage meter" });
  expect(body.note.place.selector).not.toBe("");
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
