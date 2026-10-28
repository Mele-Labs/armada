// The Logbook page: what agents know, read on open, and the retro items on their own tab. `docs/concepts/logbook.md`.
//
// **Nothing in the entries waits on the owner.** He reads an entry and its evidence, and can edit, lock or forget it.
// Each act goes to Fleet and the entry is drawn as it came back. The retro items are the host's (`fixes`).

import { useCallback, useEffect, useState, type ReactNode } from "react";

import { LogbookView, type LogbookShown } from "@armada/components";
import type { LogbookAnswer, LogbookEntry, LogbookForgotten, LogbookRead, LogbookTidied, LogbookTidy, LogbookTidyLast, Outcome } from "@armada/protocol";

import { refusalWords } from "../refusal-words";
import { groupsOf, orderOf } from "./groups";
import { forgetTidyWatch, tidyWatch, watchTidy } from "./tidy-watch";

export type LogbookProps = {
  onRead: () => Promise<LogbookRead>;
  onEdit: (id: string, text: string) => Promise<LogbookAnswer>;
  /** Lock or unlock an entry; the wire's word for it is still pin. */
  onLock: (id: string, locked: boolean) => Promise<LogbookAnswer>;
  onForget: (id: string) => Promise<LogbookForgotten>;
  /** Folds near-duplicates together across every repository. */
  onTidy: () => Promise<LogbookTidied>;
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

/** Fleet's refusal of the last act, and the entry it was on. */
type Refused = { id: string; title: string; said: string };

export function Logbook({ onRead, onEdit, onLock, onForget, onTidy, onOpenJob, also, fixes, toFixes }: LogbookProps) {
  const [held, setHeld] = useState<Held>({ state: "reading" });
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

  const startTidy = () => {
    if (tidy.running) return;
    watchTidy({ ...(tidy.last === undefined ? {} : { last: tidy.last.at }) });
    setTidy({ ...tidy, running: true });
    setTidied(undefined);
    onTidy()
      .then(async (answer) => {
        if (!answer.ok) {
          forgetTidyWatch();
          setTidy({ ...tidy, running: false });
          return setTidied(refusalWords(answer.outcome));
        }
        // Fleet answers at once: the pass may already be over, and the read says so.
        if (!answer.running) await read(() => true);
      })
      .catch(() => {
        forgetTidyWatch();
        setTidy({ ...tidy, running: false });
        setTidied(refusalWords({ ok: false, why: "not_connected" }));
      });
  };

  const entries = held.state === "read" ? held.entries : [];
  const panels = shown === "retros" ? [] : groupsOf(entries, shown);
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
      onTidy={startTidy}
      tidying={tidy.running}
      {...(tidy.running ? { tidied: "Tidying" } : tidied === undefined ? {} : { tidied })}
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
