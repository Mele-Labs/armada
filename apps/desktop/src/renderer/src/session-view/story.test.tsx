// The Job's story in the right column: every zone open at every stage, what a press on a band holds until the Job is reset, how the
// whole-blueprint key swaps the Now view, and which keys mean what. Browser: a fold is read off a rendering, since it lives in a store a component hears.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import type { Session, SessionAttachment, WorkGroup } from "@armada/screens/src/draft/sessions";

import { bandOf, isArtifactsKey, isLedgerKey, isWholeKey, pressBand, pressWhole, resetFolds, useFold, useWhole, type Area } from "./story";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
const used = new Set<string>();
afterEach(() => {
  for (const id of used) resetFolds(id);
  used.clear();
  localStorage.clear();
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const pull = (state: "draft" | "open" | "merged"): SessionAttachment => ({ kind: "pull_request", number: 7, title: "Retry the lease", branch: "armada/retry", address: "https://forge/7", checks: { state: "passed" }, state, auto: false });
const group = (...states: string[]): WorkGroup => ({ id: "g1", title: "Retry", agent: "rust-engineer", paths: [], tasks: states.map((state, at) => ({ id: `t${at}`, text: `Task ${at}`, state })) }) as unknown as WorkGroup;

/** A Job at a stage: a plan drafted, a plan approved with work left, or one built (every task done). The stage names where it is; it opens and folds nothing. */
type At = "drafted" | "building" | "built";
function job(id: string, at: At, attachments: SessionAttachment[] = []): Session {
  const groups = [group(...(at === "built" ? ["done", "done"] : ["done", "open"]))];
  return { id, attachments, rows: [], turn: { state: "idle" }, work: { plan: { scope: [], approved: at !== "drafted", groups } } } as unknown as Session;
}

const AREAS: Area[] = ["blueprint", "done", "now", "next"];

function Probe({ session }: { session: Session }) {
  return (
    <>
      {AREAS.map((area) => (
        <Zone key={area} session={session} area={area} />
      ))}
      <Whole session={session.id} />
    </>
  );
}
function Zone({ session, area }: { session: Session; area: Area }) {
  const fold = useFold(session, area);
  return <button type="button" data-area={area} data-open={fold.open} data-current={fold.current} onClick={fold.toggle} />;
}
function Whole({ session }: { session: string }) {
  const { whole, toggle } = useWhole(session);
  return <button type="button" data-whole={whole} onClick={toggle} />;
}

function mount(session: Session) {
  used.add(session.id);
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Probe session={session} />));
  roots.push({ root, host });
  return {
    show: (next: Session) => act(() => root.render(<Probe session={next} />)),
    open: () => Object.fromEntries(AREAS.map((area) => [area, host.querySelector(`[data-area="${area}"]`)?.getAttribute("data-open") === "true"])),
    current: () => Object.fromEntries(AREAS.map((area) => [area, host.querySelector(`[data-area="${area}"]`)?.getAttribute("data-current") === "true"])),
    whole: () => host.querySelector("[data-whole]")?.getAttribute("data-whole") === "true",
    press: (area: Area) => act(() => host.querySelector<HTMLElement>(`[data-area="${area}"]`)!.click()),
  };
}

const ALL_OPEN = { blueprint: true, done: true, now: true, next: true };

test("every zone is open at every stage: drafted, building, and built", () => {
  expect(mount(job("fold-drafted", "drafted")).open()).toEqual(ALL_OPEN);
  expect(mount(job("fold-building", "building")).open()).toEqual(ALL_OPEN);
  expect(mount(job("fold-built", "built", [pull("open")])).open()).toEqual(ALL_OPEN);
});

test("the stage only marks which panel the Job is at, and Now is always current", () => {
  expect(mount(job("current-building", "building")).current()).toEqual({ blueprint: false, done: false, now: true, next: false });
});

test("a press on a band's key folds an open zone and opens a folded one", () => {
  const session = job("press-band", "building");
  const view = mount(session);
  act(() => pressBand(session, "next"));
  act(() => pressBand(session, "done"));
  expect(view.open()).toEqual({ ...ALL_OPEN, done: false, next: false });
  act(() => pressBand(session, "next"));
  expect(view.open().next).toBe(true);
});

test("the band's own press folds and opens it the same way", () => {
  const view = mount(job("press-toggle", "building"));
  view.press("done");
  expect(view.open().done).toBe(false);
  view.press("done");
  expect(view.open().done).toBe(true);
});

test("a press holds when the stage moves on, until resetFolds opens every zone again", () => {
  const id = "press-holds";
  const view = mount(job(id, "drafted"));
  act(() => pressBand(job(id, "drafted"), "now"));
  expect(view.open().now).toBe(false);
  view.show(job(id, "built"));
  expect(view.open().now).toBe(false);
  act(() => resetFolds(id));
  expect(view.open()).toEqual(ALL_OPEN);
});

test("resetFolds clears one Job's presses and leaves another's alone", () => {
  const one = job("reset-one", "building");
  const two = job("reset-two", "building");
  const first = mount(one);
  const second = mount(two);
  act(() => pressBand(one, "next"));
  act(() => pressBand(two, "next"));
  act(() => resetFolds("reset-one"));
  expect(first.open().next).toBe(true);
  expect(second.open().next).toBe(false);
});

