// One pane: a strip of its tabs with a + that adds one, over the body of the tab in front. A tab is dragged to another pane's strip to move there, or to a pane's edge to split it.

import {
  useEffect,
  useRef,
  useState,
  type DragEvent,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import { LoaderCircle, Minimize2, Plus, X, type LucideIcon } from "lucide-react";
import {
  Button,
  Dialog,
  DropdownMenu,
  SessionMark,
  Tooltip,
  boundKeyFor,
  tagIntoDraft,
  type DropdownMenuEntry,
} from "@armada/components";

import { copyAnnounced } from "../copied";
import { shellsOf } from "../job-shell";
import { askComposerFocus } from "../session-focus";
import { hostOf, peekJobBrowser } from "../job-browser/store";
import { useModList } from "../mod-list";
import { stateOf } from "../session-state";
import { useSessions } from "../sessions-draft";
import { select } from "../cockpit/deck/store";
import { ModJobCard } from "./custom-card";
import { customLabel, peekCustom, seedBuilt, tabTitle, useCustom, useCustomTick } from "./custom";
import { tabGlyph } from "./tab-glyph";
import { splitFocused, focusIn } from "./keys";
import { KINDS } from "./kinds";
import { usePaneFamily, type PaneFamily } from "./store";
import { moveToWindow, whyNotWindow } from "./windows";
import { toggleZoom } from "./zoom";
import {
  activate,
  addTab,
  closePane,
  closeTab,
  focusPane,
  hasKind,
  markTabMod,
  moveTabToEdge,
  moveTabToPane,
  panesOf,
  renameTab,
  type Pane,
  type Side,
  type Tab,
  type TabKind,
} from "./tree";

/** The number in a shell's ID, which a terminal tab reads: `t2` is Terminal 2. A tab with no ID draws the Job's first. */
const terminalNumber = (terminal: string | undefined): number =>
  Number(terminal?.replace(/^t/, "")) || 1;

/** The ID a terminal tab sends and copies. */
const terminalIdOf = (tab: Tab): string => tab.terminal ?? `t${terminalNumber(tab.terminal)}`;

/** The words a tab reads: its own label where it has one, else its kind. A Custom tab reads the name of the mod it built, once it has one. A Terminal tab reads its number until it is renamed. */
export const labelOf = (tab: Tab): string =>
  (tab.kind === "custom" ? customLabel(tab.id) : undefined) ??
  tab.label ??
  (tab.kind === "terminal" ? `Terminal ${terminalNumber(tab.terminal)}` : KINDS[tab.kind].label);

/** What a new tab of a kind is called: a browser by its page's host. The kinds there is one of keep their names, and a terminal reads its number. */
export function labelFor(jobId: string, kind: TabKind): string | undefined {
  const url = kind === "browser" ? peekJobBrowser(jobId)?.url : undefined;
  return url === undefined || url === "" ? undefined : hostOf(url);
}

const ZONES: readonly (Side | "middle")[] = [
  "left",
  "right",
  "top",
  "bottom",
  "middle",
];

/** The tab's own menu. Move to window is there for the kinds that have a window, and says why for the rest. */
function tabEntries(kind: TabKind, zoomed: boolean, editable: boolean): DropdownMenuEntry[] {
  const why = whyNotWindow(kind);
  return [
    ...(kind === "terminal"
      ? ([
          {
            kind: "item",
            id: "send",
            label: "Send to agent",
          },
          {
            kind: "item",
            id: "copy",
            label: "Copy terminal ID",
          },
          {
            kind: "item",
            id: "rename",
            label: "Rename",
          },
          {
            kind: "separator",
            id: "terminal-rule",
          },
        ] as const satisfies readonly DropdownMenuEntry[])
      : []),
    {
      kind: "item",
      id: "window",
      label: "Move to window",
      ...(why === undefined
        ? {}
        : {
            disabled: true,
            hint: why,
          }),
    },
    {
      kind: "item",
      id: "zoom",
      label: zoomed ? "Restore" : "Zoom",
      shortcut: boundKeyFor("pane_zoom"),
    },
    {
      kind: "item",
      id: "right",
      label: "Split right",
      shortcut: boundKeyFor("pane_split_right"),
    },
    {
      kind: "item",
      id: "down",
      label: "Split down",
      shortcut: boundKeyFor("pane_split_down"),
    },
    ...(editable
      ? [
          {
            kind: "item",
            id: "edit",
            label: "Edit with its Job",
          } as const,
          {
            kind: "item",
            id: "job",
            label: "Open the Job that built it",
          } as const,
        ]
      : []),
    {
      kind: "separator",
      id: "rule",
    },
    {
      kind: "item",
      id: "close",
      label: "Close",
      danger: true,
    },
  ];
}

function TabButton({
  jobId,
  pane,
  tab,
  zoomed,
  dragging,
  onDrag,
  glyphOf,
}: {
  jobId: string;
  pane: Pane;
  tab: Tab;
  zoomed: boolean;
  dragging: boolean;
  onDrag: (tabId: string | undefined) => void;
  glyphOf?: ((tab: Tab) => LucideIcon | undefined) | undefined;
}) {
  const family = usePaneFamily();
  const { look, rearrange, peek, jobOpens } = family;
  const label = labelOf(tab);
  const active = pane.active === tab.id;
  const custom = useCustom(tab.id, tab.kind === "custom");
  // The tab keeps its mod's name, so after a restart it is drawn as the mod it was, not a fresh composer.
  useEffect(() => {
    if (tab.kind !== "custom") return;
    if (tab.mod !== undefined && peekCustom(tab.id).stage === "compose") seedBuilt(tab.id, tab.mod);
    else if (custom.stage === "built" && custom.mod !== undefined && custom.mod !== tab.mod)
      look(jobId, (arrangement) => markTabMod(arrangement, tab.id, custom.mod!));
  }, [
    jobId,
    tab.id,
    tab.kind,
    tab.mod,
    custom.stage,
    custom.mod,
  ]);
  // A built Custom tab wears its mod's glyph.
  const Glyph =
    tab.kind === "custom" && custom.stage === "built"
      ? tabGlyph(custom.glyph)
      : (glyphOf?.(tab) ?? KINDS[tab.kind].Glyph);
  const building = tab.kind === "custom" && custom.stage === "building";
  const modJob = useSessions().find((one) => one.id === custom.job);
  // The tab's glyph turns while its Job works, on the first build and on each change after it.
  // The Mod Job is held on a question to him: the tab wears the mark a Job's row wears when it needs him.
  const asking =
    tab.kind === "custom" && modJob !== undefined && stateOf(modJob).state === "waiting";
  const working =
    building ||
    (tab.kind === "custom" && custom.stage === "built" && modJob?.turn.state === "working");
  /** The tab's menu asked for its Job's card, with the caret in its composer. */
  const [summon, setSummon] = useState(false);
  const [point, setPoint] = useState<{
    x: number;
    y: number;
  } | null>(null);
  /** A program is running in the terminal this tab draws, and closing would end it. */
  const [ending, setEnding] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const shut = () => {
    // A terminal tab closing ends its shell, whichever way it was closed.
    if (tab.kind === "terminal") {
      const shells = shellsOf(window.armada);
      shells?.closeShell(shells.keyOf(jobId, tab.terminal));
    }
    rearrange(jobId, (arrangement) => closeTab(arrangement, tab.id));
    focusIn(peek(jobId).focus);
  };
  /** Closed by its own ×, a middle click or its menu: the tab does not come to the front first. A terminal with a program running asks first. */
  const close = () => {
    const shells = tab.kind === "terminal" ? shellsOf(window.armada) : null;
    if (shells === null) return shut();
    void shells
      .busy(shells.keyOf(jobId, tab.terminal))
      .then((busy) => (busy ? setEnding(true) : shut()));
  };
  const rename = (value: string) => {
    setRenaming(false);
    look(jobId, (arrangement) => renameTab(arrangement, tab.id, value));
    // Fleet keeps the name too, so the agent's list of the Job's terminals shows it. The tab reads as it was named whatever Fleet says.
    if (tab.kind === "terminal" && value.trim() !== label) {
      const shells = shellsOf(window.armada);
      shells?.rename(shells.keyOf(jobId, tab.terminal), value.trim());
    }
  };
  // A press that starts on the × is not the start of a drag of the tab.
  const grabbed = useRef(false);
  const choose = (id: string) => {
    look(jobId, (arrangement) => focusPane(activate(arrangement, tab.id), pane.id));
    if (id === "window") moveToWindow(jobId, tab);
    else if (id === "zoom") toggleZoom(jobId, pane.id);
    // A beat after the menu has shut, which gives focus back to the tab and would take it from the card.
    else if (id === "edit") window.setTimeout(() => setSummon(true), 60);
    else if (id === "job")
      custom.job !== undefined &&
        select({
          kind: "session",
          key: `session:${custom.job}`,
        });
    else if (id === "right" || id === "down") splitFocused(family, jobId, id);
    else if (id === "close") close();
    else if (id === "copy") copyAnnounced(terminalIdOf(tab));
    // A beat after the menu has shut, which gives focus back to the tab: the box and the field take it after.
    else if (id === "rename") window.setTimeout(() => setRenaming(true), 60);
    else if (id === "send") {
      tagIntoDraft(jobId, {
        kind: "terminal",
        id: terminalIdOf(tab),
        title: label,
      });
      const thread = panesOf(peek(jobId).root)
        .flatMap((one) => one.tabs)
        .find((one) => one.kind === "thread");
      if (thread === undefined) jobOpens(jobId, "thread");
      else look(jobId, (arrangement) => activate(arrangement, thread.id));
      window.setTimeout(() => askComposerFocus(jobId), 60);
    }
  };
  return (
    <div
      className="job-pane__tab"
      data-active={active || undefined}
      data-dragging={dragging || undefined}
      draggable={!renaming}
      onPointerDown={(event) => {
        grabbed.current = (event.target as Element).closest(".job-pane__close") !== null;
      }}
      onMouseDown={(event) => event.button === 1 && event.preventDefault()}
      onAuxClick={(event) => {
        if (event.button !== 1) return;
        event.preventDefault();
        close();
      }}
      onDragStart={(event: DragEvent) => {
        if (grabbed.current) return event.preventDefault();
        event.dataTransfer.setData("text/plain", tab.id);
        event.dataTransfer.effectAllowed = "move";
        onDrag(tab.id);
      }}
      onDragEnd={() => onDrag(undefined)}
      onContextMenu={(event) => {
        event.preventDefault();
        setPoint({
          x: event.clientX,
          y: event.clientY,
        });
      }}
    >
      <ModJobCard
        tabId={tab.id}
        job={modJob}
        building={building}
        summon={summon}
        onSummoned={() => setSummon(false)}
      >
        {renaming ? (
          <RenameField label={label} onDone={rename} onCancel={() => setRenaming(false)} />
        ) : (
          <button
            type="button"
            role="tab"
            className="job-pane__label"
            aria-selected={active}
            tabIndex={active ? 0 : -1}
            onClick={() => look(jobId, (arrangement) => activate(arrangement, tab.id))}
            onKeyDown={(event) => {
              // Shift+F10 and the Menu key open the tab's menu, anchored to the tab.
              if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
              event.preventDefault();
              const box = event.currentTarget.getBoundingClientRect();
              setPoint({
                x: box.left,
                y: box.bottom,
              });
            }}
          >
            {asking ? (
              <span className="job-pane__asks">
                <SessionMark state="waiting" said="Waiting on you" />
              </span>
            ) : working ? (
              <Tooltip label={modJob?.title ?? "Mod Job"} asChild>
                <span
                  className="job-pane__working"
                  role="img"
                  aria-label={modJob?.title ?? "Mod Job"}
                >
                  <LoaderCircle size={16} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            ) : (
              <Glyph size={16} strokeWidth={2} aria-hidden />
            )}
            <span>{label}</span>
          </button>
        )}
      </ModJobCard>
      <span className="job-pane__close">
        <Tooltip label="Close">
          <Button
            variant="ghost"
            size="sm"
            iconOnly
            aria-label={`Close ${label}`}
            draggable={false}
            onClick={close}
          >
            <X size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      </span>
      <DropdownMenu
        triggerLabel="Tab actions"
        at={
          point ?? {
            x: 0,
            y: 0,
          }
        }
        open={point !== null}
        onOpenChange={(open) => !open && setPoint(null)}
        portal
        entries={tabEntries(tab.kind, zoomed, tab.kind === "custom" && custom.stage === "built")}
        onSelect={choose}
      />
      <Dialog
        open={ending}
        title={`End ${label}`}
        confirmLabel="End terminal"
        onConfirm={() => {
          setEnding(false);
          shut();
        }}
        onCancel={() => setEnding(false)}
      >
        {`A program is running in ${label}.`}
      </Dialog>
    </div>
  );
}

/** A tab's name as a field in the tab: Enter keeps what was typed, Escape leaves the name as it was. */
function RenameField({
  label,
  onDone,
  onCancel,
}: {
  label: string;
  onDone: (value: string) => void;
  onCancel: () => void;
}) {
  // Enter and Escape unmount the field, which can blur it on the way out: the first answer stands.
  const settled = useRef(false);
  const settle = (answer: () => void) => {
    if (settled.current) return;
    settled.current = true;
    answer();
  };
  return (
    <input
      className="job-pane__rename"
      aria-label={`Rename ${label}`}
      defaultValue={label}
      spellCheck={false}
      ref={(field) => {
        field?.focus();
        field?.select();
      }}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === "Enter") {
          event.preventDefault();
          settle(() => onDone(event.currentTarget.value));
        } else if (event.key === "Escape") {
          event.preventDefault();
          settle(onCancel);
        }
      }}
      onBlur={(event) => settle(() => onDone(event.currentTarget.value))}
    />
  );
}

