import { useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { ChevronDown } from "lucide-react";

import { Input } from "../../primitives/Input/Input";

/**
 * A theme field: one choice out of many groups, narrowed by typing.
 *
 * **Searchable because the list is long.** The catalogue is forty themes and a machine may add the
 * rest of a collection, so a radio list stops being a list a person can read. The field shows the
 * chosen theme, and typing narrows the groups under it to the titles that match.
 *
 * **Grouped, and a group with no match is not drawn.** Dark, Light and From mods are how a person
 * thinks of a theme before they think of its name.
 */
export type ThemeChoice = {
  id: string;
  title: string;
  /**
   * The colours the theme is previewed by, ground first, as the theme's own tokens give them. Absent
   * draws a neutral strip of the same size, so a row never shifts when a theme's colours arrive.
   */
  swatch?: readonly string[];
};
export type ThemeGroup = { label: string; choices: readonly ThemeChoice[] };

export type ThemePickerProps = {
  /** Sentence case, no Wh- opener. Names the field and the list it opens. */
  label: string;
  /** The id of the chosen theme. */
  value: string;
  groups: readonly ThemeGroup[];
  onValue: (id: string) => void;
};

const PLACEHOLDER_CELLS = 8;

/** The theme's colours as one compact strip. Decoration: the name beside it says which theme. */
function Swatch({ colours }: { colours: readonly string[] | undefined }) {
  const cells = colours ?? Array.from({ length: PLACEHOLDER_CELLS }, () => null);
  return (
    <span className="armada-themepick__swatch" data-placeholder={colours === undefined || undefined} aria-hidden>
      {cells.map((colour, at) => (
        <span key={at} className="armada-themepick__cell" {...(colour === null ? {} : { style: { background: colour } })} />
      ))}
    </span>
  );
}

export function ThemePicker({ label, value, groups, onValue }: ThemePickerProps) {
  const listId = useId();
  const optionId = (index: number): string => `${listId}-o${index}`;
  const [open, setOpen] = useState(false);
  // What was typed, or null while the field shows the chosen title.
  const [query, setQuery] = useState<string | null>(null);
  const [active, setActive] = useState(0);
  const list = useRef<HTMLDivElement>(null);

  const needle = (query ?? "").trim().toLowerCase();
  const shown = groups
    .map((group) => ({ label: group.label, choices: group.choices.filter((one) => one.title.toLowerCase().includes(needle)) }))
    .filter((group) => group.choices.length > 0);
  const flat = shown.flatMap((group) => group.choices);
  const chosen = groups.flatMap((group) => group.choices).find((one) => one.id === value);

  // The pointer or an arrow key moved the highlight: bring that row into view.
  useEffect(() => {
    if (open) list.current?.querySelector(`[id="${optionId(active)}"]`)?.scrollIntoView({ block: "nearest" });
  }, [open, active]);

  function show(): void {
    setOpen(true);
    setActive(Math.max(0, flat.findIndex((one) => one.id === value)));
  }

  function close(): void {
    setOpen(false);
    setQuery(null);
  }

  function choose(index: number): void {
    const picked = flat[index];
    if (picked === undefined) return;
    onValue(picked.id);
    close();
  }

  function onKeyDown(event: KeyboardEvent<HTMLInputElement>): void {
    if (event.key === "Escape") {
      // Left alone while the list is shut, so the same key still leaves the surface the field sits in.
      if (!open) return;
      event.preventDefault();
      close();
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      if (!open) show();
      else if (flat.length > 0) setActive((n) => (n + 1) % flat.length);
    } else if (event.key === "ArrowUp") {
      if (!open) return;
      event.preventDefault();
      if (flat.length > 0) setActive((n) => (n - 1 + flat.length) % flat.length);
    } else if (event.key === "Enter" && open && flat.length > 0) {
      event.preventDefault();
      choose(active);
    } else if (event.key === "Tab") close();
  }

  let at = -1;
  return (
    <div className="armada-themepick">
      <Input
        label={label}
        value={query ?? chosen?.title ?? ""}
        role="combobox"
        aria-expanded={open && flat.length > 0}
        aria-controls={listId}
        aria-activedescendant={open && flat.length > 0 ? optionId(active) : undefined}
        aria-autocomplete="list"
        autoComplete="off"
        trailing={
          <span className="armada-themepick__trailing">
            {chosen === undefined || query !== null ? null : <Swatch colours={chosen.swatch} />}
            <ChevronDown className="armada-themepick__caret" size={16} strokeWidth={2} aria-hidden />
          </span>
        }
        onChange={(event) => {
          setQuery(event.target.value);
          setOpen(true);
          setActive(0);
        }}
        onFocus={(event) => {
          show();
          event.target.select();
        }}
        // A row press never reaches this: the row prevents its own `mousedown`, so the field keeps
        // focus through a choice.
        onBlur={close}
        onKeyDown={onKeyDown}
      />
      {!open || flat.length === 0 ? null : (
        <div className="armada-themepick__list" id={listId} role="listbox" aria-label={label} ref={list}>
          {shown.map((group) => (
            <div key={group.label} role="group" aria-label={group.label}>
              <div className="armada-themepick__group caps" aria-hidden>
                {group.label}
              </div>
              {group.choices.map((choice) => {
                at += 1;
                const index = at;
                return (
                  <div
                    key={choice.id}
                    id={optionId(index)}
                    role="option"
                    aria-selected={choice.id === value}
                    className={index === active ? "armada-themepick__row armada-themepick__row--active" : "armada-themepick__row"}
                    onMouseEnter={() => setActive(index)}
                    // `onMouseDown`, not `onClick`: a click fires after the field's own blur, which has already closed the list.
                    onMouseDown={(event) => {
                      event.preventDefault();
                      choose(index);
                    }}
                  >
                    <span className="armada-themepick__title">{choice.title}</span>
                    <Swatch colours={choice.swatch} />
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
