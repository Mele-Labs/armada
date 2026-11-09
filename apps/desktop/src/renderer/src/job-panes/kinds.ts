// What a tab can be: its name, and the glyph it wears. The glyphs are first choices, kept in one place so the registry's can replace them.

import {
  Activity,
  Bot,
  Clock,
  ListOrdered,
  ScrollText,
  SlidersVertical,
  Circle,
  CircleCheck,
  ClipboardList,
  CircleDashed,
  Cpu,
  Diff,
  Eye,
  FileDiff,
  FileText,
  Files,
  FolderTree,
  GitCommitHorizontal,
  GitPullRequest,
  Globe,
  LayoutGrid,
  Link,
  ListChecks,
  MessageSquare,
  MessageSquareText,
  MessagesSquare,
  NotebookPen,
  PanelTop,
  Scale,
  Send,
  SquarePen,
  SquareTerminal,
  Ticket,
  Waypoints,
  Workflow,
  type LucideIcon,
} from "lucide-react";

import { glyphOf } from "../cockpit/deck/flight-glyphs";
import type { TabKind } from "./tree";

export const KINDS: Readonly<
  Record<
    TabKind,
    {
      label: string;
      Glyph: LucideIcon;
    }
  >
> = {
  thread: {
    label: "Thread",
    Glyph: MessageSquareText,
  },
  browser: {
    label: "Browser",
    Glyph: Globe,
  },
  terminal: {
    label: "Terminal",
    Glyph: SquareTerminal,
  },
  done: {
    label: "Done",
    Glyph: CircleCheck,
  },
  now: {
    label: "Now",
    Glyph: Activity,
  },
  open: {
    label: "Open",
    Glyph: Circle,
  },
  deferred: {
    label: "Deferred",
    Glyph: CircleDashed,
  },
  changes: {
    label: "Changes",
    Glyph: Diff,
  },
  plan: {
    label: "Plan",
    Glyph: ListChecks,
  },
  pulls: {
    label: "Pull Requests",
    Glyph: GitPullRequest,
  },
  issues: {
    label: "Issues",
    Glyph: Ticket,
  },
  jobs: {
    label: "Jobs",
    Glyph: Link,
  },
  ledger: {
    label: "Ledger",
    Glyph: Files,
  },
  files: {
    label: "Files",
    Glyph: FolderTree,
  },
  request: {
    label: "Request",
    Glyph: Send,
  },
  subagents: {
    label: "Subagents",
    Glyph: glyphOf("drone", "zap"),
  },
  checks: {
    label: "Checks",
    Glyph: glyphOf("shield-check", "zap"),
  },
  workflow: {
    label: "Workflow",
    Glyph: Workflow,
  },
  system: {
    label: "System",
    Glyph: Cpu,
  },
  bays: {
    label: "Bays",
    Glyph: LayoutGrid,
  },
  spine: {
    label: "Spine",
    Glyph: Waypoints,
  },
  judge: {
    label: "Judge",
    Glyph: Scale,
  },
  custom: {
    label: "Custom",
    Glyph: PanelTop,
  },
  draft: {
    label: "Draft",
    Glyph: NotebookPen,
  },
  walk: {
    label: "Walk",
    Glyph: Eye,
  },
  document: {
    label: "Document",
    Glyph: FileText,
  },
  "pr-overview": {
    label: "Overview",
    Glyph: FileText,
  },
  "pr-files": {
    label: "Files",
    Glyph: FileDiff,
  },
  "pr-checks": {
    label: "Checks",
    Glyph: glyphOf("shield-check", "zap"),
  },
  "pr-commits": {
    label: "Commits",
    Glyph: GitCommitHorizontal,
  },
  "pr-review": {
    label: "Review",
    Glyph: SquarePen,
  },
  "pr-tool": {
    label: "Tool",
    Glyph: ClipboardList,
  },
  "pr-comments": {
    label: "Comments",
    Glyph: MessageSquare,
  },
  "pr-ask": {
    label: "Ask",
    Glyph: MessagesSquare,
  },
  "op-talk": {
    label: "Discussion",
    Glyph: Bot,
  },
  "op-queue": {
    label: "Queue",
    Glyph: ListOrdered,
  },
  "op-log": {
    label: "Log",
    Glyph: ScrollText,
  },
  "op-waiting": {
    label: "Waiting to start",
    Glyph: Clock,
  },
  "op-permissions": {
    label: "Permissions",
    Glyph: SlidersVertical,
  },
};
