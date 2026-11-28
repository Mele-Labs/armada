// To review: what every Job on every paired Mac waits on, read from each Mac's `/api/sessions/to_review` and merged into
// one list grouped as Fleet groups it. A Mac's list is read again on every change its stream tells of, so an answered
// item leaves when Fleet stops listing it. `docs/practices/protocol/to-review-across-jobs.md` is the wire.

import { useEffect, useSyncExternalStore } from "react";

import { Refused, call, getJson, refusal } from "./client";
import { loadLists, onChange, useFleet, type Mac } from "./fleet";
import type { Machine } from "./machines";
import type {
  AnswerItems,
  AnswerWaiting,
  DismissWaiting,
  DoneWaiting,
  ItemsSettled,
  ToReview,
  ToReviewHead,
  ToReviewItem,
} from "./wire";

/** One item, with the Mac whose Fleet listed it. */
export type Entry = {
  mac: Mac;
  item: ToReviewItem;
};

/** A heading and its items, from every Mac that listed one under it. */
export type Group = {
  key: string;
  head: ToReviewHead;
  entries: Entry[];
};

/** Where a head stands among the others: sub projects, then features, then the Jobs in neither. */
const RANK: Record<ToReviewHead["kind"], number> = {
  sub_project: 0,
  feature: 1,
  neither: 2,
};

/** Whether two Macs' heads name one group: a sub project by its id, else its goal; a feature by its project and slug. */
function same(a: ToReviewHead, b: ToReviewHead): boolean {
  if (a.kind === "sub_project" && b.kind === "sub_project")
    return a.id === b.id || a.goal === b.goal;
  if (a.kind === "feature" && b.kind === "feature")
    return a.project === b.project && a.slug === b.slug;
  return a.kind === "neither" && b.kind === "neither";
}

const keyOf = (head: ToReviewHead): string =>
  head.kind === "sub_project"
    ? `sub_project ${head.id}`
    : head.kind === "feature"
      ? `feature ${head.project} ${head.slug}`
      : "neither";

/**
 * Every Mac's groups as one list. Each Mac's own order holds within its kind of head, the first Mac's before the next's,
 * and a head two Macs share is one group holding both Macs' items, the first Mac's first.
 */
export function mergedOf(
  reads: readonly {
    mac: Mac;
    list: ToReview;
  }[],
): Group[] {
  const groups: Group[] = [];
  for (const { mac, list } of reads)
    for (const group of list.groups) {
      const entries = group.items.map((item) => ({
        mac,
        item,
      }));
      const kept = groups.find((one) => same(one.head, group.head));
      if (kept === undefined)
        groups.push({
          key: keyOf(group.head),
          head: group.head,
          entries,
        });
      else kept.entries.push(...entries);
    }
  return groups
    .map((group, at) => ({
      group,
      at,
    }))
    .sort((a, b) => RANK[a.group.head.kind] - RANK[b.group.head.kind] || a.at - b.at)
    .map(({ group }) => group);
}

/** Each Mac's list as last read, or why it could not be. */
type Read = {
  list?: ToReview;
  error?: string;
};

let reads: Record<string, Read> = {};
const watchers = new Set<() => void>();
const watching = (watch: () => void) => {
  watchers.add(watch);
  return () => watchers.delete(watch);
};
const set = (origin: string, read: Read) => {
  reads = {
    ...reads,
    [origin]: read,
  };
  watchers.forEach((watch) => watch());
};

/** Reads one Mac's list again. */
export async function readList(machine: Machine): Promise<void> {
  try {
    const list = await getJson<ToReview>(machine, "/api/sessions/to_review");
    set(machine.origin, {
      list,
    });
  } catch (why) {
    set(machine.origin, {
      ...reads[machine.origin],
      error: refusal(why, machine),
    });
  }
}

export type ToReviewRead = {
  groups: Group[];
  /** More than one Mac is paired, so each item names its Mac. */
  several: boolean;
  /** Every paired Mac's list has been read once, or refused. */
  loaded: boolean;
  /** The Macs whose list could not be read, each with its sentence. */
  troubled: {
    mac: Mac;
    says: string;
  }[];
};

/** The merged list, read on mount and again on every change a paired Mac tells of. */
export function useToReview(): ToReviewRead {
  const fleet = useFleet();
  const seen = useSyncExternalStore(watching, () => reads);
  const paired = fleet.macs.filter((mac) => mac.paired);
  const origins = paired.map((mac) => mac.machine.origin).join(" ");
  useEffect(() => {
    paired.forEach((mac) => void readList(mac.machine));
    const waits = new Map<string, ReturnType<typeof setTimeout>>();
    const stop = onChange((origin) => {
      const mac = paired.find((one) => one.machine.origin === origin);
      if (mac === undefined) return;
      clearTimeout(waits.get(origin));
      waits.set(
        origin,
        setTimeout(() => void readList(mac.machine), 250),
      );
    });
    return () => {
      stop();
      waits.forEach((wait) => clearTimeout(wait));
    };
  }, [
    origins,
  ]);
  const listed = paired.flatMap((mac) => {
    const list = seen[mac.machine.origin]?.list;
    return list === undefined
      ? []
      : [
          {
            mac,
            list,
          },
        ];
  });
  return {
    groups: mergedOf(listed),
    several: fleet.macs.length > 1,
    loaded: fleet.loaded && paired.every((mac) => seen[mac.machine.origin] !== undefined),
    troubled: paired.flatMap((mac) => {
      const says = seen[mac.machine.origin]?.error;
      return says === undefined
        ? []
        : [
            {
              mac,
              says,
            },
          ];
    }),
  };
}

/** The answers an item takes here, each posted to its Mac and followed by a read of that Mac's list. */
export function answersOn(machine: Machine) {
  const after = () => {
    void readList(machine);
    void loadLists(machine.origin);
  };
  const send = (path: string) => async (body: unknown) => {
    await call(machine, path, {
      method: "POST",
      body,
    });
    after();
  };
  return {
    /** Approve, or send back with a note. Fleet answers 200 whatever became of the item, so a refusal is thrown here. */
    answerItems: async (body: AnswerItems) => {
      const settled = (await (
        await call(machine, "/api/sessions/items/answer", {
          method: "POST",
          body,
        })
      ).json()) as ItemsSettled;
      after();
      const refused = settled.outcomes.find((one) => one.outcome.kind === "refused")?.outcome;
      if (refused?.kind === "refused") throw new Refused(409, refused.message);
    },
    answerWaiting: send("/api/sessions/waiting/answer") as (body: AnswerWaiting) => Promise<void>,
    doneWaiting: send("/api/sessions/waiting/done") as (body: DoneWaiting) => Promise<void>,
    dismissWaiting: send("/api/sessions/waiting/dismiss") as (
      body: DismissWaiting,
    ) => Promise<void>,
  };
}