/** The kinds the + offers. A Job has one thread, so it is offered only when none is open. A Draft tab is one draft's, opened from where the draft is listed. */
function addEntries(
  family: PaneFamily,
  thread: boolean,
  built: readonly {
    name: string;
    glyph?: string | undefined;
  }[],
  open: readonly {
    terminal: string;
    name: string;
  }[],
): DropdownMenuEntry[] {
  const kinds = family.kinds
    .filter((kind) => kind !== "custom" && kind !== "draft" && (kind !== "thread" || !thread))
    .flatMap((kind): DropdownMenuEntry[] => [
    {
      kind: "item",
      id: kind,
      label: KINDS[kind].label,
      icon: KINDS[kind].Glyph,
    },
    // Right under the new shell: the Job's shells that have no tab, the agent's among them.
    ...(kind !== "terminal"
      ? []
      : open.map(
          (one): DropdownMenuEntry => ({
            kind: "item",
            id: `terminal:${one.terminal}`,
            label: `Open ${one.name}`,
            icon: KINDS.terminal.Glyph,
          }),
        )),
    ]);
  // Under the rule, the tabs a Mod Job built by name, then Custom, which builds another.
  const { Glyph } = KINDS.custom;
  return [
    ...kinds,
    {
      kind: "separator",
      id: "custom-rule",
    },
    ...built.map(
      (one): DropdownMenuEntry => ({
        kind: "item",
        id: `mod:${one.name}`,
        label: tabTitle(one.name),
        icon: tabGlyph(one.glyph),
      }),
    ),
    {
      kind: "item",
      id: "custom",
      label: KINDS.custom.label,
      icon: Glyph,
    },
  ];
}

