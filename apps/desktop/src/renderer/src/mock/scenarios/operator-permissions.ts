// The Operator's levels and its pause, for the walk `operatorPermissions`: the board of `operator-take-board`, paused past its
// share of the week, with a note the person sent straight to a Job under Done as his own.

import type { Scenario } from "../moment";
import { operatorTakeBoard } from "./operator-take-board";

const board = operatorTakeBoard.draft!.operator!;

export const operatorPermissions: Scenario = {
  ...operatorTakeBoard,
  name: "operator-permissions",
  // The week's plan past the share, read as the title bar's meter reads it.
  behaves: (handle) => ({
    ...(operatorTakeBoard.behaves?.(handle) ?? {}),
    getPlanUsage: async () => ({
      ok: true,
      value: {
        windows: [
          {
            kind: "seven_day",
            percent_used: 83,
          },
        ],
        read_at: new Date().toISOString(),
      },
    }),
    onPlanUsageChanged: () => () => {},
  }),
  says: "The Operator's Permissions as Read-only, Suggest and Decide, paused past its share of the week, and a note he sent to a Job under Done as his",
  draft: {
    ...operatorTakeBoard.draft,
    operator: {
      ...board,
      positions: {
        dispatch: "suggest",
        delegate: "suggest",
        answer: "read_only",
        file: "read_only",
      },
      pause: 80,
      paused: {
        week: 83,
        at: 80,
      },
      raises: [
        ...(board.raises ?? []),
        {
          id: "note-sent-placeholder",
          tag: "Note",
          suggests: false,
          does: "existing",
          repository: "armada",
          item: "Show a placeholder in an empty lane",
          read: "There is a lot of empty space here when nothing is in it",
          ways: [],
          settled: "Sent: to Rework the Map legend",
          settledBy: "You",
          settledAt: "2026-11-06T14:47:37.000Z",
        },
      ],
    },
  },
};
