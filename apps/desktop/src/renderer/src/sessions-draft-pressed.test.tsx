// A press on Pause, Auto land or Approve shows at once over whatever serves Sessions, until Fleet's row carries it; a refusal puts it back and says why.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { SessionsFrom, useSessions, useSessionsDraft, useSessionsSaid } from "./sessions-draft";

const ONE: Session = { id: "s1", title: "A Session", turn: { state: "idle" }, rows: [], attachments: [], work: { scope: [], groups: [], plan: { version: 2, scope: [], groups: [], approved: false } } } as unknown as Session;

const SESSIONS = [ONE];

/** A draft whose three presses wait for the test to answer them, and which says the words of a refusal. */
function fleet() {
  const waiting: ((went: boolean) => void)[] = [];
  let words: string | undefined;
  const listeners = new Set<() => void>();
  const ask = () => new Promise<boolean>((done) => waiting.push(done));
  const draft = {
    get: () => SESSIONS,
    subscribe: (listener: () => void) => (listeners.add(listener), () => void listeners.delete(listener)),
    said: () => words,
    pause: ask,
    autoLand: ask,
    work: { workflows: [], choose: () => undefined, unpick: () => undefined, approve: ask, suggest: () => undefined, sendConcern: () => undefined, dismissConcern: () => undefined },
  } as unknown as SessionsDraft;
  return {
    draft,
    answer: (went: boolean, said?: string) => {
      words = said;
      listeners.forEach((one) => one());
      waiting.shift()!(went);
    },
  };
}

function Probe() {
  const one = useSessions()[0]!;
  const said = useSessionsSaid();
  return <output>{`paused=${one.paused === true} auto=${one.autoLand === true} approved=${one.work?.plan?.approved === true} going=${(one.pressed ?? []).join(",")} said=${said ?? ""}`}</output>;
}

let mounted: { root: Root; host: HTMLElement } | undefined;
let held: ReturnType<typeof fleet>;
let presses: SessionsDraft | undefined;

function Grab() {
  presses = useSessionsDraft();
  return null;
}

beforeEach(async () => {
  held = fleet();
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  root.render(
    <SessionsFrom held={held.draft}>
      <Grab />
      <Probe />
    </SessionsFrom>,
  );
  await next();
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
});

const text = () => mounted!.host.querySelector("output")!.textContent;
function next() {
  return new Promise<void>((done) => setTimeout(done, 20));
}

test("Pause shows paused before Fleet answers, with the going mark", async () => {
  expect(text()).toContain("paused=false");
  void presses!.pause!("s1");
  await next();
  expect(text()).toContain("paused=true");
  expect(text()).toContain("going=paused");
});

test("Auto land shows on before Fleet answers", async () => {
  void presses!.autoLand!("s1", true);
  await next();
  expect(text()).toContain("auto=true");
  expect(text()).toContain("going=autoLand");
});

test("Approve shows the plan approved before Fleet answers", async () => {
  presses!.work!.approve("s1");
  await next();
  expect(text()).toContain("approved=true");
  expect(text()).toContain("going=plan");
});

test("a refused Pause reverts, and the refusal's words are said", async () => {
  void presses!.pause!("s1");
  await next();
  expect(text()).toContain("paused=true");
  held.answer(false, "Fleet cannot pause a Session with no slot.");
  await next();
  expect(text()).toContain("paused=false");
  expect(text()).toContain("said=Fleet cannot pause a Session with no slot.");
});

test("a refused Approve reverts, and the refusal's words are said", async () => {
  presses!.work!.approve("s1");
  await next();
  expect(text()).toContain("approved=true");
  held.answer(false, "That version is no longer the plan.");
  await next();
  expect(text()).toContain("approved=false");
  expect(text()).toContain("said=That version is no longer the plan.");
});
