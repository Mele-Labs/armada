// What a tab can be: its name, and the glyph it wears. The glyphs are first choices, kept in one place so the registry's can replace them.

import { Activity, Circle, CircleCheck, Cpu, Diff, Files, FolderTree, GitPullRequest, Globe, LayoutGrid, Link, ListChecks, MessageSquareText, PanelTop, Send, SquareTerminal, Ticket, Waypoints, Workflow, type LucideIcon } from "lucide-react";

import { glyphOf } from "../cockpit/deck/flight-glyphs";
import type { TabKind } from "./tree";

export const KINDS: Readonly<Record<TabKind, { label: string; Glyph: LucideIcon }>> = {
  thread: { label: "Thread", Glyph: MessageSquareText },
  browser: { label: "Browser", Glyph: Globe },
  terminal: { label: "Terminal", Glyph: SquareTerminal },
  done: { label: "Done", Glyph: CircleCheck },
  now: { label: "Now", Glyph: Activity },
  open: { label: "Open", Glyph: Circle },
  changes: { label: "Changes", Glyph: Diff },
  plan: { label: "Plan", Glyph: ListChecks },
  pulls: { label: "Pull Requests", Glyph: GitPullRequest },
  issues: { label: "Issues", Glyph: Ticket },
  jobs: { label: "Jobs", Glyph: Link },
  ledger: { label: "Ledger", Glyph: Files },
  files: { label: "Files", Glyph: FolderTree },
  request: { label: "Request", Glyph: Send },
  subagents: { label: "Subagents", Glyph: glyphOf("drone", "zap") },
  checks: { label: "Checks", Glyph: glyphOf("shield-check", "zap") },
  workflow: { label: "Workflow", Glyph: Workflow },
  system: { label: "System", Glyph: Cpu },
  bays: { label: "Bays", Glyph: LayoutGrid },
  spine: { label: "Spine", Glyph: Waypoints },
  custom: { label: "Custom", Glyph: PanelTop },
};
