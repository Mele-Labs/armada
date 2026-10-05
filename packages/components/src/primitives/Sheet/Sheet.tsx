import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode, type Ref, type RefObject } from "react";
import { ChevronLeft, X } from "lucide-react";
import { Button } from "../Button/Button";
import { KbdBinding } from "../Kbd/Kbd";
import { DockHandle, clampToRange, defaultDockWidth, dockWidthRange, tokenPx } from "../../dock-handle";
import { rememberSheetWidth, useSheetWidth } from "../../sheet-width";

/**
 * A panel that enters from an edge. The contract gives it exactly one line —
 * "Sheet and dialog use the same surface treatment at `--radius-lg`" — and the
 * component sheet draws it nowhere, so everything below the surface treatment
 * is read off Dialog and reported as underspecified.
 *
 * `x` is the shadcn dialog close, which the icon registry sanctions as chrome.
 *
 * # The trailing sheet, and why the parts are slots
 *
 * Journey 4's frames `4i`–`4m` put two readings on this layer — a step's
 * activity log and the Job's whole patch — and neither is a longer version of
 * something a panel can hold: 1676 entries pushes the rest of the screen off
 * the bottom and a patch in a 602px column is unreadable. The frames draw four
 * parts this component had no slot for, so each is a slot rather than a second
 * component: a subtitle under the title, controls in the header, full-width
 * bands under it, and a body that carries its own padding.
 *
 * **Two exits and no third.** The labelled control and `Esc`. A click on the
 * ground behind does not close a sheet — a 1676-entry read must not be
 * dismissed by a stray click, so the scrim takes no press.
 *
 * **`Esc` is caught in the capture phase and stopped there.** The registry row
 * reads *closes an overlay, or returns to the list from a detail route*, and
 * both clauses are bound on `window`: without the stop, one press would close
 * the sheet and leave the Job at the same time.
 */
export type SheetSide = "right" | "left";

/**
 * How much of the ground the sheet takes. The drawing measures both as a
 * fraction rather than a width, because what has to fit is the reading and not
 * a column: `wide` is `4i`'s 62% and `widest` is `4j`'s 76%, which is the file
 * rail plus a patch line that does not wrap.
 *
 * `default` is `--w-sheet`, as the component sheet drew it; `dock`, `--w-dock`.
 *
 * `reading` is the run sheet's own, at 88% — Journey 9, running a Manifest
 * entry against a real Fleet. Nick's own note, watching it work: it opened far
 * too narrow. The run sheet's list column is a fixed 240px and everything past
 * it is the run's own output, which is most of the reading — 62% leaves the
 * output the same narrow column `wide` gives a log with a file rail beside it,
 * and this sheet has no rail to share that width with.
 *
 * MISSING TOKEN, reported: `--w-sheet` is 480px and describes none of these.
 * A fraction of the ground is not a width and has no token to be.
 */
export type SheetSize = "default" | "dock" | "wide" | "widest" | "reading";

/**
 * The way back to where a person was before a press elsewhere opened this
 * sheet — Plan's task panel sending them to its Drone, a Check on the plan
 * board to its Record row. **One slot for every sheet**, so a jump reads the
 * same wherever it lands.
 *
 * `chevron-left` leads the label, 12px at strokeWidth 2 — the back half of the
 * registry's `history` act, registered by the owner on 30 Sep 2026. The label
 * names the thing returned to — `Back to T6` — and the tooltip names the
 * destination too: `Back to Plan · T6`.
 */
export type SheetBack = {
  label: string;
  tooltip: string;
  /** Drawn beside the label, as the close draws `Esc`. */
  binding?: string;
  onBack: () => void;
};

/**
 * Whose width the sheet draws. **Every sheet resizes** by Helm's handle on its
 * inner edge (owner, 2 Oct 2026), so every sheet names one or the other.
 */
