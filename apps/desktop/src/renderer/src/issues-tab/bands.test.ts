// What the Jobs say they hold of a repository, as one string that moves when a Job takes or gives back an issue.

import type { Session } from "@armada/screens/src/draft/sessions";
import { expect, test } from "vitest";

import { issueHoldings } from "./bands";

const job = (id: string, manifestId: string, numbers: string[], over: Partial<Session> = {}): Session =>
  ({ id, manifestId, attachments: numbers.map((number) => ({ kind: "issue", number, title: "", address: "", state: "open" })), ...over }) as unknown as Session;

test("it moves when a Job takes an issue of the repository and when it gives it back", () => {
  const none = issueHoldings([job("a", "storefront", [])], "storefront");
  const took = issueHoldings([job("a", "storefront", ["2810"])], "storefront");
  expect(took).not.toBe(none);
  expect(issueHoldings([job("a", "storefront", [])], "storefront")).toBe(none);
});

test("it does not move for a Job in another repository or for what is not an issue", () => {
  const before = issueHoldings([job("a", "storefront", ["2810"])], "storefront");
  const other = job("b", "ops", ["ABC-41"]);
  const pull = job("c", "storefront", [], { attachments: [{ kind: "slot", slot: 1 }] } as unknown as Partial<Session>);
  expect(issueHoldings([job("a", "storefront", ["2810"]), other, pull], "storefront")).toBe(before);
});

test("it is the same whatever order the Jobs come in", () => {
  const a = job("a", "storefront", ["1"]);
  const b = job("b", "storefront", ["2"]);
  expect(issueHoldings([a, b], "storefront")).toBe(issueHoldings([b, a], "storefront"));
});
