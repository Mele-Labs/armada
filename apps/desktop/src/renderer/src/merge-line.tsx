// The merge line, as the window draws it: panels under Overview's lists, and a rail surface of its
// own. **The same panels in both places**, the same rows and the same acts, so nothing on the line
// needs the other view to reach it. They share one fold too.
//
// **One panel per repository Fleet serves a line for**, off `mergeLineViews`: the picked one, or
// every one on All, named once there is more than one. With none served for the pick, neither the
// panels nor the rail row draws, rather than a sentence about an absence or a row opening nothing.

import { useEffect, useRef, useState, type RefObject } from "react";

import type { FixMain, LandCheckAt, RepositorySummary } from "@armada/protocol";
import { MergeLine, isPressed, pressedSlot } from "@armada/components";
import { mergeLineViews, type MergeLineView } from "@armada/screens";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";
import { LandCheckLogSheet } from "@armada/jobs";
import { Boundary, SURFACE, useAtFloor } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { followLandCheck } from "./commands";
import { usePanelOpen } from "./panel-open";

type Lined = Pick<BridgeState, "mergeLines" | "repository" | "holds" | "landFollowed" | "jobs">;

export function viewsOf({ mergeLines, repository, holds, jobs }: Lined): readonly MergeLineView[] {
  const repositories: readonly RepositorySummary[] = holds.repositories ?? [];
  return mergeLineViews(mergeLines, repository, repositories, jobs);
}

/** The surfaces the rail and the palette leave off: the merge line's, until Fleet serves one. */
export function hiddenSurfaces(state: Lined): readonly string[] {
  return viewsOf(state).length === 0 ? [SURFACE.mergeLine] : [];
}

/**
 * A panel for each line there is, and the log panel a Check on any of them opens in: **one, held
 * here**, so a second press replaces the first rather than stacking a panel per line.
 */
export function MergeLinePanel({
  state,
  onOpenLink,
  onOpenJob,
  onFix,
  focus,
}: {
  state: Lined;
  onOpenLink: (address: string) => void;
  /** A Job the head or a pull request names. Absent, it is not a press. */
  onOpenJob?: (jobId: string) => void;
  /** Hands main's red to a Job. Absent, the band offers no way to. */
  onFix?: (fix: FixMain) => void;
  focus?: string;
}) {
  const [reading, setReading] = useState<LandCheckAt | null>(null);
  const floor = useAtFloor();
  return (
    <>
      {viewsOf(state).map((view) => (
        <OneLine
          key={view.root}
          view={view}
          {...(focus === undefined ? {} : { focus })}
          {...(onOpenJob === undefined ? {} : { onOpenJob })}
          {...(onFix === undefined ? {} : { onFix })}
          onOpenLink={onOpenLink}
          onOpenCheck={(branch, check) => setReading({ root: view.root, branch, check })}
        />
      ))}
      {reading === null ? null : (
        <LandCheckLogSheet
          at={reading}
          followed={state.landFollowed}
          onFollow={followLandCheck}
          floor={floor}
          onClose={() => setReading(null)}
        />
      )}
    </>
  );
}

