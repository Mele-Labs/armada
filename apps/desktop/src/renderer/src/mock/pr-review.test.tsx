// The Reviews tab through `App`, on the `pr-review` scenario whose mock Fleet serves the shapes the real one does: the four
// lists read when the tab opens, a pull request's page with its threads, and a review posted as one call with its line
// comments placed by path, line and side. A refused review keeps what he wrote. Hand-run with the rest of `mock/`.

import type { WritePullRequest } from "@armada/protocol";
import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { MockForge } from "./review-forge";
import { s262PrReview } from "./scenarios/pr-review";
import { s262PrReviewPanes } from "./scenarios/pr-review-panes";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () =>
  page.getByRole("region", {
    name: "Pull requests",
  });
const reader = (number: number) =>
  page.getByRole("region", {
    name: `Pull request #${number}`,
  });
const yours = () =>
  page.getByRole("region", {
    name: "Your review",
  });
/** The tools pane's +, the second of the reader's three panes, and one of its entries. */
const plus = (number: number) =>
  reader(number)
    .getByRole("button", {
      name: "Add tab",
    })
    .nth(1);
async function pick(number: number, name: string) {
  await plus(number).click();
  await page
    .getByRole("menuitem", {
      name,
    })
    .click();
}
/** The Review tab's pane, and a line of its file by the gutter's name for it. */
const reviewPane = (number: number) =>
  reader(number).getByRole("region", {
    name: "Pane: Review",
  });
const gutter = (number: number, words: RegExp) =>
  reviewPane(number).getByRole("button", {
    name: new RegExp(`^Comment on line \\d+: ${words.source}`),
  });
async function openReview(number: number) {
  await reader(number)
    .getByRole("tab", {
      name: "Review",
      exact: true,
    })
    .click();
}
async function reviewChanges(number: number) {
  await reviewPane(number)
    .getByRole("button", {
      name: "Review changes",
    })
    .click();
}
const row = (number: number) =>
  list().getByRole("button", {
    name: new RegExp(`^#${number} `),
  });

async function reviews(scenario = s262PrReview) {
  const app = mount(scenario);
  await onScreen();
  await page
    .getByRole("navigation", {
      name: "Places",
    })
    .getByRole("tab", {
      name: "Reviews",
    })
    .click();
  await expect.element(list()).toBeVisible();
  return app;
}

/** A row of the list opens the pull request in the reader the Reviews page keeps, and no side panel opens over it. */
async function open(number: number) {
  await row(number).click();
  await expect.element(reader(number)).toBeVisible();
  await expect
    .element(
      page.getByRole("dialog", {
        name: `Pull request #${number}`,
      }),
    )
    .not.toBeInTheDocument();
}

async function choose(name: string) {
  await list()
    .getByRole("button", {
      name: "List",
    })
    .click();
  await page
    .getByRole("menuitem", {
      name,
    })
    .click();
}

test("the tab lists the four lists as the forge answered them", async () => {
  await reviews();
  // Requested for Review is where it opens.
  await expect.element(row(1899)).toBeVisible();
  await expect.element(row(212)).toBeVisible();
  await expect.element(row(1903)).not.toBeInTheDocument();

  await choose("Team");
  await expect.element(row(1903)).toBeVisible();
  await expect.element(row(1907)).toBeVisible();
  // A pull request on two lists is on both, with the team that was asked named on the Team list's row.
  await expect.element(row(1899)).toBeVisible();
  await expect
    .element(
      list().getByRole("note", {
        name: "@Mele-Labs/bridge",
      }),
    )
    .toBeVisible();
  await expect.element(row(212)).not.toBeInTheDocument();

  await choose("Assigned");
  await expect.element(row(1908)).toBeVisible();
  await expect.element(row(1899)).toBeVisible();
  await expect.element(row(1903)).not.toBeInTheDocument();

  await choose("My PRs");
  await expect.element(row(1896)).toBeVisible();
  await expect.element(row(1892)).toBeVisible();
  await expect.element(row(1899)).not.toBeInTheDocument();
});