/** `+` Terminal: a new shell, in a tab of this pane in front. */
function newTerminal(family: PaneFamily, jobId: string, paneId: string): void {
  const open = (terminal: string | undefined) =>
    family.rearrange(jobId, (arrangement) =>
      addTab(arrangement, paneId, "terminal", undefined, undefined, undefined, undefined, terminal),
    );
  const shells = shellsOf(window.armada);
  if (shells === null) return open(undefined);
  void shells.openNew(jobId, open);
}

/** `+` Open <name>: a shell the Job already has, in a tab of this pane in front. */
function openTerminalTab(
  family: PaneFamily,
  jobId: string,
  paneId: string,
  terminal: string,
  name: string,
): void {
  const shells = shellsOf(window.armada);
  // The Job's first shell is the one a tab with no ID draws.
  const own = shells?.keyOf(jobId, terminal) === jobId ? undefined : terminal;
  family.rearrange(jobId, (arrangement) =>
    addTab(arrangement, paneId, "terminal", name, undefined, undefined, undefined, own),
  );
  shells?.show("page", shells.keyOf(jobId, terminal), true);
}

/** What a family other than the Jobs' offers under +, in place of the Job's kinds and mods, and what a pick does in the pane it was made in. */
export type AddMenu = {
  entries: (pane: Pane) => DropdownMenuEntry[];
  onSelect: (id: string, pane: Pane) => void;
};

