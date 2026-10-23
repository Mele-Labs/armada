// The Logbook page: what agents know, read on open, and Armada fixes below it. `docs/concepts/logbook.md`.
//
// **Nothing in the entries waits on the owner.** He reads an entry and its evidence, and can edit, lock or forget it.
// Each act goes to Fleet and the entry is drawn as it came back. Armada fixes, the retro items, is the host's (`fixes`).

import { useEffect, useState, type ReactNode } from "react";

import { LogbookView } from "@armada/components";
import type { LogbookAnswer, LogbookEntry, LogbookForgotten, LogbookRead, LogbookState } from "@armada/protocol";

import { groupsOf, orderOf } from "./groups";

export type LogbookProps = {
  onRead: () => Promise<LogbookRead>;
  onEdit: (id: string, text: string) => Promise<LogbookAnswer>;
  /** Lock or unlock an entry; the wire's word for it is still pin. */
  onLock: (id: string, locked: boolean) => Promise<LogbookAnswer>;
  onForget: (id: string) => Promise<LogbookForgotten>;
  onOpenJob?: (jobId: string) => void;
  /** What else an entry changed, drawn under its text. */
  also?: (entry: LogbookEntry) => ReactNode;
  /** Armada fixes, the retro items, drawn below the entries. */
  fixes?: ReactNode;
  /** Changes when something asks for Armada fixes. */
  toFixes?: number;
};

/** What the page holds: the entries once read, or that the read failed. */
type Held = { state: "reading" } | { state: "read"; entries: LogbookEntry[] } | { state: "unread" };

export function Logbook({ onRead, onEdit, onLock, onForget, onOpenJob, also, fixes, toFixes }: LogbookProps) {
  const [held, setHeld] = useState<Held>({ state: "reading" });
  const [shown, setShown] = useState<LogbookState>("standing");
  const [selected, setSelected] = useState<string>();
  const [editing, setEditing] = useState(false);

  useEffect(() => {
    let live = true;
    onRead()
      .then((read) => live && setHeld(read.ok ? { state: "read", entries: read.entries } : { state: "unread" }))
      .catch(() => live && setHeld({ state: "unread" }));
    return () => {
      live = false;
    };
  }, [onRead]);

  const entries = held.state === "read" ? held.entries : [];
  const panels = groupsOf(entries, shown);
  const order = orderOf(panels);
  // The first entry is open until another is picked, and again after the open one is no longer drawn.
  const open = order.some((one) => one.id === selected) ? selected : order[0]?.id;

  const replaced = (answer: LogbookAnswer) => {
    if (!answer.ok || held.state !== "read") return;
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
      {...(held.state === "unread" ? { unread: true } : {})}
      {...(fixes === undefined ? {} : { fixes })}
      {...(toFixes === undefined ? {} : { toFixes })}
      {...(open === undefined ? {} : { selected: open })}
      onSelect={(id) => {
        setSelected(id);
        setEditing(false);
      }}
      editing={editing}
      onEdit={setEditing}
      onSave={(id, text) => void onEdit(id, text).then(replaced)}
      onLock={(id, locked) => void onLock(id, locked).then(replaced)}
      onForget={(id) =>
        void onForget(id).then((answer) => {
          if (!answer.ok || held.state !== "read") return;
          // The next entry down takes its place, or the one above where it was the last.
          const at = order.findIndex((one) => one.id === id);
          setSelected((order[at + 1] ?? order[at - 1])?.id);
          setHeld({ state: "read", entries: held.entries.filter((one) => one.id !== id) });
        })
      }
      {...(onOpenJob === undefined ? {} : { onOpenJob })}
      {...(also === undefined ? {} : { also })}
      now={Date.now()}
    />
  );
}