test("a review goes out as one call with its line comments placed, and then it is recorded", async () => {
  const app = await reviews();
  const wrote: WritePullRequest[] = [];
  const write = app.api.writeReview;
  app.api.writeReview = async (manifestId, one) => {
    wrote.push(one);
    return write(manifestId, one);
  };

  await open(1899);
  await openReview(1899);
  const comment = async (words: RegExp, body: string, add: string) => {
    await gutter(1899, words).click();
    await reviewPane(1899)
      .getByRole("textbox", {
        name: "Comment",
      })
      .fill(body);
    await reviewPane(1899)
      .getByRole("button", {
        name: add,
      })
      .click();
  };
  await comment(
    /let mut held = self\.held\.lock/,
    "Take the lock after the call.",
    "Start a review",
  );
  await comment(
    /if let Some\(one\) = held\.as_ref\(\)/,
    "This branch wants a test.",
    "Add to review (1)",
  );
  await reviewChanges(1899);
  await expect
    .element(
      yours().getByRole("list", {
        name: "Pending comments",
      }),
    )
    .toBeVisible();

  await yours()
    .getByRole("textbox", {
      name: "Summary",
    })
    .fill("The lock goes after the call.");
  await yours()
    .getByRole("tab", {
      name: "Request changes",
    })
    .click();
  await yours()
    .getByRole("button", {
      name: "Submit review",
    })
    .click();

  await expect
    .element(
      yours().getByRole("list", {
        name: "Submitted reviews",
      }),
    )
    .toBeVisible();
  await expect
    .element(
      yours().getByRole("list", {
        name: "Pending comments",
      }),
    )
    .not.toBeInTheDocument();
  await expect
    .element(
      list().getByRole("img", {
        name: "Changes requested",
      }),
    )
    .toBeVisible();

  expect(wrote).toHaveLength(1);
  const sent = wrote[0]!;
  expect(sent).toMatchObject({
    pull_request: 1899,
    kind: "request_changes",
    body: "The lock goes after the call.",
  });
  expect(sent.commit).toBeTruthy();
  // A line is a place in a file, not an index into the drawn lines.
  expect(sent.comments).toEqual([
    {
      kind: "line",
      path: expect.stringContaining("/"),
      line: 31,
      side: "right",
      body: "Take the lock after the call.",
    },
    {
      kind: "line",
      path: expect.stringContaining("/"),
      line: 32,
      side: "right",
      body: "This branch wants a test.",
    },
  ]);
});

test("a review the forge refuses keeps the comment and the summary, and says why", async () => {
  const app = await reviews();
  (app.api as typeof app.api & Pick<MockForge, "refuseNextWrite">).refuseNextWrite(
    "Can not approve your own pull request",
  );

  await open(1899);
  await openReview(1899);
  await reviewChanges(1899);
  await yours()
    .getByRole("textbox", {
      name: "Summary",
    })
    .fill("Looks right to me.");
  await yours()
    .getByRole("tab", {
      name: "Approve",
    })
    .click();
  await yours()
    .getByRole("button", {
      name: "Submit review",
    })
    .click();

  await expect
    .element(yours().getByRole("alert"))
    .toHaveTextContent("Can not approve your own pull request");
  await expect
    .element(
      yours().getByRole("textbox", {
        name: "Summary",
      }),
    )
    .toHaveValue("Looks right to me.");
  await expect
    .element(
      yours().getByRole("list", {
        name: "Submitted reviews",
      }),
    )
    .not.toBeInTheDocument();

  // Pressed again, the forge takes it.
  await yours()
    .getByRole("button", {
      name: "Submit review",
    })
    .click();
  await expect
    .element(
      yours().getByRole("list", {
        name: "Submitted reviews",
      }),
    )
    .toBeVisible();
  await expect.element(yours().getByRole("alert")).not.toBeInTheDocument();
});

