// The Logbook page: what agents know, read on open, and the retro items on their own tab. `docs/concepts/logbook.md`.
//
// **Nothing in the entries waits on the owner.** He reads an entry and its evidence, and can edit, lock or forget it.
// Each act goes to Fleet and the entry is drawn as it came back. The retro items are the host's (`fixes`).

import { useCallback, useEffect, useState, type ReactNode } from "react";

import { LogbookView, type LogbookShown, type OrganizeAction, type OrganizeProposal, type OrganizeRow } from "@armada/components";
import type { LogbookAnswer, LogbookEntry, LogbookForgotten, LogbookOrganizeApplied, LogbookOrganized, LogbookRead, LogbookTidy, LogbookTidyLast, LogbookTopic, OrganizeProposal as Proposed, Outcome } from "@armada/protocol";

import { refusalWords } from "../refusal-words";
import { groupsOf, orderOf, topicsOf } from "./groups";
import { forgetTidyWatch, tidyWatch, watchTidy } from "./tidy-watch";

export type LogbookProps = {
  onRead: () => Promise<LogbookRead>;
  onEdit: (id: string, text: string) => Promise<LogbookAnswer>;
  /** Lock or unlock an entry; the wire's word for it is still pin. */
  onLock: (id: string, locked: boolean) => Promise<LogbookAnswer>;
  onForget: (id: string) => Promise<LogbookForgotten>;
  /** Organize: what an action proposes. Nothing changes until the proposals are applied. */
  onOrganize: (action: OrganizeAction) => Promise<LogbookOrganized>;
  /** Carries out the proposals left in a card list, as Organize answered them. */
  onApplyOrganize: (action: OrganizeAction, proposals: Proposed[]) => Promise<LogbookOrganizeApplied>;
  onOpenJob?: (jobId: string) => void;
  /** What else an entry changed, drawn under its text. */
  also?: (entry: LogbookEntry) => ReactNode;
  /** The retro items, drawn under the Retros tab while it is open. */
  fixes?: ReactNode;
  /** Changes when something asks for the retro items: the Retros tab opens. */
  toFixes?: number;
};

/** What the page holds: the entries once read, or that the read failed. */
type Held = { state: "reading" } | { state: "read"; entries: LogbookEntry[] } | { state: "unread" };

/** How often the page asks again while a Tidy pass runs. */
const TIDY_POLL_MS = 3_000;

const entriesOf = (count: number) => `${count} ${count === 1 ? "entry" : "entries"}`;

/** A finished pass as a plain fact. */
function tidiedWords(last: LogbookTidyLast): string {
  if (last.failed) return last.merged === 0 ? "Tidy failed before merging anything" : `Tidy failed after merging ${entriesOf(last.merged)}`;
  return last.merged === 0 ? "Nothing to merge" : `Merged ${entriesOf(last.merged)}`;
}

/** What an apply came to, as a plain fact. Entries Fleet skipped because they changed since are not counted. */
function appliedWords(action: OrganizeAction, answer: { applied: number; reported: Proposed[] }): string {
  if (action === "bugs") return answer.reported.length === 0 ? "Nothing reported" : "Reported to the Armada bug list";
  if (answer.applied === 0) return "Nothing changed";
  return `${action === "merge" ? "Merged" : "Archived"} ${entriesOf(answer.applied)}`;
}

/** Fleet's proposals as the cards draw them: each entry id mapped to the entry in the loaded Logbook, and left out where it is gone. */
function proposalOf(action: OrganizeAction, proposals: readonly Proposed[], entries: readonly LogbookEntry[]): OrganizeProposal {
  const folded = (id: string) => {
    const one = entries.find((entry) => entry.id === id);
    return one === undefined ? [] : [{ text: one.title ?? one.text, kind: one.kind, confirmed: one.last_confirmed }];
  };
  const rows = proposals.flatMap((one): OrganizeRow[] => {
    const kept = entries.find((entry) => entry.id === one.entry_ids[0]);
    if (kept === undefined) return [];
    return [{
      id: one.id,
      text: action === "merge" ? one.text : (kept.title ?? one.text),
      kind: kept.kind,
      confirmed: kept.last_confirmed,
      note: one.note,
      ...(action === "merge" ? { absorbed: one.entry_ids.slice(1).flatMap(folded) } : {}),
    }];
  });
  return { action, rows };
}

