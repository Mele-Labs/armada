// A Job opened from the Operator: the Job's whole page laid over the Operator, which stays where it was behind it. Back or Esc closes
// it onto the same tab and scroll; Open as page leaves for the Job's own page.

import { useEffect, useRef, useSyncExternalStore } from "react";
import { ArrowLeft, ExternalLink, Square } from "lucide-react";
import { Button, Kbd } from "@armada/components";
import { askToCloseOperator } from "@armada/shell";
import { popupIsOpen } from "@armada/screens/src/list-keyboard";

import { select } from "../cockpit/deck/store";
import { OPEN_SESSION_ATTRIBUTE, askToOpenSession } from "../open-session";
import { SessionView } from "../sessions";
import { useSessions, useSessionsDraft } from "../sessions-draft";

let over: string | null = null;
// Where focus was when the Job opened, so closing it puts him back on the card he pressed.
let opener: HTMLElement | null = null;
const listeners = new Set<() => void>();
const changed = () => {
  for (const one of listeners) one();
};
const subscribe = (listen: () => void) => {
  listeners.add(listen);
  return () => listeners.delete(listen);
};

export function openJobOver(id: string): void {
  if (over === null && document.activeElement instanceof HTMLElement)
    opener = document.activeElement;
  over = id;
  changed();
}

export function closeJobOver(): void {
  if (over === null) return;
  over = null;
  changed();
  if (opener?.isConnected === true) opener.focus();
  opener = null;
}

/** Leaves the Operator for the Job's own page. */
export function openJobPage(id: string): void {
  closeJobOver();
  select({
    kind: "session",
    key: `session:${id}`,
  });
  askToCloseOperator();
}

const useJobOver = (): string | null => useSyncExternalStore(subscribe, () => over);

const PANED = {
  side: null,
} as const;

const HELD = {
  held: {
    state: "none",
  },
  onWant: () => {},
} as const;

/** The open Job over the Operator, or nothing. */
export function JobOver() {
  const id = useJobOver();
  const session = useSessions().find((one) => one.id === id);
  const draft = useSessionsDraft();
  const back = useRef<HTMLButtonElement>(null);
  const layer = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (id !== null) back.current?.focus();
  }, [
    id,
  ]);

  // Esc goes back, unless something inside the Job holds it: a menu, a dialog or a layer over its thread.
  useEffect(() => {
    if (id === null) return;
    const listen = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || popupIsOpen()) return;
      if (layer.current?.querySelector('[data-keeps-escape], [role="dialog"]') != null) return;
      event.preventDefault();
      closeJobOver();
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, [
    id,
  ]);

  if (id === null || session === undefined || draft === undefined) return null;
  return (
    <div className="armada-operator__over" data-keeps-escape="">
      <div className="armada-operator__over-scrim" onClick={closeJobOver} aria-hidden />
      <div
        ref={layer}
        className="armada-operator__over-job"
        role="region"
        aria-label={`Job over the Operator: ${session.title ?? session.id}`}
      >
        <header className="armada-operator__over-bar">
          <Button ref={back} size="sm" variant="ghost" onClick={closeJobOver}>
            <ArrowLeft size={14} strokeWidth={2} aria-hidden />
            Back to Operator
            <Kbd>Esc</Kbd>
          </Button>
          <div className="armada-operator__over-title">
            <span className="armada-operator__over-name">{session.title ?? session.id}</span>
          </div>
          <Button size="sm" variant="secondary" onClick={() => openJobPage(session.id)}>
            <ExternalLink size={14} strokeWidth={2} aria-hidden />
            Open as page
          </Button>
        </header>
        <section
          className="deck-pane armada-operator__over-page"
          data-frame=""
          data-bare={session.work === undefined ? undefined : ""}
          aria-label={`Job: ${session.title ?? session.id}`}
          {...{
            [OPEN_SESSION_ATTRIBUTE]: session.id,
          }}
        >
          <SessionView
            session={session}
            goes={{
              onGoTo: () => {},
              onOpenLink: () => {},
              onOpenCheck: () => {},
              onOpenLoadout: () => {},
            }}
            onOpen={askToOpenSession}
            held={HELD}
            deck={{
              ledger: null,
              back: null,
              // Hosted with no side column, a Job draws its own area in panes: its tabs and layout as he set them on its page.
              hosts: PANED,
              acts:
                draft.stop === undefined
                  ? []
                  : [
                      {
                        id: "stop",
                        label: "Stop",
                        icon: Square,
                        run: () => draft.stop?.(session.id),
                      },
                    ],
            }}
          />
        </section>
      </div>
    </div>
  );
}
