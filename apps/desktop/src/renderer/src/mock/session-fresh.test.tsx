// The model a Session's composer reads, and the fresh start offered once it is big or old, through `App` on a
// Fleet that serves Sessions (`sessions-fleet.ts`). The walk (`walks/session-fresh.ts`) shows how it reads.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { HANDOFF_ASK } from "../session-handoff";
import { onBoard } from "./scenario";
import type { Scenario } from "./scenario";
import { FakeSessionsFleet, hosted, terminal } from "./sessions-fleet";
import { mount, onScreen, toPlace, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const AT = "2026-10-07T13:48:02.000Z";

const ago = (hours: number): string => new Date(Date.now() - hours * 3_600_000).toISOString();
const used = (context_tokens: number) => ({ context_tokens, context_window: 1_000_000 });
const facts = (change: { model?: string; resolved_model?: string } = {}) => ({ turn: { state: "idle" as const }, mode: "auto" as const, running: true, ...change });

const BIG = "01BIGAAAAAAAAAAAAAAAAAAAAA";

const records = () => [
  hosted(BIG, { title: "Big one", started_at: ago(3), usage: used(612_000), hosted: facts({ resolved_model: "vendor-opus-5-5" }) }),
  hosted("01OLDAAAAAAAAAAAAAAAAAAAAA", { title: "Old one", started_at: ago(52), usage: used(88_000), hosted: facts({ model: "sonnet", resolved_model: "vendor-sonnet-5-5" }) }),
  hosted("01SMALLAAAAAAAAAAAAAAAAAAA", { title: "Small one", started_at: ago(2), usage: used(53_840), hosted: facts({ model: "sonnet" }) }),
  hosted("01BLANKAAAAAAAAAAAAAAAAAAA", { title: "Blank one", started_at: ago(1) }),
  terminal("01TERMAAAAAAAAAAAAAAAAAAAA", { title: "A terminal", started_at: ago(1) }),
  hosted("01ENDEDAAAAAAAAAAAAAAAAAAA", { title: "Ended one", state: "ended", started_at: ago(52), usage: used(612_000), last_seen_at: new Date().toISOString() }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [BIG]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "Find why totals are off by a cent." },
    { kind: "message", id: "m2", at: AT, from: { kind: "agent" }, text: "Each line is rounded before the sum." },
  ],
});

function served(fleet: FakeSessionsFleet): Scenario {
  const board = onBoard([job("escalated", { id: "01JOB", handle: "52-the-retry-loop", title: "The retry loop", owner_manifest_id: "armada" })], {
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  return fleet.scenario(board);
}

async function open(title: string, view = "Active") {
  await toPlace("Sessions");
  const list = page.getByRole("region", { name: "Sessions" });
  await userEvent.click(list.getByRole("tab", { name: view }));
  await userEvent.click(list.getByRole("button", { name: title }));
}
const mark = () => page.getByRole("button", { name: "Fresh start" });

test("The composer's Model select reads the version the process resolved, and the alias or the default before it has said", async () => {
  const scenario = served(new FakeSessionsFleet(records(), threads()));
  mount({ ...scenario, state: { ...scenario.state, holds: { ...scenario.state.holds, models: { models: ["haiku", "sonnet", "opus"], default: "sonnet", session: "opus" } } } });
  await onScreen();
  const model = () => page.getByRole("combobox", { name: "Model", exact: true });

  await open("Big one");
  await expect.element(model()).toHaveDisplayValue("Opus 5.5 · default");
  await toPlace("Sessions");
  await open("Old one");
  await expect.element(model()).toHaveDisplayValue("Sonnet 5.5");
  await toPlace("Sessions");
  await open("Small one");
  await expect.element(model()).toHaveDisplayValue("sonnet");
  await toPlace("Sessions");
  await open("Blank one");
  await expect.element(model()).toHaveDisplayValue("opus · default");
  // The head names no model: the composer does.
  await expect.element(page.getByRole("img", { name: /^Model/ })).not.toBeInTheDocument();
});

test("A Session past 300k of context, or a day, offers a fresh start, and a small new one or an ended one does not", async () => {
  mount(served(new FakeSessionsFleet(records(), threads())));
  await onScreen();

  await open("Big one");
  await expect.element(mark()).toBeVisible();
  await toPlace("Sessions");
  await open("Old one");
  await expect.element(mark()).toBeVisible();
  await toPlace("Sessions");
  await open("Small one");
  await expect.element(mark()).not.toBeInTheDocument();
  await toPlace("Sessions");
  await open("Ended one", "Ended");
  await expect.element(mark()).not.toBeInTheDocument();
});

test("Fork starts a new Session from the conversation as it stands, and the old one is left as it was", async () => {
  const fleet = new FakeSessionsFleet(records(), threads());
  mount(served(fleet));
  await onScreen();

  await open("Big one");
  await userEvent.click(mark());
  await userEvent.click(page.getByRole("dialog", { name: "Fresh start" }).getByRole("button", { name: "Fork" }));

  await expect.poll(() => fleet.calls.forkedAt).toEqual([{ sessionId: BIG, at: "m2" }]);
  await expect.element(page.getByRole("group", { name: "Forked from Big one" })).toBeVisible();
});

test("New Session asks the old one for its handoff and opens a new one whose first message is the reply", async () => {
  const fleet = new FakeSessionsFleet(records(), threads());
  fleet.startsNow = true;
  fleet.keepsRows = true;
  fleet.onSend = (send) => {
    if (send.text === HANDOFF_ASK) void fleet.agentTurn(send.session_id, [{ kind: "message", id: "h1", at: AT, from: { kind: "agent" }, text: "Goal: totals match. Next: the refund path." }]);
  };
  mount(served(fleet));
  await onScreen();

  await open("Big one");
  await userEvent.click(mark());
  await userEvent.click(page.getByRole("dialog", { name: "Fresh start" }).getByRole("button", { name: "New Session" }));

  // The old Session is asked, in words of the person's, and nothing is typed.
  await expect.poll(() => fleet.calls.sent[0]).toMatchObject({ session_id: BIG, text: HANDOFF_ASK });
  await expect.poll(() => fleet.calls.started).toBe(1);
  await expect.poll(() => fleet.calls.sent.length).toBe(2);
  expect(fleet.calls.sent[1]?.session_id).not.toBe(BIG);
  expect(fleet.calls.sent[1]?.text).toBe("Handoff from Big one\n\nGoal: totals match. Next: the refund path.");
  await expect.element(page.getByText("Next: the refund path.").last()).toBeVisible();
});
