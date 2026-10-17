// A Session row's context menu: Pause (Resume while paused), Auto Pilot, Close. It opens at the pointer on a right press, and beside the
// row on the ContextMenu key or Shift+F10; the row carries no icon for it. Mounted once by the list.

import { CirclePause, Play, Plane, X } from "lucide-react";
import { DropdownMenu, type DropdownMenuEntry } from "@armada/components";

import { useSessions, useSessionsDraft } from "../../sessions-draft";
import { PAUSE_KEY } from "./session-tiles";

export type RowMenuAt = { id: string; root: string; x: number; y: number; row: HTMLElement | null };

export function RowMenu({ at, onClose, onAutopilot }: { at: RowMenuAt | undefined; onClose: () => void; onAutopilot: (root: string) => void }) {
  const draft = useSessionsDraft();
  const paused = useSessions().find((one) => one.id === at?.id)?.paused === true;
  if (at === undefined) return null;
  const entries: DropdownMenuEntry[] = [
    ...(draft?.pause === undefined ? [] : [{ kind: "item" as const, id: "pause", label: paused ? "Resume" : "Pause", shortcut: PAUSE_KEY, icon: paused ? Play : CirclePause }]),
    { kind: "item", id: "autopilot", label: "Auto Pilot", icon: Plane },
    ...(draft?.close === undefined ? [] : [{ kind: "separator" as const, id: "rule" }, { kind: "item" as const, id: "close", label: "Close", danger: true, icon: X }]),
  ];
  return (
    <DropdownMenu
      triggerLabel="Job"
      at={{ x: at.x, y: at.y }}
      open
      entries={entries}
      onOpenChange={(open) => {
        if (open) return;
        onClose();
        at.row?.focus();
      }}
      onSelect={(chosen) => {
        if (chosen === "pause") void (paused ? draft?.resume : draft?.pause)?.(at.id);
        if (chosen === "autopilot") onAutopilot(at.root);
        if (chosen === "close") draft?.close?.(at.id);
      }}
    />
  );
}
