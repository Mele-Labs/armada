// Now for a plan that ends on gates: one row a chunk, a line a group, what ends the live chunk and what is being done under it, every chunk's tasks folded until pressed; and the
// gate line that heads a finished chunk in Done.
// Browser: the drawing is what is read.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { WorkGroup } from "@armada/screens/src/draft/sessions";
import { mount, unmount } from "@armada/screens/src/mounted";

import { GateClosed, Helpers, PlanChunks } from "./PlanNow";
import { stretchesOf, type Stretch } from "./stretches";

afterEach(unmount);

const group = (id: string, title: string, extra: Partial<WorkGroup> = {}): WorkGroup => ({
  id,
  agent: "implementer",
  title,
  paths: [],
  tasks: [],
  ...extra,
});
const closed = {
  at: "2026-11-09T10:00:00Z",
  fact: "Plan approved by you",
};

const tasksOf = (stretch: Stretch) => <p>{`Tasks of ${stretch.name}`}</p>;

function chunks(
  groups: readonly WorkGroup[],
  held?: {
    group: string;
    since: string;
  },
) {
  mount(
    <PlanChunks
      stretches={stretchesOf(groups, held)}
      doing={<p>Writing the gate line</p>}
      tasksOf={tasksOf}
    />,
  );
}

/** Done, then Now over two groups ending on the walk, then Next. */
const PLAN = [
  group("a", "Plan the work", {
    gate: "plan_approved",
    gateClosed: closed,
  }),
  group("b", "Build the rows"),
  group("c", "Wire the gates", {
    gate: "walk_approved",
  }),
  group("d", "Land it", {
    gate: "merged",
  }),
];

test("the plan is a list named Plan with one row a chunk, and the live one is the current step", async () => {
  chunks(PLAN);
  await expect
    .element(
      page.getByRole("list", {
        name: "Plan",
      }),
    )
    .toBeVisible();
  const rows = page.getByRole("listitem");
  await expect.element(rows).toHaveLength(3);
  await expect.element(rows.nth(0)).toHaveTextContent("Plan the work");
  // A chunk of two groups names each on a line of its own.
  await expect.element(rows.nth(1)).toHaveTextContent("Build the rows");
  await expect.element(rows.nth(1)).toHaveTextContent("Wire the gates");
  expect(rows.nth(1).element().textContent).not.toContain("·");
  await expect.element(rows.nth(2)).toHaveTextContent("Land it");
  expect(rows.nth(0).element().getAttribute("aria-current")).toBeNull();
  expect(rows.nth(1).element().getAttribute("aria-current")).toBe("step");
  expect(rows.nth(2).element().getAttribute("aria-current")).toBeNull();
});

test("each chunk wears its state: done, now, next", async () => {
  chunks(PLAN);
  await expect
    .element(
      page.getByRole("img", {
        name: "Done",
        exact: true,
      }),
    )
    .toBeVisible();
  await expect
    .element(
      page.getByRole("img", {
        name: "Now",
        exact: true,
      }),
    )
    .toBeVisible();
  await expect
    .element(
      page.getByRole("img", {
        name: "Next",
        exact: true,
      }),
    )
    .toBeVisible();
});

test("a chunk that is not live is a button: its tasks show after a press and hide after a second", async () => {
  chunks(PLAN);
  const next = page.getByRole("button", {
    name: /Land it/,
  });
  await expect.element(next).toHaveAttribute("aria-expanded", "false");
  await expect.element(page.getByText("Tasks of Land it")).not.toBeInTheDocument();

  await userEvent.click(next);
  await expect.element(next).toHaveAttribute("aria-expanded", "true");
  await expect.element(page.getByText("Tasks of Land it")).toBeVisible();

  await userEvent.click(next);
  await expect.element(next).toHaveAttribute("aria-expanded", "false");
  await expect.element(page.getByText("Tasks of Land it")).not.toBeInTheDocument();
});

