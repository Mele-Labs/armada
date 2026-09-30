// Dispatching leaves the composer and lands on the proposal, through `App` and
// nothing else — and the answer moves you on.
//
// **The claim the owner's note of 28 Sep 2026 asks for**, decided 30 Sep as
// *the wait is a destination*. Nothing here
// asserts a component's name: each test reads what is on screen, so the screen
// can be rebuilt underneath it and the claim still fails the day it stops being
// true.

import { expect, test, describe } from "vitest";
import { page } from "vitest/browser";

import { refusedWith } from "@armada/protocol";
import { arcJob } from "@armada/screens/src/fixtures/build/arc-base";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { scenarioNamed } from "./scenario";
import type { Scenario } from "./scenario";

unmountAfterEach();

/** The composer, open on a request already typed. */
const composer = () => page.getByRole("textbox", { name: "Request" });

/** What is in the request field. A textarea holds it as a value, not an attribute. */
const typed = () => (composer().element() as HTMLTextAreaElement).value;

/**
 * A request typed over the moment's own draft. **Distinct on purpose**: the
 * composer opens holding the draft, so a claim about the words coming back has
 * to be about words the window could only have carried.
 */
const ASKED = "Make the Drones stat say what is running, and keep it live.";
/**
 * The card's own press. **`last`, because the title row carries one too** — a
 * `Dispatch` that opens the composer and a `Dispatch` that sends it, the same
 * reading `arc-dispatch.test.tsx` already takes.
 */
const dispatch = () => page.getByRole("button", { name: "Dispatch", exact: true }).last();
const heading = (says: string) => page.getByRole("heading", { name: says });

/**
 * The composer, open on a request already typed. **Built on the moment rather
 * than beside it**: the request, the board and the settings are
 * `arc/dispatch-typing`'s, and each case below changes only what Fleet answers.
 */
function typingOn(): Scenario {
  const typing = scenarioNamed("arc/dispatch-typing");
  if (typing === undefined) throw new Error("no scenario named arc/dispatch-typing");
  return typing;
}

/**
 * A moment whose dispatch never answers, so the wait can be read.
 *
 * **It publishes the call, the way Fleet does.** `proposal.moved` is what the
 * screen reads the model, the reach and the budget off, and publishing it here
 * is also what puts the adoption rule under the rail claim at the foot.
 */
function neverAnswers(): Scenario {
  return {
    ...typingOn(),
    behaves: (fleet) => ({
      proposeFromRequest: () => {
        fleet.publish({
          proposing: {
            proposal_id: "01M2D3ZF41003NEVERANSWERS",
            client_ref: "bridge-1",
            model: "sonnet",
            since: new Date(Date.now() - 41_000).toISOString(),
            budget_ms: 600_000,
            reached: "thinking",
            thinking_tokens: 763,
          },
        });
        return new Promise(() => {});
      },
    }),
  };
}

/**
 * A moment whose dispatch answers with one Job, which the proposer's ordinary
 * answer is. **The Job joins the board as Fleet's own would**, because the
 * screen it opens reads the board and not this answer.
 */
function becomesOne(): Scenario {
  const typing = typingOn();
  const job = { ...arcJob("awaiting_approval"), branch: undefined };
  return {
    ...typing,
    // No fixture behind it, which opens the Job onto `unanswered` — the claim is
    // that the proposal became the Job, not what the Job's own reads say.
    behaves: (fleet) => ({
      proposeFromRequest: async () => {
        fleet.publish({ jobs: [...fleet.state().jobs, job] });
        return { ok: true, jobs: [job] };
      },
    }),
  };
}

/** A moment whose dispatch is refused before it is sent — nothing was asked. */
function refuses(): Scenario {
  return {
    ...typingOn(),
    behaves: () => ({
      proposeFromRequest: async () => ({
        ok: false,
        why: "refused",
        outcome: { ok: false, why: "not_connected" },
      }),
    }),
  };
}

/** A moment whose dispatch is declined: no workflow fits, and nothing is created. */
function declines(): Scenario {
  const typing = typingOn();
  return {
    ...typing,
    behaves: () => ({
      proposeFromRequest: async (request) => ({
        ok: false,
        why: "unresolved",
        request,
        // Through the one parser main reads a refusal with, so the body is a
        // body rather than a hand-built `WireError`.
        outcome: refusedWith(
          422,
          JSON.stringify({
            code: "fleet.no_workflow_fits",
            message: "No workflow fits this request.",
            fields: { request },
          }),
          { method: "POST", path: "/jobs/from_request" },
        ),
      }),
    }),
  };
}

