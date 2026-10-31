// One pane: a strip of its tabs with a + that adds one, over the body of the tab in front. A tab is dragged to another pane's strip to move there, or to a pane's edge to split it.

import { useEffect, useState, type DragEvent, type KeyboardEvent, type ReactNode } from "react";
import { ChevronDown, Plus, X } from "lucide-react";
import { Button, DropdownMenu, Tooltip, boundKeyFor, type DropdownMenuEntry } from "@armada/components";

import { hostOf, peekJobBrowser } from "../job-browser/store";
import { splitFocused, focusIn } from "./keys";
import { KINDS } from "./kinds";
import { droppable, look, peek, rearrange } from "./store";
import { TAB_KINDS, activate, addTab, closePane, closeTab, focusPane, hasKind, moveTabToEdge, moveTabToPane, type Pane, type Side, type Tab, type TabKind } from "./tree";

/** The words a tab reads: its own label where it has one, else its kind. */
export const labelOf = (tab: Tab): string => tab.label ?? KINDS[tab.kind].label;

/** What a new tab of a kind is called: a browser by its page's host, a terminal by its shell. The kinds there is one of keep their names. The mock's shell is zsh. */
function labelFor(jobId: string, kind: TabKind): string | undefined {
  if (kind === "terminal") return "zsh";
  const url = kind === "browser" ? peekJobBrowser(jobId)?.url : undefined;
  return url === undefined || url === "" ? undefined : hostOf(url);
}

const ZONES: readonly (Side | "middle")[] = ["left", "right", "top", "bottom", "middle"];

/** The tab's own menu. Move to window is here and does nothing until a pane can leave the window. */
function tabEntries(): DropdownMenuEntry[] {
  return [
    { kind: "item", id: "window", label: "Move to window" },
    { kind: "item", id: "right", label: "Split right", shortcut: boundKeyFor("pane_split_right") },
    { kind: "item", id: "down", label: "Split down", shortcut: boundKeyFor("pane_split_down") },
    { kind: "separator", id: "rule" },
    { kind: "item", id: "close", label: "Close", danger: true },
  ];
}

function TabButton({ jobId, pane, tab, dragging, onDrag }: { jobId: string; pane: Pane; tab: Tab; dragging: boolean; onDrag: (tabId: string | undefined) => void }) {
  const label = labelOf(tab);
  const { Glyph } = KINDS[tab.kind];
  const active = pane.active === tab.id;
  const [point, setPoint] = useState<{ x: number; y: number } | null>(null);
  const choose = (id: string) => {
    look(jobId, (arrangement) => focusPane(activate(arrangement, tab.id), pane.id));
    if (id === "right" || id === "down") splitFocused(jobId, id);
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
      onDragStart={(event: DragEvent) => {
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
      <button type="button" role="tab" className="job-pane__label" aria-selected={active} tabIndex={active ? 0 : -1} onClick={() => look(jobId, (arrangement) => activate(arrangement, tab.id))}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
        <span>{label}</span>
      </button>
      {active ? <DropdownMenu triggerLabel="Tab menu" icon={ChevronDown} portal entries={tabEntries()} onSelect={choose} /> : null}
      <DropdownMenu triggerLabel="Tab actions" at={point ?? { x: 0, y: 0 }} open={point !== null} onOpenChange={(open) => !open && setPoint(null)} portal entries={tabEntries()} onSelect={choose} />
    </div>
  );
}

/** The kinds the + offers. A Job has one thread, so it is offered only when none is open. */
function addEntries(thread: boolean): DropdownMenuEntry[] {
  return TAB_KINDS.filter((kind) => kind !== "thread" || !thread).map((kind): DropdownMenuEntry => ({ kind: "item", id: kind, label: KINDS[kind].label, icon: KINDS[kind].Glyph }));
}

export function PaneView({ jobId, pane, focused, thread, dragging, onDrag, children }: { jobId: string; pane: Pane; focused: boolean; thread: boolean; dragging: string | undefined; onDrag: (tabId: string | undefined) => void; children: ReactNode }) {
  const [over, setOver] = useState<Side | "middle" | undefined>(undefined);
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
      <div className="job-pane__strip" role="tablist" aria-label="Tabs" onKeyDown={walk} onDragOver={(event) => event.preventDefault()} onDrop={dropOnStrip}>
        {pane.tabs.map((tab) => (
          <TabButton key={tab.id} jobId={jobId} pane={pane} tab={tab} dragging={dragging === tab.id} onDrag={onDrag} />
        ))}
        <DropdownMenu
          triggerLabel="Add tab"
          icon={Plus}
          portal
          entries={addEntries(thread)}
          onSelect={(id) => {
            const kind = id as TabKind;
            rearrange(jobId, (arrangement) => (kind === "thread" && hasKind(arrangement, "thread") ? arrangement : addTab(arrangement, pane.id, kind, labelFor(jobId, kind))));
            focusIn(pane.id);
          }}
        />
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
