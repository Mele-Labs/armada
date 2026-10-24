// A Session row's context menu: Pause (Resume while paused), Auto Pilot, Mod, Report, Teach, Close. It opens at the pointer on a right press, and beside the
// row on the ContextMenu key or Shift+F10; the row carries no icon for it. Mounted once by the list.

import { CirclePause, Flag, GraduationCap, Play, Plane, Puzzle, X } from "lucide-react";
import { DropdownMenu, type DropdownMenuEntry } from "@armada/components";

import { startModJob } from "../../mod-job";
import { askToReport, canReport } from "../../report";
import { useSessions, useSessionsDraft } from "../../sessions-draft";
import { useTeachBridge } from "../../teach";
import { PAUSE_KEY } from "./session-tiles";

export type RowMenuAt = { id: string; root: string; title: string; x: number; y: number; row: HTMLElement | null };

export function RowMenu({ at, onClose, onAutopilot }: { at: RowMenuAt | undefined; onClose: () => void; onAutopilot: (root: string) => void }) {
  const draft = useSessionsDraft();
  const teach = useTeachBridge();
  const paused = useSessions().find((one) => one.id === at?.id)?.paused === true;
  if (at === undefined) return null;
  const entries: DropdownMenuEntry[] = [
    ...(draft?.pause === undefined ? [] : [{ kind: "item" as const, id: "pause", label: paused ? "Resume" : "Pause", shortcut: PAUSE_KEY, icon: paused ? Play : CirclePause }]),
    { kind: "item", id: "autopilot", label: "Auto Pilot", icon: Plane },
    ...(at.row !== null ? [{ kind: "item" as const, id: "mod", label: "Mod", icon: Puzzle }] : []),
    ...(canReport() && at.row !== null ? [{ kind: "item" as const, id: "report", label: "Report", icon: Flag }] : []),
    ...(teach === undefined ? [] : [{ kind: "item" as const, id: "teach", label: "Teach", icon: GraduationCap }]),
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
        if (chosen === "mod" && at.row !== null) void startModJob(at.row, at.title);
        if (chosen === "report" && at.row !== null) askToReport(at.row);
        if (chosen === "teach") teach?.("row", at.title);
        if (chosen === "close") draft?.close?.(at.id);
      }}
    />
  );
}
