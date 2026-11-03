// A ticket's menu of the Ticket lifecycle's states, wherever a ticket row is drawn: choosing one teaches Armada that the tracker's status
// reads as that state (`teachTicketWord`), in the Workspace's Ticket lifecycle and every live Job's copy, so the Jobs' lanes follow by
// `job_lifecycles.changed`. The state the status reads as now is ticked, from the running Ticket lifecycle read each time the menu opens.
// A refusal is the window's toast, in Fleet's words.

import { useRef, useState, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import { DropdownMenu } from "@armada/components";
import type { DropdownMenuEntry } from "@armada/components";
import type { AppliedLifecycle } from "@armada/protocol";

import { raiseOutcome } from "./tell";

/** The states `teach_ticket_word` takes, with Armada's own label for each. A lifecycle that names one otherwise is read for its label. */
const TICKET_STATES = [
  {
    id: "todo",
    label: "To do",
  },
  {
    id: "in_progress",
    label: "In progress",
  },
  {
    id: "in_review",
    label: "In review",
  },
  {
    id: "closed",
    label: "Closed",
  },
] as const;

const PICK = "ticket-state:";
const REACH: Readonly<Record<string, number>> = {
  everywhere: 0,
  workspace: 1,
  repository: 2,
};

/** What the tracker calls a ticket's state: its own status, else `Open` or `Closed`, as Fleet reads it. */
export const ticketWordOf = (status: string | undefined, state: "open" | "closed"): string => {
  const said = status?.trim() ?? "";
  return said !== "" ? said : state === "closed" ? "Closed" : "Open";
};

/** The menu's states, each labelled as the running Ticket lifecycle labels it, and the one whose tracker words hold `word` exactly. */
export type TicketReading = {
  states: readonly {
    id: string;
    label: string;
  }[];
  ticked?: string;
};

/** The Ticket lifecycle that runs for the repository: of those not replaced, the one kept nearest it. */
function runningTicket(applied: readonly AppliedLifecycle[]): AppliedLifecycle | undefined {
  return applied
    .filter((one) => one.lifecycle.id === "ticket" && one.kept.overridden_by === undefined)
    .sort((a, b) => (REACH[b.kept.rank.reach] ?? 0) - (REACH[a.kept.rank.reach] ?? 0))[0];
}

export async function readTicketWord(manifestId: string, word: string): Promise<TicketReading> {
  const read = await window.armada.listLifecycles(manifestId);
  const ticket = read.ok ? runningTicket(read.value.lifecycles)?.lifecycle : undefined;
  const ticked = ticket?.states.find(
    (one) => one.read.source === "tracker" && one.read.words.includes(word),
  )?.id;
  return {
    states: TICKET_STATES.map(({ id, label }) => ({
      id,
      label: ticket?.states.find((one) => one.id === id)?.label ?? label,
    })),
    ...(ticked === undefined
      ? {}
      : {
          ticked,
        }),
  };
}

/** The heading and a choosable item for each state, the one the word reads as now ticked. */
export const ticketWordEntries = (
  reference: string,
  word: string,
  reading: TicketReading,
): DropdownMenuEntry[] => [
  {
    kind: "label",
    id: "ticket-word",
    label: `${reference} is ${word}. In Armada that is`,
  },
  ...reading.states.map(
    (one): DropdownMenuEntry => ({
      kind: "item",
      id: `${PICK}${one.id}`,
      label: one.label,
      selected: reading.ticked === one.id,
    }),
  ),
];

/** The state a pick names, where it is one of the menu's. */
export const ticketStateOf = (chosen: string): string | undefined =>
  chosen.startsWith(PICK) ? chosen.slice(PICK.length) : undefined;

/** One call. Success shows in the lanes by Fleet's event; a refusal is the toast. */
export async function teachTicketWord(
  manifestId: string,
  word: string,
  state: string,
): Promise<void> {
  const done = await window.armada.teachTicketWord({
    manifest_id: manifestId,
    word,
    state,
  });
  if (!done.ok) raiseOutcome(done.outcome);
}

type At = {
  x: number;
  y: number;
  from: HTMLElement;
  reading?: TicketReading;
};

/** What a ticket row spreads onto its own element, and the menu to draw beside it. */
export type TicketWordMenu = {
  on: {
    onContextMenu?: (event: MouseEvent<HTMLElement>) => void;
    onKeyDown?: (event: KeyboardEvent<HTMLElement>) => void;
  };
  menu: ReactNode;
};

/** A row read in no repository has no Workspace to teach, and no menu. The menu draws once the lifecycle is read, so its tick does not arrive late. */
export function useTicketWordMenu(
  manifestId: string | undefined,
  reference: string,
  word: string,
): TicketWordMenu {
  const [at, setAt] = useState<At | undefined>(undefined);
  // An answer to an older open, or to a menu since closed, is not drawn.
  const latest = useRef(0);
  if (manifestId === undefined)
    return {
      on: {},
      menu: null,
    };
  const open = (x: number, y: number, from: HTMLElement) => {
    const mine = (latest.current += 1);
    setAt({
      x,
      y,
      from,
    });
    void readTicketWord(manifestId, word).then((reading) => {
      if (mine !== latest.current) return;
      setAt((was) =>
        was === undefined
          ? was
          : {
              ...was,
              reading,
            },
      );
    });
  };
  const on: TicketWordMenu["on"] = {
    onContextMenu: (event) => {
      event.preventDefault();
      open(event.clientX, event.clientY, event.currentTarget);
    },
    onKeyDown: (event) => {
      if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
      event.preventDefault();
      const box = event.currentTarget.getBoundingClientRect();
      open(box.left + box.width / 2, box.bottom, event.currentTarget);
    },
  };
  const menu =
    at?.reading === undefined ? null : (
      <DropdownMenu
        triggerLabel={reference}
        at={{
          x: at.x,
          y: at.y,
        }}
        open
        entries={ticketWordEntries(reference, word, at.reading)}
        onOpenChange={(opened) => {
          if (opened) return;
          latest.current += 1;
          setAt(undefined);
          at.from.focus();
        }}
        onSelect={(chosen) => {
          const state = ticketStateOf(chosen);
          if (state !== undefined) void teachTicketWord(manifestId, word, state);
        }}
      />
    );
  return {
    on,
    menu,
  };
}
