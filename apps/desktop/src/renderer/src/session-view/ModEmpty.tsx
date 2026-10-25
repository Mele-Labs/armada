// A Mod Job with nothing said yet: what a Mod Job can make, one row each, over the box he starts typing in.

import { Bell, Code, Link, Palette, PanelsTopLeft, Ticket } from "lucide-react";

import "./mod-empty.css";

const CAN = [
  { glyph: Palette, name: "Colours", what: "A theme over the whole app" },
  { glyph: PanelsTopLeft, name: "Panels", what: "Hide or reorder the Cockpit's and a Job's panels" },
  { glyph: Link, name: "Words", what: "Turn matching words into links" },
  { glyph: Bell, name: "Notices", what: "How Fleet tells you about a kind of notice" },
  { glyph: Ticket, name: "Issue trackers", what: "One Armada has no adapter for" },
  { glyph: Code, name: "Anything else", what: "A change to Armada's own code, brought in with a preview restart" },
] as const;

export function ModEmpty() {
  return (
    <section className="sv-mod-empty" aria-label="What a Mod Job makes">
      <ul className="sv-mod-empty__list">
        {CAN.map(({ glyph: Glyph, name, what }) => (
          <li key={name} className="sv-mod-empty__row">
            <Glyph className="sv-mod-empty__glyph" size={16} strokeWidth={2} aria-hidden />
            <span className="sv-mod-empty__name">{name}</span>
            <span className="sv-mod-empty__what">{what}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
