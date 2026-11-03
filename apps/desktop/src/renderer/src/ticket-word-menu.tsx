// A ticket's menu of the Ticket lifecycle's states, wherever a ticket row is drawn: choosing one teaches Armada that the tracker's status
// reads as that state (`teachTicketWord`), in the Workspace's Ticket lifecycle and every live Job's copy, so the Jobs' lanes follow by
// `job_lifecycles.changed`. The running Ticket lifecycle is read once per repository for every row drawn from it, again as a menu opens
// and after a teach: a row's mark is the state its status reads as, and the menu ticks it. A refusal is the window's toast, in Fleet's words.

import {
  useState,
  useSyncExternalStore,
  type KeyboardEvent,
  type MouseEvent,
  type ReactNode,
} from "react";
import { CircleCheck, CircleDashed, CircleDot, Eye, type LucideIcon } from "lucide-react";
import { DropdownMenu } from "@armada/components";
import type { DropdownMenuEntry } from "@armada/components";
import type { AppliedLifecycle, LifecycleStateDefinition } from "@armada/protocol";

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

/** The registry's glyph for each Ticket state (`packages/icons`: circle-dashed, circle-dot, eye, circle-check). */
const GLYPH: Readonly<Record<string, LucideIcon>> = {
  todo: CircleDashed,
  in_progress: CircleDot,
  in_review: Eye,
  closed: CircleCheck,
};

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

/** The running Ticket lifecycle's states for a repository. Empty where it has none, or the read was refused. */
type TicketStates = readonly LifecycleStateDefinition[];

/** The Ticket lifecycle that runs for the repository: of those not replaced, the one kept nearest it. */
function runningTicket(applied: readonly AppliedLifecycle[]): AppliedLifecycle | undefined {
  return applied
    .filter((one) => one.lifecycle.id === "ticket" && one.kept.overridden_by === undefined)
    .sort((a, b) => (REACH[b.kept.rank.reach] ?? 0) - (REACH[a.kept.rank.reach] ?? 0))[0];
}

// ---- One reading per repository, shared by its rows ------------------------------------------------------------------------

const held = new Map<string, TicketStates>();
const asked = new Map<string, number>();
const hearing = new Map<string, Set<() => void>>();

function told(manifestId: string): void {
  for (const one of hearing.get(manifestId) ?? []) one();
}

/**
 * Reads the repository's Ticket lifecycle again. An answer to an older read is dropped; a refusal, or a window with no api to
 * ask, reads as no states, so drawing a row never throws.
 */
function readAgain(manifestId: string): void {
  const mine = (asked.get(manifestId) ?? 0) + 1;
  asked.set(manifestId, mine);
  void Promise.resolve()
    .then(() => window.armada.listLifecycles(manifestId))
    .then(
      (read) => (read.ok ? (runningTicket(read.value.lifecycles)?.lifecycle.states ?? []) : []),
      () => [],
    )
    .then((states) => {
      if (asked.get(manifestId) !== mine) return;
      held.set(manifestId, states);
      told(manifestId);
    });
}

/** The first row drawn from a repository reads it; when the last goes, the reading goes with it, so a window drawn again reads afresh. */
function follow(manifestId: string, onChange: () => void): () => void {
  const all = hearing.get(manifestId) ?? new Set();
  hearing.set(manifestId, all);
  all.add(onChange);
  if (all.size === 1) readAgain(manifestId);
  return () => {
    all.delete(onChange);
    if (all.size > 0) return;
    hearing.delete(manifestId);
    held.delete(manifestId);
    asked.delete(manifestId);
  };
}

const NOTHING = () => () => undefined;
// One subscribe per repository, so a row drawn again does not leave and follow afresh.
const following = new Map<string, (onChange: () => void) => () => void>();
const followerOf = (manifestId: string) => {
  const known = following.get(manifestId);
  if (known !== undefined) return known;
  const made = (onChange: () => void) => follow(manifestId, onChange);
  following.set(manifestId, made);
  return made;
};

