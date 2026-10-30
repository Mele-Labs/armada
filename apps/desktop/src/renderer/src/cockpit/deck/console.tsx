// The Cockpit's flight controls drawn as a console, for the mock walk `cockpit-console`. Three takes, picked by `?take=arc|wrap|dash` (`mock/main.tsx`
// puts it on the root). Nothing sets it in the app, so nothing here draws there. To drop the experiment: delete this file and `console.css`, and
// the lines in `Deck.tsx` that name `take`, `ConsoleBlock` and `DispatchKey`.

import type { CSSProperties, KeyboardEvent, ReactNode } from "react";

export type ConsoleTake = "arc" | "wrap" | "dash";

const TAKES: readonly string[] = ["arc", "wrap", "dash"];

/** The take the mock asked for, and nothing in the app. */
export function consoleTake(): ConsoleTake | undefined {
  const asked = document.documentElement.dataset.consoleTake;
  return asked !== undefined && TAKES.includes(asked) ? (asked as ConsoleTake) : undefined;
}

// The top edge of the panel as a path in a 100 by 100 box that is stretched to the panel's width and to the height it rises (`--rise`):
// the arc dips to the box's floor at the middle, the dash steps down by two S curves into a flat tray.
const EDGE: Record<"arc" | "dash", string> = {
  arc: "M0,100 L0,0 Q50,200 100,0 L100,100",
  dash: "M0,100 L0,0 L6,0 C13,0 9,100 16,100 L84,100 C91,100 87,0 94,0 L100,0 L100,100",
};

function Edge({ take }: { take: "arc" | "dash" }) {
  return (
    <svg className="deck-console__edge" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true" focusable="false">
      <path className="deck-console__face" d={`${EDGE[take]} Z`} />
      <path className="deck-console__rim" d={EDGE[take]} />
    </svg>
  );
}

/** Arrows and Home and End move along the keys, and Enter presses the one focused. Tab already reaches each. */
function along(event: KeyboardEvent<HTMLElement>) {
  const keys = [...event.currentTarget.querySelectorAll<HTMLElement>(".deck-tile")];
  const at = keys.indexOf(document.activeElement as HTMLElement);
  if (at < 0) return;
  if (event.key === "Enter") {
    event.preventDefault();
    keys[at]!.click();
    return;
  }
  const to = event.key === "ArrowRight" ? at + 1 : event.key === "ArrowLeft" ? at - 1 : event.key === "Home" ? 0 : event.key === "End" ? keys.length - 1 : undefined;
  if (to === undefined) return;
  event.preventDefault();
  keys[Math.min(Math.max(to, 0), keys.length - 1)]!.focus();
}

/**
 * The deck's tiles in three groups, Dispatch in the middle one: `keys` split in half either side of it. Each key sits in a seat that carries its
 * distance `--d` from the middle, in keys, which is what the arc bends by. The groups are boxes only in `wrap`, where they are the console's sections.
 */
export function ConsoleBlock({ take, keys, centre }: { take: ConsoleTake; keys: ReactNode[]; centre: ReactNode | undefined }) {
  const left = Math.ceil(keys.length / 2);
  const seat = (key: ReactNode, d: number, index: number) => (
    <span key={index} className="deck-console__seat" style={{ "--d": d } as CSSProperties}>
      {key}
    </span>
  );
  const style = { "--n": keys.length + (centre === undefined ? 0 : 1), "--l": `${left}fr`, "--r": `${keys.length - left}fr` } as CSSProperties;
  return (
    <div className="deck-controls__block deck-console__block" style={style} onKeyDown={along}>
      {take === "wrap" ? null : <Edge take={take} />}
      <div className="deck-console__wing" data-side="left">
        {keys.slice(0, left).map((key, i) => seat(key, i - left, i))}
      </div>
      <div className="deck-console__core">{centre === undefined ? null : seat(centre, 0, left)}</div>
      <div className="deck-console__wing" data-side="right">
        {keys.slice(left).map((key, i) => seat(key, i + 1, left + 1 + i))}
      </div>
    </div>
  );
}
