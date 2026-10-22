// The pieces every phone screen is made of: the band at the top, a list row, and a refusal's sentence.

import type { ReactNode } from "react";
import { ChevronLeft } from "lucide-react";
import { SessionMark, type SessionState } from "@armada/components";

export function Band({ title, onBack, back, end }: { title: string; onBack?: () => void; back?: string; end?: ReactNode }) {
  return (
    <header className="pk-band">
      {onBack !== undefined && (
        <button className="pk-icon" aria-label={back ?? "Back"} onClick={onBack}><ChevronLeft /></button>
      )}
      <h1 className="pk-band__title">{title}</h1>
      {end}
    </header>
  );
}

/** One Job in a list: its state as a registry mark with a tooltip, its title, where it is, and an age. */
export function Row({ state, said, title, where, mac, age, onOpen }: { state: SessionState; said: string; title: string; where: string; mac?: string; age: string; onOpen?: () => void }) {
  const body = (
    <>
      <span className="pk-row__mark"><SessionMark state={state} said={said} /></span>
      <span className="pk-row__title">{title}</span>
      <span className="pk-row__meta">
        {where}
        {mac === undefined ? null : <span className="pk-row__mac">{mac}</span>}
      </span>
      <span className="pk-row__age">{age}</span>
    </>
  );
  return onOpen === undefined ? <li className="pk-row" data-still>{body}</li> : <li><button className="pk-row" onClick={onOpen}>{body}</button></li>;
}

export const Trouble = ({ says }: { says: string }) => (says === "" ? null : <p className="pk-fact" role="status">{says}</p>);

export function Section({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="pk-section" aria-label={label}>
      <h2 className="pk-section__head">{label}</h2>
      <ul className="pk-list">{children}</ul>
    </section>
  );
}
