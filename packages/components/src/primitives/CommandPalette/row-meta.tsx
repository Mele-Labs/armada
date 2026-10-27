// What a rich palette row says besides its label: the Job's state, a number, the repository, an alias.
// Each sits in a fixed column, so a number is under the number above it however long the title runs.

import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../Tooltip/Tooltip";

export type PaletteMeta = {
  /** The repository's name, in its own column. */
  repo?: string;
  /** `#2911`, a pull request's, an issue's or the one a Job holds. Searched as well as drawn. */
  number?: string;
  /** A Job's state: a registry glyph and the tooltip that names it. Never a phrase. */
  state?: { icon: LucideIcon; said: string; pulsing?: true };
  /** What a Commands row is also found by, drawn after its label. */
  alias?: string;
};

/** Which columns the rows showing hold, so a row without one keeps the space and the rest stay in line. */
export type PaletteColumns = { state: boolean; number: boolean; repo: boolean };

export function columnsOf(metas: readonly (PaletteMeta | undefined)[]): PaletteColumns {
  return {
    state: metas.some((one) => one?.state !== undefined),
    number: metas.some((one) => one?.number !== undefined),
    repo: metas.some((one) => one?.repo !== undefined),
  };
}

/** Beside the label: the alias, small and quiet. */
export function LabelMeta({ meta }: { meta: PaletteMeta | undefined }) {
  return meta?.alias === undefined ? null : <span className="armada-palette__alias">{meta.alias}</span>;
}

/** The columns after the label, in the order state, number, repository. */
export function MetaCells({ meta, columns }: { meta: PaletteMeta | undefined; columns: PaletteColumns }) {
  const Glyph = meta?.state?.icon;
  return (
    <>
      {!columns.state ? null : (
        <span className="armada-palette__col armada-palette__col--state">
          {Glyph === undefined || meta?.state === undefined ? null : (
            <Tooltip label={meta.state.said}>
              <span className="armada-session-mark" role="img" aria-label={meta.state.said} data-pulsing={meta.state.pulsing}>
                <Glyph size={16} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          )}
        </span>
      )}
      {!columns.number ? null : <span className="armada-palette__col armada-palette__col--number">{meta?.number}</span>}
      {!columns.repo ? null : <span className="armada-palette__col armada-palette__col--repo">{meta?.repo}</span>}
    </>
  );
}
