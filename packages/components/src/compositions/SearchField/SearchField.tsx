import { useEffect, useRef } from "react";

import { Input } from "../../primitives/Input/Input";
import { Kbd } from "../../primitives/Kbd/Kbd";

/**
 * The search bar every list shares: a field that says "Search", with the key that reaches it drawn inside.
 *
 * **What it holds is the caller's** (`value` is its own state, kept with `useKept` where it should survive
 * leaving the page). Esc clears a non-empty field and goes no further; on an empty one it falls through to the
 * page's own Esc.
 *
 * **`cap` reaches it from a page with nothing focused**: pressed with focus on the body, a visible field takes
 * it. A page that wants the key from further in, a row focused for one, answers it in its own keys, and finds
 * the field by `[data-search-field]`.
 */
export type SearchFieldProps = {
  /** The accessible name: what is searched, as in "Search System". */
  label: string;
  value: string;
  onValue: (value: string) => void;
  /** The key that focuses it, as drawn and as pressed. */
  cap?: string;
};

export function SearchField({ label, value, onValue, cap = "/" }: SearchFieldProps) {
  const field = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const reach = (event: KeyboardEvent) => {
      const input = field.current;
      if (event.key !== cap || event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey || document.activeElement !== document.body) return;
      if (input === null || input.getClientRects().length === 0) return;
      event.preventDefault();
      input.focus();
    };
    document.addEventListener("keydown", reach);
    return () => document.removeEventListener("keydown", reach);
  }, [cap]);
  return (
    <Input
      ref={field}
      type="search"
      aria-label={label}
      placeholder="Search"
      autoComplete="off"
      data-search-field=""
      value={value}
      onChange={(event) => onValue(event.target.value)}
      onKeyDown={(event) => {
        if (event.key === "Escape" && value !== "") {
          event.preventDefault();
          event.stopPropagation();
          onValue("");
        }
      }}
      trailing={<Kbd>{cap}</Kbd>}
    />
  );
}
