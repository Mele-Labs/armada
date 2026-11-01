// The menu a right press opens where the app has none of its own: Mod, and Report where the annotation layer is mounted.
// An app menu gets the same two from `point-acts.tsx`; this one lists them itself, so it clears that context.
// It opens at the pointer, and beside the element on the ContextMenu key or Shift+F10, which send the same event.

import { useEffect, useState } from "react";
import { Flag, Puzzle } from "lucide-react";
import { DropdownMenu, PointActs, type DropdownMenuEntry } from "@armada/components";

import { inLayer, targetOf } from "./annotate/target";
import { startModJob } from "./mod-job";
import { askToReport, canReport } from "./report";

export function PointMenu() {
  const [at, setAt] = useState<{ x: number; y: number; element: Element } | null>(null);
  useEffect(() => {
    function pressed(event: MouseEvent): void {
      // A press the app handled has its own menu, which gets Mod and Report from `PointActs`.
      if (event.defaultPrevented || inLayer(event.target)) return;
      const element = targetOf(event.target);
      if (element === null) return;
      event.preventDefault();
      setAt({ x: event.clientX, y: event.clientY, element });
    }
    window.addEventListener("contextmenu", pressed);
    return () => window.removeEventListener("contextmenu", pressed);
  }, []);
  if (at === null) return null;
  const entries: DropdownMenuEntry[] = [
    { kind: "item", id: "mod", label: "Mod", icon: Puzzle },
    ...(canReport() ? [{ kind: "item" as const, id: "report", label: "Report", icon: Flag }] : []),
  ];
  return (
    <PointActs.Provider value={null}>
    <DropdownMenu
      triggerLabel="Element"
      at={{ x: at.x, y: at.y }}
      open
      entries={entries}
      onOpenChange={(open) => {
        if (!open) setAt(null);
      }}
      onSelect={(chosen) => {
        if (chosen === "mod") void startModJob(at.element);
        if (chosen === "report") askToReport(at.element);
      }}
    />
    </PointActs.Provider>
  );
}
