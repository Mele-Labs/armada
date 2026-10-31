// The pieces every phone screen is made of: Bridge's panel header, a list row and its heading, and a
// refusal's sentence. The rows and headings carry the Cockpit's class names, so `depth.css` draws them.

import type { ReactNode } from "react";
import { Link as Linked, PowerOff, X } from "lucide-react";
import { Button, Readout, SessionMark, Stamp, Tooltip, type SessionState } from "@armada/components";
import { tokensSaid } from "@armada/components/src/compositions/SessionFrame/ContextFigure";

/** A header's icon press, named by its tooltip as Bridge's are. */
export function IconButton({ label, onClick, children }: { label: string; onClick: () => void; children: ReactNode }) {
  return (
    <Tooltip label={label} asChild>
      <Button variant="ghost" size="sm" iconOnly aria-label={label} onClick={onClick}>
        {children}
      </Button>
    </Tooltip>
  );
}

/** Back, as Bridge's layers draw it: the close glyph at the header's trailing end. */
export const Back = ({ label, onClick }: { label: string; onClick: () => void }) => (
  <IconButton label={label} onClick={onClick}><X size={16} aria-hidden="true" /></IconButton>
);

export function Band({ title, onBack, back, end }: { title: string; onBack?: () => void; back?: string; end?: ReactNode }) {
  return (
    <header className="deck-band pk-band">
      <h1 className="pk-band__title">{title}</h1>
      {end}
      {onBack === undefined ? null : <Back label={back ?? "Back"} onClick={onBack} />}
    </header>
  );
}

/** A figure with a tooltip naming it, in a fixed column unless `loose`; an empty one keeps the column. */
export function Figure({ name, loose = false, children }: { name: string; loose?: boolean; children?: string | undefined }) {
  if (children === undefined) return <Readout fixed={!loose} />;
  return (
    <Tooltip label={name}>
      <Readout fixed={!loose} aria-label={`${name} ${children}`}>{children}</Readout>
    </Tooltip>
  );
}

/**
 * One Job in the Cockpit's list: its title and state mark, what it says and its age. `from` is the title of the Job
 * that started it, a link mark beside the state; `context` the tokens in its window, in a column of its own.
 */
export function Row({ state, said, title, says, mac, at, from, context, onOpen, onEnd }: { state: SessionState; said: string; title: string; says: string; mac?: string; at: string; from?: string; context?: number; onOpen: () => void; onEnd?: () => void }) {
  const needs = state === "waiting";
  return (
    <li className="deck-list__item">
      <button type="button" className="deck-row" data-needs={needs || undefined} data-tone={needs ? "ask" : "calm"} aria-label={title} onClick={onOpen}>
        <span className="deck-row__title">{title}</span>
        <span className="deck-row__link">
          {from === undefined ? null : (
            <Tooltip label={from}>
              <span role="img" aria-label={from}><Linked size={12} strokeWidth={2} aria-hidden="true" /></span>
            </Tooltip>
          )}
        </span>
        <span className="deck-row__marks"><SessionMark state={state} said={said} /></span>
        <span className="deck-row__says">
          {says}
          {mac === undefined ? null : <span className="pk-row__mac">{mac}</span>}
        </span>
        <span className="deck-row__context"><Figure name="Context">{context === undefined ? undefined : tokensSaid(context)}</Figure></span>
        <span className="deck-row__said"><Stamp still first="age" at={at} /></span>
      </button>
      {onEnd === undefined ? null : (
        <IconButton label="End job" onClick={onEnd}><PowerOff size={16} aria-hidden="true" /></IconButton>
      )}
    </li>
  );
}

export const Trouble = ({ says }: { says: string }) => (says === "" ? null : <p className="pk-fact" role="status">{says}</p>);

/** A heading as the Cockpit's list draws one, over its rows. */
export function Section({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="pk-section" aria-label={label}>
      <h2 className="deck-list__repository">{label}</h2>
      <ul className="deck-list__rows">{children}</ul>
    </section>
  );
}
