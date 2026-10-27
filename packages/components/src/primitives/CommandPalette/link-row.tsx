// The palette's link row: what was typed, where it is a link to a pull request or an issue, offered as that page opened.

import { CircleDot, GitPullRequest } from "lucide-react";

export type PaletteLinkKind = "pull" | "issue";

export type PaletteLink = {
  /** What the words are a link to, or null. The host reads it, since the palette does not know a forge's addresses. */
  kindOf: (typed: string) => PaletteLinkKind | null;
  /** Opens what was typed. False is refused, and the palette stays up saying so. */
  onOpen: (typed: string, kind: PaletteLinkKind) => Promise<boolean> | boolean;
};

export const LINK_ROW = "armada-palette-row-link";

const ROWS = { pull: { label: "Open review", Glyph: GitPullRequest, kind: "Pull request" }, issue: { label: "Open issue", Glyph: CircleDot, kind: "Issue" } } as const;

export function LinkRow({ kind, link, active, ref, onEnter, onChoose }: { kind: PaletteLinkKind; link: string; active: boolean; ref?: React.Ref<HTMLDivElement> | undefined; onEnter: () => void; onChoose: () => void }) {
  const { label, Glyph, kind: drawn } = ROWS[kind];
  return (
    <div
      ref={ref}
      id={LINK_ROW}
      role="option"
      aria-selected={active}
      className={["armada-palette__row", active ? "armada-palette__row--active" : ""].filter((one) => one !== "").join(" ")}
      onMouseEnter={onEnter}
      onClick={onChoose}
    >
      <span className="armada-palette__glyph">
        <Glyph size={16} strokeWidth={2} aria-hidden />
      </span>
      <span className="armada-palette__label">
        <span className="armada-palette__title">
          {label} <span className="armada-palette__words">{link}</span>
        </span>
      </span>
      <span className="armada-palette__keys-col" />
      <span className="armada-palette__kind">{drawn}</span>
    </div>
  );
}
