import { describe, expect, test, vi } from "vitest";
import type { WaitingItem } from "@armada/protocol";

import { choicesDrawn, creatingSlot, drawsHeldSlots, heldSlotsOf } from "./held-slots";

const SINCE = "2026-11-02T09:00:00.000Z";

const call = (extra: Partial<WaitingItem> = {}): WaitingItem => ({
  id: "slot-choice",
  text: "Every slot is in use",
  since: SINCE,
  source: "slot_choice",
  act: {
    kind: "answer",
    target: "slot-choice",
  },
  options: [
    {
      label: "Wait for a slot",
    },
    {
      label: "Add a slot",
    },
  ],
  ...extra,
});

const holders = [
  {
    slot: 2,
    job: {
      id: "j1",
      title: "Move the run ledger",
    },
    quiet_since: "2026-10-21T09:00:00.000Z",
  },
  {
    slot: 3,
    job: {
      id: "j2",
      title: "Rework the logbook filters",
    },
    quiet_since: "2026-10-29T09:00:00.000Z",
    keeps: "uncommitted" as const,
  },
];

describe("a call with no holders draws as it did", () => {
  test("two choices and no rows", () => {
    const wire = call();
    expect(heldSlotsOf(wire, vi.fn(), vi.fn(), vi.fn())).toBeUndefined();
    expect(choicesDrawn(wire, true)).toEqual([
      0,
      1,
    ]);
    expect(drawsHeldSlots(wire, true)).toBe(false);
  });

  test("an empty list of holders is the same", () => {
    expect(
      heldSlotsOf(
        call({
          holders: [],
        }),
        vi.fn(),
        vi.fn(),
        vi.fn(),
      ),
    ).toBeUndefined();
    expect(
      choicesDrawn(
        call({
          holders: [],
        }),
        true,
      ),
    ).toEqual([
      0,
      1,
    ]);
  });
});

describe("a call with holders", () => {
  test("draws a row for each, as Fleet sent them", () => {
    const open = vi.fn();
    const drawn = heldSlotsOf(
      call({
        holders,
      }),
      vi.fn(),
      open,
      vi.fn(),
    );
    expect(drawn?.rows).toEqual([
      {
        slot: 2,
        job: {
          id: "j1",
          title: "Move the run ledger",
        },
        quietSince: "2026-10-21T09:00:00.000Z",
      },
      {
        slot: 3,
        job: {
          id: "j2",
          title: "Rework the logbook filters",
        },
        quietSince: "2026-10-29T09:00:00.000Z",
        keeps: "uncommitted",
      },
    ]);
    expect(drawn?.onOpen).toBe(open);
  });

  test("hides the Add choice, which its Create stands in for, and keeps the index of the rest", () => {
    expect(
      choicesDrawn(
        call({
          holders,
        }),
        true,
      ),
    ).toEqual([
      0,
    ]);
  });

  test("draws no rows, and keeps both choices, where nothing serves a Free", () => {
    expect(
      heldSlotsOf(
        call({
          holders,
        }),
        undefined,
        vi.fn(),
        vi.fn(),
      ),
    ).toBeUndefined();
    expect(
      choicesDrawn(
        call({
          holders,
        }),
        false,
      ),
    ).toEqual([
      0,
      1,
    ]);
  });

  test("carries the wait and its refusal through", () => {
    const making = heldSlotsOf(
      call({
        holders,
        creating_since: SINCE,
      }),
      vi.fn(),
      vi.fn(),
      vi.fn(),
    );
    expect(making?.creatingSince).toBe(SINCE);
    expect(making?.createRefused).toBeUndefined();
    const refused = heldSlotsOf(
      call({
        holders,
        create_refused: "no space left on the disk",
      }),
      vi.fn(),
      vi.fn(),
      vi.fn(),
    );
    expect(refused?.createRefused).toBe("no space left on the disk");
    expect(refused?.creatingSince).toBeUndefined();
  });

  test("Create is the rows' own only where something serves it", () => {
    expect(
      heldSlotsOf(
        call({
          holders,
        }),
        vi.fn(),
        vi.fn(),
      )?.onCreate,
    ).toBeUndefined();
  });

  test("is only for a full pool's call", () => {
    expect(
      heldSlotsOf(
        call({
          holders,
          source: "agent",
        }),
        vi.fn(),
        vi.fn(),
        vi.fn(),
      ),
    ).toBeUndefined();
  });
});

describe("a slot being made", () => {
  test("is read off the call", () => {
    expect(
      creatingSlot(
        call({
          creating_since: SINCE,
        }),
      ),
    ).toBe(true);
    expect(creatingSlot(call())).toBe(false);
    expect(
      creatingSlot(
        call({
          create_refused: "no space left on the disk",
        }),
      ),
    ).toBe(false);
  });
});
