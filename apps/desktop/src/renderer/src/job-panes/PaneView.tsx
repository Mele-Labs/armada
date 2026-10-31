// One pane: a strip of its tabs with a + that adds one, over the body of the tab in front. A tab is dragged to another pane's strip to move there, or to a pane's edge to split it.

import { useEffect, useRef, useState, type DragEvent, type KeyboardEvent, type ReactNode } from "react";
import { LoaderCircle, Minimize2, Plus, X } from "lucide-react";
import { Button, DropdownMenu, Tooltip, boundKeyFor, seedDraft, type DropdownMenuEntry } from "@armada/components";

import { hostOf, peekJobBrowser } from "../job-browser/store";
import { useModList } from "../mod-list";
import { useSessions } from "../sessions-draft";
import { changeWords } from "../shared-panel/mod";
import { customLabel, seedBuilt, setEditing, tabTitle, useCustom, useCustomTick } from "./custom";
import { splitFocused, focusIn } from "./keys";
import { KINDS } from "./kinds";
import { droppable, look, peek, rearrange } from "./store";
import { moveToWindow, whyNotWindow } from "./windows";
import { toggleZoom } from "./zoom";
import { TAB_KINDS, activate, addTab, closePane, closeTab, focusPane, hasKind, moveTabToEdge, moveTabToPane, panesOf, type Pane, type Side, type Tab, type TabKind } from "./tree";

/** The words a tab reads: its own label where it has one, else its kind. */
/** A Custom tab reads the name of the mod it built, once it has one. */
export const labelOf = (tab: Tab): string => (tab.kind === "custom" ? customLabel(tab.id) : undefined) ?? tab.label ?? KINDS[tab.kind].label;

/** What a new tab of a kind is called: a browser by its page's host, a terminal by its shell. The kinds there is one of keep their names. The mock's shell is zsh. */
function labelFor(jobId: string, kind: TabKind): string | undefined {
  if (kind === "terminal") return "zsh";
  const url = kind === "browser" ? peekJobBrowser(jobId)?.url : undefined;
  return url === undefined || url === "" ? undefined : hostOf(url);
}

const ZONES: readonly (Side | "middle")[] = ["left", "right", "top", "bottom", "middle"];

/** The tab's own menu. Move to window is there for the kinds that have a window, and says why for the rest. */
function tabEntries(kind: TabKind, zoomed: boolean, editable: boolean): DropdownMenuEntry[] {
  const why = whyNotWindow(kind);
  return [
    { kind: "item", id: "window", label: "Move to window", ...(why === undefined ? {} : { disabled: true, hint: why }) },
    { kind: "item", id: "zoom", label: zoomed ? "Restore" : "Zoom", shortcut: boundKeyFor("pane_zoom") },
    { kind: "item", id: "right", label: "Split right", shortcut: boundKeyFor("pane_split_right") },
    { kind: "item", id: "down", label: "Split down", shortcut: boundKeyFor("pane_split_down") },
    ...(editable ? [{ kind: "item", id: "edit", label: "Edit with a Job" } as const] : []),
    { kind: "separator", id: "rule" },
    { kind: "item", id: "close", label: "Close", danger: true },
  ];
}

