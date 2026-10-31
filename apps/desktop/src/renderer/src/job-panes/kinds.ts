// What a tab can be: its name, and the glyph it wears. The glyphs are first choices, kept in one place so the registry's can replace them.

import { Activity, Circle, CircleCheck, Cpu, Diff, Files, Globe, LayoutGrid, ListChecks, MessageSquareText, Send, SquareTerminal, Waypoints, type LucideIcon } from "lucide-react";

import type { TabKind } from "./tree";

export const KINDS: Readonly<Record<TabKind, { label: string; Glyph: LucideIcon }>> = {
  thread: { label: "Thread", Glyph: MessageSquareText },
  browser: { label: "Browser", Glyph: Globe },
  terminal: { label: "Terminal", Glyph: SquareTerminal },
  done: { label: "Done", Glyph: CircleCheck },
  now: { label: "Now", Glyph: Activity },
  open: { label: "Open", Glyph: Circle },
  diff: { label: "Diff", Glyph: Diff },
  plan: { label: "Plan", Glyph: ListChecks },
  files: { label: "Files", Glyph: Files },
  request: { label: "Request", Glyph: Send },
  system: { label: "System", Glyph: Cpu },
  bays: { label: "Bays", Glyph: LayoutGrid },
  spine: { label: "Spine", Glyph: Waypoints },
};
