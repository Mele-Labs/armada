// To review over several Macs: one list, a head two Macs share one group, and the kinds of head in Fleet's order.

import { describe, expect, it } from "vitest";

import type { Mac } from "./fleet";
import { mergedOf } from "./reviewList";
import type { ToReview, ToReviewHead, ToReviewItem } from "./wire";

const mac = (name: string): Mac => ({
  machine: {
    origin: `https://${name}.tail0.ts.net`,
    name,
    deviceId: `phone-on-${name}`,
  } as Mac["machine"],
  paired: true,
  needs: [],
  live: [],
});

const asked = (id: string): ToReviewItem => ({
  kind: "question",
  row: {
    id,
    source: "agent",
    text: id,
    since: "2026-11-27T10:00:00Z",
    glyph: {
      kind: "reply",
      said: id,
    },
    act: {
      kind: "reply",
    },
    dismiss: "plain",
  },
  order: 1,
  job: {
    id: `job-${id}`,
    title: id,
  },
  answer: {
    op: "answer_waiting",
  },
});

const goal = (id: string, words: string): ToReviewHead => ({
  kind: "sub_project",
  id,
  name: words,
  goal: words,
});
const feature = (slug: string): ToReviewHead => ({
  kind: "feature",
  project: "armada",
  slug,
  title: slug,
  area: "bridge",
  area_title: "Bridge",
  area_place: 1,
  place: 1,
});

const list = (
  ...groups: [
    ToReviewHead,
    string[],
  ][]
): ToReview => ({
  groups: groups.map(([head, ids]) => ({
    head,
    items: ids.map(asked),
  })),
});

describe("mergedOf", () => {
  const studio = mac("studio");
  const laptop = mac("laptop");

  it("joins a sub project two Macs list, by its id or else its goal, the first Mac's items first", () => {
    const groups = mergedOf([
      {
        mac: studio,
        list: list(
          [
            goal("sp-1", "Ship the phone"),
            [
              "a",
            ],
          ],
          [
            {
              kind: "neither",
            },
            [
              "n",
            ],
          ],
        ),
      },
      {
        mac: laptop,
        list: list(
          [
            goal("sp-9", "Ship the phone"),
            [
              "b",
            ],
          ],
          [
            goal("sp-2", "Other"),
            [
              "c",
            ],
          ],
        ),
      },
    ]);
    expect(
      groups.map((one) => [
        one.key,
        one.entries.map((entry) => entry.item.row.id),
      ]),
    ).toEqual([
      [
        "sub_project sp-1",
        [
          "a",
          "b",
        ],
      ],
      [
        "sub_project sp-2",
        [
          "c",
        ],
      ],
      [
        "neither",
        [
          "n",
        ],
      ],
    ]);
    expect(groups[0]!.entries.map((entry) => entry.mac.machine.name)).toEqual([
      "studio",
      "laptop",
    ]);
  });

  it("stands sub projects before features and features before the Jobs in neither, whichever Mac listed them", () => {
    const groups = mergedOf([
      {
        mac: studio,
        list: list(
          [
            feature("pocket"),
            [
              "f",
            ],
          ],
          [
            {
              kind: "neither",
            },
            [
              "n",
            ],
          ],
        ),
      },
      {
        mac: laptop,
        list: list(
          [
            goal("sp-1", "Ship the phone"),
            [
              "s",
            ],
          ],
          [
            feature("pocket"),
            [
              "g",
            ],
          ],
        ),
      },
    ]);
    expect(
      groups.map((one) => [
        one.head.kind,
        one.entries.map((entry) => entry.item.row.id),
      ]),
    ).toEqual([
      [
        "sub_project",
        [
          "s",
        ],
      ],
      [
        "feature",
        [
          "f",
          "g",
        ],
      ],
      [
        "neither",
        [
          "n",
        ],
      ],
    ]);
  });
});