test("a tool that found nothing draws its picture and says what it checked", async () => {
  await reviews();
  await open(212);
  await pick(212, "Risks");
  // The tab holds the Run: opening it spends nothing while the setting asks first.
  await reader(212)
    .getByRole("button", {
      name: "Run",
      exact: true,
    })
    .click();
  const results = page.getByRole("region", {
    name: "Risks results",
  });
  await expect
    .element(
      results.getByRole("img", {
        name: "Nothing found",
      }),
    )
    .toBeVisible();
  await expect.element(results.getByText(/^No risks in \d+ changes?$/)).toBeVisible();
});

test("a pull request one of his Sessions holds opens with what was kept for it, and no run is asked for", async () => {
  await reviews();
  await choose("My PRs");
  await open(1896);
  await expect
    .element(
      reader(1896).getByRole("button", {
        name: "Job: Fix the reconnect wait",
      }),
    )
    .toBeVisible();
  await pick(1896, "How it works");
  const how = reader(1896).getByRole("region", {
    name: "How it works",
  });
  await expect
    .element(
      how.getByRole("img", {
        name: /^Kept/,
      }),
    )
    .toBeVisible();
  await pick(1896, "Why");
  await expect
    .element(
      reader(1896)
        .getByRole("region", {
          name: "Why",
        })
        .getByRole("img", {
          name: /^Kept/,
        }),
    )
    .toBeVisible();
  await expect.element(page.getByRole("dialog")).not.toBeInTheDocument();
});

test("a run that fails shows Fleet's reason where the result would be, and Run again starts another", async () => {
  const app = await reviews();
  (app.api as typeof app.api & Pick<MockForge, "failNextRun">).failNextRun(
    "The answer was not valid JSON",
  );
  await open(212);
  await pick(212, "Risks");
  // The tab holds the Run: opening it spends nothing while the setting asks first.
  await reader(212)
    .getByRole("button", {
      name: "Run",
      exact: true,
    })
    .click();
  const results = page.getByRole("region", {
    name: "Risks results",
  });
  await expect
    .element(results.getByRole("alert"))
    .toHaveTextContent("The answer was not valid JSON");
  await expect
    .element(
      results.getByRole("img", {
        name: "Nothing found",
      }),
    )
    .not.toBeInTheDocument();

  await results
    .getByRole("button", {
      name: "Run again",
    })
    .click();
  await expect
    .element(
      results.getByRole("img", {
        name: "Nothing found",
      }),
    )
    .toBeVisible();
  await expect.element(results.getByRole("alert")).not.toBeInTheDocument();
});

test("Esc in the list leaves the pull request open: only another row changes it", async () => {
  await reviews();
  await open(1899);
  (row(1899).element() as HTMLElement).focus();
  await userEvent.keyboard("{Escape}");
  await userEvent.keyboard("{Escape}");
  await expect.element(reader(1899)).toBeVisible();
  await open(212);
  await expect.element(reader(1899)).not.toBeInTheDocument();
});

test("a pull request opened by its link has a row above whichever list is chosen", async () => {
  await reviews();
  await list()
    .getByRole("textbox", {
      name: "Pull request link",
    })
    .fill("https://forge.example/mele-labs/armada/pull/1905");
  await userEvent.keyboard("{Enter}");
  await expect.element(reader(1905)).toBeVisible();
  await expect.element(row(1905)).toHaveAttribute("aria-current", "true");
  await open(1899);
  await choose("Team");
  await row(1905).click();
  await expect.element(reader(1905)).toBeVisible();
});

test("Review, a tab after Files, holds his review under Review changes, with Comment, Approve and Request changes", async () => {
  await reviews();
  await open(1899);
  await openReview(1899);
  await expect.element(yours()).not.toBeInTheDocument();
  await reviewChanges(1899);
  await expect
    .element(
      yours().getByRole("tab", {
        name: "Approve",
      }),
    )
    .toBeVisible();
  await expect
    .element(
      yours().getByRole("tab", {
        name: "Request changes",
      }),
    )
    .toBeVisible();
  await expect
    .element(
      yours().getByRole("button", {
        name: "Submit review",
      }),
    )
    .toBeVisible();
});

