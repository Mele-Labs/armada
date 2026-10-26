// Which of a Job's documents the reading layer reads: the page it was opened on, and the first where it was told none.

import { describe, expect, it } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import { documentOf } from "./document-state";

const page = (id: string, title: string, revisions?: string[]) => ({ kind: "artifact" as const, form: "document" as const, id, title, ...(revisions === undefined ? {} : { revisions }) });
const job = (...attachments: ReturnType<typeof page>[]) => ({ id: "j", attachments }) as unknown as Session;

describe("the page a Job's document layer reads", () => {
  const two = job(page("/w/a.html", "Findings", ["/s/a/r1.html", "/s/a/r2.html"]), page("/w/b.html", "Costs", ["/s/b/r1.html"]));

  it("takes the first page where none is asked for", () => {
    expect(documentOf(two)).toEqual({ id: "/w/a.html", title: "Findings", revisions: ["/s/a/r1.html", "/s/a/r2.html"] });
  });

  it("takes the page that was pressed", () => {
    expect(documentOf(two, "/w/b.html")).toEqual({ id: "/w/b.html", title: "Costs", revisions: ["/s/b/r1.html"] });
  });

  it("finds nothing for a page the Job does not hold, or one with no revisions", () => {
    expect(documentOf(two, "/w/c.html")).toBeUndefined();
    expect(documentOf(job(page("/w/a.html", "Findings")))).toBeUndefined();
  });
});
