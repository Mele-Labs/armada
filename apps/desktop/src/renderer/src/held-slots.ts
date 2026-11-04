// The slots a full pool is held by, as the call that offers to wait or add one carries them. A call with no `holders`, which is
// every call from a Fleet that read none, draws as it did: two choices and no rows.

import type { HeldSlotsProps } from "@armada/components";
import type { WaitingItem } from "@armada/protocol";
import { creatingSlot } from "@armada/screens/src/session-state";

/** The Add choice of a `slot_choice` call, which the rows' Create stands in for. */
export const ADD_CHOICE = 1;

/** Whether a call draws the held slots as rows, which is when Fleet sent some and something serves a Free. Its Add choice is then the rows' Create, and is not drawn twice. */
export const drawsHeldSlots = (item: WaitingItem, canFree: boolean): boolean =>
  item.source === "slot_choice" && item.holders !== undefined && item.holders.length > 0 && canFree;

/** The choices a call offers as buttons: all of them, except Add where the rows have a Create. A choice keeps its index in the item, which is what `answer_waiting` takes. */
export const choicesDrawn = (item: WaitingItem, canFree: boolean): number[] =>
  (item.options ?? []).flatMap((_, at) =>
    at === ADD_CHOICE && drawsHeldSlots(item, canFree)
      ? []
      : [
          at,
        ],
  );

export { creatingSlot };

/** The rows a call draws under its choices: none for another item, a call with no holders, or where nothing serves Free. */
export function heldSlotsOf(
  item: WaitingItem,
  free: ((slot: number) => void | Promise<unknown>) | undefined,
  open: (jobId: string) => void,
  create?: () => void | Promise<unknown>,
): HeldSlotsProps | undefined {
  const { holders, creating_since: creatingSince, create_refused: createRefused } = item;
  if (holders === undefined || !drawsHeldSlots(item, free !== undefined) || free === undefined)
    return undefined;
  return {
    rows: holders.map((one) => ({
      slot: one.slot,
      job: one.job,
      quietSince: one.quiet_since,
      ...(one.keeps === undefined
        ? {}
        : {
            keeps: one.keeps,
          }),
    })),
    onOpen: open,
    onFree: free,
    ...(create === undefined
      ? {}
      : {
          onCreate: create,
        }),
    ...(creatingSince === undefined
      ? {}
      : {
          creatingSince,
        }),
    ...(createRefused === undefined
      ? {}
      : {
          createRefused,
        }),
  };
}