/** The repository's Ticket states, `undefined` until read. */
export function useTicketStates(manifestId: string | undefined): TicketStates | undefined {
  return useSyncExternalStore(manifestId === undefined ? NOTHING : followerOf(manifestId), () =>
    manifestId === undefined ? undefined : held.get(manifestId),
  );
}

/** The state whose tracker words hold `word` exactly. */
const readAs = (
  states: TicketStates | undefined,
  word: string,
): LifecycleStateDefinition | undefined =>
  states?.find((one) => one.read.source === "tracker" && one.read.words.includes(word));

// ---- The mark ----------------------------------------------------------------------------------------------------------------

/** The glyph and words of the Ticket state a status reads as. `undefined` where it reads as none the registry draws: the row keeps its Open or Closed. */
export function ticketMarkOf(
  states: TicketStates | undefined,
  word: string,
):
  | {
      Glyph: LucideIcon;
      said: string;
    }
  | undefined {
  const state = readAs(states, word);
  const Glyph = state === undefined ? undefined : GLYPH[state.id];
  return state === undefined || Glyph === undefined
    ? undefined
    : {
        Glyph,
        said: `Ticket state: ${state.label}`,
      };
}

// ---- The menu ----------------------------------------------------------------------------------------------------------------

/** The heading and a choosable item for each state, the one the word reads as now ticked. Before the read lands, none is. */
export const ticketWordEntries = (
  reference: string,
  word: string,
  states: TicketStates | undefined,
): DropdownMenuEntry[] => {
  const ticked = readAs(states, word)?.id;
  return [
    {
      kind: "label",
      id: "ticket-word",
      label: `${reference} is ${word}. In Armada that is`,
    },
    ...TICKET_STATES.map(
      ({ id, label }): DropdownMenuEntry => ({
        kind: "item",
        id: `${PICK}${id}`,
        label: states?.find((one) => one.id === id)?.label ?? label,
        selected: ticked === id,
      }),
    ),
  ];
};

/** The state a pick names, where it is one of the menu's. */
export const ticketStateOf = (chosen: string): string | undefined =>
  chosen.startsWith(PICK) ? chosen.slice(PICK.length) : undefined;

/** A menu opening reads the lifecycle again, so a tick reflects a word taught elsewhere. */
export const readTicketStates = (manifestId: string): void => readAgain(manifestId);

/** One call. The rows read the lifecycle again once it is taken; a refusal is the toast. */
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
  if (!done.ok) return raiseOutcome(done.outcome);
  readAgain(manifestId);
}

type At = {
  x: number;
  y: number;
  from: HTMLElement;
};

/** What a ticket row spreads onto its own element, and the menu to draw beside it. */
export type TicketWordMenu = {
  on: {
    onContextMenu?: (event: MouseEvent<HTMLElement>) => void;
    onKeyDown?: (event: KeyboardEvent<HTMLElement>) => void;
  };
  menu: ReactNode;
};

/** A row read in no repository has no Workspace to teach, and no menu. `states` is the row's own reading, which the menu ticks from. */
export function useTicketWordMenu(
  manifestId: string | undefined,
  reference: string,
  word: string,
  states: TicketStates | undefined,
): TicketWordMenu {
  const [at, setAt] = useState<At | undefined>(undefined);
  if (manifestId === undefined)
    return {
      on: {},
      menu: null,
    };
  const open = (x: number, y: number, from: HTMLElement) => {
    setAt({
      x,
      y,
      from,
    });
    readAgain(manifestId);
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
    at === undefined ? null : (
      <DropdownMenu
        triggerLabel={reference}
        at={{
          x: at.x,
          y: at.y,
        }}
        open
        entries={ticketWordEntries(reference, word, states)}
        onOpenChange={(opened) => {
          if (opened) return;
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
