// Helm routes a thought into running work, through `App` on the `route-a-thought` moment: Fleet reads
// the words, Bridge delivers them, and Fleet keeps the row the thread then draws. The look is the
// walk's; this holds what the walk cannot see, which is where the words did and did not go.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { Outcome, ThoughtRead, ThoughtReading } from "@armada/protocol";
import { mount, openHelm, unmountAfterEach } from "./testing";

unmountAfterEach();

const helm = () => page.getByRole("complementary", { name: "Helm" });
const box = () => helm().getByRole("textbox", { name: "Ask Helm" });
const SENT_CHIP = (what: string) => page.getByRole("button", { name: `Sent to ${what}` });

const SAID = "Job 54 is not running a Drone that can be told";
const REFUSAL: Outcome = {
  ok: false,
  why: "refused",
  error: { code: "fleet.redirect_refused", message: SAID, run_id: "mock", fields: {}, chain: [] },
};

/** Type a message in Helm's box and send it with ↵, as the walk does. */
async function say(words: string): Promise<void> {
  await box().fill(words);
  await userEvent.keyboard("{Enter}");
}

/** The mock's own reading, replaced by this one so the test does not wait on the pause or on its scoring. */
function reading(app: ReturnType<typeof mount>, answer: ThoughtRead | Promise<ThoughtRead>): void {
  app.api.readThought = async () => answer;
}

const thought = (target?: Extract<ThoughtReading, { kind: "thought" }>["target"]): ThoughtRead => ({
  ok: true,
  reading: target === undefined ? { kind: "thought" } : { kind: "thought", target },
});

const BILLING_JOB = (app: ReturnType<typeof mount>) => {
  const job = app.scenario.state.jobs.find((one) => one.title === "Retry failed invoice charges with backoff");
  if (job === undefined) throw new Error("the moment holds the billing Job");
  return job;
};

test("words sent to a Session arrive as its message, are recorded once, and never reach Helm's own session", async () => {
  const app = mount("route-a-thought");
  const asked = vi.spyOn(app.api, "askHelm");
  const delivered = vi.spyOn(app.api, "sendSessionMessage");
  const recorded = vi.spyOn(app.api, "recordThoughtSent");
  await openHelm();

  const words = "I wish that the sessions page would move sessions into a new status that says they are just waiting for work to land";
  await say(words);
  await expect.element(helm().getByRole("group", { name: "Send to", exact: true })).toBeVisible();
  // Fleet has said nothing is sent yet: the row is the thread's, and the thread has none.
  expect(SENT_CHIP("Session Sessions list views").query()).toBeNull();

  await userEvent.keyboard("{Enter}");
  await expect.element(SENT_CHIP("Session Sessions list views")).toBeVisible();

  expect(delivered).toHaveBeenCalledTimes(1);
  expect(delivered).toHaveBeenCalledWith(expect.objectContaining({ session_id: "01LISTVIEWSAAAAAAAAAAAAAAA", text: words }));
  expect(recorded).toHaveBeenCalledTimes(1);
  expect(recorded).toHaveBeenCalledWith("armada", words, {
    kind: "session",
    id: "01LISTVIEWSAAAAAAAAAAAAAAA",
    title: "Sessions list views",
    manifest_id: "armada",
    slot: 3,
  });
  expect(asked).not.toHaveBeenCalled();
  // The card is gone, and pressing the chip is the row's own act.
  expect(helm().getByRole("group", { name: "Send to", exact: true }).query()).toBeNull();
});

test("words sent to a Job are a redirect to its Drone, recorded with the Job's repository", async () => {
  const app = mount("route-a-thought");
  const job = BILLING_JOB(app);
  const redirected = vi.spyOn(app.api, "redirectDrone");
  const recorded = vi.spyOn(app.api, "recordThoughtSent");
  const asked = vi.spyOn(app.api, "askHelm");
  reading(app, thought({ kind: "job", id: job.id, title: job.title, manifest_id: "billing" }));
  await openHelm();

  const words = "retries should back off longer after the third failed charge";
  await say(words);
  await expect.element(helm().getByText("billing", { exact: true })).toBeVisible();
  await userEvent.keyboard("{Enter}");
  await expect.element(SENT_CHIP("Job Retry failed invoice charges with backoff")).toBeVisible();

  expect(redirected).toHaveBeenCalledWith(job.id, words);
  expect(recorded).toHaveBeenCalledWith("armada", words, { kind: "job", id: job.id, title: job.title, manifest_id: "billing" });
  expect(asked).not.toHaveBeenCalled();
});

test("a delivery Fleet refuses is said in Fleet's words, leaves no row, and keeps the card and the words", async () => {
  const app = mount("route-a-thought");
  const job = BILLING_JOB(app);
  app.api.redirectDrone = async () => REFUSAL;
  const recorded = vi.spyOn(app.api, "recordThoughtSent");
  const asked = vi.spyOn(app.api, "askHelm");
  reading(app, thought({ kind: "job", id: job.id, title: job.title, manifest_id: "billing" }));
  await openHelm();

  await say("retries should back off longer after the third failed charge");
  await expect.element(helm().getByRole("group", { name: "Send to", exact: true })).toBeVisible();
  await userEvent.keyboard("{Enter}");

  await expect.element(page.getByText(SAID).first()).toBeVisible();
  expect(recorded).not.toHaveBeenCalled();
  expect(asked).not.toHaveBeenCalled();
  expect(SENT_CHIP("Job Retry failed invoice charges with backoff").query()).toBeNull();
  // Nothing was lost: the card still offers the words another way.
  await expect.element(helm().getByRole("group", { name: "Send to", exact: true })).toBeVisible();
  await expect.element(helm().getByRole("button", { name: "Ask Helm" })).toBeVisible();
});

test("a read that failed is a question, and the words go to Helm", async () => {
  const app = mount("route-a-thought");
  const asked = vi.spyOn(app.api, "askHelm");
  reading(app, { ok: false, outcome: { ok: false, why: "not_connected" } });
  await openHelm();

  await say("why did Job 12 fail?");
  await vi.waitFor(() => expect(asked).toHaveBeenCalledTimes(1));
  expect(asked.mock.calls[0]?.[0]).toBe("why did Job 12 fail?");
  expect(helm().getByRole("group", { name: "Send to", exact: true }).query()).toBeNull();
});

test("a read that was rejected is a question too", async () => {
  const app = mount("route-a-thought");
  const asked = vi.spyOn(app.api, "askHelm");
  app.api.readThought = () => Promise.reject(new Error("the connection closed"));
  await openHelm();

  await say("the merge line page should group PRs by repository");
  await vi.waitFor(() => expect(asked).toHaveBeenCalledTimes(1));
  expect(asked.mock.calls[0]?.[0]).toBe("the merge line page should group PRs by repository");
});

test("a thought nothing is about offers a new Session or Job and sends nothing", async () => {
  const app = mount("route-a-thought");
  const delivered = vi.spyOn(app.api, "sendSessionMessage");
  const redirected = vi.spyOn(app.api, "redirectDrone");
  const recorded = vi.spyOn(app.api, "recordThoughtSent");
  reading(app, thought());
  await openHelm();

  await say("the merge line page should group PRs by repository");
  await expect.element(helm().getByRole("group", { name: "Start new", exact: true })).toBeVisible();
  expect(delivered).not.toHaveBeenCalled();
  expect(redirected).not.toHaveBeenCalled();
  expect(recorded).not.toHaveBeenCalled();
});