describe("dispatching leaves the composer", () => {
  test(
    "the press takes the field off the screen and puts the request, the model and a stop on it",
    async () => {
      mount(neverAnswers());
      await onScreen();

      await expect.element(composer()).toBeVisible();
      const asked = typed();
      await dispatch().click();

      // **Gone, not disabled.** The wait was inside this card until 30 Sep 2026
      // and the whole of the owner's complaint is that it was.
      await expect.element(heading("Reading the request")).toBeVisible();
      expect(composer().query(), "the composer is still on screen").toBeNull();
      // The one `Dispatch` left is the title row's, which opens the composer.
      expect(page.getByRole("button", { name: "Dispatch", exact: true }).all()).toHaveLength(1);

      // The request, as sent — nothing summarised and nothing trimmed.
      await expect.element(page.getByText(asked, { exact: false })).toBeVisible();
      // What is reading it, how far it has got, how long it has been and how
      // much of the budget is left. An elapsed count alone draws "thinking hard"
      // and "never reached the vendor" identically.
      const wait = page.getByRole("status").first();
      await expect.element(wait).toHaveTextContent("The model is thinking");
      await expect.element(wait).toHaveTextContent("sonnet");
      await expect.element(wait).toHaveTextContent("left");
      await expect.element(wait).toHaveTextContent(/\d+[ms]/);
      // The stop is on it from the first frame, because this screen exists to be
      // where that decision is made.
      await expect.element(page.getByRole("button", { name: "Stop the proposer" })).toBeVisible();
    },
  );

  test("the request the proposer is reading is not a Job, and the screen claims none", async () => {
    mount(neverAnswers());
    await onScreen();
    await dispatch().click();
    await expect.element(heading("Reading the request")).toBeVisible();

    // No Job exists yet, so nothing on screen is a Job's reading: none of the
    // Job's own destinations, and no status badge for a Job at a gate.
    expect(page.getByRole("tab", { name: /^Overview/ }).all()).toHaveLength(0);
    expect(page.getByRole("tab", { name: /^Plan/ }).all()).toHaveLength(0);
    expect(page.getByText("needs approval", { exact: false }).all()).toHaveLength(0);
  });

  test("a declined request comes back to the composer with the words in the field", async () => {
    mount(declines());
    await onScreen();
    // **Typed rather than the moment's own prompt.** The composer opens holding
    // the draft, so a claim made against that string would pass against a
    // window that threw the words away and re-seeded the field.
    await composer().fill(ASKED);
    await dispatch().click();

    await expect.element(heading("No workflow fits this request")).toBeVisible();
    await expect.element(page.getByText(/Nothing was created/)).toBeVisible();

    await page.getByRole("button", { name: "Edit the request" }).click();
    // The claim that a refused request is unchanged, made true rather than
    // stated: it is the field it comes back to.
    await expect.element(composer()).toBeVisible();
    await expect.element(composer()).toHaveValue(ASKED);
  });

  /**
   * A refusal this screen has no drawing for — disconnected, or a code nothing
   * matched — is drawn by the app's own failure pipeline. **What must not happen
   * is losing the words**: the composer is remounted, so what it opens on is the
   * window's and not its own.
   */
  test("a refusal with no drawing hands the composer back, words and all", async () => {
    mount(refuses());
    await onScreen();
    await composer().fill(ASKED);
    await dispatch().click();

    await expect.element(composer()).toBeVisible();
    await expect.element(composer()).toHaveValue(ASKED);
    expect(heading("Reading the request").all()).toHaveLength(0);
  });
});

describe("the answer moves you on", () => {
  /**
   * **A list of one is not a list.** The owner asked to be taken to the Job with
   * its details filling in; what he was given is a proposal that becomes the Job
   * the moment there is one, and this is that claim.
   */
  test("a request that became one job opens that job, with no list in between", async () => {
    mount(becomesOne());
    await onScreen();
    await dispatch().click();

    // The Job, whole — its destinations, not a row with a Review button on it.
    await expect.element(page.getByRole("tab", { name: /^Overview/ })).toBeVisible();
    expect(heading("What the request became").all()).toHaveLength(0);
    expect(page.getByRole("button", { name: /^Review / }).all()).toHaveLength(0);
  });

  test(
    "a request the proposer split lists the jobs in order, and only the head has a gate",
    async () => {
      mount("arc/proposing-several");
      await onScreen();
      await dispatch().click();

      await expect.element(heading("What the request became")).toBeVisible();
      await expect.element(page.getByText("Say what the Drones stat counts")).toBeVisible();
      await expect.element(page.getByText("Open the Drone behind the stat")).toBeVisible();
      // The second waits on the first, which is the whole of the graph.
      await expect.element(page.getByText("Waits on job 1.")).toBeVisible();

      // One gate, the head's — Fleet's rule is strictly one by one.
      expect(page.getByRole("button", { name: /^Approve/ }).all()).toHaveLength(1);
      await page
        .getByRole("button", { name: "Approve Say what the Drones stat counts" })
        .click();

      // Released, and the row follows the board rather than the answer.
      await expect
        .poll(() => page.getByRole("button", { name: /^Approve/ }).all().length)
        .toBe(0);
    },
  );

  test("a job on the list is opened from it, and the proposal is left behind", async () => {
    mount("arc/proposing-several");
    await onScreen();
    await dispatch().click();
    await expect.element(heading("What the request became")).toBeVisible();

    await page.getByRole("button", { name: "Review Open the Drone behind the stat" }).click();

    // The Job, whole — and nothing of the proposal's screen left standing.
    await expect.element(page.getByRole("tab", { name: /^Overview/ })).toBeVisible();
    expect(heading("What the request became").all()).toHaveLength(0);
  });
});

describe("a proposal is left rather than carried", () => {
  test("the rail works while the proposer is out, and does not pull you back", async () => {
    mount(neverAnswers());
    await onScreen();
    await dispatch().click();
    await expect.element(heading("Reading the request")).toBeVisible();

    // A published call the window has a screen for would otherwise be adopted
    // straight back on the next render, which would make the rail do nothing.
    await page.getByRole("navigation", { name: "Work" }).getByText("Overview").click();
    await expect.element(page.getByRole("heading", { name: "Running" }).first()).toBeVisible();
    expect(heading("Reading the request").all()).toHaveLength(0);
  });
});