test("pressing the whole-blueprint key on an open Blueprint swaps the Now view for the whole plan, and again swaps it back", () => {
  const session = job("whole-open", "building");
  const view = mount(session);
  expect(view.whole()).toBe(false);
  act(() => pressWhole(session));
  expect(view.whole()).toBe(true);
  expect(view.open().blueprint).toBe(true);
  act(() => pressWhole(session));
  expect(view.whole()).toBe(false);
});

test("a Blueprint folded by hand opens on the whole plan at the whole-blueprint key", () => {
  const session = job("whole-hand", "building");
  const view = mount(session);
  act(() => pressBand(session, "blueprint"));
  expect(view.open().blueprint).toBe(false);
  act(() => pressWhole(session));
  expect(view.open().blueprint).toBe(true);
  expect(view.whole()).toBe(true);
});

test("resetFolds also takes the whole plan back to the Now view", () => {
  const session = job("whole-reset", "building");
  const view = mount(session);
  act(() => pressWhole(session));
  expect(view.whole()).toBe(true);
  act(() => resetFolds(session.id));
  expect(view.whole()).toBe(false);
});

// ---- Kept per Job -------------------------------------------------------------------------------------------

const kept = (id: string) => localStorage.getItem(`armada.session-folds.${id}`);

test("what a Job's zones were left at is read back from the window's storage when the Job is first looked at", () => {
  const id = "kept-read";
  localStorage.setItem(`armada.session-folds.${id}`, JSON.stringify({ folds: { done: true, next: false }, whole: true }));
  const view = mount(job(id, "building"));
  expect(view.open()).toEqual({ ...ALL_OPEN, next: false });
  expect(view.whole()).toBe(true);
});

test("a press and the whole-plan key are written to the window's storage, without a stage", () => {
  const id = "kept-write";
  const session = job(id, "building");
  mount(session);
  act(() => pressBand(session, "next"));
  act(() => pressWhole(session));
  expect(JSON.parse(kept(id)!)).toEqual({ folds: { next: false }, whole: true });
});

// ---- Keys ---------------------------------------------------------------------------------------------------

const keydown = (init: KeyboardEventInit) => new KeyboardEvent("keydown", init);

test("⌥W, ⌥1, ⌥2, ⌥3 and ⌥F each name their band, by the physical key", () => {
  expect(["KeyW", "Digit1", "Digit2", "Digit3", "KeyF"].map((code) => bandOf(keydown({ altKey: true, code }))?.area)).toEqual(["workflow", "done", "now", "next", "field"]);
  // ⌥ changes the character the key types; the band is still the key's.
  expect(bandOf(keydown({ altKey: true, code: "Digit2", key: "™" }))?.area).toBe("now");
});

test("a band key needs ⌥ alone: the bare key, or ⌥ with another modifier, names none", () => {
  expect(bandOf(keydown({ code: "Digit2" }))).toBeUndefined();
  for (const other of [{ metaKey: true }, { ctrlKey: true }, { shiftKey: true }]) expect(bandOf(keydown({ altKey: true, code: "Digit2", ...other }))).toBeUndefined();
  expect(bandOf(keydown({ altKey: true, code: "KeyZ" }))).toBeUndefined();
});

test("⌥N is the whole-blueprint key, and only ⌥ alone", () => {
  expect(isWholeKey(keydown({ altKey: true, code: "KeyN" }))).toBe(true);
  expect(isWholeKey(keydown({ code: "KeyN" }))).toBe(false);
  expect(isWholeKey(keydown({ altKey: true, shiftKey: true, code: "KeyN" }))).toBe(false);
  expect(isWholeKey(keydown({ altKey: true, metaKey: true, code: "KeyN" }))).toBe(false);
  expect(isWholeKey(keydown({ altKey: true, code: "Digit2" }))).toBe(false);
  // It is not a band: ⌥N does not fold anything.
  expect(bandOf(keydown({ altKey: true, code: "KeyN" }))).toBeUndefined();
});

test("⌥L is the Completed view key, and only ⌥ alone", () => {
  expect(isLedgerKey(keydown({ altKey: true, code: "KeyL" }))).toBe(true);
  expect(isLedgerKey(keydown({ code: "KeyL" }))).toBe(false);
  expect(isLedgerKey(keydown({ altKey: true, shiftKey: true, code: "KeyL" }))).toBe(false);
  expect(bandOf(keydown({ altKey: true, code: "KeyL" }))).toBeUndefined();
});

test("⇧⌘A is the Artifacts key, and only ⇧⌘", () => {
  expect(isArtifactsKey(keydown({ metaKey: true, shiftKey: true, code: "KeyA" }))).toBe(true);
  expect(isArtifactsKey(keydown({ metaKey: true, code: "KeyA" }))).toBe(false);
  expect(isArtifactsKey(keydown({ shiftKey: true, code: "KeyA" }))).toBe(false);
  expect(isArtifactsKey(keydown({ ctrlKey: true, shiftKey: true, code: "KeyA" }))).toBe(false);
  expect(isArtifactsKey(keydown({ metaKey: true, shiftKey: true, altKey: true, code: "KeyA" }))).toBe(false);
  expect(isArtifactsKey(keydown({ metaKey: true, shiftKey: true, code: "KeyB" }))).toBe(false);
});
