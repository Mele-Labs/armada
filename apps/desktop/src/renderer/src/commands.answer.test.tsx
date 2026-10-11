// How long the line Fleet's answer draws on a control stands: until the next
// press, or until `--duration-answer` is over. The screens' own tests seed
// `answered`, so this is the one place the clearing in `useCommands` is run. #1237.

import type { Outcome } from "@armada/protocol";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { useCommands, type Sending } from "./commands";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const JOB = "01JOB";
const refused: Outcome = { ok: false, why: "already_killing" };

const sending: Sending = { onOpen: () => {}, onRead: () => {}, jobs: [], onPaused: () => {} };
let commands: ReturnType<typeof useCommands>;
let root: Root;
let host: HTMLElement;

function Probe(): null {
  commands = useCommands(sending);
  return null;
}

/** `window.armada` with only the calls a test presses, each answering `answer` unless it says otherwise. */
function fleet(answer: Outcome, over: Record<string, () => Promise<Outcome>> = {}): void {
  const answering = async (): Promise<Outcome> => answer;
  Object.assign(window, {
    armada: {
      killJob: answering,
      forgetJob: answering,
      redispatchJob: answering,
      rejectWork: answering,
      rerunChecks: answering,
      ...over,
    },
  });
}

/** The stylesheet's clock, set on the page so a test says which one it runs against. */
function holdFor(value: string): void {
  document.documentElement.style.setProperty("--duration-answer", value);
}

async function press(send: () => Promise<unknown>): Promise<void> {
  await act(async () => {
    await send();
  });
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  act(() => root.render(<Probe />));
});

afterEach(() => {
  act(() => root.unmount());
  host.remove();
  document.documentElement.style.removeProperty("--duration-answer");
  vi.useRealTimers();
});

// Each of the three ways an act on a Job goes out: `acted`, `decided` and `rerunChecks`.
const sends: [string, "killJob" | "rejectWork" | "rerunChecks", () => Promise<unknown>][] = [
  ["a header act", "killJob", () => commands.act("kill_job", JOB)],
  ["a decision at the review gate", "rejectWork", () => commands.decide(JOB, "reject")],
  ["a rerun of the stopped step's Checks", "rerunChecks", () => commands.rerunChecks(JOB)],
];

test.each(sends)("%s clears the previous answer before it goes out", async (_, call, send) => {
  holdFor("1600ms");
  fleet(refused);
  await press(() => commands.act("kill_job", JOB));
  expect(commands.answeredOn(JOB)).toEqual({ act: "kill_job", answer: "refused" });

  let answer: (outcome: Outcome) => void = () => {};
  fleet(refused, { [call]: () => new Promise<Outcome>((resolve) => (answer = resolve)) });
  let out: Promise<unknown> = Promise.resolve();
  act(() => {
    out = send();
  });
  // Fleet has not answered and the hold is not over, so only the press cleared it.
  expect(commands.answeredOn(JOB)).toBeUndefined();

  await act(async () => {
    answer({ ok: true });
    await out;
  });
});

test("the answer clears once --duration-answer is over", async () => {
  holdFor("2s");
  fleet(refused);
  await press(() => commands.act("kill_job", JOB));

  act(() => vi.advanceTimersByTime(1_999));
  expect(commands.answeredOn(JOB)).toEqual({ act: "kill_job", answer: "refused" });
  act(() => vi.advanceTimersByTime(1));
  expect(commands.answeredOn(JOB)).toBeUndefined();
});

test("an unreadable --duration-answer holds the answer until the next press", async () => {
  holdFor("soon");
  fleet(refused);
  await press(() => commands.act("kill_job", JOB));

  act(() => vi.advanceTimersByTime(60_000));
  expect(commands.answeredOn(JOB)).toEqual({ act: "kill_job", answer: "refused" });

  fleet({ ok: true });
  await press(() => commands.act("forget_job", JOB));
  expect(commands.answeredOn(JOB)).toBeUndefined();
});

test.each(["forget_job", "redispatch"] as const)("an accepted %s sets no answer", async (pressed) => {
  holdFor("1600ms");
  fleet({ ok: true });
  // The same answer on an act whose control stays on screen does set one.
  await press(() => commands.act("kill_job", JOB));
  expect(commands.answeredOn(JOB)).toEqual({ act: "kill_job", answer: "accepted" });

  await press(() => commands.act(pressed, JOB));
  expect(commands.answeredOn(JOB)).toBeUndefined();
});
