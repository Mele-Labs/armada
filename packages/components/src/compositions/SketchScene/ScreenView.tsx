import {
  ArrowLeft, ArrowRight, ArrowUpFromLine, Bell, Bookmark, Bot, Check, ChevronDown, ChevronLeft, ChevronRight, ChevronUp, CircleCheck, CircleHelp,
  Clock, Copy, Database, Download, Ellipsis, ExternalLink, Eye, FileText, Files, Filter, Flag, Folder, FolderPlus, GitBranch, Globe, History, House,
  Image as ImageIcon, Key, Layers, LayoutGrid, Link as LinkIcon, ListChecks, Lock, Mail, Menu, MessageSquare, Monitor, Package, PanelLeftOpen,
  Paperclip, Pause, Pencil, Pin, Play, Plus, RefreshCw, Search, Send, Settings, SlidersHorizontal, Smartphone, Terminal, ThumbsDown, ThumbsUp,
  Ticket, Trash2, TriangleAlert, UserCheck, X,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { CSSProperties, HTMLAttributes } from "react";

import { childrenOf, holdsStep, screenLabel, type ScreenAct, type ScreenElement, type ScreenIcon } from "./screen";

const ICON: Record<ScreenIcon, LucideIcon> = {
  "arrow-left": ArrowLeft, "arrow-right": ArrowRight, "arrow-up-from-line": ArrowUpFromLine, bell: Bell, bookmark: Bookmark, bot: Bot, check: Check,
  "chevron-down": ChevronDown, "chevron-left": ChevronLeft, "chevron-right": ChevronRight, "chevron-up": ChevronUp, "circle-check": CircleCheck,
  "circle-help": CircleHelp, clock: Clock, copy: Copy, database: Database, download: Download, ellipsis: Ellipsis, "external-link": ExternalLink,
  eye: Eye, "file-text": FileText, files: Files, filter: Filter, flag: Flag, folder: Folder, "folder-plus": FolderPlus, "git-branch": GitBranch,
  globe: Globe, history: History, house: House, image: ImageIcon, key: Key, layers: Layers, "layout-grid": LayoutGrid, link: LinkIcon,
  "list-checks": ListChecks, lock: Lock, mail: Mail, menu: Menu, "message-square": MessageSquare, monitor: Monitor, package: Package,
  "panel-left-open": PanelLeftOpen, paperclip: Paperclip, pause: Pause, pencil: Pencil, pin: Pin, play: Play, plus: Plus, "refresh-cw": RefreshCw,
  search: Search, send: Send, settings: Settings, "sliders-horizontal": SlidersHorizontal, smartphone: Smartphone, terminal: Terminal,
  "thumbs-down": ThumbsDown, "thumbs-up": ThumbsUp, ticket: Ticket, "trash-2": Trash2, "triangle-alert": TriangleAlert, "user-check": UserCheck, x: X,
};

/** The step playing, if this screen holds it; and what the walk down from the root has found so far. */
type Ctx = {
  at: number | undefined;
  /** An element above is the one playing, so nothing under it stands back. */
  inside: boolean;
  /** The way the parent lays its children, which decides how a child sizes and sits. */
  axis: "row" | "column" | "grid" | undefined;
};

type Roots = HTMLAttributes<HTMLElement> & { [name: `data-${string}`]: string | true | undefined };

const BAR_WIDTHS = ["100%", "92%", "96%", "84%"];

const flexOf = (el: ScreenElement, axis: Ctx["axis"]): string | undefined => {
  if (axis === "grid" || axis === undefined) return undefined;
  if (el.size === "fill") return "1 1 0";
  return typeof el.size === "number" ? `${String(el.size)} 1 0` : undefined;
};

function Glyph({ name, size = 16 }: { name: ScreenIcon; size?: number }) {
  const Icon = ICON[name];
  return <Icon size={size} strokeWidth={1.75} aria-hidden />;
}

/** A button, or a bar's item: the words, the glyph, or both. */
function Act({ act, variant = "ghost", compact = false }: { act: ScreenAct; variant?: "primary" | "secondary" | "ghost"; compact?: boolean }) {
  const one = typeof act === "string" ? { text: act } : act;
  return (
    <span className="armada-wf__button" data-variant={variant} data-compact={compact || undefined} data-icon-only={one.text === undefined || undefined}>
      {one.icon === undefined ? null : <Glyph name={one.icon} size={compact ? 14 : 16} />}
      {one.text === undefined ? null : <span>{one.text}</span>}
    </span>
  );
}

const sameItem = (act: ScreenAct, active: string | undefined): boolean => {
  if (active === undefined) return false;
  const one = typeof act === "string" ? { text: act } : act;
  return (one.text ?? one.icon) === active;
};

function Items({ items, active }: { items: readonly ScreenAct[]; active: string | undefined }) {
  return items.map((act, at) => {
    const one = typeof act === "string" ? { text: act } : act;
    return (
      <span key={`${one.text ?? one.icon ?? ""}${String(at)}`} className="armada-wf__item" data-active={sameItem(act, active) || undefined} data-icon-only={one.text === undefined || undefined}>
        {one.icon === undefined ? null : <Glyph name={one.icon} />}
        {one.text === undefined ? null : <span>{one.text}</span>}
      </span>
    );
  });
}

function Kids({ el, ctx, axis }: { el: ScreenElement; ctx: Ctx; axis: Ctx["axis"] }) {
  return childrenOf(el).map((one, at) => <Part key={`${one.type}${String(at)}`} el={one} ctx={{ ...ctx, axis }} />);
}

function Field({ el, root }: { el: Extract<ScreenElement, { type: "field" }>; root: Roots }) {
  const kind = el.kind ?? "input";
  const shown = el.value ?? el.placeholder;
  const on = el.value === "on";
  return (
    <div {...root} data-kind={kind}>
      {el.label === undefined || kind === "checkbox" || kind === "toggle" ? null : <span className="armada-wf__label">{el.label}</span>}
      {kind === "checkbox" || kind === "toggle" ? (
        <span className="armada-wf__choice">
          <span className="armada-wf__mark" data-kind={kind} data-on={on || undefined}>
            {kind === "checkbox" && on ? <Check size={12} strokeWidth={2.5} aria-hidden /> : null}
          </span>
          {el.label === undefined ? null : <span>{el.label}</span>}
        </span>
      ) : (
        <span className="armada-wf__control" data-placeholder={el.value === undefined || undefined}>
          {kind === "search" ? <Search size={14} strokeWidth={1.75} aria-hidden /> : null}
          <span className="armada-wf__value">{shown ?? " "}</span>
          {kind === "select" ? <ChevronDown size={14} strokeWidth={1.75} aria-hidden /> : null}
        </span>
      )}
    </div>
  );
}

/** One element and everything under it. Standing back is decided here, on the way down. */
function Part({ el, ctx }: { el: ScreenElement; ctx: Ctx }) {
  const at = ctx.at;
  const current = at !== undefined && el.step === at;
  const dim = at !== undefined && !ctx.inside && !holdsStep(el, at);
  // A modal with a step comes up when its step does, and the steps before it show the screen without it.
  const hidden = at !== undefined && el.type === "modal" && el.step !== undefined && at < el.step;
  // Under a part that stands back, nothing else is decided: it is one faded thing.
  const down: Ctx = dim ? { ...ctx, at: undefined } : { ...ctx, inside: ctx.inside || current };
  const root: Roots = {
    className: `armada-wf armada-wf--${el.type}`,
    "data-axis": ctx.axis,
    "data-align": el.align,
    // A modal rings its dialog, not the scrim round it.
    "data-current": (current && el.type !== "modal") || undefined,
    "data-dim": dim || undefined,
    ...(el.id === undefined && el.step === undefined ? {} : { role: "group", "aria-label": screenLabel(el) }),
    ...(flexOf(el, ctx.axis) === undefined ? {} : { style: { flex: flexOf(el, ctx.axis) } as CSSProperties }),
  };

  switch (el.type) {
    case "row":
    case "column":
    case "grid":
      return (
        <div
          {...root}
          data-gap={el.gap ?? "sm"}
          data-pad={el.pad ?? "none"}
          data-border={el.border || undefined}
          {...(el.type === "grid" ? { style: { ...root.style, ["--wf-cols" as string]: el.cols } as CSSProperties } : {})}
        >
          <Kids el={el} ctx={down} axis={el.type} />
        </div>
      );
    case "window":
      return (
        <div {...root} data-chrome={el.chrome}>
          {el.chrome === "desktop" ? (
            <div className="armada-wf__chrome">
              <span className="armada-wf__dots" aria-hidden>
                <span />
                <span />
                <span />
              </span>
              <span className="armada-wf__window-title">{el.title}</span>
            </div>
          ) : el.chrome === "browser" ? (
            <div className="armada-wf__chrome">
              <div className="armada-wf__tabstrip">
                <span className="armada-wf__tab">{el.title ?? " "}</span>
              </div>
              <div className="armada-wf__toolbar">
                <ChevronLeft size={16} strokeWidth={1.75} aria-hidden />
                <ChevronRight size={16} strokeWidth={1.75} aria-hidden />
                <RefreshCw size={14} strokeWidth={1.75} aria-hidden />
                <span className="armada-wf__address">
                  <Lock size={12} strokeWidth={1.75} aria-hidden />
                  <span>{el.url}</span>
                </span>
              </div>
            </div>
          ) : (
            <div className="armada-wf__chrome">
              <span className="armada-wf__clock">9:41</span>
              <span className="armada-wf__signal" aria-hidden>
                <span />
                <span />
                <span />
                <span className="armada-wf__battery" />
              </span>
            </div>
          )}
          <div className="armada-wf__window-body">
            <Kids el={el} ctx={down} axis="column" />
          </div>
          {el.chrome === "phone" ? <span className="armada-wf__home" aria-hidden /> : null}
        </div>
      );
    case "nav":
      return (
        <div {...root} data-vertical={el.vertical || undefined}>
          {el.brand === undefined ? null : (
            <span className="armada-wf__brand">
              {el.vertical ? null : <span className="armada-wf__logo" aria-hidden />}
              <span>{el.brand}</span>
            </span>
          )}
          {el.items === undefined ? null : (
            <span className="armada-wf__items">
              <Items items={el.items} active={el.active} />
            </span>
          )}
          {el.actions === undefined ? null : (
            <span className="armada-wf__actions">
              {el.actions.map((act, at) => (
                <Act key={String(at)} act={act} />
              ))}
            </span>
          )}
        </div>
      );
    case "panel":
      return (
        <div {...root}>
          {el.title === undefined && el.icon === undefined && el.actions === undefined ? null : (
            <div className="armada-wf__head">
              {el.icon === undefined ? null : <Glyph name={el.icon} size={14} />}
              <span className="armada-wf__head-title">{el.title}</span>
              {el.actions?.map((act, at) => <Act key={String(at)} act={act} variant="secondary" compact />)}
            </div>
          )}
          <div className="armada-wf__panel-body" data-gap={el.gap ?? "sm"} data-pad={el.pad ?? "md"}>
            <Kids el={el} ctx={down} axis="column" />
          </div>
        </div>
      );
    case "tabs":
      return (
        <div {...root}>
          {el.items.map((one, at) => (
            <span key={`${one}${String(at)}`} className="armada-wf__tab-item" data-active={one === el.active || undefined}>
              {one}
            </span>
          ))}
        </div>
      );
    case "list":
      return (
        <div {...root}>
          {el.rows.map((one, at) => (
            <div key={`${one.text}${String(at)}`} className="armada-wf__row" data-selected={one.selected || undefined}>
              {one.icon === undefined ? null : <Glyph name={one.icon} size={14} />}
              <span className="armada-wf__row-text">{one.text}</span>
              {one.meta === undefined ? null : <span className="armada-wf__meta">{one.meta}</span>}
            </div>
          ))}
        </div>
      );
    case "table":
      return (
        <div {...root}>
          <table>
            <thead>
              <tr>
                {el.columns.map((one, at) => (
                  <th key={`${one}${String(at)}`}>{one}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {el.rows.map((cells, at) => (
                <tr key={String(at)}>
                  {cells.map((one, cell) => (
                    <td key={String(cell)}>{one}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
    case "button":
      return (
        <div {...root}>
          <Act act={{ ...(el.text === undefined ? {} : { text: el.text }), ...(el.icon === undefined ? {} : { icon: el.icon }) }} variant={el.variant ?? "secondary"} />
        </div>
      );
    case "field":
      return <Field el={el} root={root} />;
    case "text": {
      const style = el.style ?? "body";
      if (el.text !== undefined) return <p {...root} data-style={style}>{el.text}</p>;
      const lines = el.lines ?? 1;
      return (
        <div {...root} data-style={style} data-bars>
          {Array.from({ length: lines }, (_, at) => (
            <span key={String(at)} className="armada-wf__bar" style={{ inlineSize: lines > 1 && at === lines - 1 ? "60%" : BAR_WIDTHS[at % BAR_WIDTHS.length] }} aria-hidden />
          ))}
        </div>
      );
    }
    case "image":
      return (
        <div {...root}>
          <svg className="armada-wf__cross" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden>
            <line x1="0" y1="0" x2="100" y2="100" vectorEffect="non-scaling-stroke" />
            <line x1="100" y1="0" x2="0" y2="100" vectorEffect="non-scaling-stroke" />
          </svg>
          {el.label === undefined ? null : <span className="armada-wf__caption">{el.label}</span>}
        </div>
      );
    case "avatar":
      return (
        <div {...root}>
          {el.text !== undefined ? (
            <span>{el.text}</span>
          ) : (
            <svg viewBox="0 0 24 24" fill="none" strokeWidth="1.75" strokeLinecap="round" aria-hidden>
              <circle cx="12" cy="9" r="3.5" />
              <path d="M5 20c0-3.6 3.1-6 7-6s7 2.4 7 6" />
            </svg>
          )}
        </div>
      );
    case "badge":
      return <div {...root}>{el.text}</div>;
    case "divider":
      return <div {...root} />;
    case "spacer":
      return <div {...root} />;
    case "chat":
      return (
        <div {...root}>
          {el.messages.map((one, at) => (
            <p key={String(at)} className="armada-wf__message" data-from={one.from}>
              {one.text}
            </p>
          ))}
        </div>
      );
    case "modal":
      return (
        <div {...root} data-hidden={hidden || undefined}>
          <div className="armada-wf__dialog" data-current={current || undefined}>
            <div className="armada-wf__head">
              <span className="armada-wf__head-title">{el.title}</span>
              <X size={16} strokeWidth={1.75} aria-hidden />
            </div>
            <div className="armada-wf__panel-body" data-gap="sm" data-pad="md">
              <Kids el={el} ctx={down} axis="column" />
            </div>
          </div>
        </div>
      );
  }
}

/**
 * A screen, drawn as HTML: **layout is the browser's**, so a tree says what is on the screen and never where. `at` is the step
 * playing when this screen holds it; the element carrying it is ringed and the rest of the screen stands back.
 */
export function ScreenView({ screen, at }: { screen: ScreenElement; at: number | undefined }) {
  return (
    <div className="armada-screen">
      <Part el={screen} ctx={{ at, inside: false, axis: undefined }} />
    </div>
  );
}