test("the live chunk shows what is being done at once, and its tasks only after the Tasks press", async () => {
  chunks(PLAN);
  await expect.element(page.getByText("Writing the gate line")).toBeVisible();
  // The live row is not a button of its own: only the Tasks press opens it.
  await expect
    .element(
      page.getByRole("button", {
        name: /Build the rows/,
      }),
    )
    .not.toBeInTheDocument();
  await expect
    .element(page.getByText("Tasks of Build the rows · Wire the gates"))
    .not.toBeInTheDocument();

  const tasks = page.getByRole("button", {
    name: "Tasks",
    exact: true,
  });
  await expect.element(tasks).toHaveAttribute("aria-expanded", "false");
  await userEvent.click(tasks);
  await expect.element(tasks).toHaveAttribute("aria-expanded", "true");
  await expect.element(page.getByText("Tasks of Build the rows · Wire the gates")).toBeVisible();
  await expect.element(page.getByText("Writing the gate line")).toBeVisible();

  await userEvent.click(tasks);
  await expect
    .element(page.getByText("Tasks of Build the rows · Wire the gates"))
    .not.toBeInTheDocument();
});

test("the live chunk says what ends it; a chunk to come says it only in its gate's tooltip", async () => {
  chunks(PLAN);
  // The glyph's tooltip carries the same words, drawn only on hover, so what counts is the words shown.
  const shown = (text: string) =>
    page
      .getByText(text)
      .elements()
      .filter((one) => one.checkVisibility());
  await expect.poll(() => shown("Ends when you approve the walk")).toHaveLength(1);
  expect(shown("Ends when it merges into main")).toHaveLength(0);
  await expect
    .element(
      page.getByRole("img", {
        name: "Ends when it merges into main",
      }),
    )
    .toBeVisible();
});

test("a closed gate reads the fact Fleet saw once its chunk is opened", async () => {
  chunks(PLAN);
  await expect.element(page.getByText("Plan approved by you")).not.toBeInTheDocument();
  await userEvent.click(
    page.getByRole("button", {
      name: /Plan the work/,
    }),
  );
  await expect.element(page.getByText("Plan approved by you")).toBeVisible();
});

test("a Checks gate that is running says at which commit", async () => {
  chunks([
    group("a", "Build", {
      gate: "checks_green",
      gateChecks: {
        commit: "abc1234",
        state: "running",
      },
    }),
  ]);
  await expect.element(page.getByText("Checks running at abc1234")).toBeVisible();
  await expect
    .element(
      page.getByRole("list", {
        name: "Tests",
      }),
    )
    .not.toBeInTheDocument();
});

test("a Checks gate that is red says at which commit and lists the tests that hold it and those red on main", async () => {
  chunks([
    group("a", "Build", {
      gate: "checks_green",
      gateChecks: {
        commit: "abc1234",
        state: "red",
        failed: [
          "plan_now_rows",
          "gate_line",
        ],
        already_red: [
          "flaky_on_main",
        ],
      },
    }),
  ]);
  await expect.element(page.getByText("Checks failed at abc1234")).toBeVisible();
  const tests = page.getByRole("list", {
    name: "Tests",
  });
  await expect.element(tests).toBeVisible();
  await expect.element(tests.getByRole("listitem")).toHaveLength(3);
  await expect.element(tests).toHaveTextContent("plan_now_rows");
  await expect.element(tests).toHaveTextContent("gate_line");
  await expect.element(tests).toHaveTextContent("flaky_on_main");
});

test("a gate with no tests to name draws no Tests list", async () => {
  chunks(PLAN);
  await expect
    .element(
      page.getByRole("list", {
        name: "Tests",
      }),
    )
    .not.toBeInTheDocument();
});

test("Done heads a finished chunk with the fact that closed its gate, and names the tests red on main", async () => {
  const [finished] = stretchesOf([
    group("a", "Build", {
      gate: "checks_green",
      gateClosed: {
        at: "2026-11-09T10:00:00Z",
        fact: "Checks passed at abc1234",
        already_red: [
          "flaky_on_main",
        ],
      },
    }),
  ]);
  mount(<GateClosed gate={finished!.gate} />);
  await expect
    .element(
      page.getByRole("img", {
        name: "Gate, closed by Fleet",
      }),
    )
    .toBeVisible();
  await expect.element(page.getByText("Checks passed at abc1234")).toBeVisible();
  await expect
    .element(
      page.getByRole("list", {
        name: "Tests",
      }),
    )
    .toHaveTextContent("flaky_on_main");
});

