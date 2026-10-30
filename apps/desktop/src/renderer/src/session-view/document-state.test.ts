// Which of a Job's documents the reading layer reads: the page it was opened on, and the first where it was told none.

import { describe, expect, it } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import { documentOf, waitsOnHim } from "./document-state";

const page = (id: string, title: string, revisions?: string[]) => ({ kind: "artifact" as const, form: "document" as const, id, title, ...(revisions === undefined ? {} : { revisions }) });
const job = (...attachments: ReturnType<typeof page>[]) => ({ id: "j", attachments }) as unknown as Session;

describe("the page a Job's document layer reads", () => {
  const two = job(page("/w/a.html", "Findings", ["/s/a/r1.html", "/s/a/r2.html"]), page("/w/b.html", "Costs", ["/s/b/r1.html"]));

  it("takes the first page where none is asked for", () => {
    expect(documentOf(two)).toEqual({ id: "/w/a.html", title: "Findings", revisions: ["/s/a/r1.html", "/s/a/r2.html"], waits: false });
  });

  it("takes the page that was pressed", () => {
    expect(documentOf(two, "/w/b.html")).toEqual({ id: "/w/b.html", title: "Costs", revisions: ["/s/b/r1.html"], waits: false });
  });

  it("finds nothing for a page the Job does not hold, or one with no revisions", () => {
    expect(documentOf(two, "/w/c.html")).toBeUndefined();
    expect(documentOf(job(page("/w/a.html", "Findings")))).toBeUndefined();
  });
});

describe("a Job's document waiting on him", () => {
  const asked = { ...page("/w/a.html", "Findings", ["/s/a/r1.html", "/s/a/r2.html"]), asked: true as const, revision: "2" };

  it("waits while its newest revision is asked and not settled", () => {
    expect(waitsOnHim(asked)).toBe(true);
    expect(documentOf(job(asked))?.waits).toBe(true);
  });

  it("waits no more once he answered it, either way", () => {
    expect(waitsOnHim({ ...asked, settled: { how: "approved", at: "2026-10-30T10:05:00Z" } })).toBe(false);
    expect(waitsOnHim({ ...asked, settled: { how: "sent_back", at: "2026-10-30T10:05:00Z", words: "Shorter." } })).toBe(false);
  });

  it("never waits where Fleet did not ask", () => {
    expect(waitsOnHim(page("/w/a.html", "Findings", ["/s/a/r1.html"]))).toBe(false);
  });
});
