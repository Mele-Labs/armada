// The palette's path row: what was typed, where it reads as a file's path, offered as that file opened in the panel.

import { File } from "lucide-react";

export type PalettePath = {
  /** Opens what was typed. False is refused, and the palette stays up saying so. */
  onOpen: (path: string) => Promise<boolean> | boolean;
};

const QUOTES = /^["'`“”‘’]+|["'`“”‘’]+$/g;

/**
 * The path the words read as, trimmed of whitespace and quotes, or null. Absolute and `~/` paths read as one;
 * a relative one needs a slash and an extension, and no spaces, so a sentence about `a/b` is not one. A link is not.
 */
export function readsAsPath(query: string): string | null {
  const typed = query.trim().replace(QUOTES, "").trim();
  if (typed === "" || /^[a-z][a-z0-9+.-]*:\/\//i.test(typed) || typed.includes("\n")) return null;
  if (typed.startsWith("/") || typed.startsWith("~/")) return typed.length > 1 && !typed.endsWith("/") ? typed : null;
  return /^\S+\/[^/\s]*\.[A-Za-z0-9]+$/.test(typed) ? typed : null;
}

export const PATH_ROW = "armada-palette-row-path";

export function PathRow({ path, active, ref, onEnter, onChoose }: { path: string; active: boolean; ref?: React.Ref<HTMLDivElement> | undefined; onEnter: () => void; onChoose: () => void }) {
  return (
    <div
      ref={ref}
      id={PATH_ROW}
      role="option"
      aria-selected={active}
      className={["armada-palette__row", active ? "armada-palette__row--active" : ""].filter((one) => one !== "").join(" ")}
      onMouseEnter={onEnter}
      onClick={onChoose}
    >
      <span className="armada-palette__glyph">
        <File size={16} strokeWidth={2} aria-hidden />
      </span>
      <span className="armada-palette__label">
        <span className="armada-palette__title">
          Open <span className="armada-palette__words">{path}</span>
        </span>
      </span>
      <span className="armada-palette__kind">File</span>
    </div>
  );
}