export function PaneView({
  jobId,
  pane,
  focused,
  zoomed,
  thread,
  dragging,
  gapLit,
  onDrag,
  add,
  glyphOf,
  children,
}: {
  jobId: string;
  pane: Pane;
  focused: boolean;
  zoomed: boolean;
  thread: boolean;
  dragging: string | undefined /** A gap between panes is lit, so no pane is. */;
  gapLit: boolean;
  onDrag: (tabId: string | undefined) => void;
  add?: AddMenu | undefined;
  glyphOf?: ((tab: Tab) => LucideIcon | undefined) | undefined;
  children: ReactNode;
}) {
  const family = usePaneFamily();
  const { look, rearrange, peek, droppable } = family;
  const [over, setOver] = useState<Side | "middle" | undefined>(undefined);
  // The Job's shells with no tab, as Fleet lists them each time the + opens.
  const [untabbed, setUntabbed] = useState<
    {
      terminal: string;
      name: string;
    }[]
  >([]);
  const listUntabbed = () => {
    const shells = shellsOf(window.armada);
    if (shells === null) return;
    void shells.openTerminals(jobId).then((open) => {
      const tabbed = panesOf(peek(jobId).root).flatMap((one) =>
        one.tabs.flatMap((tab) =>
          tab.kind !== "terminal"
            ? []
            : [
                shells.keyOf(jobId, tab.terminal),
              ],
        ),
      );
      setUntabbed(open.filter((one) => !tabbed.includes(shells.keyOf(jobId, one.terminal))));
    });
  };
  // A Custom tab turning over changes the words and glyph of its tab.
  useCustomTick();
  const built = useModList()
    .filter((one) => one.kind === "tab" && one.valid && one.enabled)
    .map((one) => ({
      name: one.name,
      glyph: one.glyph,
    }));
  // The pane under the pointer while a tab is dragged, lit as a whole.
  const [hot, setHot] = useState(false);
  useEffect(() => {
    if (dragging !== undefined) return;
    setOver(undefined);
    setHot(false);
  }, [
    dragging,
  ]);
  const lands =
    !gapLit &&
    dragging !== undefined &&
    over !== undefined &&
    droppable(jobId, dragging, pane.id, over);
  const active = pane.tabs.find((tab) => tab.id === pane.active);

  // ← and → walk the strip, Home and End go to its ends; the tab reached comes to the front.
  const walk = (event: KeyboardEvent<HTMLDivElement>) => {
    const keys = [
      "ArrowLeft",
      "ArrowRight",
      "Home",
      "End",
    ];
    if (!keys.includes(event.key) || (event.target as HTMLElement).getAttribute("role") !== "tab")
      return;
    const at = pane.tabs.findIndex((tab) => tab.id === pane.active);
    const to =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? pane.tabs.length - 1
          : Math.min(pane.tabs.length - 1, Math.max(0, at + (event.key === "ArrowRight" ? 1 : -1)));
    event.preventDefault();
    const strip = event.currentTarget;
    look(jobId, (arrangement) => activate(arrangement, pane.tabs[to]!.id));
    window.requestAnimationFrame(() =>
      strip.querySelectorAll<HTMLElement>('[role="tab"]')[to]?.focus(),
    );
  };

  const dropOnStrip = (event: DragEvent<HTMLDivElement>) => {
    const id = event.dataTransfer.getData("text/plain");
    if (id === "") return;
    if (!droppable(jobId, id, pane.id, "middle") && !pane.tabs.some((tab) => tab.id === id)) return;
    event.preventDefault();
    const tabs = [
      ...event.currentTarget.querySelectorAll<HTMLElement>(".job-pane__tab"),
    ];
    const index = tabs.filter(
      (one) =>
        one.getBoundingClientRect().left + one.getBoundingClientRect().width / 2 < event.clientX,
    ).length;
    // Counted among the others, since the tab leaves its place first.
    const moving = pane.tabs.findIndex((tab) => tab.id === id);
    rearrange(jobId, (arrangement) =>
      moveTabToPane(arrangement, id, pane.id, moving !== -1 && moving < index ? index - 1 : index),
    );
    onDrag(undefined);
  };

  const dropOnBody = (event: DragEvent<HTMLElement>, zone: Side | "middle") => {
    const id = event.dataTransfer.getData("text/plain");
    event.preventDefault();
    setOver(undefined);
    setHot(false);
    onDrag(undefined);
    if (id === "") return;
    rearrange(jobId, (arrangement) =>
      zone === "middle"
        ? moveTabToPane(arrangement, id, pane.id)
        : moveTabToEdge(arrangement, id, pane.id, zone),
    );
  };

  return (
    <section
      className="deck-pane job-pane"
      data-frame=""
      data-pane={pane.id}
      data-focused={focused || undefined}
      data-hot={(hot && dragging !== undefined && !gapLit) || undefined}
      aria-label={active === undefined ? "Empty pane" : `Pane: ${labelOf(active)}`}
      onFocusCapture={() => look(jobId, (arrangement) => focusPane(arrangement, pane.id))}
      onPointerDownCapture={() => look(jobId, (arrangement) => focusPane(arrangement, pane.id))}
      onDragOver={() => dragging !== undefined && setHot(true)}
      onDragLeave={(event) =>
        !event.currentTarget.contains(event.relatedTarget as Node | null) && setHot(false)
      }
    >
      <div
        className="job-pane__strip"
        role="tablist"
        aria-label="Tabs"
        onKeyDown={walk}
        onDoubleClick={(event) => {
          // A tab or the strip's empty space zooms the pane; the ×, the + and the menus keep their own presses.
          if (
            (event.target as Element).closest(
              '.job-pane__close, .armada-dropdown-menu, button:not([role="tab"])',
            ) !== null
          )
            return;
          toggleZoom(jobId, pane.id);
        }}
        onDragOver={(event) => event.preventDefault()}
        onDrop={dropOnStrip}
      >
        {pane.tabs.map((tab) => (
          <TabButton
            key={tab.id}
            jobId={jobId}
            pane={pane}
            tab={tab}
            zoomed={zoomed}
            dragging={dragging === tab.id}
            onDrag={onDrag}
            glyphOf={glyphOf}
          />
        ))}
        <DropdownMenu
          triggerLabel="Add tab"
          icon={Plus}
          portal
          entries={
            add === undefined ? addEntries(family, thread, built, untabbed) : add.entries(pane)
          }
          onOpenChange={(open) => open && add === undefined && listUntabbed()}
          onSelect={(id) => {
            if (add !== undefined) return add.onSelect(id, pane);
            if (id.startsWith("terminal:")) {
              const terminal = id.slice("terminal:".length);
              const named = untabbed.find((one) => one.terminal === terminal);
              return openTerminalTab(family, jobId, pane.id, terminal, named?.name ?? terminal);
            }
            const mod = id.startsWith("mod:") ? id.slice("mod:".length) : undefined;
            const kind = (mod === undefined ? id : "custom") as TabKind;
            // A terminal here is a new shell, whose ID Fleet gives; its tab opens when Fleet has answered.
            if (kind === "terminal") return newTerminal(family, jobId, pane.id);
            rearrange(jobId, (arrangement) =>
              kind === "thread" && hasKind(arrangement, "thread")
                ? arrangement
                : addTab(
                    arrangement,
                    pane.id,
                    kind,
                    mod === undefined ? labelFor(jobId, kind) : tabTitle(mod),
                  ),
            );
            // The tab is the pane's front tab now; a built mod's is drawn as built.
            const front = panesOf(peek(jobId).root).find((one) => one.id === pane.id)?.active;
            if (mod !== undefined && front !== undefined) seedBuilt(front, mod);
            focusIn(pane.id);
          }}
        />
        {!zoomed ? null : (
          <Tooltip label="Restore" shortcut={boundKeyFor("pane_zoom")}>
            <Button
              variant="ghost"
              size="sm"
              iconOnly
              aria-label="Restore"
              aria-pressed
              onClick={() => toggleZoom(jobId, pane.id)}
            >
              <Minimize2 size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        )}
        {pane.tabs.length > 0 ? null : (
          <Tooltip label="Close pane">
            <Button
              variant="ghost"
              size="sm"
              iconOnly
              aria-label="Close pane"
              onClick={() => rearrange(jobId, (arrangement) => closePane(arrangement, pane.id))}
            >
              <X size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        )}
      </div>
      <div
        className="job-pane__body"
        data-kind={active?.kind}
        role="tabpanel"
        aria-label={active === undefined ? "Empty" : labelOf(active)}
      >
        {children}
        {dragging === undefined ? null : (
          <div className="job-pane__drops" aria-hidden="true">
            {ZONES.map((zone) => (
              <div
                key={zone}
                className="job-pane__drop"
                data-zone={zone}
                onDragOver={(event) => {
                  setOver(zone);
                  if (droppable(jobId, dragging, pane.id, zone)) event.preventDefault();
                  else event.dataTransfer.dropEffect = "none";
                }}
                onDragLeave={() => setOver((now) => (now === zone ? undefined : now))}
                onDrop={(event) => dropOnBody(event, zone)}
              />
            ))}
            {lands ? <div className="job-pane__preview" data-zone={over} /> : null}
          </div>
        )}
      </div>
    </section>
  );
}