test("a pending comment says the file, line and code it is on, and a press goes to it in its file", async () => {
  await reviews();
  await open(1899);
  await openReview(1899);
  await gutter(1899, /let mut held = self\.held\.lock/).click();
  await reviewPane(1899)
    .getByRole("textbox", {
      name: "Comment",
    })
    .fill("Take the lock after the call.");
  await reviewPane(1899)
    .getByRole("button", {
      name: "Start a review",
    })
    .click();
  await reviewChanges(1899);

  const pending = yours().getByRole("button", {
    name: /^Go to your comment on .+:31$/,
  });
  await expect.element(pending).toHaveTextContent("let mut held = self.held.lock");
  const card = reviewPane(1899).getByRole("article", {
    name: "Pending comment",
  });
  await expect.element(card).toBeVisible();
  // Another file, where the comment is not drawn, and the press reads its file again.
  const at = reviewPane(1899).getByText(/^file \d+ of \d+$/);
  const was = at.element().textContent ?? "";
  await reviewPane(1899)
    .getByRole("button", {
      name: was.startsWith("file 1 ") ? "Next file" : "Previous file",
    })
    .click();
  await expect.element(card).not.toBeInTheDocument();

  await pending.click();
  await expect.element(at).toHaveTextContent(was);
  await expect.element(card).toHaveFocus();
});

test("the tools pane's + lists the explanations beside the tools, and no Comments tool", async () => {
  await reviews();
  await open(1899);
  await plus(1899).click();
  await expect
    .element(
      page.getByRole("menuitem", {
        name: "Risks",
      }),
    )
    .toBeVisible();
  await page
    .getByRole("menuitem", {
      name: "How it works",
    })
    .click();
  // The tab holds the Run: opening it spends nothing while the setting asks first.
  await reader(1899)
    .getByRole("button", {
      name: "Run",
      exact: true,
    })
    .click();
  await expect
    .element(
      reader(1899).getByRole("region", {
        name: "Theme: Fleet keeps the last read",
      }),
    )
    .toBeVisible();
  await expect
    .element(
      reader(1899).getByRole("tab", {
        name: "How it works",
      }),
    )
    .toHaveAttribute("aria-selected", "true");
});

test("a tool whose name is taken is refused in Fleet's words, beside Save", async () => {
  await reviews();
  await open(1899);
  await pick(1899, "Add a tool");
  const sheet = page.getByRole("dialog", {
    name: "Add a tool",
  });
  await sheet
    .getByRole("textbox", {
      name: "Name",
    })
    .fill("Test gaps");
  await sheet
    .getByRole("textbox", {
      name: "What it checks",
    })
    .fill("What no test would notice.");
  await sheet
    .getByRole("button", {
      name: "Save",
    })
    .click();
  await expect
    .element(sheet.getByRole("alert"))
    .toHaveTextContent("test-gaps.md is already in the Kit");
  await expect
    .element(
      sheet.getByRole("list", {
        name: "Left out",
      }),
    )
    .toBeVisible();
});

/** A test that opens panes, runs a tool and reads more than once, which a busy machine draws slowly. */
const SLOW = 90_000;

/** The + menu's rows as the person reads them, each with the number or hint it carries. */
const menuRows = () =>
  page
    .getByRole("menuitem")
    .elements()
    .map((one) => one.textContent ?? "");

