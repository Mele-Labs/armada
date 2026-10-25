// A Mod Job with nothing said yet: what a Mod Job can make, a card each, over the box he starts typing in. He may pick one; it travels with his first message.

import type { KeyboardEvent } from "react";
import { Bell, Check, Code, Link, Palette, PanelsTopLeft, Ticket } from "lucide-react";
import { Tooltip, useKept } from "@armada/components";
import type { ModAim } from "@armada/protocol";

import { aimKey } from "./mod-aim";
import "./mod-empty.css";

const COLUMNS = 2;
const CAN = [
  { aim: "theme", glyph: Palette, name: "Colours", what: "A theme over the whole app" },
  { aim: "layout", glyph: PanelsTopLeft, name: "Panels", what: "Hide or reorder the Cockpit's and a Job's panels" },
  { aim: "words", glyph: Link, name: "Words", what: "Turn matching words into links" },
  { aim: "fleet", glyph: Bell, name: "Notices", what: "How Fleet tells you about a kind of notice" },
  { aim: "tracker", glyph: Ticket, name: "Issue trackers", what: "One Armada has no adapter for" },
  { aim: "source", glyph: Code, name: "Anything else", what: "A change to Armada's own code, brought in with a preview restart" },
] as const satisfies readonly { aim: ModAim; glyph: unknown; name: string; what: string }[];

export function ModEmpty({ sessionId }: { sessionId: string }) {
  const [picked, pick, clear] = useKept<ModAim | undefined>(aimKey(sessionId), undefined);
  const stop = CAN.find((one) => one.aim === picked)?.aim ?? CAN[0].aim;

  const move = (event: KeyboardEvent<HTMLUListElement>) => {
    const by = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : event.key === "ArrowDown" ? COLUMNS : event.key === "ArrowUp" ? -COLUMNS : 0;
    if (by === 0) return;
    const all = [...event.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]')];
    const to = all.indexOf(document.activeElement as HTMLElement) + by;
    if (to < 0 || to >= all.length) return;
    event.preventDefault();
    all[to]?.focus();
  };

  return (
    <section className="sv-mod-empty" aria-label="What a Mod Job makes">
      <ul className="sv-mod-empty__grid" role="radiogroup" aria-label="Aim" onKeyDown={move}>
        {CAN.map(({ aim, glyph: Glyph, name, what }) => (
          <li key={aim} className="sv-mod-empty__item" role="none">
            <Tooltip label={what} asChild>
              <button
                type="button"
                role="radio"
                aria-checked={aim === picked}
                aria-label={name}
                tabIndex={aim === stop ? 0 : -1}
                className="sv-mod-empty__card"
                onClick={() => (aim === picked ? clear() : pick(aim))}
              >
                <Glyph className="sv-mod-empty__glyph" size={16} strokeWidth={2} aria-hidden />
                <span className="sv-mod-empty__name">{name}</span>
                {aim === picked ? <Check className="sv-mod-empty__check" size={16} strokeWidth={2} aria-hidden /> : null}
                <span className="sv-mod-empty__what">{what}</span>
              </button>
            </Tooltip>
          </li>
        ))}
      </ul>
    </section>
  );
}