function TabButton({ jobId, pane, tab, zoomed, dragging, onDrag }: { jobId: string; pane: Pane; tab: Tab; zoomed: boolean; dragging: boolean; onDrag: (tabId: string | undefined) => void }) {
  const label = labelOf(tab);
  const { Glyph } = KINDS[tab.kind];
  const active = pane.active === tab.id;
  const custom = useCustom(tab.id, tab.kind === "custom");
  const building = tab.kind === "custom" && custom.stage === "building";
  const modJob = useSessions().find((one) => one.id === custom.job);
  const [point, setPoint] = useState<{ x: number; y: number } | null>(null);
  /** Closed by its own ×, or a middle click: the tab does not come to the front first. */
  const close = () => {
    rearrange(jobId, (arrangement) => closeTab(arrangement, tab.id));
    focusIn(peek(jobId).focus);
  };
  // A press that starts on the × is not the start of a drag of the tab.
  const grabbed = useRef(false);
  const choose = (id: string) => {
    look(jobId, (arrangement) => focusPane(activate(arrangement, tab.id), pane.id));
    if (id === "window") moveToWindow(jobId, tab);
    else if (id === "zoom") toggleZoom(jobId, pane.id);
    else if (id === "edit") {
      seedDraft(`custom:${tab.id}`, changeWords(label, "tab"));
      setEditing(tab.id, true);
    }
    else if (id === "right" || id === "down") splitFocused(jobId, id);
    else if (id === "close") {
      rearrange(jobId, (arrangement) => closeTab(arrangement, tab.id));
      focusIn(peek(jobId).focus);
    }
  };
  return (
    <div
      className="job-pane__tab"
      data-active={active || undefined}
      data-dragging={dragging || undefined}
      draggable
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
        setPoint({ x: event.clientX, y: event.clientY });
      }}
    >
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
          setPoint({ x: box.left, y: box.bottom });
        }}
      >
        {building ? (
          <Tooltip label={modJob?.title ?? "Mod Job"} asChild>
            <span className="job-pane__working" role="img" aria-label={modJob?.title ?? "Mod Job"}>
              <LoaderCircle size={16} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        ) : (
          <Glyph size={16} strokeWidth={2} aria-hidden />
        )}
        <span>{label}</span>
      </button>
      <span className="job-pane__close">
        <Tooltip label="Close">
          <Button variant="ghost" size="sm" iconOnly aria-label={`Close ${label}`} draggable={false} onClick={close}>
            <X size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      </span>
      <DropdownMenu triggerLabel="Tab actions" at={point ?? { x: 0, y: 0 }} open={point !== null} onOpenChange={(open) => !open && setPoint(null)} portal entries={tabEntries(tab.kind, zoomed, tab.kind === "custom" && custom.stage === "built")} onSelect={choose} />
    </div>
  );
}

/** The kinds the + offers. A Job has one thread, so it is offered only when none is open. */
function addEntries(thread: boolean, built: readonly string[]): DropdownMenuEntry[] {
  const kinds = TAB_KINDS.filter((kind) => kind !== "custom" && (kind !== "thread" || !thread)).map((kind): DropdownMenuEntry => ({ kind: "item", id: kind, label: KINDS[kind].label, icon: KINDS[kind].Glyph }));
  // Under the rule, the tabs a Mod Job built by name, then Custom, which builds another.
  const { Glyph } = KINDS.custom;
  return [...kinds, { kind: "separator", id: "custom-rule" }, ...built.map((name): DropdownMenuEntry => ({ kind: "item", id: `mod:${name}`, label: tabTitle(name), icon: Glyph })), { kind: "item", id: "custom", label: KINDS.custom.label, icon: Glyph }];
}

