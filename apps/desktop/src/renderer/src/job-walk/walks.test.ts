// The walks a Job owes him, read from its attachments. A real walk is owed while Fleet's item for it stands; the mock's own is owed until the mock approves it.

import type { Session } from "@armada/screens/src/draft/sessions";
import { beforeEach, describe, expect, test } from "vitest";

import { approveWalk, forgetWalks } from "./store";
import { owedWalks, walksOf } from "./walks";

const PAGE = "http://localhost:5394/?walk=billing";

const job = (parts: Partial<Session>): Session =>
  ({
    id: "job",
    title: "Billing",
    rows: [],
    attachments: [],
    ...parts,
  }) as unknown as Session;

const real = (id = PAGE) =>
  ({
    kind: "artifact",
    form: "window",
    id,
    title: "Billing page",
    window: {
      brief: "The plan card changed",
      takes: [
        "Compact",
        "Roomy",
      ],
      steps: [
        {
          say: "Look at the card",
        },
      ],
    },
  }) as const;

const waiting = (
  target = PAGE,
  source: "walk" | "agent" = "walk",
  kind: "walk" | "run" = "walk",
) => ({
  id: `walk:${target}`,
  text: "Billing page",
  since: "2026-11-03T10:00:00Z",
  source,
  act: {
    kind,
    target,
  },
});

beforeEach(forgetWalks);

describe("walksOf", () => {
  test("reads a real walk from its window row: the page's address, name, takes, steps and brief", () => {
    expect(
      walksOf(
        job({
          attachments: [
            real(),
          ],
        }),
      ),
    ).toEqual([
      {
        id: PAGE,
        title: "Billing page",
        brief: "The plan card changed",
        takes: [
          "Compact",
          "Roomy",
        ],
        steps: [
          {
            say: "Look at the card",
          },
        ],
        mock: false,
      },
    ]);
  });

  test("reads the mock's walk through the same function, and leaves every other artifact out", () => {
    const walks = walksOf(
      job({
        attachments: [
          {
            kind: "artifact",
            form: "window",
            id: "walk:billing",
            title: "Billing",
            walk: {
              steps: [
                {
                  say: "Look",
                  target: "plan",
                },
              ],
            },
          },
          {
            kind: "artifact",
            form: "file",
            id: "notes.md",
            title: "notes.md",
          },
          {
            kind: "artifact",
            form: "window",
            id: "http://x/bare",
            title: "A row with nothing of a walk",
          },
        ],
      }),
    );
    expect(walks).toHaveLength(1);
    expect(walks[0]).toMatchObject({
      id: "walk:billing",
      mock: true,
      takes: [],
    });
  });
});

describe("owedWalks", () => {
  test("a real walk is owed while the Job's walk item for its address stands", () => {
    const attachments = [
      real(),
    ];
    expect(
      owedWalks(
        job({
          attachments,
          waitingFor: [
            waiting(),
          ],
        }),
      ).map((one) => one.id),
    ).toEqual([
      PAGE,
    ]);
    // Approved: Fleet dropped the item, and the walk stays in the Job's record but is not owed.
    expect(
      owedWalks(
        job({
          attachments,
          waitingFor: [],
        }),
      ),
    ).toEqual([]);
  });

  test("an item for another address, another source or another act does not owe it", () => {
    const attachments = [
      real(),
    ];
    for (const item of [
      waiting("http://localhost:5394/?walk=settings"),
      waiting(PAGE, "agent"),
      waiting(PAGE, "walk", "run"),
    ])
      expect(
        owedWalks(
          job({
            attachments,
            waitingFor: [
              item,
            ],
          }),
        ),
      ).toEqual([]);
  });

  test("the mock's walk is owed until the mock approves it, whatever Fleet says", () => {
    const attachments = [
      {
        kind: "artifact",
        form: "window",
        id: "walk:billing",
        title: "Billing",
        walk: {
          steps: [],
        },
      },
    ] as const;
    expect(
      owedWalks(
        job({
          attachments: [
            ...attachments,
          ],
        }),
      ),
    ).toHaveLength(1);
    approveWalk("job", "walk:billing");
    expect(
      owedWalks(
        job({
          attachments: [
            ...attachments,
          ],
        }),
      ),
    ).toEqual([]);
  });
});
