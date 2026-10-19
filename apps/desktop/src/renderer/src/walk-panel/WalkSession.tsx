// A Job beside the page it showed: `SessionView`, the Cockpit's own, over the Sessions the window holds. The
// panel navigates nowhere and opens no window (`docs/practices/capture-window.md`, *The panel beside the page*),
// so what the view would open goes to Bridge's window by `goInBridge`; its plain acts are the Cockpit's own calls.
// The notes held on the Job are the last entry of its thread, as main tells them (*Pending notes*).

import { useEffect, useMemo, useState } from "react";
import type { Outcome } from "@armada/protocol";
import { said } from "@armada/screens/src/copy";
import type { PanelHeldNote } from "@armada/studios/api";

import { NOTHING_YET } from "../../../shared/bridge";
import type { BridgeState } from "../../../shared/bridge";
import { useViewing } from "../cockpit/viewed";
import { SessionView } from "../sessions";
import type { HeldReads, LedgerGoes } from "../sessions";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { stateOf } from "../state-feed";
import { HeldNotes, type HeldAnswer, type HeldNotesActs } from "./HeldNotes";

const NOTHING = () => {};

const answered = (outcome: Outcome): HeldAnswer => (outcome.ok ? { ok: true } : { ok: false, said: said(outcome) });

/** The notes main holds on this Job, asked for again as the Job changes. The calls name no Job: main reads it off this panel. */
function useHeld(sessionId: string): { notes: PanelHeldNote[]; acts: HeldNotesActs } {
  const [notes, setNotes] = useState<PanelHeldNote[]>([]);
  const { held, editNote, removeNote, sendNotes } = window.armada.walkPanel;
  useEffect(() => {
    setNotes([]);
    return held(setNotes);
  }, [held, sessionId]);
  const acts = useMemo<HeldNotesActs>(
    () => ({
      onEdit: async (id, words) => answered(await editNote(id, words)),
      onRemove: async (id) => answered(await removeNote(id)),
      onSend: async () => answered(await sendNotes()),
    }),
    [editNote, removeNote, sendNotes],
  );
  return { notes, acts };
}

export function WalkSession({ sessionId }: { sessionId: string }) {
  const session = useSessions().find((one) => one.id === sessionId);
  const served = useSessionsDraft() !== undefined;
  const [state, setState] = useState<BridgeState>(NOTHING_YET);
  useEffect(() => stateOf(window.armada).subscribe(setState), []);
  useViewing(session === undefined ? undefined : sessionId);
  const notes = useHeld(sessionId);
  const { goInBridge, openInBridge } = window.armada.walkPanel;
  const goes = useMemo<LedgerGoes>(
    () => ({
      onGoTo: (id) => void goInBridge({ kind: "surface", id }),
      onOpenLink: (address) => void goInBridge({ kind: "link", address }),
      onOpenCheck: (_session, run) => void goInBridge({ kind: "check", run }),
      onOpenLoadout: () => void goInBridge({ kind: "loadout" }),
    }),
    [goInBridge],
  );
  // Bridge's window wants the pool while it shows it; a want from here would drop that one when this closes.
  const held = useMemo<HeldReads>(() => ({ held: state.held, onWant: NOTHING }), [state.held]);
  if (session === undefined) return <p className="text-fg-muted">{served ? "Fleet holds no Job by this id now." : "Reading the Job."}</p>;
  return (
    <section className="deck-pane armada-walk-job" data-frame="" aria-label={`Job: ${session.title ?? session.id}`}>
      <SessionView
        key={session.id}
        session={session}
        goes={goes}
        onOpen={(id) => void (id === sessionId ? openInBridge() : goInBridge({ kind: "session", id }))}
        held={held}
        repositories={state.holds.repositories ?? []}
        {...(notes.notes.length === 0 ? {} : { trailing: <HeldNotes notes={notes.notes} {...notes.acts} /> })}
      />
    </section>
  );
}
