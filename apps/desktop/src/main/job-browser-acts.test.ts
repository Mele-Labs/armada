import { describe, expect, it } from "vitest";

import type { AnswerBrowserRequest, BrowserAct } from "@armada/protocol";

import { actOn, CLOSED, type ActsHost, type JobPage } from "./job-browser-acts";
import type { Sent } from "./capture/stepwalk";

/** A page with a Save button at 100,200 and nothing else. */
function fakePage(): JobPage & {
  sent: Sent[];
  loads: string[];
} {
  const sent: Sent[] = [];
  const loads: string[] = [];
  return {
    sent,
    loads,
    url: () => "http://localhost:5173/checkout",
    title: () => "Checkout",
    load: async (url) => {
      loads.push(url);
      return {
        loaded: true,
      };
    },
    find: async (target) =>
      target.role === "button" && target.name === "Save"
        ? {
            x: 90,
            y: 190,
            width: 20,
            height: 20,
          }
        : null,
    focus: async () => "other",
    send: (one) => void sent.push(one),
    selectAll: () => {},
    settle: async () => {},
    snapshot: async () => '- button "Save"',
    capture: async () => true,
    evaluate: async () => ({
      result: "1",
    }),
  };
}

function host(page: JobPage | null) {
  const answered: AnswerBrowserRequest[] = [];
  const opened: string[] = [];
  const marks: string[] = [];
  const fresh = fakePage();
  const acts: ActsHost = {
    page: () => page,
    acting: (_sessionId, sentence) => {
      marks.push(`start ${sentence}`);
      return () => void marks.push(`end ${sentence}`);
    },
    openClosed: (_sessionId, url) => {
      opened.push(url);
      return fresh;
    },
    answer: async (body) => void answered.push(body),
  };
  return {
    acts,
    answered,
    opened,
    fresh,
    marks,
  };
}

const asked = (act: BrowserAct) => ({
  session_id: "01JOB",
  request_id: "browser-01",
  act,
});

describe("an act on a held request", () => {
  it("presses the target the finder found, with a real pointer, and answers the page", async () => {
    const page = fakePage();
    const { acts, answered } = host(page);
    await actOn(
      acts,
      asked({
        kind: "click",
        target: {
          role: "button",
          name: "Save",
        },
      }),
    );
    expect(page.sent.map((one) => ("type" in one ? one.type : "insert"))).toEqual([
      "mouseMove",
      "mouseDown",
      "mouseUp",
    ]);
    expect(page.sent[1]).toMatchObject({
      x: 100,
      y: 200,
    });
    expect(answered).toEqual([
      {
        request_id: "browser-01",
        answer: {
          kind: "page",
          url: "http://localhost:5173/checkout",
          title: "Checkout",
          snapshot: '- button "Save"',
        },
      },
    ]);
  });

  it("says which act is running while it runs, and ends it once the act answers", async () => {
    const page = fakePage();
    const { acts, answered, marks } = host(page);
    const seen: string[][] = [];
    page.snapshot = async () => {
      seen.push([
        ...marks,
      ]);
      return "page";
    };
    await actOn(
      acts,
      asked({
        kind: "snapshot",
      }),
    );
    expect(seen).toEqual([
      [
        "start Reading the page",
      ],
    ]);
    expect(marks).toEqual([
      "start Reading the page",
      "end Reading the page",
    ]);
    expect(answered).toHaveLength(1);
  });

  it("ends the act's mark when the act throws, and when the page is closed", async () => {
    const page = fakePage();
    page.snapshot = async () => {
      throw new Error("the page crashed");
    };
    const threw = host(page);
    await actOn(
      threw.acts,
      asked({
        kind: "snapshot",
      }),
    );
    expect(threw.marks).toEqual([
      "start Reading the page",
      "end Reading the page",
    ]);
    const closed = host(null);
    await actOn(
      closed.acts,
      asked({
        kind: "click",
        target: {
          role: "button",
          name: "Save",
        },
      }),
    );
    expect(closed.marks).toEqual([
      "start Clicking Save",
      "end Clicking Save",
    ]);
  });

  it("answers not_found where the finder found nothing, and sends the page nothing", async () => {
    const page = fakePage();
    const { acts, answered } = host(page);
    await actOn(
      acts,
      asked({
        kind: "click",
        target: {
          role: "button",
          name: "Delete",
        },
      }),
    );
    expect(page.sent).toEqual([]);
    expect(answered).toEqual([
      {
        request_id: "browser-01",
        error: {
          kind: "not_found",
        },
      },
    ]);
  });

  it("opens a page for a Job with no Browser tab on the agent's open, and answers its page", async () => {
    const { acts, answered, opened, fresh } = host(null);
    await actOn(
      acts,
      asked({
        kind: "open",
        url: "http://localhost:5173/cart",
      }),
    );
    expect(opened).toEqual([
      "http://localhost:5173/cart",
    ]);
    expect(fresh.loads).toEqual([
      "http://localhost:5173/cart",
    ]);
    expect(answered[0]?.answer).toMatchObject({
      kind: "page",
      title: "Checkout",
    });
  });

  it("refuses any other act on a closed Job browser, in words that name the way out", async () => {
    const { acts, answered, opened } = host(null);
    await actOn(
      acts,
      asked({
        kind: "snapshot",
      }),
    );
    expect(opened).toEqual([]);
    expect(answered).toEqual([
      {
        request_id: "browser-01",
        error: {
          kind: "failed",
          why: CLOSED,
        },
      },
    ]);
  });

  it("answers a throw on the way as failed, so the agent's call is never left held", async () => {
    const page = {
      ...fakePage(),
      snapshot: async () => Promise.reject(new Error("The page went away.")),
    };
    const { acts, answered } = host(page);
    await actOn(
      acts,
      asked({
        kind: "snapshot",
      }),
    );
    expect(answered).toEqual([
      {
        request_id: "browser-01",
        error: {
          kind: "failed",
          why: "The page went away.",
        },
      },
    ]);
  });
});
