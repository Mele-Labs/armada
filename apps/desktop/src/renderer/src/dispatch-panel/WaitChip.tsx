// The sentence over the request: the proposer's reading of the wait the brief names, `Starts when [Session]'s [PR lands] · from [main] · needs [..]`.
// Each bracketed part is a button. Tab moves between them; Space, Enter, ↑ or ↓ on one moves it through its alternatives (↑ back, the rest on), and
// on the Session opens the `@` menu again. A part set by hand has the accent at its edge and says so on hover until the words say otherwise.

import { useId, useState, type KeyboardEvent } from "react";
import { SquareTerminal } from "lucide-react";
import { Tooltip } from "@armada/components";

import { SessionMenu, type Offered } from "./SessionMenu";
import { cycled, type Cycle, type WaitChip as Chip, type WaitPart } from "./waits";

const TIPS: Record<WaitPart, string> = { on: "The Job it waits on", when: "When it starts", base: "Where it starts from", needs: "What it needs" };

export function WaitChip({ chip, sessions, disabled, onLeave }: { chip: Chip; sessions: readonly Offered[]; disabled: boolean; onLeave: () => void }) {
  const menuId = useId();
  const [menu, setMenu] = useState<number | null>(null);
  const his = (part: WaitPart) => chip.his.includes(part);
  const tip = (part: WaitPart, text?: string) => `${text ?? TIPS[part]}${his(part) ? ", set by you" : ""}`;

  // Esc gives the keys back to the request, or closes the menu first.
  const leave = (event: KeyboardEvent<HTMLElement>, inMenu: boolean) => {
    if (event.key !== "Escape") return false;
    event.preventDefault();
    event.stopPropagation();
    if (inMenu) setMenu(null);
    else onLeave();
    return true;
  };

  const cycle = (part: WaitPart, one: Cycle) => ({
    onClick: () => chip.set(part, cycled(one.options, one.value, 1)),
    onKeyDown: (event: KeyboardEvent<HTMLElement>) => {
      if (leave(event, false)) return;
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
      event.preventDefault();
      chip.set(part, cycled(one.options, one.value, event.key === "ArrowUp" ? -1 : 1));
    },
  });

  const others = sessions.filter((one) => one.id !== chip.on.id);
  const sessionKeys = (event: KeyboardEvent<HTMLElement>) => {
    if (leave(event, menu !== null)) return;
    if (menu === null) {
      if (event.key === "ArrowDown" || event.key === "ArrowUp" || event.key === " " || event.key === "Enter") {
        event.preventDefault();
        setMenu(0);
      }
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setMenu((menu + (event.key === "ArrowDown" ? 1 : -1) + others.length) % others.length);
    } else if (event.key === " " || event.key === "Enter" || event.key === "Tab") {
      if (event.key !== "Tab") event.preventDefault();
      const one = others[menu];
      setMenu(null);
      if (one !== undefined) chip.set("on", one.id);
    }
  };

  const part = (name: WaitPart, one: Cycle, text: string = one.label) => (
    <Tooltip label={tip(name, name === "needs" ? one.value : undefined)} asChild>
      <button type="button" className="armada-wait__part" data-his={his(name) || undefined} aria-label={`${name === "when" ? "Condition" : name === "base" ? "Starts from" : "Needs"} ${one.label}`} disabled={disabled} {...cycle(name, one)}>
        <span className="armada-wait__text">{text}</span>
      </button>
    </Tooltip>
  );

  return (
    <div className="armada-wait" role="group" aria-label="Start condition">
      <div className="armada-wait__line">
        <span>Starts when</span>
        <Tooltip label={tip("on")} asChild>
          <button
            type="button"
            className="armada-wait__part armada-wait__part--session"
            data-his={his("on") || undefined}
            aria-label={`Job ${chip.on.title}`}
            aria-haspopup="listbox"
            aria-expanded={menu !== null}
            disabled={disabled || others.length === 0}
            onClick={() => setMenu(menu === null ? 0 : null)}
            onKeyDown={sessionKeys}
            onKeyUp={(event) => event.key === " " && event.preventDefault()}
            onBlur={() => setMenu(null)}
          >
            <SquareTerminal size={12} strokeWidth={2} aria-hidden />
            <span className="armada-wait__text">{chip.on.title}</span>
          </button>
        </Tooltip>
        {chip.when.value === "ends" ? null : <span className="armada-wait__glue">'s</span>}
        {part("when", chip.when)}
        <span aria-hidden>·</span>
        <span>from</span>
        {part("base", chip.base)}
        {chip.needs === undefined ? null : (
          <>
            <span aria-hidden>·</span>
            <span>needs</span>
            {part("needs", chip.needs)}
          </>
        )}
      </div>
      {menu === null ? null : <SessionMenu id={menuId} label="Jobs" sessions={others} active={menu} onHover={setMenu} onChoose={(one) => (setMenu(null), chip.set("on", one.id))} />}
    </div>
  );
}
