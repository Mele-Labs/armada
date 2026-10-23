// The Logbook page: what Armada learned from every Job, read on open. `docs/concepts/logbook.md`.
//
// **Nothing on it waits on the owner.** He reads an entry and its evidence, and can edit, pin or forget it. Each act
// goes to Fleet and the entry is drawn as it came back.

import { useEffect, useState } from "react";

import { LogbookView } from "@armada/components";
import type { LogbookAnswer, LogbookEntry, LogbookForgotten, LogbookRead, LogbookState } from "@armada/protocol";

import { groupsOf, orderOf } from "./groups";

export type LogbookProps = {
  onRead: () => Promise<LogbookRead>;
  onEdit: (id: string, text: string) => Promise<LogbookAnswer>;
  onPin: (id: string, pinned: boolean) => Promise<LogbookAnswer>;
  onForget: (id: string) => Promise<LogbookForgotten>;
  onOpenJob?: (jobId: string) => void;
};

/** What the page holds: the entries once read, or that the read failed. */
type Held = { state: "reading" } | { state: "read"; entries: LogbookEntry[] } | { state: "unread" };

export function Logbook({ onRead, onEdit, onPin, onForget, onOpenJob }: LogbookProps) {
  const [held, setHeld] = useState<Held>({ state: "reading" });
  const [filter, setFilter] = useState<LogbookState>("standing");
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
  const panels = groupsOf(entries, filter);
  const order = orderOf(panels);
  // The first entry is open until another is picked, and again after the open one leaves the filter.
  const open = order.some((one) => one.id === selected) ? selected : order[0]?.id;

  const replaced = (answer: LogbookAnswer) => {
    if (!answer.ok || held.state !== "read") return;
    setHeld({ state: "read", entries: held.entries.map((one) => (one.id === answer.entry.id ? answer.entry : one)) });
  };

  if (held.state === "unread") return <p className="armada-logbook__unread">Fleet serves no Logbook yet.</p>;
  return (
    <LogbookView
      panels={panels}
      filter={filter}
      onFilter={(next) => {
        setFilter(next);
        setEditing(false);
      }}
      {...(open === undefined ? {} : { selected: open })}
      onSelect={(id) => {
        setSelected(id);
        setEditing(false);
      }}
      editing={editing}
      onEdit={setEditing}
      onSave={(id, text) => void onEdit(id, text).then(replaced)}
      onPin={(id, pinned) => void onPin(id, pinned).then(replaced)}
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
      now={Date.now()}
    />
  );
}
