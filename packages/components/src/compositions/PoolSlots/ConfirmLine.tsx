import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

export function Line({ Glyph, said, word, figure, children }: { Glyph: LucideIcon; said: string; word: string; figure?: ReactNode; children?: ReactNode }) {
  return (
    <div className="armada-confirm__row">
      <div className="armada-confirm__line">
        <Tooltip label={said}>
          <span className="armada-confirm__mark" role="img" aria-label={said}>
            <Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span className="armada-confirm__word">{word}</span>
        {figure}
      </div>
      {children}
    </div>
  );
}

export function Mono({ items, label }: { items: readonly string[]; label: string }) {
  return (
    <ul className="armada-confirm__names" aria-label={label}>
      {items.map((one) => (
        <li key={one}>{one}</li>
      ))}
    </ul>
  );
}
