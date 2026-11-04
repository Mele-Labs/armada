// A Job's Needs you card and review sheet over real Jobs, through `App` on `job-review-sheet`. What he answers (a question, a command, a Job its agent started) is a row of the card and is
// answered there; what he reads (a plan, sketch, draft, document, walk) is one Review row on it, which opens the sheet. A Job with nothing to review has no such row, a Job with only
// review items opens the sheet from the band, and one item alone stays where it lives. The walk of the same name draws it; this holds that the presses reach the same code.

import { beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { SENT, s310JobReviewSheet } from "./scenarios/job-review-sheet";
import {
  band,
  bandKey,
  detail,
  list,
  names,
  openSheet,
  reviewRow,
  row,
  sheet,
} from "./review-sheet-kit";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();
beforeEach(() => {
  SENT.suggest.length = 0;
  SENT.sketch.length = 0;
  SENT.document.length = 0;
  SENT.waiting.length = 0;
});

const JOB = "Retry backoff cap";
const ASK_JOB = "Name the cache key";
const READ_JOB = "Draw the retry curve";
const ONE_WALK_JOB = "Check the usage page";
const ONE_SKETCH_JOB = "Sketch the cap";
/** What he reads, oldest first. The question and the command are on the card. */
const ORDER = [
  "Plan v2",
  "Backoff curve",
  "Pull request text",
  "Retry design",
  "Backoff table",
];
const thread = () =>
  page.getByRole("region", {
    name: "Thread",
    exact: true,
  });
/** The mark on a row of the list: Approved, Sent back, Answered or Dismissed. */
const marked = (name: RegExp, said: string) =>
  row(JOB, name).getByRole("img", {
    name: said,
  });
/** A row of the card, by its item's words. */
const card = (name: string | RegExp) =>
  band().getByRole("button", {
    name: typeof name === "string" ? `Open ${name}` : name,
    exact: typeof name === "string",
  });
const focused = () => document.activeElement;
const SHIFT_META_Y = "{Shift>}{Meta>}y{/Meta}{/Shift}";

async function open(title: RegExp = /Retry backoff cap/) {
  mount(s310JobReviewSheet);
  await onScreen();
  await toSession(title);
}

test("the card keeps the question and the command as rows and has one Review row for what he reads", async () => {
  await open();
  await expect.element(reviewRow()).toBeVisible();
  await expect.element(card("Keep the old fixture?")).toBeVisible();
  await expect
    .element(
      band().getByRole("group", {
        name: "scripts/rebuild-index",
      }),
    )
    .toBeVisible();
  // A plan, sketch, draft, document and walk are not rows of their own, and the sheet is shut.
  for (const name of ORDER) expect(card(new RegExp(`^Open ${name}`)).query()).toBeNull();
  expect(sheet(JOB).query()).toBeNull();
  // The one row names them all, under its own eyebrow.
  await expect
    .element(
      band().getByRole("region", {
        name: "To review",
      }),
    )
    .toBeVisible();
  await expect.element(reviewRow()).toHaveTextContent(ORDER.join(", "));
});

test("one question beside the review row shows both rows, not the question's card", async () => {
  await open();
  await expect.element(card("Keep the old fixture?")).toBeVisible();
  await expect.element(reviewRow()).toBeVisible();
  expect(
    page
      .getByRole("article", {
        name: "Waiting on you",
      })
      .query(),
  ).toBeNull();
});

test("the question is answered on the card, and the sheet stays shut", async () => {
  await open();
  await userEvent.click(card("Keep the old fixture?"));
  await userEvent.click(
    page.getByRole("radio", {
      name: /Keep it/,
    }),
  );
  await userEvent.click(
    page.getByRole("button", {
      name: "Answer",
    }),
  );
  await expect.element(card("Keep the old fixture?")).not.toBeInTheDocument();
  expect(sheet(JOB).query()).toBeNull();
  // What he reads is still there to be reviewed.
  await expect.element(reviewRow()).toBeVisible();
});

test("a Job with only a question and a Job its agent started has no Review row and never opens the sheet", async () => {
  await open(/Name the cache key/);
  await expect.element(card(/^Open What should the cache key include\?/)).toBeVisible();
  await expect.element(band()).toBeVisible();
  expect(reviewRow().query()).toBeNull();
  expect(
    band()
      .getByRole("region", {
        name: "To review",
      })
      .query(),
  ).toBeNull();
  // The question answers on the card, where the press stays.
  await userEvent.click(card(/^Open What should the cache key include\?/));
  await expect
    .element(
      page.getByRole("radio", {
        name: /The path and the commit/,
      }),
    )
    .toBeVisible();
  expect(sheet(ASK_JOB).query()).toBeNull();
});

test("a Job its agent started is a row of the card with its Start", async () => {
  await open(/Name the cache key/);
  await expect.element(card(/Press Start on "Add the retry metrics"/)).toBeVisible();
  const start = band().getByRole("button", {
    name: /^Start/,
  });
  await expect.element(start).toBeVisible();
  await userEvent.click(start);
  // The Job begins and its row has no Start left to press.
  await expect.element(start).not.toBeInTheDocument();
  expect(sheet(ASK_JOB).query()).toBeNull();
});

test("the band's glyphs include what he reads, and the sheet's list is the same, oldest first", async () => {
  await open();
  // The glyph is the registered one; its lucide class names it.
  const drawn = (from: Element, within: string) =>
    [
      ...from.querySelectorAll(within),
    ].map(
      (one) =>
        one
          .querySelector("svg")
          ?.getAttribute("class")
          ?.match(/lucide-\S+/)?.[0],
    );
  await expect.element(reviewRow()).toBeVisible();
  const trigger = bandKey().element();
  const told = [
    ...trigger.querySelectorAll("[role='img']"),
  ].map((one) => one.getAttribute("aria-label"));
  // The question and the command first, then each item he reads, each told by its name.
  expect(told.slice(-ORDER.length)).toEqual(ORDER);
  const glyphs = drawn(trigger, "[role='img']").slice(-ORDER.length);
  await openSheet(JOB);
  expect(names(JOB)).toEqual(ORDER);
  expect(glyphs).toEqual(drawn(list(JOB).element(), ".armada-job-review__kind"));
});

test("⇧⌘Y opens the sheet where the Job has things to review", async () => {
  await open();
  await expect.element(reviewRow()).toBeVisible();
  expect(sheet(JOB).query()).toBeNull();
  await userEvent.keyboard(SHIFT_META_Y);
  await expect.element(sheet(JOB)).toBeVisible();
});

test("⇧⌘Y folds the card where the Job has nothing to review, and opens no sheet", async () => {
  await open(/Name the cache key/);
  await expect.element(bandKey()).toHaveAttribute("aria-expanded", "true");
  await userEvent.keyboard(SHIFT_META_Y);
  await expect.element(bandKey()).toHaveAttribute("aria-expanded", "false");
  expect(sheet(ASK_JOB).query()).toBeNull();
  await userEvent.keyboard(SHIFT_META_Y);
  await expect.element(bandKey()).toHaveAttribute("aria-expanded", "true");
  expect(sheet(ASK_JOB).query()).toBeNull();
});

test("a Job with only things to review opens the sheet from a press on the band", async () => {
  await open(/Draw the retry curve/);
  await expect.element(bandKey()).toBeVisible();
  // There is no list worth folding, so the card has no Review row.
  expect(reviewRow().query()).toBeNull();
  expect(sheet(READ_JOB).query()).toBeNull();
  await userEvent.click(bandKey());
  await expect.element(sheet(READ_JOB)).toBeVisible();
  expect(names(READ_JOB)).toEqual([
    "Retry curve",
    "Retry table",
  ]);
});

test("the sheet stays open on the last item when an answer leaves one", async () => {
  await open(/Draw the retry curve/);
  await userEvent.click(bandKey());
  await expect.element(sheet(READ_JOB)).toBeVisible();
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  await expect.element(detail(READ_JOB, "Retry table")).toBeVisible();
  await expect.element(sheet(READ_JOB)).toBeVisible();
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  await expect.element(sheet(READ_JOB)).not.toBeInTheDocument();
});

test("one walk is a row on the card that brings its tab forward, and no sheet", async () => {
  await open(/Check the usage page/);
  await expect.element(bandKey()).toBeVisible();
  await userEvent.click(bandKey());
  await expect.element(card("Usage page")).toBeVisible();
  expect(reviewRow().query()).toBeNull();
  expect(sheet(ONE_WALK_JOB).query()).toBeNull();
  await userEvent.keyboard(SHIFT_META_Y);
  expect(sheet(ONE_WALK_JOB).query()).toBeNull();
  await userEvent.keyboard(SHIFT_META_Y);
  await userEvent.click(card("Usage page"));
  await expect
    .element(
      page.getByRole("tab", {
        name: "Usage page",
      }),
    )
    .toHaveAttribute("aria-selected", "true");
  expect(sheet(ONE_WALK_JOB).query()).toBeNull();
});

test("one sketch is its card in the thread, with no band and no sheet", async () => {
  await open(/Sketch the cap/);
  await expect.element(thread().getByText("The cap")).toBeVisible();
  expect(bandKey().query()).toBeNull();
  await userEvent.keyboard(SHIFT_META_Y);
  expect(sheet(ONE_SKETCH_JOB).query()).toBeNull();
});

test("⇧⌘Y opens the sheet on a Job with only something to review", async () => {
  await open(/Draw the retry curve/);
  await expect.element(bandKey()).toBeVisible();
  await userEvent.keyboard(SHIFT_META_Y);
  await expect.element(sheet(READ_JOB)).toBeVisible();
});

test("Approve names the plan and settles only the plan, and the next open item comes up", async () => {
  await open();
  await openSheet(JOB);
  await userEvent.click(row(JOB, /Plan v2/));
  const approve = sheet(JOB).getByRole("button", {
    name: /^Approve Plan v2/,
  });
  await expect.element(approve).toBeVisible();
  await userEvent.click(approve);

  await expect.element(marked(/Plan v2/, "Approved")).toBeVisible();
  // Only the plan is marked, and the sketch after it is the one drawn.
  for (const name of [
    /Backoff curve/,
    /Pull request text/,
    /Retry design/,
    /Backoff table/,
  ])
    expect(marked(name, "Approved").query()).toBeNull();
  await expect.element(detail(JOB, "Backoff curve")).toBeVisible();
});

test("an answer in the sheet settles the thread's card: the document approved here is approved there", async () => {
  await open();
  // The document's card sits in the thread with its own Approve until it is answered anywhere.
  await expect
    .element(
      thread()
        .getByText("Approved", {
          exact: true,
        })
        .first(),
    )
    .not.toBeInTheDocument();
  await openSheet(JOB);
  await userEvent.click(row(JOB, /Retry design/));
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  await expect.element(marked(/Retry design/, "Approved")).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(sheet(JOB)).not.toBeInTheDocument();
  await expect
    .element(
      thread()
        .getByText("Approved", {
          exact: true,
        })
        .first(),
    )
    .toBeInTheDocument();
});

test("Send back carries his note as a note on the plan's first group", async () => {
  await open();
  await openSheet(JOB);
  await userEvent.click(row(JOB, /Plan v2/));
  await userEvent.click(
    sheet(JOB).getByRole("button", {
      name: /^Send back/,
    }),
  );
  await userEvent.type(
    sheet(JOB).getByRole("textbox", {
      name: "Note",
    }),
    "Cap it at twenty seconds.",
  );
  await userEvent.click(
    sheet(JOB)
      .getByRole("button", {
        name: /^Send back/,
        exact: false,
      })
      .last(),
  );
  await expect.element(marked(/Plan v2/, "Sent back")).toBeVisible();
  expect(SENT.suggest).toHaveLength(1);
  expect(SENT.suggest[0]!.approving).toBeUndefined();
  expect(SENT.suggest[0]!.marks).toEqual([
    expect.objectContaining({
      kind: "note",
      group: "p1",
      text: "Cap it at twenty seconds.",
    }),
  ]);
});

test("the plan reads its summary first, then its groups", async () => {
  await open();
  await openSheet(JOB);
  await userEvent.click(row(JOB, /Plan v2/));
  const body = detail(JOB, "Plan v2");
  const summary = body.getByText(
    "Cap the retry delay at thirty seconds, and test the curve up to the cap.",
  );
  await expect.element(summary).toBeVisible();
  const groups = body.getByRole("region", {
    name: "Cap the delay",
  });
  await expect.element(groups).toBeVisible();
  expect(
    summary.element().compareDocumentPosition(groups.element()) & Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy();
});

test("the draft's Approve and Send back are in the head, named, and its card keeps only what a draft has of its own", async () => {
  await open();
  await openSheet(JOB);
  await userEvent.click(row(JOB, /Pull request text/));
  await expect
    .element(
      sheet(JOB).getByRole("button", {
        name: /^Approve Pull request text/,
      }),
    )
    .toBeVisible();
  await expect
    .element(
      sheet(JOB).getByRole("button", {
        name: /^Send back/,
      }),
    )
    .toBeVisible();
  // The card holds the text, the text it replaces, Edit and Withdraw: no second Approve and no Note.
  const body = detail(JOB, "Pull request text");
  for (const name of [
    "Edit",
    "Withdraw",
    "Text it replaces",
  ])
    await expect
      .element(
        body.getByRole("button", {
          name,
        }),
      )
      .toBeVisible();
  for (const name of [
    /^Approve/,
    /^Note/,
  ])
    expect(
      body
        .getByRole("button", {
          name,
        })
        .query(),
    ).toBeNull();
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  await expect.element(marked(/Pull request text/, "Approved")).toBeVisible();
  expect(SENT.waiting).toEqual([
    expect.objectContaining({
      item: expect.stringMatching(/^draft:/),
      given: {
        choice: 0,
      },
    }),
  ]);
});

test("⌥R sends the draft back with his note, and ⌘↵ inside Edit's box stays with the box", async () => {
  await open();
  await openSheet(JOB);
  await userEvent.click(row(JOB, /Pull request text/));
  await userEvent.keyboard("{Alt>}r{/Alt}");
  const note = sheet(JOB).getByRole("textbox", {
    name: "Note",
  });
  await expect.element(note).toBeVisible();
  await userEvent.type(note, "Say how long the cap is.");
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  await expect.element(marked(/Pull request text/, "Sent back")).toBeVisible();
  expect(SENT.waiting).toEqual([
    expect.objectContaining({
      item: expect.stringMatching(/^draft:/),
      given: {
        text: "Say how long the cap is.",
      },
    }),
  ]);
});

test("⌘↵ in Edit's box approves the draft with its fixes, not as it stood", async () => {
  await open();
  await openSheet(JOB);
  await userEvent.click(row(JOB, /Pull request text/));
  await userEvent.click(
    detail(JOB, "Pull request text").getByRole("button", {
      name: "Edit",
    }),
  );
  const box = sheet(JOB).getByRole("textbox", {
    name: "Draft text",
  });
  await expect.element(box).toBeVisible();
  await userEvent.type(box, " Zero is refused.");
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  // The box took the key: what went is the text with its fixes, not the head's plain approval.
  await expect.element(marked(/Pull request text/, "Answered")).toBeVisible();
  expect(SENT.waiting).toEqual([
    expect.objectContaining({
      item: expect.stringMatching(/^draft:/),
      given: {
        fixed: expect.stringContaining("Zero is refused."),
      },
    }),
  ]);
});

test("⌥R opens the note on a document, ⌘↵ in it sends the document back with his words", async () => {
  await open();
  await openSheet(JOB);
  await userEvent.click(row(JOB, /Retry design/));
  await userEvent.keyboard("{Alt>}r{/Alt}");
  const note = sheet(JOB).getByRole("textbox", {
    name: "Note",
  });
  await expect.element(note).toBeVisible();
  await expect.poll(() => focused() === note.element()).toBe(true);
  await userEvent.type(note, "Say why the cap is thirty seconds.");
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  await expect.element(marked(/Retry design/, "Sent back")).toBeVisible();
  expect(SENT.document).toEqual([
    {
      how: "sent_back",
      words: "Say why the cap is thirty seconds.",
    },
  ]);
});

test("⌘↵ approves the picked sketch alone, and the arrows move between items", async () => {
  await open();
  await openSheet(JOB);
  await userEvent.click(row(JOB, /Backoff curve/));
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  await expect.element(marked(/Backoff curve/, "Approved")).toBeVisible();
  expect(SENT.sketch).toEqual([
    {
      how: "approved",
      words: undefined,
    },
  ]);
  expect(marked(/Plan v2/, "Approved").query()).toBeNull();

  // ↓ draws the next item at once, and ↑ goes back.
  row(JOB, /Pull request text/)
    .element()
    .focus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(detail(JOB, "Retry design")).toBeVisible();
  await userEvent.keyboard("{ArrowUp}");
  await expect.element(detail(JOB, "Pull request text")).toBeVisible();
});

test("a walk plays in the sheet's pane, and the head has no Approve for it", async () => {
  await open();
  await openSheet(JOB);
  await userEvent.click(row(JOB, /Backoff table/));
  // The walk's own bar holds its Approve and Send back stays off the head: nothing here is the head's to answer.
  await expect
    .element(
      detail(JOB, "Backoff table").getByRole("button", {
        name: /^Approve/,
      }),
    )
    .toBeVisible();
  expect(sheet(JOB).element().querySelector(".job-walk")).not.toBeNull();
  expect(
    sheet(JOB)
      .getByRole("button", {
        name: /^Approve Backoff table/,
      })
      .query(),
  ).toBeNull();
  expect(
    sheet(JOB)
      .getByRole("button", {
        name: /^Send back/,
      })
      .query(),
  ).toBeNull();
});

test("Esc closes the sheet and focus goes back to the Review row that opened it", async () => {
  await open();
  await openSheet(JOB);
  await userEvent.keyboard("{Escape}");
  await expect.element(sheet(JOB)).not.toBeInTheDocument();
  await expect.poll(() => focused() === reviewRow().element()).toBe(true);
});

test("Esc closes the sheet and focus goes back to the band when ⇧⌘Y opened it", async () => {
  await open();
  await expect.element(reviewRow()).toBeVisible();
  await userEvent.keyboard(SHIFT_META_Y);
  await expect.element(sheet(JOB)).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(sheet(JOB)).not.toBeInTheDocument();
  await expect.poll(() => focused() === bandKey().element()).toBe(true);
});

test("the last answer closes the sheet", async () => {
  await open();
  await openSheet(JOB);
  // The walk is put away from its menu, so nothing is left waiting on a page.
  await userEvent.click(row(JOB, /Backoff table/), {
    button: "right",
  });
  await userEvent.click(
    page.getByRole("menuitem", {
      name: /^Dismiss/,
    }),
  );
  await expect.element(marked(/Backoff table/, "Dismissed")).toBeVisible();

  for (const name of [
    /Plan v2/,
    /Backoff curve/,
    /Retry design/,
  ]) {
    await userEvent.click(row(JOB, name));
    await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  }
  // One is left, and the sheet is still up.
  await expect.element(sheet(JOB)).toBeVisible();
  await userEvent.click(row(JOB, /Pull request text/));
  await userEvent.click(
    sheet(JOB).getByRole("button", {
      name: /^Approve Pull request text/,
    }),
  );
  await expect.element(sheet(JOB)).not.toBeInTheDocument();
  expect(list(JOB).query()).toBeNull();
});
