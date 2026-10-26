// The Job's story in the right column: which panel stands open at which stage, what a press on a band holds until the story moves on, how the
// whole-blueprint key swaps the Now view, and which keys mean what. Browser: a fold is read off a rendering, since it lives in a store a component hears.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import type { Session, SessionAttachment, WorkGroup } from "@armada/screens/src/draft/sessions";

import { bandOf, isArtifactsKey, isWholeKey, pressBand, pressWhole, resetFolds, useFold, useWhole, type Stage } from "./story";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
const used = new Set<string>();
afterEach(() => {
  for (const id of used) resetFolds(id);
  used.clear();
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const pull = (state: "draft" | "open" | "merged"): SessionAttachment => ({ kind: "pull_request", number: 7, title: "Retry the lease", branch: "armada/retry", address: "https://forge/7", checks: { state: "passed" }, state, auto: false });
const group = (...states: string[]): WorkGroup => ({ id: "g1", title: "Retry", agent: "rust-engineer", paths: [], tasks: states.map((state, at) => ({ id: `t${at}`, text: `Task ${at}`, state })) }) as unknown as WorkGroup;

/** A Job at a stage: a plan drafted, a plan approved with work left, or one built (every task done). */
type At = "drafted" | "building" | "built";
function job(id: string, at: At, attachments: SessionAttachment[] = []): Session {
  const groups = [group(...(at === "built" ? ["done", "done"] : ["done", "open"]))];
  return { id, attachments, rows: [], turn: { state: "idle" }, work: { plan: { scope: [], approved: at !== "drafted", groups } } } as unknown as Session;
}

const AREAS: Stage[] = ["blueprint", "field", "landing"];

function Probe({ session }: { session: Session }) {
  return (
    <>
      {AREAS.map((area) => (
        <Area key={area} session={session} area={area} />
      ))}
      <Whole session={session.id} />
    </>
  );
}
function Area({ session, area }: { session: Session; area: Stage }) {
  const fold = useFold(session, area);
  return <button type="button" data-area={area} data-open={fold.open} onClick={fold.toggle} />;
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
    whole: () => host.querySelector("[data-whole]")?.getAttribute("data-whole") === "true",
    press: (area: Stage) => act(() => host.querySelector<HTMLElement>(`[data-area="${area}"]`)!.click()),
  };
}

test("a plan being drafted opens the Blueprint alone, and the Field and Landing stand folded to their bands", () => {
  const view = mount(job("fold-drafted", "drafted"));
  expect(view.open()).toEqual({ blueprint: true, field: false, landing: false });
});

test("an approved plan with work left opens the Field and keeps the Blueprint open beside it", () => {
  const view = mount(job("fold-building", "building"));
  expect(view.open()).toEqual({ blueprint: true, field: true, landing: false });
});

test("a draft pull request is still being built, so the Field stays the open stage", () => {
  const view = mount(job("fold-draft-pr", "building", [pull("draft")]));
  expect(view.open()).toEqual({ blueprint: true, field: true, landing: false });
});

test("a Job that has landed opens the Landing and folds the Blueprint and the Field", () => {
  expect(mount(job("fold-built", "built")).open()).toEqual({ blueprint: false, field: false, landing: true });
  expect(mount(job("fold-pr", "building", [pull("open")])).open()).toEqual({ blueprint: false, field: false, landing: true });
});

test("the Blueprint stays open as the Job moves from drafted through building, and folds only at the landing", () => {
  const id = "fold-journey";
  const view = mount(job(id, "drafted"));
  expect(view.open().blueprint).toBe(true);
  view.show(job(id, "building"));
  expect(view.open().blueprint).toBe(true);
  view.show(job(id, "built"));
  expect(view.open().blueprint).toBe(false);
});

test("a press on a band's key folds an open panel and opens a folded one", () => {
  const session = job("press-band", "building");
  const view = mount(session);
  act(() => pressBand(session, "field"));
  act(() => pressBand(session, "landing"));
  expect(view.open()).toEqual({ blueprint: true, field: false, landing: true });
  act(() => pressBand(session, "field"));
  expect(view.open().field).toBe(true);
});

test("the band's own press folds and opens it the same way", () => {
  const view = mount(job("press-toggle", "building"));
  view.press("landing");
  expect(view.open().landing).toBe(true);
  view.press("landing");
  expect(view.open().landing).toBe(false);
});

test("a press holds when the stage moves on, until resetFolds gives the panels back to the stage", () => {
  const id = "press-holds";
  const view = mount(job(id, "drafted"));
  act(() => pressBand(job(id, "drafted"), "blueprint"));
  expect(view.open().blueprint).toBe(false);
  // The stage moves to the build, which keeps the Blueprint open; the hand's fold holds.
  view.show(job(id, "building"));
  expect(view.open().blueprint).toBe(false);
  act(() => resetFolds(id));
  expect(view.open()).toEqual({ blueprint: true, field: true, landing: false });
});

test("resetFolds clears one Job's presses and leaves another's alone", () => {
  const one = job("reset-one", "building");
  const two = job("reset-two", "building");
  const first = mount(one);
  const second = mount(two);
  act(() => pressBand(one, "landing"));
  act(() => pressBand(two, "landing"));
  act(() => resetFolds("reset-one"));
  expect(first.open().landing).toBe(false);
  expect(second.open().landing).toBe(true);
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

test("pressing the whole-blueprint key on a folded Blueprint opens it on the whole plan, not on the Now view", () => {
  const session = job("whole-folded", "built");
  const view = mount(session);
  expect(view.open().blueprint).toBe(false);
  act(() => pressWhole(session));
  expect(view.open().blueprint).toBe(true);
  expect(view.whole()).toBe(true);
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

// ---- Keys ---------------------------------------------------------------------------------------------------

const keydown = (init: KeyboardEventInit) => new KeyboardEvent("keydown", init);

test("⌥W, ⌥B, ⌥F and ⌥L each name their band, by the physical key", () => {
  expect(["KeyW", "KeyB", "KeyF", "KeyL"].map((code) => bandOf(keydown({ altKey: true, code }))?.area)).toEqual(["workflow", "blueprint", "field", "landing"]);
  // ⌥ changes the letter the key types; the band is still the key's.
  expect(bandOf(keydown({ altKey: true, code: "KeyB", key: "∫" }))?.area).toBe("blueprint");
});

test("a band key needs ⌥ alone: the bare letter, or ⌥ with another modifier, names none", () => {
  expect(bandOf(keydown({ code: "KeyB" }))).toBeUndefined();
  for (const other of [{ metaKey: true }, { ctrlKey: true }, { shiftKey: true }]) expect(bandOf(keydown({ altKey: true, code: "KeyB", ...other }))).toBeUndefined();
  expect(bandOf(keydown({ altKey: true, code: "KeyZ" }))).toBeUndefined();
});

test("⌥N is the whole-blueprint key, and only ⌥ alone", () => {
  expect(isWholeKey(keydown({ altKey: true, code: "KeyN" }))).toBe(true);
  expect(isWholeKey(keydown({ code: "KeyN" }))).toBe(false);
  expect(isWholeKey(keydown({ altKey: true, shiftKey: true, code: "KeyN" }))).toBe(false);
  expect(isWholeKey(keydown({ altKey: true, metaKey: true, code: "KeyN" }))).toBe(false);
  expect(isWholeKey(keydown({ altKey: true, code: "KeyB" }))).toBe(false);
  // It is not a band: ⌥N does not fold anything.
  expect(bandOf(keydown({ altKey: true, code: "KeyN" }))).toBeUndefined();
});

test("⇧⌘A is the Artifacts key, and only ⇧⌘", () => {
  expect(isArtifactsKey(keydown({ metaKey: true, shiftKey: true, code: "KeyA" }))).toBe(true);
  expect(isArtifactsKey(keydown({ metaKey: true, code: "KeyA" }))).toBe(false);
  expect(isArtifactsKey(keydown({ shiftKey: true, code: "KeyA" }))).toBe(false);
  expect(isArtifactsKey(keydown({ ctrlKey: true, shiftKey: true, code: "KeyA" }))).toBe(false);
  expect(isArtifactsKey(keydown({ metaKey: true, shiftKey: true, altKey: true, code: "KeyA" }))).toBe(false);
  expect(isArtifactsKey(keydown({ metaKey: true, shiftKey: true, code: "KeyB" }))).toBe(false);
});
