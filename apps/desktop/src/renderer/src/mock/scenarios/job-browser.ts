// A Job driving its Job browser: the agent opened the shop's checkout in it, typed a discount code and applied it, and its
// thread shows each act. Once time passes the agent reaches for Playwright's own browser instead, and the thread says it has
// one of its own. The browser is the store's (`../../job-browser/store.ts`), which only the mock fills. Mock data only.
// The walk `job-browser` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { remindOfBrowser, setJobBrowser } from "../../job-browser/store";
import { resetCheckout, shopPage } from "../job-browser-page";
import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";
import { onTimePassing } from "../time-passes";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const JOB = "01SESSIONJOBBROWSERAAAAAAA";
const CART = "http://localhost:5173/cart";
const CHECKOUT = "http://localhost:5173/checkout";

function fleet(): FakeSessionsFleet {
  const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
  const records = [hosted(JOB, { title: "Fix the checkout total", attachments: [held("slot", "4"), held("branch", "fix/checkout-total")] })];
  const thread = {
    [JOB]: [
      { kind: "message" as const, id: "m1", at: ago(9), from: { kind: "you" as const }, text: "The checkout total ignores the discount code. Find out why." },
      { kind: "tool" as const, id: "t0", at: ago(8), text: `browser_open ${CART}` },
      { kind: "tool" as const, id: "t1", at: ago(8), text: "browser_click Check out" },
      { kind: "tool" as const, id: "t2", at: ago(8), text: "browser_type SPRING10 into Discount code" },
      { kind: "tool" as const, id: "t3", at: ago(7), text: "browser_click Apply" },
      { kind: "tool" as const, id: "t4", at: ago(7), text: "browser_snapshot" },
      { kind: "message" as const, id: "m2", at: ago(6), from: { kind: "agent" as const }, text: "SPRING10 is applied and the discount shows as $8.40, but the total stays at $84.00." },
    ],
  };
  return new FakeSessionsFleet(records, thread);
}

/** The Job's browser as the agent left it: the agent driving, on the checkout. Flight control's Browser opens it in the side panel. */
function seed(): void {
  resetCheckout();
  setJobBrowser(JOB, { visited: [CART, CHECKOUT], at: 1, loads: 0, driver: "agent", windowed: false, pins: [], reminders: [], threadHidden: false, page: shopPage });
}

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  return {
    ...fleet().scenario(board),
    name: "job-browser",
    says: "A Job driving its Job browser, taken over and annotated, moved to a window, and reminded of it when it reaches for Playwright",
    // A Fleet and a browser of their own for each window, so a walk told twice starts from the same page.
    behaves: (handle) => {
      seed();
      const served = fleet();
      const own = served.scenario(board).behaves!(handle);
      // After the window's fake has registered its own, which clears the hook of a scenario with no `later`.
      queueMicrotask(() =>
        onTimePassing(() => {
          const at = new Date().toISOString();
          void served.agentTurn(JOB, [{ kind: "tool", id: "t5", at, text: `mcp__playwright__browser_navigate ${CHECKOUT}` }]).then(() => remindOfBrowser(JOB, { id: "r1", at, tool: "Playwright", url: CHECKOUT }));
        }),
      );
      return own;
    },
  };
}

export const jobBrowser: Scenario = build();
