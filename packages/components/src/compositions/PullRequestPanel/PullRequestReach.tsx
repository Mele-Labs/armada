import type { KeyboardEvent } from "react";
import type { LucideIcon } from "lucide-react";
import { ClockAlert, FolderTree, Gauge, Route, ScanText, Unplug } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * The tests that reach a changed file, as the test map answers them (`docs/concepts/test-map.md`).
 * **One row of the Files list gains a mark at its trailing end, and opens onto its tests.** The mark
 * is the sharpest source that saw the file, and a second one when an edge is stale. A file no test
 * reaches draws the one mark and says so when opened. A file the host did not ask about has no
 * entry and draws nothing. **No count sits beside any of it.**
 *
 * The shapes follow `TestsReaching` on the wire, flattened to the file: Fleet groups by Check, and
 * a row reads by path.
 */
export type ReachSource = "measured" | "static" | "reading" | "declared";

export type ReachTest = {
  /** The Check it runs under, as `armada check` takes it. */
  check: string;
  /** The test's name, or its file where `grain` is `file`. */
  test: string;
  source: ReachSource;
  grain: "test" | "file";
  /** The file changed since the edge was read. */
  stale: boolean;
};

/** An empty `tests` is a file the map holds edges for and none reaches it. */
export type FileReach = { tests: readonly ReachTest[] };

/** By repository-relative path. Absent for a path the host did not ask the map about. */
export type FilesReach = Readonly<Record<string, FileReach>>;

type Said = { Glyph: LucideIcon; said: string };

// REPORTED: the registry rows for these are Proposed, in `packages/icons/icons`, group Test map.
const SOURCE: Record<ReachSource, Said> = {
  measured: { Glyph: Gauge, said: "Measured" },
  static: { Glyph: Route, said: "Static graph" },
  reading: { Glyph: ScanText, said: "A model's reading" },
  declared: { Glyph: FolderTree, said: "Declared in the Manifest" },
};
const STALE: Said = { Glyph: ClockAlert, said: "Stale" };
const UNREACHED: Said = { Glyph: Unplug, said: "No test reaches this file" };

/** Sharpest first: the map ranks a measured edge over a static one, a static over a reading, and the declaration is the floor. */
const SHARPEST: readonly ReachSource[] = ["measured", "static", "reading", "declared"];

/** The marks for a set of edges: its sharpest source, then stale where any edge is. None reaching it is one mark. */
function marksOf(tests: readonly ReachTest[]): Said[] {
  if (tests.length === 0) return [UNREACHED];
  const source = SHARPEST.find((one) => tests.some((test) => test.source === one));
  return [...(source === undefined ? [] : [SOURCE[source]]), ...(tests.some((test) => test.stale) ? [STALE] : [])];
}

function Marks({ marks }: { marks: readonly Said[] }) {
  return (
    <span className="armada-pr-reach__marks">
      {marks.map(({ Glyph, said }) => (
        <span key={said} className="armada-pr-mark" aria-hidden>
          <Glyph size={16} strokeWidth={2} aria-hidden />
        </span>
      ))}
    </span>
  );
}

/** Each mark's word, one under another: the tooltip of whatever the marks sit in. */
function Words({ marks }: { marks: readonly Said[] }) {
  return (
    <span className="armada-pr-reach__words">
      {marks.map(({ said }) => (
        <span key={said}>{said}</span>
      ))}
    </span>
  );
}

/** The press at a file row's trailing end: its marks, and opening or closing the tests under it. */
export function ReachToggle({ name, reach, open, onToggle }: { name: string; reach: FileReach; open: boolean; onToggle: () => void }) {
  const marks = marksOf(reach.tests);
  return (
    <Tooltip label={<Words marks={marks} />}>
      {/* Arrow keys on the row do this from the keyboard, so the press is not a second stop. */}
      <button type="button" className="armada-pr-reach__toggle" aria-label={`Tests reaching ${name}`} aria-expanded={open} tabIndex={-1} onClick={onToggle}>
        <Marks marks={marks} />
      </button>
    </Tooltip>
  );
}

/**
 * The tests under an opened file, one row each: how the map knows, whether the edge is stale, the
 * test and its Check. **A test opens only where it is a file** (a `file` grain edge names its file;
 * a named test does not say where it lives), so the others are rows to move through and nothing
 * to press. ← closes the file and returns to its row.
 */
// A narrow panel cuts a test's directories or modules and keeps its own name whole.
function cutAt(name: string): number {
  const at = Math.max(name.lastIndexOf("/") + 1, name.lastIndexOf("::") + 2);
  return at > 1 ? at : 0;
}

const headOf = (name: string) => name.slice(0, cutAt(name));
const leafOf = (name: string) => name.slice(cutAt(name));

export function ReachTests({ name, reach, nested, onTest, onClose }: { name: string; reach: FileReach; nested: boolean; onTest?: ((test: ReachTest) => void) | undefined; onClose: () => void }) {
  const leave = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "ArrowLeft") return;
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.closest("li.armada-pr-tree__row")?.querySelector<HTMLElement>("[data-nav]")?.focus();
    onClose();
  };
  return (
    <ul className="armada-pr-list armada-pr-reach" aria-label={`${name}, tests`} data-nested={nested || undefined}>
      {reach.tests.length === 0 ? (
        <li className="armada-pr-reach__none">
          <div className="armada-pr-reach__item" data-nav tabIndex={-1} onKeyDown={leave}>
            {UNREACHED.said}
          </div>
        </li>
      ) : (
        reach.tests.map((test) => {
          const marks = marksOf([test]);
          const body = (
            <>
              <Tooltip label={<Words marks={marks} />} decorative>
                <Marks marks={marks} />
              </Tooltip>
              <Tooltip label={test.test} decorative>
                <span className="armada-pr-reach__test mono">
                  <span className="armada-pr-reach__head">{headOf(test.test)}</span>
                  <span className="armada-pr-reach__leaf">{leafOf(test.test)}</span>
                </span>
              </Tooltip>
              <span className="armada-pr-reach__check mono">{test.check}</span>
            </>
          );
          // The marks are pointer-only, so the words they carry are in the row's name.
          const label = [test.test, ...marks.map((mark) => mark.said), test.check].join(", ");
          return (
            <li key={`${test.check}:${test.test}`} className="armada-pr-reach__row">
              {test.grain === "file" && onTest !== undefined ? (
                <button type="button" className="armada-pr-reach__item" data-nav aria-label={label} onClick={() => onTest(test)} onKeyDown={leave}>
                  {body}
                </button>
              ) : (
                <div className="armada-pr-reach__item" data-nav role="group" aria-label={label} tabIndex={-1} onKeyDown={leave}>
                  {body}
                </div>
              )}
            </li>
          );
        })
      )}
    </ul>
  );
}
