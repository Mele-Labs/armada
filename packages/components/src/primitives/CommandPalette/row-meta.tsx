// What a rich palette row says besides its label: the Job's state, a number, the repository.
// Laid out as Raycast lays a row (the owner, 27 Oct 2026): the state takes the glyph's place, the repository
// and number follow the label as one muted line, and the right edge holds only the binding and the kind.

import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../Tooltip/Tooltip";

export type PaletteMeta = {
  /** The repository's name, after the label. */
  repo?: string;
  /** `#2911`, a pull request's, an issue's or the one a Job holds. Searched as well as drawn. */
  number?: string;
  /** A Job's state: a registry glyph and the tooltip that names it, in the glyph's place. Never a phrase. */
  state?: { icon: LucideIcon; said: string; pulsing?: true };
};

/** The state, drawn where another row draws its glyph, so every row starts on one column. */
export function StateGlyph({ state }: { state: NonNullable<PaletteMeta["state"]> }) {
  const Glyph = state.icon;
  return (
    <Tooltip label={state.said}>
      <span className="armada-session-mark" role="img" aria-label={state.said} data-pulsing={state.pulsing}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** After the label, muted: the repository, then the number. */
export function Subtitle({ meta, value, dormant }: { meta: PaletteMeta | undefined; value: string | undefined; dormant: string | undefined }) {
  const parts = [
    meta?.repo === undefined ? null : <span key="repo">{meta.repo}</span>,
    meta?.number === undefined ? null : <span key="number" className="armada-palette__mono">{meta.number}</span>,
    value === undefined ? null : <span key="value" className="armada-palette__mono">{value}</span>,
    dormant === undefined ? null : <span key="dormant">{dormant}</span>,
  ].filter((one) => one !== null);
  return parts.length === 0 ? null : <span className="armada-palette__subtitle">{parts}</span>;
}