export function PaneView({ jobId, pane, focused, zoomed, thread, dragging, onDrag, children }: { jobId: string; pane: Pane; focused: boolean; zoomed: boolean; thread: boolean; dragging: string | undefined; onDrag: (tabId: string | undefined) => void; children: ReactNode }) {
  const [over, setOver] = useState<Side | "middle" | undefined>(undefined);
  // A Custom tab turning over changes the words and glyph of its tab.
  useCustomTick();
  const built = useModList().filter((one) => one.kind === "tab" && one.valid && one.enabled).map((one) => one.name);
  // The pane under the pointer while a tab is dragged, lit as a whole.
  const [hot, setHot] = useState(false);
  useEffect(() => {
    if (dragging !== undefined) return;
    setOver(undefined);
    setHot(false);
  }, [dragging]);
  const lands = dragging !== undefined && over !== undefined && droppable(jobId, dragging, pane.id, over);
  const active = pane.tabs.find((tab) => tab.id === pane.active);

  // ← and → walk the strip, Home and End go to its ends; the tab reached comes to the front.
  const walk = (event: KeyboardEvent<HTMLDivElement>) => {
    const keys = ["ArrowLeft", "ArrowRight", "Home", "End"];
    if (!keys.includes(event.key) || (event.target as HTMLElement).getAttribute("role") !== "tab") return;
    const at = pane.tabs.findIndex((tab) => tab.id === pane.active);
    const to = event.key === "Home" ? 0 : event.key === "End" ? pane.tabs.length - 1 : Math.min(pane.tabs.length - 1, Math.max(0, at + (event.key === "ArrowRight" ? 1 : -1)));
    event.preventDefault();
    const strip = event.currentTarget;
    look(jobId, (arrangement) => activate(arrangement, pane.tabs[to]!.id));
    window.requestAnimationFrame(() => strip.querySelectorAll<HTMLElement>('[role="tab"]')[to]?.focus());
  };

  const dropOnStrip = (event: DragEvent<HTMLDivElement>) => {
    const id = event.dataTransfer.getData("text/plain");
    if (id === "") return;
    if (!droppable(jobId, id, pane.id, "middle") && !pane.tabs.some((tab) => tab.id === id)) return;
    event.preventDefault();
    const tabs = [...event.currentTarget.querySelectorAll<HTMLElement>(".job-pane__tab")];
    const index = tabs.filter((one) => one.getBoundingClientRect().left + one.getBoundingClientRect().width / 2 < event.clientX).length;
    // Counted among the others, since the tab leaves its place first.
    const moving = pane.tabs.findIndex((tab) => tab.id === id);
    rearrange(jobId, (arrangement) => moveTabToPane(arrangement, id, pane.id, moving !== -1 && moving < index ? index - 1 : index));
    onDrag(undefined);
  };

  const dropOnBody = (event: DragEvent<HTMLElement>, zone: Side | "middle") => {
    const id = event.dataTransfer.getData("text/plain");
    event.preventDefault();
    setOver(undefined);
    setHot(false);
    onDrag(undefined);
    if (id === "") return;
    rearrange(jobId, (arrangement) => (zone === "middle" ? moveTabToPane(arrangement, id, pane.id) : moveTabToEdge(arrangement, id, pane.id, zone)));
  };

  return (
    <section
      className="deck-pane job-pane"
      data-frame=""
      data-pane={pane.id}
      data-focused={focused || undefined}
      data-hot={(hot && dragging !== undefined) || undefined}
      aria-label={active === undefined ? "Empty pane" : `Pane: ${labelOf(active)}`}
      onFocusCapture={() => look(jobId, (arrangement) => focusPane(arrangement, pane.id))}
      onPointerDownCapture={() => look(jobId, (arrangement) => focusPane(arrangement, pane.id))}
      onDragOver={() => dragging !== undefined && setHot(true)}
      onDragLeave={(event) => !event.currentTarget.contains(event.relatedTarget as Node | null) && setHot(false)}
    >
      <div
        className="job-pane__strip"
        role="tablist"
        aria-label="Tabs"
        onKeyDown={walk}
        onDoubleClick={(event) => {
          // A tab or the strip's empty space zooms the pane; the ×, the + and the menus keep their own presses.
          if ((event.target as Element).closest(".job-pane__close, .armada-dropdown-menu, button:not([role=\"tab\"])") !== null) return;
          toggleZoom(jobId, pane.id);
        }} onDragOver={(event) => event.preventDefault()} onDrop={dropOnStrip}>
        {pane.tabs.map((tab) => (
          <TabButton key={tab.id} jobId={jobId} pane={pane} tab={tab} zoomed={zoomed} dragging={dragging === tab.id} onDrag={onDrag} />
        ))}
        <DropdownMenu
          triggerLabel="Add tab"
          icon={Plus}
          portal
          entries={addEntries(thread, built)}
          onSelect={(id) => {
            const mod = id.startsWith("mod:") ? id.slice("mod:".length) : undefined;
            const kind = (mod === undefined ? id : "custom") as TabKind;
            rearrange(jobId, (arrangement) => (kind === "thread" && hasKind(arrangement, "thread") ? arrangement : addTab(arrangement, pane.id, kind, mod === undefined ? labelFor(jobId, kind) : tabTitle(mod))));
            // The tab is the pane's front tab now; a built mod's is drawn as built.
            const front = panesOf(peek(jobId).root).find((one) => one.id === pane.id)?.active;
            if (mod !== undefined && front !== undefined) seedBuilt(front, mod);
            focusIn(pane.id);
          }}
        />
        {!zoomed ? null : (
          <Tooltip label="Restore" shortcut={boundKeyFor("pane_zoom")}>
            <Button variant="ghost" size="sm" iconOnly aria-label="Restore" aria-pressed onClick={() => toggleZoom(jobId, pane.id)}>
              <Minimize2 size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        )}
        {pane.tabs.length > 0 ? null : (
          <Tooltip label="Close pane">
            <Button variant="ghost" size="sm" iconOnly aria-label="Close pane" onClick={() => rearrange(jobId, (arrangement) => closePane(arrangement, pane.id))}>
              <X size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        )}
      </div>
      <div className="job-pane__body" role="tabpanel" aria-label={active === undefined ? "Empty" : labelOf(active)}>
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
