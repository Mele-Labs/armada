// Go to file's ranking: letters in order, with runs, word starts and the file's own name scoring higher.

import { expect, test } from "vitest";

import { ranked, scoreOf } from "./fuzzy";

const PATHS = [
  "app/models/payment.rb",
  "app/services/refund_issuer.rb",
  "app/controllers/refunds_controller.rb",
  "app/jobs/refund_retry_job.rb",
  "config/routes.rb",
  "spec/services/refund_issuer_spec.rb",
];

test("letters out of order match nothing", () => {
  expect(scoreOf("bur", "app/models/payment.rb")).toBeUndefined();
});

test("the empty query keeps every path in its order", () => {
  expect(ranked("", PATHS)).toEqual(PATHS);
});

test("word starts in the file's name win over letters scattered through the path", () => {
  expect(ranked("rfi", PATHS)[0]).toBe("app/services/refund_issuer.rb");
});

test("a run of letters ranks the file it names first", () => {
  expect(ranked("routes", PATHS)[0]).toBe("config/routes.rb");
  expect(ranked("retry", PATHS)[0]).toBe("app/jobs/refund_retry_job.rb");
});

test("a letter skipped ahead to a word start still lets the rest match", () => {
  // The `r` that starts a word comes after the only `c`, so taking it would lose the match.
  expect(scoreOf("rc", "arc_r")).toBeDefined();
});

test("of two equal matches the shorter path comes first", () => {
  expect(ranked("refundissuer", PATHS)).toEqual([
    "app/services/refund_issuer.rb",
    "spec/services/refund_issuer_spec.rb",
  ]);
});
