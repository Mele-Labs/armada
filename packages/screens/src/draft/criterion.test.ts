// What a criterion says about how it is answered, and where its words began.

import { describe, expect, it } from "vitest";

import {
  criterionViewOf,
  criterionViewsOf,
  criterionWritten,
  decidedSaidOf,
  originLineOf,
  originSaidOf,
} from "./criterion";
import type { CriterionView } from "./criterion";
import { sampleDetail } from "./sample";

function criterion(source: string, text = "The gate refuses a main-process import") {
  return { criterion_id: "AC1", text, source };
}

describe("how a criterion is answered", () => {
  it("renames the wire's source rather than keeping two meanings on one row", () => {
    const view = criterionViewOf(criterion("check"), "manual");

    expect(view.verified_by).toBe("check");
    expect("source" in view).toBe(false);
  });

  it("carries all three of the registry's spellings", () => {
    expect(criterionViewOf(criterion("check"), "manual").verified_by).toBe("check");
    expect(criterionViewOf(criterion("judge"), "manual").verified_by).toBe("judge");
    expect(criterionViewOf(criterion("attested"), "manual").verified_by).toBe("attested");
  });

  it("reads a spelling it does not know as judge, never as check", () => {
    expect(criterionViewOf(criterion("something_new"), "manual").verified_by).toBe("judge");
  });
});

describe("where the words came from", () => {
  it("is the prompt where a person dispatched the Job, because the wire records no issue", () => {
    for (const origin of ["manual", "studio_dispatched", "helm_drafted", "studio_helm_drafted"]) {
      expect(criterionViewOf(criterion("judge"), origin).origin).toEqual({ origin: "prompt" });
    }
  });

  // `#1748` row 17: every criterion read *From your prompt* on a Job Fleet
  // found itself.
  it("is unsaid where no person dispatched it, and nothing is drawn for it", () => {
    for (const origin of ["auto_detected", "workflow_triggered", "sub_dispatched", "drone_drafted"]) {
      const view = criterionViewOf(criterion("judge"), origin);
      expect(view.origin).toEqual({ origin: "unsaid" });
      expect(originSaidOf(view)).toBeUndefined();
      expect(originLineOf(view)).toBeUndefined();
    }
  });

  it("says nothing about the source having moved", () => {
    expect(criterionViewOf(criterion("judge"), "manual").origin_moved_at).toBeUndefined();
  });
});

describe("a Job's criteria", () => {
  it("keeps the order they were given in", () => {
    const detail = sampleDetail({
      acceptance_criteria: [
        { criterion_id: "AC1", text: "first", source: "check" },
        { criterion_id: "AC2", text: "second", source: "judge" },
      ],
    });

    expect(criterionViewsOf(detail).map((view) => view.text)).toEqual(["first", "second"]);
  });

  it("is empty on a Job held to nothing", () => {
    expect(criterionViewsOf(sampleDetail())).toEqual([]);
  });
});

describe("what a criterion says about itself", () => {
  const written = (over: Partial<CriterionView> = {}): CriterionView => ({
    text: "The stat reads one running",
    verified_by: "judge",
    origin: { origin: "prompt" },
    ...over,
  });

  // The owner read `answered by the check` on a Job nobody had approved and
  // took it for a verdict already in (`f9yw`, 28 Sep).
  it("says what will decide it, never what decided it", () => {
    expect(decidedSaidOf(written({ verified_by: "check" }))).toBe("A Check will decide it");
    expect(decidedSaidOf(written({ verified_by: "judge" }))).toBe("The Judge will decide it");
    expect(decidedSaidOf(written({ verified_by: "attested" }))).toBe("You will decide it");
  });

  // A bare `owner/number` is a repository, a path and a branch as readily as
  // an issue, which is what he asked (`u7y9`).
  it("names an issue as an issue, and hands the reference over whole", () => {
    const said = originSaidOf(written({ origin: { origin: "issue", ref: "armada/1162" } }));

    expect(said?.said).toBe("From issue");
    expect(said?.issue).toEqual({ ref: "armada/1162" });
  });

  it("carries the forge address where a surface has been given one", () => {
    const said = originSaidOf(
      written({ origin: { origin: "issue", ref: "armada/1162", url: "https://example/1162" } }),
    );

    expect(said?.issue?.url).toBe("https://example/1162");
  });

  it("gives the other two origins no reference, because they are not places", () => {
    expect(originSaidOf(written({ origin: { origin: "prompt" } }))).toEqual({
      said: "From your prompt",
    });
    expect(originSaidOf(written({ origin: { origin: "person" } }))).toEqual({
      said: "You wrote this",
    });
  });

  it("flattens to one line for a surface that draws no link", () => {
    expect(originLineOf(written({ origin: { origin: "issue", ref: "armada/1162" } }))).toBe(
      "From issue armada/1162",
    );
    expect(originLineOf(written())).toBe("From your prompt");
  });
});

describe("a criterion somebody adds at the gate", () => {
  // A Check is the workflow's and is frozen at creation, so a line typed here
  // has no Check to run against it.
  it("is empty, is yours, and is the Judge's to decide", () => {
    const fresh = criterionWritten();

    expect(fresh.text).toBe("");
    expect(fresh.origin).toEqual({ origin: "person" });
    expect(fresh.verified_by).toBe("judge");
  });

  // Absent is the honest answer: nothing has minted one, and a Bridge-side id
  // would be a value Fleet never agreed to.
  it("carries no id, because nothing has minted one", () => {
    expect(criterionWritten().criterion_id).toBeUndefined();
  });
});