test("a gate glyph says whether the gate is open, closed, or holds the Job on him", async () => {
  chunks(PLAN);
  await expect
    .element(
      page.getByRole("img", {
        name: "Gate, closed by Fleet",
      }),
    )
    .toBeVisible();
  await expect
    .element(
      page.getByRole("img", {
        name: "Ends when you approve the walk",
      }),
    )
    .toBeVisible();
  await expect
    .element(
      page.getByRole("img", {
        name: "Ends when it merges into main",
      }),
    )
    .toBeVisible();
  await expect
    .element(
      page.getByRole("img", {
        name: "Gate, the Job is held on you",
      }),
    )
    .not.toBeInTheDocument();
});

test("the gate the Job is held on has the held glyph, and says how long", async () => {
  chunks(PLAN, {
    group: "c",
    since: new Date(Date.now() - 5 * 60_000).toISOString(),
  });
  await expect
    .element(
      page.getByRole("img", {
        name: "Gate, the Job is held on you",
      }),
    )
    .toBeVisible();
  await expect
    .element(
      page.getByRole("img", {
        name: "Ends when it merges into main",
      }),
    )
    .toBeVisible();
  await expect
    .element(
      page.getByRole("img", {
        name: /^Held on you for: /,
      }),
    )
    .toBeVisible();
});

test("a plan with a last run that ends on no gate draws that chunk with no gate glyph", async () => {
  chunks([
    group("a", "Plan the work", {
      gate: "plan_approved",
      gateClosed: closed,
    }),
    group("b", "Tidy up"),
  ]);
  await expect.element(page.getByRole("listitem")).toHaveLength(2);
  await expect
    .element(
      page.getByRole("img", {
        name: /^Gate, /,
      }),
    )
    .toHaveLength(1);
});

test("Helpers draws one button a subagent, and a press opens that one's thread", async () => {
  const onOpen = vi.fn();
  mount(
    <Helpers
      rows={[
        {
          id: "h1",
          task: "Write the rows",
          group: "Build the rows",
        },
        {
          id: "h2",
          task: "Read the gates",
        },
      ]}
      onOpen={onOpen}
    />,
  );
  await expect
    .element(
      page.getByRole("list", {
        name: "Subagents at work",
      }),
    )
    .toBeVisible();
  const rows = page.getByRole("button");
  await expect.element(rows).toHaveLength(2);

  await userEvent.click(
    page.getByRole("button", {
      name: /Read the gates/,
    }),
  );
  expect(onOpen).toHaveBeenCalledExactlyOnceWith("h2");
  await userEvent.click(
    page.getByRole("button", {
      name: /Write the rows/,
    }),
  );
  expect(onOpen).toHaveBeenLastCalledWith("h1");
});

test("Helpers names the group a subagent works on when its brief named one, and only then", async () => {
  mount(
    <Helpers
      rows={[
        {
          id: "h1",
          task: "Write the rows",
          group: "Build the rows",
        },
        {
          id: "h2",
          task: "Read the gates",
        },
      ]}
      onOpen={() => undefined}
    />,
  );
  await expect
    .element(
      page.getByRole("button", {
        name: /Write the rows/,
      }),
    )
    .toHaveTextContent("Build the rows");
  await expect
    .element(
      page.getByRole("button", {
        name: /Read the gates/,
      }),
    )
    .not.toHaveTextContent("Build the rows");
});

test("Helpers with no rows draws nothing", async () => {
  mount(
    <>
      <p>Beside</p>
      <Helpers rows={[]} onOpen={() => undefined} />
    </>,
  );
  await expect.element(page.getByText("Beside")).toBeVisible();
  // An empty list is not in the accessibility tree, so the page itself is read for it.
  expect(document.body.querySelector("ul")).toBeNull();
  expect(document.body.querySelector("button")).toBeNull();
});

test("Done draws no gate line for a gate still open", async () => {
  mount(
    <>
      <p>Beside</p>
      <GateClosed
        gate={
          stretchesOf([
            group("a", "Build", {
              gate: "merged",
            }),
          ])[0]!.gate
        }
      />
    </>,
  );
  await expect.element(page.getByText("Beside")).toBeVisible();
  expect(document.body.querySelector('[role="img"]')).toBeNull();
});
