// Mock only: what the Operator view reads while it has no Gateway behind it. Two Macs, the Operator's recent
// sends and the words it decides with. Nothing here is on the wire except the `operator.dispatched` fields.

import { besideOf } from "./seed";

export type Way = "started" | "handed" | "asked" | "parked";
export type Kind = "idea" | "feature" | "bug";

/** `MacLink` (crates/ipc/src/phone.rs), the two states this view tells apart. */
export type Link =
  | {
      state: "linked";
    }
  | {
      state: "asleep";
      /** Unix seconds. */
      since: number;
    };

export type OperatorMac = {
  name: string;
  /** Where its Jobs open. */
  origin: string;
  /** The Mac serving this phone's app. */
  here: boolean;
  link: Link;
  /** The repositories checked out on it. */
  repositories: string[];
};

/** The fields of the event `operator.dispatched`. */
export type Dispatched = {
  dispatch_id: string;
  manifest_id: string;
  way: Way;
  session_id?: string;
  title?: string;
};

export type Stage =
  | {
      phase: "reading";
    }
  | ({
      phase: "dispatched";
    } & Dispatched)
  | {
      phase: "queued";
      /** Unix seconds: when the Mac stopped answering. */
      since: number;
    }
  | {
      phase: "refused";
      why: string;
    };

export type Send = {
  id: string;
  /** RFC 3339. */
  at: string;
  mac: string;
  repository: string;
  words: string;
  kind?: Kind;
  stage: Stage;
};

const ago = (now: number, minutes: number) => new Date(now - minutes * 60_000).toISOString();

export function operatorMacs(here: string, asleep: boolean, now: number): OperatorMac[] {
  return [
    {
      name: "studio",
      origin: here,
      here: true,
      link: {
        state: "linked",
      },
      repositories: [
        "armada",
        "store",
      ],
    },
    {
      name: "laptop",
      origin: besideOf(here),
      here: false,
      link: asleep
        ? {
            state: "asleep",
            since: Math.floor((now - 23 * 60_000) / 1000),
          }
        : {
            state: "linked",
          },
      repositories: [
        "armada",
        "site",
      ],
    },
  ];
}

/** Recent sends, newest first. The asleep variant opens with a send already waiting for the laptop. */
export function operatorSends(asleep: boolean, now: number): Send[] {
  const waiting: Send[] = asleep
    ? [
        {
          id: "d9",
          at: ago(now, 6),
          mac: "laptop",
          repository: "site",
          words: "Move the pricing table above the fold",
          kind: "feature",
          stage: {
            phase: "queued",
            since: Math.floor((now - 23 * 60_000) / 1000),
          },
        },
      ]
    : [];
  return [
    ...waiting,
    {
      id: "d8",
      at: ago(now, 4),
      mac: "studio",
      repository: "armada",
      words: "Cut the Drone pool to four and make the count a setting",
      kind: "feature",
      stage: {
        phase: "dispatched",
        dispatch_id: "d8",
        manifest_id: "armada",
        way: "started",
        session_id: "a1",
        title: "Trim the Drone pool",
      },
    },
    {
      id: "d7",
      at: ago(now, 18),
      mac: "studio",
      repository: "armada",
      words: "The Needs you band clips on a small phone",
      kind: "bug",
      stage: {
        phase: "dispatched",
        dispatch_id: "d7",
        manifest_id: "armada",
        way: "handed",
        session_id: "a2",
        title: "Redraw the Needs you band",
      },
    },
    {
      id: "d6",
      at: ago(now, 41),
      mac: "studio",
      repository: "armada",
      words: "Add the export",
      stage: {
        phase: "dispatched",
        dispatch_id: "d6",
        manifest_id: "armada",
        way: "asked",
      },
    },
    {
      id: "d5",
      at: ago(now, 75),
      mac: "studio",
      repository: "armada",
      words: "Maybe cache the slot listing, not now",
      kind: "idea",
      stage: {
        phase: "dispatched",
        dispatch_id: "i3",
        manifest_id: "armada",
        way: "parked",
        title: "Cache the slot listing",
      },
    },
    {
      id: "d4",
      at: ago(now, 130),
      mac: "laptop",
      repository: "site",
      words: "Rename the footer links",
      stage: {
        phase: "refused",
        why: "laptop runs an older build than this Mac, so it took nothing. Update laptop, then send again.",
      },
    },
  ];
}

/** A live Job to fold a bug into, by Mac and repository. */
const FOLDS: Record<string, [string, string]> = {
  "studio armada": [
    "a2",
    "Redraw the Needs you band",
  ],
  "studio store": [
    "a4",
    "Check the restart notes",
  ],
};

/** The first words of a send as a Job title, at most six of them. */
const titled = (words: string) => {
  const head = words.trim().split(/\s+/).slice(0, 6).join(" ");
  return head.charAt(0).toUpperCase() + head.slice(1);
};

/**
 * What the Operator does with a send, for the mock: a bug folds into a live Job of that repository where there is one, a line
 * ending in a question mark is asked, and anything else starts a Job. The kind is a hint, so an idea is not parked for being one.
 */
export function decide(send: Send): Stage {
  const base = {
    phase: "dispatched",
    dispatch_id: send.id,
    manifest_id: send.repository,
  } as const;
  if (send.words.trim().endsWith("?"))
    return {
      ...base,
      way: "asked",
    };
  const fold = send.kind === "bug" ? FOLDS[`${send.mac} ${send.repository}`] : undefined;
  if (fold !== undefined)
    return {
      ...base,
      way: "handed",
      session_id: fold[0],
      title: fold[1],
    };
  return {
    ...base,
    way: "started",
    session_id: send.mac === "laptop" ? "b1" : "a4",
    title: titled(send.words),
  };
}