test(
  "the tools pane's + offers the pull request's tabs, every tool with its number, and Add a tool",
  async () => {
    await reviews();
    await open(1899);
    await plus(1899).click();
    await expect
      .element(
        page.getByRole("menuitem", {
          name: "Add a tool",
        }),
      )
      .toBeVisible();
    const rows = menuRows();
    expect(rows.slice(0, 7)).toEqual([
      "Overview",
      "Files",
      "Review",
      "Checks",
      "Commits",
      "Comments",
      "Ask",
    ]);
    // The explanations come first, then the tools; the first nine carry the key that opens them.
    expect(rows.slice(7, 10).map((one) => one.replace(/Armada's tool$/, ""))).toEqual([
      "How it works1",
      "User Impact2",
      "How to Test3",
    ]);
    expect(rows.find((one) => one.startsWith("Test gaps"))).toMatch(/^Test gaps9/);
    expect(rows.at(-1)).toBe("Add a tool");
    // None of the Job's own kinds is offered.
    for (const jobs of [
      "Browser",
      "Terminal",
      "Thread",
      "Custom",
    ])
      expect(rows, jobs).not.toContain(jobs);
  },
  SLOW,
);

test(
  "opening a tool again from + brings its tab forward and opens no second",
  async () => {
    await reviews();
    await open(1899);
    await pick(1899, "Risks");
    await pick(1899, "Possible bugs");
    const tabs = (name: string) =>
      reader(1899).getByRole("tab", {
        name,
        exact: true,
      });
    await expect.element(tabs("Possible bugs")).toHaveAttribute("aria-selected", "true");
    await pick(1899, "Risks");
    await expect.element(tabs("Risks")).toHaveAttribute("aria-selected", "true");
    expect(tabs("Risks").elements()).toHaveLength(1);
  },
  SLOW,
);

test(
  "a tool opened with Don't ask again ticked on a Run lets the next one run with no Run to press",
  async () => {
    await reviews();
    await open(1899);
    await pick(1899, "Risks");
    await expect.element(reader(1899).getByText("Running this uses tokens.")).toBeVisible();
    await reader(1899)
      .getByRole("checkbox", {
        name: "Don't ask again",
      })
      .click();
    await reader(1899)
      .getByRole("button", {
        name: "Run",
        exact: true,
      })
      .click();
    await expect
      .element(
        reader(1899).getByRole("region", {
          name: "Risks results",
        }),
      )
      .toBeVisible();
    // The next tool opens and runs: its tab holds no Run and says nothing about tokens.
    await pick(1899, "Possible bugs");
    await expect
      .element(
        reader(1899).getByRole("region", {
          name: "Possible bugs results",
        }),
      )
      .toBeVisible();
    await expect
      .element(reader(1899).getByText("Running this uses tokens."))
      .not.toBeInTheDocument();
  },
  SLOW,
);

test(
  "an empty tools pane lists the tools, and a row opens its tool's tab",
  async () => {
    await reviews();
    await open(1899);
    const tools = reader(1899).getByRole("list", {
      name: "Tools",
    });
    await expect.element(tools).toBeVisible();
    await expect
      .element(
        tools.getByRole("button", {
          name: "Risks",
        }),
      )
      .toBeVisible();
    await expect
      .element(
        reader(1899).getByRole("button", {
          name: "Add a tool",
        }),
      )
      .toBeVisible();
    await tools
      .getByRole("button", {
        name: "Risks",
      })
      .click();
    await expect
      .element(
        reader(1899).getByRole("tab", {
          name: "Risks",
          exact: true,
        }),
      )
      .toBeVisible();
  },
  SLOW,
);

const comments = (number: number) =>
  reader(number).getByRole("region", {
    name: "Comments",
    exact: true,
  });
const go = (number: number) =>
  comments(number).getByRole("button", {
    name: /^Go to crates\/fleet\/src\/plan_usage\.rs:\d+ in Files$/,
  });

test(
  "Comments lists each open thread by its file and line, folds the resolved under their count, and opens a thread in place with Reply and Resolve",
  async () => {
    await reviews();
    await open(1899);
    await expect.element(go(1899)).toBeVisible();
    await expect
      .element(
        comments(1899).getByRole("button", {
          name: /^Go to packages\/protocol\/src\/plan-usage\.ts:\d+ in Files$/,
        }),
      )
      .toBeVisible();
    // The resolved one is not listed until its fold is opened, and the fold says how many it holds.
    const fold = comments(1899).getByRole("button", {
      name: /^Resolved/,
    });
    await expect.element(fold).toHaveTextContent("1");
    await expect
      .element(
        comments(1899).getByRole("list", {
          name: "Resolved threads",
        }),
      )
      .not.toBeInTheDocument();
    await fold.click();
    await expect
      .element(
        comments(1899).getByRole("list", {
          name: "Resolved threads",
        }),
      )
      .toBeVisible();

    await expect
      .element(
        comments(1899).getByRole("button", {
          name: "Reply",
        }),
      )
      .not.toBeInTheDocument();
    await comments(1899)
      .getByRole("button", {
        name: /^Open the thread on crates\/fleet\/src\/plan_usage\.rs:/,
      })
      .click();
    await expect
      .element(
        comments(1899).getByRole("button", {
          name: "Reply",
        }),
      )
      .toBeVisible();
    await expect
      .element(
        comments(1899).getByRole("button", {
          name: "Resolve",
          exact: true,
        }),
      )
      .toBeVisible();
  },
  SLOW,
);

test(
  "a press on a thread brings Files forward at its line, with the thread under it",
  async () => {
    await reviews();
    await open(1899);
    const files = reader(1899).getByRole("region", {
      name: "Pane: Files",
    });
    // The protocol file is the last of the changes, so its line is below the fold until Files is sent to it.
    await comments(1899)
      .getByRole("button", {
        name: /^Go to packages\/protocol\/src\/plan-usage\.ts:\d+ in Files$/,
      })
      .click();
    await expect
      .element(
        reader(1899).getByRole("tab", {
          name: "Files",
          exact: true,
        }),
      )
      .toHaveAttribute("aria-selected", "true");
    const thread = files.getByRole("article", {
      name: /^Thread on .+ by priya$/,
    });
    await expect.element(thread).toBeVisible();
    // The line before the thread is the one it is on, drawn by index, and scrolled into the pane's view.
    const box = files.element().querySelector(".pr-pane__files")!.getBoundingClientRect();
    await vi.waitFor(() => {
      const at = thread.element().getBoundingClientRect();
      expect(at.top).toBeGreaterThanOrEqual(box.top - 1);
      expect(at.bottom).toBeLessThanOrEqual(box.bottom + 1);
    });
    expect(files.element().querySelector("[data-diff-line]")).not.toBeNull();
  },
  SLOW,
);

/** Another pull request and back: the first reader is drawn again from what was kept. */
async function leaveAndReturn() {
  await row(212).click();
  await expect.element(reader(212)).toBeVisible();
  await row(1899).click();
  await expect.element(reader(1899)).toBeVisible();
}

test(
  "Files comes back scrolled to where he was, and the threads he had open in Comments are open",
  async () => {
    await reviews();
    await open(1899);
    await reader(1899)
      .getByRole("tab", {
        name: "Files",
        exact: true,
      })
      .click();
    const files = reader(1899).getByRole("region", {
      name: "Pane: Files",
    });
    await expect
      .element(
        files.getByRole("region", {
          name: "crates/fleet/src/plan_usage.rs",
        }),
      )
      .toBeVisible();
    const scroller = () => files.element().querySelector<HTMLElement>(".pr-pane__files")!;
    await vi.waitFor(() => {
      scroller().scrollTop = 150;
      expect(scroller().scrollTop).toBeGreaterThan(100);
    });
    const where = scroller().scrollTop;
    await new Promise((done) => setTimeout(done, 300));
    // A thread opened in Comments is kept per pull request too.
    await comments(1899)
      .getByRole("button", {
        name: /^Open the thread on packages\/protocol\/src\/plan-usage\.ts:/,
      })
      .click();
    await expect
      .element(
        comments(1899).getByRole("article", {
          name: /^Thread on packages\/protocol\/src\/plan-usage\.ts/,
        }),
      )
      .toBeVisible();

    await leaveAndReturn();
    await expect
      .element(
        comments(1899).getByRole("article", {
          name: /^Thread on packages\/protocol\/src\/plan-usage\.ts/,
        }),
      )
      .toBeVisible();
    await vi.waitFor(() => {
      expect(Math.abs(scroller().scrollTop - where)).toBeLessThanOrEqual(2);
    });
  },
  SLOW,
);

/** The panes scenario, where a commit is pushed to #1899 and the next read of the lists brings it. */
async function pushed() {
  await reviews(s262PrReviewPanes);
}
const moved = (number: number) =>
  reader(number).getByRole("status", {
    name: "Pull request changed",
  });
const refresh = () =>
  list().getByRole("button", {
    name: "Refresh",
  });
/** The commit he last looked at, as it is kept for #1899. */
const seenOf = () => localStorage.getItem("armada:kept:pr-review:mele-labs/armada#1899:seen-head");

/** Runs Risks on #1899, then leaves, has the lists read again and comes back. */
async function movedAfterARun() {
  await pushed();
  await open(1899);
  await pick(1899, "Risks");
  await reader(1899)
    .getByRole("button", {
      name: "Run",
      exact: true,
    })
    .click();
  // Done, not only started: a run still going is not one kept on the older commit.
  await expect
    .element(
      reader(1899)
        .getByRole("region", {
          name: "Risks results",
        })
        .getByRole("img", {
          name: /^Kept\. Ran/,
        }),
    )
    .toBeVisible();
  await row(212).click();
  await expect.element(reader(212)).toBeVisible();
  await refresh().click();
  await row(1899).click();
  await expect.element(reader(1899)).toBeVisible();
  await expect.element(moved(1899)).toBeVisible();
}

test(
  "a pull request that moved since he ran something says so, from one commit to the next",
  async () => {
    await movedAfterARun();
    await expect.element(moved(1899)).toHaveTextContent("02d7a8c");
    await expect.element(moved(1899)).toHaveTextContent("e4f5a6b");
  },
  SLOW,
);

test(
  "a pull request that moved with nothing run on the older commit says nothing, and the new commit is recorded as seen",
  async () => {
    await pushed();
    await open(1899);
    await row(212).click();
    await expect.element(reader(212)).toBeVisible();
    await refresh().click();
    await row(1899).click();
    await expect.element(reader(1899)).toBeVisible();
    await expect.element(moved(1899)).not.toBeInTheDocument();
    await vi.waitFor(() => expect(seenOf()).toContain("e4f5a6b"));
  },
  SLOW,
);

test(
  "closing the line marks the new commit seen, and it stays gone when he comes back",
  async () => {
    await movedAfterARun();
    await moved(1899)
      .getByRole("button", {
        name: "Mark as seen",
      })
      .click();
    await expect.element(moved(1899)).not.toBeInTheDocument();
    await vi.waitFor(() => expect(seenOf()).toContain("e4f5a6b"));
    await leaveAndReturn();
    await expect.element(moved(1899)).not.toBeInTheDocument();
  },
  SLOW,
);

test(
  "Re-run runs what was kept on the older commit again at once, and the line goes",
  async () => {
    await movedAfterARun();
    await reader(1899)
      .getByRole("tab", {
        name: "Risks",
        exact: true,
      })
      .click();
    await expect
      .element(
        reader(1899)
          .getByRole("region", {
            name: "Risks results",
          })
          .getByRole("img", {
            name: /^Out of date\. Ran on/,
          }),
      )
      .toBeVisible();
    await moved(1899)
      .getByRole("button", {
        name: "Re-run",
      })
      .click();
    await expect.element(moved(1899)).not.toBeInTheDocument();
    // It ran on the new commit, with no box asking first.
    await expect
      .element(
        reader(1899)
          .getByRole("region", {
            name: "Risks results",
          })
          .getByRole("img", {
            name: /^Kept\. Ran .* on e4f5a6b/,
          }),
      )
      .toBeVisible();
    await expect.element(page.getByRole("dialog")).not.toBeInTheDocument();
  },
  SLOW,
);