type SheetWidth =
  | {
      /**
       * Which kind of sheet this is — `plan-task`, `drone`, `job-diff`. Each
       * kind remembers its own width across opens and a restart
       * (`sheet-width.ts`); one never dragged draws its `size`.
       */
      kind: string;
      width?: never;
      onResize?: never;
    }
  | {
      kind?: never;
      /**
       * A width another surface owns: Helm folded into a sheet shares the
       * dock's, so a drag of either sets both. Clamped here as a kind's is.
       */
      width: number | undefined;
      onResize: (width: number) => void;
    };

export type SheetProps = SheetWidth & {
  open: boolean;
  /** Sentence case. Panel headings may open with a Wh- word; sentences may not. */
  title: string;
  /**
   * The line under the title — what this sheet is of, and how much of it.
   * `Fix · job_2d90bb · 1676 entries · live`.
   */
  subtitle?: ReactNode;
  /**
   * Beside the title, at the head's leading edge — Helm's chip, which marks
   * its folded dock as Helm rather than one more sheet (#1320). Decoration: it
   * adds nothing to the dialog's name, which stays `title`. Absent draws
   * nothing, and the head is laid out exactly as without it.
   */
  leading?: ReactNode;
  /** The way back, at the head's leading edge before everything else. Absent draws nothing. */
  back?: SheetBack | undefined;
  children: ReactNode;
  side?: SheetSide;
  size?: SheetSize;
  /**
   * Controls in the header, between the titles and the close — the log's four
   * filters. `tabs` on a layer is a departure the drawing states: nothing else
   * places the primitive on one, and four filters over 1676 entries are three
   * hidden behind a select.
   */
  controls?: ReactNode;
  /**
   * Full-width bands between the header and the body — the held strip, the
   * escalation notice. They are bands rather than body content because they
   * stay while the body scrolls under them.
   */
  bands?: ReactNode;
  /** The body carries its own padding, for a reading that runs edge to edge. */
  bleed?: boolean;
  /**
   * The body's own scroll container, for a caller that has to read or drive
   * its scroll position. The activity log sheet followed its tail through
   * it (#1155), and nothing reads it since that sheet went on 2 Oct 2026.
   */
  bodyRef?: Ref<HTMLDivElement>;
  /**
   * Laid out inside the nearest positioned ancestor rather than over the
   * window. **What a trailing sheet takes**: the layer belongs to the screen it
   * was opened from, and a window-fixed one would cover the shell's rail as
   * well, which nothing asked it to.
   */
  contained?: boolean;
  /**
   * Beside another floating sheet rather than at the trailing edge, named by
   * that sheet's `kind` — the file diff to the left of Plan's task panel
   * (owner, 29 Sep 2026), `--space-4` from it at whatever width that one was
   * dragged to, and `--w-dock` where it never was. **Its scrim dims nothing
   * and takes no press**: the sheet it sits beside already dims the screen
   * once. Read only with `floating`; at `floor` it lies over that sheet.
   */
  beside?: string;
  /**
   * The ground behind stays pressable: the scrim dims nothing and takes no
   * press, so a press on the surface it covers still lands there. The approval
   * canvas's, where pressing another node swaps the open panel to that node.
   * Close and `Esc` still close it. Absent is every other sheet.
   */
  through?: boolean;
  /**
   * Another layer lies over this one and takes `Esc` first. Both bind on
   * `window` in the capture phase, where the first one opened runs first, so
   * the one underneath has to be told to wait.
   */
  under?: boolean;
  /**
   * A panel over the whole work area rather than a layer inside one screen:
   * under the title row, held off every edge by `--space-4`, rounded and
   * bordered — Helm's dock, as a sheet. **What a screen's own reading takes
   * where `contained` would clip it** against the content column's edge, which
   * is what the owner saw on the Record, 29 Sep 2026. Wins over `contained`.
   */
  floating?: boolean;
  /**
   * The close control's label, and the binding drawn beside it. Absent leaves
   * the close icon-only with the binding in its tooltip, which is what the
   * floor takes — `4l`, and there only.
   */
  closeLabel?: string;
  closeBinding?: string;
  /**
   * The window is at `--window-floor`. Flush to both edges, no radius, and the
   * close goes icon-only. **A prop rather than a media query**: a media query
   * cannot read a custom property, and the Tailwind breakpoint variants that
   * were meant to stand in for one emit `@media (width >= var(--layout-breakpoint))`,
   * which every browser drops. The app reads `--window-floor` itself and passes
   * the answer down. Reported.
   */
  floor?: boolean;
  /** The one action a sheet's footer carries, where it carries one. */
  footer?: ReactNode;
  onClose?: () => void;
};

