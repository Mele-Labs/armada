// The live Sessions a brief can wait on, as the `@` menu draws them: the mention list's rows (`.armada-mention__row`) with the Session's own glyph,
// set in the sheet's flow so the frame, which clips what hangs out of it, never cuts it. Used under the request while an `@` is being typed
// and under the sentence when its Session is changed.

import { useRef, useState, type ChangeEvent, type KeyboardEvent, type SyntheticEvent } from "react";
import { SquareTerminal } from "lucide-react";

export type Offered = { id: string; title: string };

/** The Sessions whose title holds what was typed after the `@`. */
export const matching = (sessions: readonly Offered[], query: string): Offered[] => sessions.filter((one) => one.title.toLowerCase().includes(query.trim().toLowerCase()));

export function SessionMenu({ id, label, sessions, active, onHover, onChoose }: { id: string; label: string; sessions: readonly Offered[]; active: number; onHover: (at: number) => void; onChoose: (one: Offered) => void }) {
  if (sessions.length === 0) return null;
  return (
    <div className="armada-mention armada-dispatch-panel__menu" id={id} role="listbox" aria-label={label}>
      {sessions.map((one, at) => (
        <div
          key={one.id}
          id={`${id}-${at}`}
          role="option"
          aria-selected={at === active}
          className={at === active ? "armada-mention__row armada-mention__row--active" : "armada-mention__row"}
          onMouseEnter={() => onHover(at)}
          // Not `onClick`: the press must not take the focus off the field the menu belongs to.
          onMouseDown={(event) => {
            event.preventDefault();
            onChoose(one);
          }}
        >
          <SquareTerminal size={12} strokeWidth={2} aria-hidden />
          {one.title}
        </div>
      ))}
    </div>
  );
}

/**
 * The `@` being typed in the request: the menu opens at an `@` that starts a word and narrows as the Session's title is typed, whitespace
 * included while some title still starts with what was typed. Arrows move, Enter or Tab picks, Esc closes it and nothing else.
 */
export function useAtMenu(words: string, onWords: (next: string) => void, sessions: readonly Offered[], id: string) {
  const [mention, setMention] = useState<{ at: number; query: string } | null>(null);
  const [active, setActive] = useState(0);
  const field = useRef<HTMLTextAreaElement | null>(null);

  function evaluate(el: HTMLTextAreaElement): void {
    field.current = el;
    const caret = el.selectionStart ?? el.value.length;
    const head = el.value.slice(0, caret);
    const at = head.lastIndexOf("@");
    const query = head.slice(at + 1);
    const starts = at === 0 || /\s/.test(head[at - 1] ?? "");
    const typing = !/\s/.test(query) || sessions.some((one) => one.title.toLowerCase().startsWith(query.toLowerCase()));
    if (at === -1 || !starts || query.includes("\n") || !typing) return setMention(null);
    setMention((was) => (was?.at === at && was.query === query ? was : { at, query }));
    setActive(0);
  }

  const offered = mention === null ? [] : matching(sessions, mention.query);
  const open = mention !== null && offered.length > 0;

  function choose(one: Offered): void {
    const el = field.current;
    if (mention === null || el === null) return;
    const caret = mention.at + 1 + mention.query.length;
    const tag = `@${one.title} `;
    onWords(`${words.slice(0, mention.at)}${tag}${words.slice(caret)}`);
    setMention(null);
    const landed = mention.at + tag.length;
    // The value has not reached the field on this tick, and a range set now would be against the text it replaces.
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(landed, landed);
    });
  }

  /** True where the key was the menu's. */
  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>): boolean {
    field.current = event.currentTarget;
    if (!open) return false;
    if (event.key === "Escape") {
      event.preventDefault();
      setMention(null);
      return true;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setActive((n) => (n + (event.key === "ArrowDown" ? 1 : -1) + offered.length) % offered.length);
      return true;
    }
    if ((event.key === "Enter" || event.key === "Tab") && !event.shiftKey && !event.altKey && !event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      const one = offered[active] ?? offered[0];
      if (one !== undefined) choose(one);
      return true;
    }
    return false;
  }

  return {
    open,
    offered,
    active,
    setActive,
    choose,
    evaluate,
    close: () => setMention(null),
    listId: id,
    onKeyDown,
    onChange: (event: ChangeEvent<HTMLTextAreaElement>) => evaluate(event.target),
    onSelect: (event: SyntheticEvent<HTMLTextAreaElement>) => evaluate(event.currentTarget),
    onBlur: () => setMention(null),
    activeId: open ? `${id}-${active}` : undefined,
  };
}