/** One repository's panel, folded on its own. */
function OneLine({
  view,
  focus,
  onOpenLink,
  onOpenJob,
  onFix,
  onOpenCheck,
}: {
  view: MergeLineView;
  /** A branch another surface asked for. The panel opens if it holds it. */
  focus?: string;
  onOpenLink: (address: string) => void;
  onOpenJob?: (jobId: string) => void;
  onFix?: (fix: FixMain) => void;
  onOpenCheck: (branch: string, check: string) => void;
}) {
  const [open, setOpen] = usePanelOpen(`merge-line:${view.root}`);
  const holds = focus !== undefined && [...view.line, ...view.landed, ...view.sentBack].some((one) => one.branch === focus);
  useEffect(() => {
    // Only when the ask arrives: `setOpen` is not stable, and a fold the person closes stays closed.
    if (holds) setOpen(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [holds, focus]);
  return (
    <MergeLine
      name={view.name}
      line={view.line}
      landed={view.landed}
      sentBack={view.sentBack}
      notice={view.notice}
      {...(view.hub === undefined ? {} : { hub: view.hub })}
      {...(onOpenJob === undefined ? {} : { onOpenJob })}
      open={open}
      {...(holds ? { focus } : {})}
      {...(onFix === undefined
        ? {}
        : {
            onFix: (choice) =>
              onFix({
                root: view.root,
                ...(choice.kind === "back" ? { job: choice.job } : { brief: choice.request }),
              }),
          })}
      onOpenChange={setOpen}
      onOpenPullRequest={onOpenLink}
      onOpenCheck={onOpenCheck}
    />
  );
}

type Arrow = "ArrowDown" | "ArrowUp" | "ArrowLeft" | "ArrowRight";
const ARROWS: readonly string[] = ["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"];

/**
 * The tile nearest `from` in an arrow's direction, by the boxes as drawn: down a column, across to the
 * other column of the board, and on into the next repository's panel. Distance across the direction
 * counts double, so Down stays in its column while that column has a tile below.
 */
function nearest(tiles: readonly HTMLElement[], from: HTMLElement, arrow: Arrow): HTMLElement | undefined {
  const a = from.getBoundingClientRect();
  const ax = a.left + a.width / 2;
  const ay = a.top + a.height / 2;
  const vertical = arrow === "ArrowDown" || arrow === "ArrowUp";
  let best: HTMLElement | undefined;
  let score = Infinity;
  for (const tile of tiles) {
    if (tile === from) continue;
    const b = tile.getBoundingClientRect();
    const dx = b.left + b.width / 2 - ax;
    const dy = b.top + b.height / 2 - ay;
    const along = arrow === "ArrowDown" ? dy : arrow === "ArrowUp" ? -dy : arrow === "ArrowRight" ? dx : -dx;
    if (along <= 1) continue;
    const near = along + 2 * Math.abs(vertical ? dx : dy);
    if (near < score) [best, score] = [tile, near];
  }
  return best;
}

/**
 * The surface's keys, as the cockpit's glass reads them. **The cursor is DOM focus**: every tile is
 * focusable, so a tile reached by Tab, by a press or by a key is the same cursor. Bare arrows move
 * spatially and are not an act; `j`/`k` walk the tiles in reading order; Enter on a tile, and `o`
 * anywhere in one, open the Job it came from.
 */
function useLineKeys(scope: RefObject<HTMLElement | null>, onOpenJob: ((jobId: string) => void) | undefined): void {
  useListKeydown((event) => {
    const root = scope.current;
    if (root === null || event.defaultPrevented) return;
    const tiles = Array.from(root.querySelectorAll<HTMLElement>("[data-merge-item]"));
    if (tiles.length === 0) return;
    const bare = !(event.metaKey || event.ctrlKey || event.altKey);
    const arrow = bare && ARROWS.includes(event.key);
    if (holdsText(event.target)) {
      // A field on the surface, empty: Escape or Down hands the keys back to the tiles.
      const field = event.target;
      const empty = (field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) && field.value === "";
      if (empty && root.contains(field) && (isPressed("close", event) || (bare && event.key === "ArrowDown"))) {
        event.preventDefault();
        tiles[0]?.focus();
      }
      return;
    }
    const at = document.activeElement;
    const step = pressedSlot("move_focus", event);
    // Focus elsewhere, on the rail row just pressed say: `j` and `k` come to the tiles, as the cockpit's
    // do, and the arrows, Enter and `o` stay with whatever holds focus.
    if (at !== null && at !== document.body && !root.contains(at)) {
      if (step !== 0 && step !== 1) return;
      event.preventDefault();
      return tiles[0]?.focus();
    }
    const here = at instanceof HTMLElement ? at.closest<HTMLElement>("[data-merge-item]") : null;
    if (arrow || step !== -1) {
      event.preventDefault();
      if (here === null) return tiles[0]?.focus();
      if (arrow) return nearest(tiles, here, event.key as Arrow)?.focus();
      const by = step === 0 || step === 2 ? 1 : -1;
      return tiles[Math.min(tiles.length - 1, Math.max(0, tiles.indexOf(here) + by))]?.focus();
    }
    // Enter belongs to whatever holds focus first: on a tile's own link or button, it is theirs.
    const opens = (isPressed("open_focused", event) && at === here) || isPressed("open", event);
    const job = here?.dataset.job;
    if (opens && job !== undefined && onOpenJob !== undefined) {
      event.preventDefault();
      onOpenJob(job);
    }
  });
}

/** The rail surface: the panels alone, on Overview's own padding. */
export function MergeLineSurface({
  state,
  bridge,
  onCopied,
  onOpenLink,
  onOpenJob,
  onFix,
  focus,
}: {
  state: Lined;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  onOpenLink: (address: string) => void;
  onOpenJob?: (jobId: string) => void;
  onFix?: (fix: FixMain) => void;
  focus?: string;
}) {
  const scope = useRef<HTMLDivElement>(null);
  useLineKeys(scope, onOpenJob);
  return (
    <Boundary region="Merge line" bridge={bridge} onCopied={onCopied}>
      <div className="armada-screen__overview" ref={scope}>
        <MergeLinePanel
          state={state}
          onOpenLink={onOpenLink}
          {...(onOpenJob === undefined ? {} : { onOpenJob })}
          {...(onFix === undefined ? {} : { onFix })}
          {...(focus === undefined ? {} : { focus })}
        />
      </div>
    </Boundary>
  );
}