export function Sheet({
  open,
  title,
  subtitle,
  leading,
  back,
  children,
  side = "right",
  size = "default",
  controls,
  bands,
  bleed = false,
  bodyRef,
  contained = false,
  beside,
  through = false,
  kind,
  width,
  onResize,
  under = false,
  floating = false,
  closeLabel,
  closeBinding,
  floor = false,
  footer,
  onClose,
}: SheetProps) {
  const closeRef = useRef<HTMLButtonElement>(null);
  const scrimRef = useRef<HTMLDivElement>(null);
  const sheetRef = useRef<HTMLDivElement>(null);

  const remembered = useSheetWidth(kind);
  const wanted = onResize === undefined ? remembered : width;
  const resize = (next: number): void => {
    if (onResize !== undefined) onResize(next);
    else if (kind !== undefined) rememberSheetWidth(kind, next);
  };
  const neighbour = useSheetWidth(beside);

  // At the floor the sheet is flush to both edges, and there is nothing to
  // resize. Everywhere else there is no ceiling but the area under the scrim
  // (owner, 2 Oct 2026): it drags to the far edge, keeping the handle's
  // `--space-4` gap and a floating sheet's margin, or, beside another sheet,
  // the gap from that one.
  const resizes = !floor;
  const follows = floating && resizes && beside !== undefined;
  const room = useWidthOf(scrimRef, open && resizes);
  const atRest = useWidthOf(sheetRef, open && resizes && wanted === undefined);
  const space = tokenPx("--space-4");
  const followed = follows ? clampToRange(neighbour ?? defaultDockWidth(), rangeIn(room - 2 * space)) : 0;
  const range = rangeIn(follows ? room - followed - 4 * space : room - (floating ? 2 : 1) * space);
  // Not measured yet (a first render, or no layout at all): as given.
  const drawn = wanted === undefined || room === 0 ? wanted : clampToRange(wanted, range);

  useEffect(() => {
    // `preventScroll`: a sheet that is still travelling in sits past the
    // edge, and focusing its close scrolls whatever holds it to reach it.
    if (open) closeRef.current?.focus({ preventScroll: true });
  }, [open]);

  // Esc closes an overlay, per the global tier — and stops there. Bound in the
  // capture phase because the other clause of the same registry row, "returns
  // to the list from a detail route", is bound on `window` too: a bubble-phase
  // listener would run second and both would answer one press.
  useEffect(() => {
    if (!open || under) return;
    function onKey(event: KeyboardEvent) {
      // A popover over the sheet is the top layer. Either it took the press already, or it is still
      // open and will: listeners on one window run in no order this can rely on.
      if (event.defaultPrevented) return;
      if (closeRef.current?.closest('[role="dialog"]')?.querySelector(".armada-popover__panel")) return;
      // The command palette opens over every surface, a sheet included, and takes the press for the same reason.
      if (document.querySelector(".armada-palette")) return;
      // A toast stands over the sheet, and a press made inside it is the toast's.
      if (document.activeElement?.closest(".armada-toast")) return;
      // A sheet drawn after this one lies over it, so the press is that one's. **The order on the
      // page, not the order they opened in**: a Check's log opened from a strip inside another
      // sheet closes first, and which listener was added first says nothing about which is on top.
      const sheets = document.querySelectorAll(".armada-sheet");
      if (sheetRef.current !== null && sheets.length > 0 && sheets[sheets.length - 1] !== sheetRef.current) return;
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onClose?.();
      }
    }
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [open, under, onClose]);

  if (!open) return null;

  const labelled = closeLabel !== undefined && !floor;
  const handle = (
    <DockHandle
      width={drawn ?? atRest}
      {...range}
      label={title}
      grows={side === "right" ? "left" : "right"}
      onResize={resize}
    />
  );
  const tooltip = closeBinding === undefined ? "Close" : `Close — ${closeBinding}`;

  const close = labelled ? (
    /* A secondary on an overlay is filled one surface step from its
       ground, which is what `ground="sunken"` spells. */
    <Button
      ref={closeRef}
      variant="secondary"
      size="sm"
      ground="sunken"
      title={tooltip}
      onClick={onClose}
    >
      {closeLabel}
      {closeBinding === undefined ? null : <KbdBinding binding={closeBinding} />}
    </Button>
  ) : (
    <button
      ref={closeRef}
      type="button"
      className="armada-sheet__close"
      aria-label="Close"
      title={tooltip}
      onClick={onClose}
    >
      <X size={16} strokeWidth={2} aria-hidden="true" />
    </button>
  );

  return (
    <div
      ref={scrimRef}
      className="armada-sheet-scrim"
      style={follows && room > 0 ? ({ "--armada-sheet-beside": `${followed}px` } as CSSProperties) : undefined}
      data-contained={(contained && !floating) || undefined}
      data-floating={floating || undefined}
      data-beside={(floating && beside !== undefined) || undefined}
      data-through={through || undefined}
    >
      {resizes && side === "right" ? handle : null}
      <div
        ref={sheetRef}
        className="armada-sheet"
        style={drawn === undefined ? undefined : { width: `${drawn}px`, maxWidth: "none" }}
        data-floating={floating || undefined}
        data-side={side}
        data-size={size}
        data-floor={floor || undefined}
        role="dialog"
        // Beside another sheet, the pair is the one modal layer: marking both
        // modal would hide each from the other.
        aria-modal={(floating && beside !== undefined) || through ? undefined : "true"}
        aria-label={title}
      >
        <div className="armada-sheet__head">
          {back === undefined ? null : (
            /* The way back and the way out share the head's first line, and
               the title takes the whole of the line under them: beside the
               title, a narrow panel's head had room for neither. */
            <div className="armada-sheet__way">
              <Button
                variant="ghost"
                size="sm"
                title={back.binding === undefined ? back.tooltip : `${back.tooltip} — ${back.binding}`}
                onClick={back.onBack}
              >
                <ChevronLeft size={12} strokeWidth={2} aria-hidden="true" />
                {back.label}
                {back.binding === undefined ? null : <KbdBinding binding={back.binding} />}
              </Button>
              {close}
            </div>
          )}
          {leading === undefined ? null : <div className="armada-sheet__leading">{leading}</div>}
          <div className="armada-sheet__titles">
            <h2 className="armada-sheet__title" data-titled={subtitle !== undefined || undefined}>
              {title}
            </h2>
            {subtitle === undefined ? null : (
              <span className="armada-sheet__subtitle">{subtitle}</span>
            )}
          </div>
          {controls === undefined ? null : (
            <div className="armada-sheet__controls">{controls}</div>
          )}
          {back === undefined ? close : null}
        </div>
        {bands === undefined ? null : <div className="armada-sheet__bands">{bands}</div>}
        <div ref={bodyRef} className="armada-sheet__body" data-bleed={bleed || undefined}>
          {children}
        </div>
        {footer ? <div className="armada-sheet__foot">{footer}</div> : null}
      </div>
      {resizes && side === "left" ? handle : null}
    </div>
  );
}

/** `--w-dock-min` up to all of `open`, the width the sheet can cover. */
function rangeIn(open: number) {
  const { min, max } = dockWidthRange(Number.NaN);
  return { min, max: Number.isFinite(open) ? Math.max(min, open) : max };
}

/**
 * An element's own width, kept current as it resizes — 0 until it is measured,
 * and while `on` is false. Measured before paint, so a remembered width is
 * clamped on the frame it first draws rather than one frame late.
 */
function useWidthOf(ref: RefObject<HTMLElement | null>, on: boolean): number {
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!on || element === null) return;
    const read = (): void => setWidth(element.getBoundingClientRect().width);
    read();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(read);
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref, on]);
  return width;
}
