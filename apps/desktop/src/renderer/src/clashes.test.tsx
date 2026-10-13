// What Fleet's `/clashes` answer comes to on the rows: the mark on both holders of a pair, the other
// holder opened by a press, and no mark where Fleet has none to give (an older Fleet answers 404 and
// main hands over an empty list, `main/clashes.test.ts`).

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import { ClashMark } from "@armada/components";
import type { WriteClash } from "@armada/protocol";

import { ClashesFromFleet } from "./clashes";
import { createFleetClashes } from "./fleet-clashes";
import { useOpenHolderAsked } from "./open-holder";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const JOB_AND_SESSION: WriteClash = {
  one: { kind: "job", id: "j1", label: "Cap the retry backoff" },
  other: { kind: "session", id: "s1", label: "Retry tuning" },
  paths: ["crates/fleet/src/retry.rs", "crates/fleet/src/backoff.rs"],
  since: "2026-10-13T09:00:00Z",
};
const SESSION_AND_MAIN: WriteClash = {
  one: { kind: "session", id: "s2", label: "Lint sweep" },
  other: { kind: "main", id: "main", label: "main" },
  paths: ["crates/store/src/open.rs"],
  since: "2026-10-13T09:05:00Z",
};

const opened: string[] = [];
const roots: { root: Root; host: HTMLElement }[] = [];

beforeEach(() => {
  opened.length = 0;
});
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

/** A window whose rows are one Job, two Sessions and a Job nobody conflicts with, over a Fleet that answers `clashes`. */
async function window(clashes: WriteClash[]): Promise<HTMLElement> {
  const connected = { connection: { state: "connected" } } as never;
  const source = createFleetClashes(
    { state: async () => connected, subscribe: () => () => {}, getWriteClashes: async () => ({ ok: true, value: clashes }) },
    () => () => {},
  );
  function Rows() {
    useOpenHolderAsked({ job: (id) => opened.push(`job ${id}`), session: (id) => opened.push(`session ${id}`) });
    return (
      <>
        <section aria-label="Job j1">
          <ClashMark kind="job" id="j1" />
        </section>
        <section aria-label="Session s1">
          <ClashMark kind="session" id="s1" />
        </section>
        <section aria-label="Session s2">
          <ClashMark kind="session" id="s2" />
        </section>
        <section aria-label="Job j9">
          <ClashMark kind="job" id="j9" />
        </section>
      </>
    );
  }
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({ root, host });
  await act(async () => root.render(<ClashesFromFleet source={source}><Rows /></ClashesFromFleet>));
  await act(async () => {});
  return host;
}

const marksOn = (host: HTMLElement, row: string) => [...host.querySelectorAll(`section[aria-label="${row}"] .armada-clash-mark`)];

test("marks both rows of a pair, each naming the other holder and the paths", async () => {
  const host = await window([JOB_AND_SESSION, SESSION_AND_MAIN]);
  expect(marksOn(host, "Job j1").map((mark) => mark.getAttribute("aria-label"))).toEqual([
    "Conflicts with Retry tuning · crates/fleet/src/retry.rs, crates/fleet/src/backoff.rs",
  ]);
  expect(marksOn(host, "Session s1").map((mark) => mark.getAttribute("aria-label"))).toEqual([
    "Conflicts with Cap the retry backoff · crates/fleet/src/retry.rs, crates/fleet/src/backoff.rs",
  ]);
  expect(marksOn(host, "Session s2").map((mark) => mark.getAttribute("aria-label"))).toEqual(["Conflicts with main · crates/store/src/open.rs"]);
});

test("leaves a row that is in no clash unmarked", async () => {
  const host = await window([JOB_AND_SESSION]);
  expect(marksOn(host, "Job j9")).toEqual([]);
});

test("marks nothing where Fleet reports none, as an older Fleet that answers 404 comes to", async () => {
  const host = await window([]);
  expect(host.querySelectorAll(".armada-clash-mark")).toHaveLength(0);
});

test("opens the other holder of a pair from either row", async () => {
  const host = await window([JOB_AND_SESSION]);
  await act(async () => (marksOn(host, "Job j1")[0] as HTMLElement).click());
  await act(async () => (marksOn(host, "Session s1")[0] as HTMLElement).click());
  expect(opened).toEqual(["session s1", "job j1"]);
});

test("draws a clash against main as a picture, with nothing to open", async () => {
  const host = await window([SESSION_AND_MAIN]);
  const [mark] = marksOn(host, "Session s2");
  expect(mark?.tagName).not.toBe("BUTTON");
  await act(async () => (mark as HTMLElement).click());
  expect(opened).toEqual([]);
});
