// @vitest-environment happy-dom
// The Judge on the phone: what to judge, then the type, then the reading. A press posts the artifact, the type and the
// line to verify; a failed reading says why; a note is sent or dismissed by the person alone.

import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import { call, getJson } from "./client";
import { JudgeView, targetsOf } from "./Judge";
import type { Machine } from "./machines";
import type { JudgeReading, SessionRecord } from "./wire";

const machine = {
  origin: "https://studio.tail0.ts.net",
  name: "Studio",
} as Machine;

let record: SessionRecord;

vi.mock("./client", async (real) => ({
  ...(await real<typeof import("./client")>()),
  call: vi.fn(async () => ({})),
  getJson: vi.fn(),
}));
vi.mock("./fleet", async (real) => ({
  ...(await real<typeof import("./fleet")>()),
  macOf: () => ({
    machine,
  }),
  start: async () => {},
  useFleet: () => ({}),
}));
vi.mock("./jobState", async (real) => ({
  ...(await real<typeof import("./jobState")>()),
  useJob: () => [
    {
      thread: {
        session: record,
        rows: [],
      },
    },
    () => {},
  ],
}));

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const reading = (over: Partial<JudgeReading>): JudgeReading => ({
  id: "r1",
  artifact: {
    kind: "job",
    id: "S1",
    title: "Pocket Judge",
  },
  prompt: "",
  state: "done",
  asked_at: "2026-11-25T10:00:00Z",
  read: true,
  notes: [],
  ...over,
});

const jobWith = (over: Partial<SessionRecord>) =>
  ({
    id: "S1",
    title: "Pocket Judge",
    manifest_id: "armada",
    attachments: [
      {
        kind: "pr",
        target: "#41",
        state: "live",
        since: "",
        changed_at: "",
      },
      {
        kind: "pull_request",
        target: "41",
        state: "live",
        since: "",
        changed_at: "",
      },
    ],
    ...over,
  }) as unknown as SessionRecord;

const TYPES = {
  types: [
    {
      id: "adversarial",
      name: "Adversarial",
    },
    {
      id: "code",
      name: "Code",
    },
  ],
};

async function open(): Promise<HTMLElement> {
  vi.mocked(getJson).mockResolvedValue(TYPES);
  const host = document.createElement("div");
  document.body.append(host);
  await act(async () => {
    createRoot(host).render(<JudgeView origin={machine.origin} id="S1" />);
  });
  return host;
}
const press = async (host: HTMLElement, name: string) => {
  const one = [
    ...host.querySelectorAll("button"),
  ].find((button) => button.textContent === name);
  expect(one, name).toBeDefined();
  await act(async () => {
    one!.click();
  });
};

afterEach(() => {
  document.body.innerHTML = "";
  vi.clearAllMocks();
});

describe("Judge on the phone", () => {
  it("offers the Job and each pull request it holds once", () => {
    expect(
      targetsOf(jobWith({})).map((one) => [
        one.kind,
        one.id,
      ]),
    ).toEqual([
      [
        "job",
        "S1",
      ],
      [
        "pull_request",
        "41",
      ],
    ]);
  });

  it("asks for the pull request as the chosen type, with the line to verify", async () => {
    record = jobWith({});
    const host = await open();
    expect(vi.mocked(getJson)).toHaveBeenCalledWith(machine, "/api/judge_types/armada");
    await press(host, "Pull request #41");
    await press(host, "Code");
    const line = host.querySelector("textarea")!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(
        line,
        "the sheet reads on a phone",
      );
      line.dispatchEvent(
        new Event("input", {
          bubbles: true,
        }),
      );
    });
    await press(host, "Judge");
    expect(vi.mocked(call)).toHaveBeenCalledWith(machine, "/api/sessions/judge", {
      method: "POST",
      body: {
        session_id: "S1",
        artifact: {
          kind: "pull_request",
          id: "41",
          title: "#41",
        },
        prompt: "the sheet reads on a phone",
        judge: "code",
      },
    });
  });

  it("says why a reading failed, and that a clean one missed nothing", async () => {
    record = jobWith({
      judge_readings: [
        reading({
          id: "r1",
          state: "failed",
          failed: "The Judge ran out of time.",
        }),
        reading({
          id: "r2",
          artifact: {
            kind: "pull_request",
            id: "41",
            title: "#41",
          },
        }),
      ],
    });
    const host = await open();
    expect(host.textContent).toContain("The Judge ran out of time.");
    expect(host.textContent).toContain("Nothing missed");
  });

  it("sends a note to the agent only when the person presses Send", async () => {
    record = jobWith({
      judge_readings: [
        reading({
          notes: [
            {
              id: "n1",
              claim: "The sheet has no empty state",
            },
          ],
        }),
      ],
    });
    const host = await open();
    expect(vi.mocked(call)).not.toHaveBeenCalledWith(
      machine,
      "/api/sessions/judge/send",
      expect.anything(),
    );
    await press(host, "Send to agent");
    expect(vi.mocked(call)).toHaveBeenCalledWith(machine, "/api/sessions/judge/send", {
      method: "POST",
      body: {
        session_id: "S1",
        reading: "r1",
        note: "n1",
      },
    });
  });
});
