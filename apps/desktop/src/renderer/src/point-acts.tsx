// Mod and Report on every context menu the app opens at a point, whoever owns it: the menu adds them after its own entries (`PointActs`).
// The element is the last one right-pressed, noted in the capture phase before an app handler prevents the event; a menu opened by key falls back to the last focused one.

import { useEffect, useMemo, useRef, type ReactNode } from "react";
import { Flag, Puzzle } from "lucide-react";
import { PointActs, type DropdownMenuEntry } from "@armada/components";

import { inLayer, targetOf } from "./annotate/target";
import { startModJob } from "./mod-job";
import { askToReport, canReport } from "./report";

const MOD = "point-mod";
const REPORT = "point-report";

const inMenu = (target: EventTarget | null): boolean => target instanceof Element && target.closest('[role="menu"]') !== null;

export function PointActsProvided({ children }: { children: ReactNode }) {
  const pressed = useRef<Element | null>(null);
  const focused = useRef<Element | null>(null);
  useEffect(() => {
    // Any press or key outside a menu forgets the last right press, so a menu opened by key is not read as that one.
    const forget = (event: Event): void => {
      if (!inMenu(event.target)) pressed.current = null;
    };
    const noted = (event: MouseEvent): void => {
      pressed.current = inLayer(event.target) ? null : targetOf(event.target);
    };
    const focus = (event: FocusEvent): void => {
      if (!inMenu(event.target) && !inLayer(event.target)) focused.current = targetOf(event.target);
    };
    window.addEventListener("pointerdown", forget, true);
    window.addEventListener("keydown", forget, true);
    window.addEventListener("contextmenu", noted, true);
    window.addEventListener("focusin", focus, true);
    return () => {
      window.removeEventListener("pointerdown", forget, true);
      window.removeEventListener("keydown", forget, true);
      window.removeEventListener("contextmenu", noted, true);
      window.removeEventListener("focusin", focus, true);
    };
  }, []);
  const value = useMemo(() => {
    const entries: DropdownMenuEntry[] = [
      { kind: "item", id: MOD, label: "Mod", icon: Puzzle },
      ...(canReport() ? [{ kind: "item" as const, id: REPORT, label: "Report", icon: Flag }] : []),
    ];
    const onSelect = (id: string): void => {
      const element = [pressed.current, focused.current].find((one) => one?.isConnected === true);
      if (element === undefined || element === null) return;
      if (id === MOD) void startModJob(element);
      if (id === REPORT) askToReport(element);
    };
    return { entries, onSelect };
  }, []);
  return <PointActs.Provider value={value}>{children}</PointActs.Provider>;
}