/** Fleet's refusal of the last act, and the entry it was on. */
type Refused = { id: string; title: string; said: string };

export function Logbook({ onRead, onEdit, onLock, onForget, onOrganize, onApplyOrganize, onOpenJob, also, fixes, toFixes }: LogbookProps) {
  const [held, setHeld] = useState<Held>({ state: "reading" });
  const [chosen, setChosen] = useState<LogbookTopic>();
  const [shown, setShown] = useState<LogbookShown>(toFixes === undefined || toFixes === 0 ? "standing" : "retros");
  // Something asked for the retro items after the page opened: their tab opens.
  useEffect(() => {
    if (toFixes !== undefined && toFixes !== 0) setShown("retros");
  }, [toFixes]);
  const [selected, setSelected] = useState<string>();
  const [editing, setEditing] = useState(false);
  const [refused, setRefused] = useState<Refused>();
  const [tidy, setTidy] = useState<LogbookTidy>({ running: false });
  const [tidied, setTidied] = useState<string>();
  const [organizing, setOrganizing] = useState<OrganizeAction>();
  const [found, setFound] = useState<{ action: OrganizeAction; proposals: Proposed[] }>();
  const read = useCallback(
    (live: () => boolean) =>
      onRead()
        .then((answer) => {
          if (!live()) return;
          if (!answer.ok) return setHeld({ state: "unread" });
          setHeld({ state: "read", entries: answer.entries });
          setTidy(answer.tidy);
          const watching = tidyWatch();
          if (answer.tidy.running) {
            if (watching === undefined) watchTidy({ ...(answer.tidy.last === undefined ? {} : { last: answer.tidy.last.at }) });
          } else if (watching !== undefined) {
            const began = watching.last;
            forgetTidyWatch();
            if (answer.tidy.last !== undefined && answer.tidy.last.at !== began) setTidied(tidiedWords(answer.tidy.last));
          }
        })
        .catch(() => live() && setHeld({ state: "unread" })),
    [onRead],
  );

  useEffect(() => {
    let live = true;
    void read(() => live);
    return () => {
      live = false;
    };
  }, [read]);

  useEffect(() => {
    if (!tidy.running) return;
    let live = true;
    const poll = setInterval(() => void read(() => live), TIDY_POLL_MS);
    return () => {
      live = false;
      clearInterval(poll);
    };
  }, [tidy.running, read]);

  const LOOKING: Record<OrganizeAction, string> = { merge: "Finding duplicates", stale: "Finding stale entries", bugs: "Finding Armada bugs" };
  const organize = (action: OrganizeAction) => {
    if (organizing !== undefined) return;
    setOrganizing(action);
    setFound(undefined);
    setTidied(undefined);
    onOrganize(action)
      .then((answer) => {
        setOrganizing(undefined);
        if (!answer.ok) return setTidied(refusalWords(answer.outcome));
        if (answer.proposals.length > 0) setFound({ action, proposals: answer.proposals });
        // Some repository failed; the others' proposals, if any, are drawn above.
        setTidied(answer.failed ? "Failed" : answer.proposals.length === 0 ? "Nothing found" : undefined);
      })
      .catch(() => {
        setOrganizing(undefined);
        setTidied(refusalWords({ ok: false, why: "not_connected" }));
      });
  };
  const applyProposals = (only?: string) => {
    if (found === undefined) return;
    const { action } = found;
    const sent = only === undefined ? found.proposals : found.proposals.filter((one) => one.id === only);
    setFound(only === undefined ? undefined : { action, proposals: found.proposals.filter((one) => one.id !== only) });
    void onApplyOrganize(action, sent)
      .then(async (answer) => {
        if (!answer.ok) return setTidied(refusalWords(answer.outcome));
        await read(() => true);
        setTidied(appliedWords(action, answer));
      })
      .catch(() => setTidied(refusalWords({ ok: false, why: "not_connected" })));
  };

  const entries = held.state === "read" ? held.entries : [];
  const proposal = found === undefined ? undefined : proposalOf(found.action, found.proposals, entries);
  const topics = shown === "retros" ? [] : topicsOf(entries, shown);
  // A topic with no entries under this tab is no longer a filter: the page shows All.
  const topic = chosen !== undefined && topics.includes(chosen) ? chosen : undefined;
  const panels = shown === "retros" ? [] : groupsOf(entries, shown, topic);
  const order = orderOf(panels);
  // The first entry is open until another is picked, and again after the open one is no longer drawn.
  const open = order.some((one) => one.id === selected) ? selected : order[0]?.id;

  /** Draws Fleet's refusal on the entry it was about; the next act or another entry clears it. */
  const refuse = (id: string, title: string, outcome: Outcome) => setRefused({ id, title, said: refusalWords(outcome) });

  const replaced = (id: string, title: string) => (answer: LogbookAnswer) => {
    if (!answer.ok) return refuse(id, title, answer.outcome);
    if (held.state !== "read") return;
    setHeld({ state: "read", entries: held.entries.map((one) => (one.id === answer.entry.id ? answer.entry : one)) });
  };

  return (
    <LogbookView
      panels={panels}
      shown={shown}
      onShown={(next) => {
        setShown(next);
        setEditing(false);
      }}
      topics={topics}
      {...(topic === undefined ? {} : { topic })}
      onTopic={setChosen}
      onOrganize={organize}
      organizing={organizing !== undefined}
      {...(proposal === undefined ? {} : { proposal })}
      onApplyAll={() => applyProposals()}
      onApplyRow={applyProposals}
      onDismissRow={(id) => setFound((was) => (was === undefined ? was : { ...was, proposals: was.proposals.filter((one) => one.id !== id) }))}
      onCloseProposal={() => setFound(undefined)}
      {...(organizing !== undefined ? { tidied: LOOKING[organizing] } : tidy.running ? { tidied: "Tidying" } : tidied === undefined ? {} : { tidied })}
      {...(held.state === "unread" ? { unread: true } : {})}
      {...(fixes === undefined ? {} : { fixes })}
      {...(toFixes === undefined ? {} : { toFixes })}
      {...(open === undefined ? {} : { selected: open })}
      {...(refused === undefined || refused.id !== open ? {} : { refusal: { title: refused.title, said: refused.said } })}
      onSelect={(id) => {
        setSelected(id);
        setRefused(undefined);
        setEditing(false);
      }}
      editing={editing}
      onEdit={setEditing}
      onSave={(id, text) => {
        setRefused(undefined);
        void onEdit(id, text).then(replaced(id, "Not saved"));
      }}
      onLock={(id, locked) => {
        setRefused(undefined);
        void onLock(id, locked).then(replaced(id, "Not changed"));
      }}
      onForget={(id) => {
        setRefused(undefined);
        void onForget(id).then((answer) => {
          if (!answer.ok) return refuse(id, "Not forgotten", answer.outcome);
          if (held.state !== "read") return;
          // The next entry down takes its place, or the one above where it was the last.
          const at = order.findIndex((one) => one.id === id);
          setSelected((order[at + 1] ?? order[at - 1])?.id);
          setHeld({ state: "read", entries: held.entries.filter((one) => one.id !== id) });
        });
      }}
      {...(onOpenJob === undefined ? {} : { onOpenJob })}
      {...(also === undefined ? {} : { also })}
      now={Date.now()}
    />
  );
}
