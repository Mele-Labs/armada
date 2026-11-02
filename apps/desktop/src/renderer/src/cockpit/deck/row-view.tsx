// What a Job's row draws, chosen by the person: a right press on a row or on the list's empty room opens a menu whose View group
// switches seven fields on and off, saved in settings.json under `bridge.rowView`. The default is what the row drew before: the Status and the Conflicts marks. Each field has a column of
// its own, in the menu's order, that is there for every row while it is on, so a row without the field leaves its place empty and nothing moves. Now is the exception: it is a second
// line, and rows grow by it; so is Active PRs, a line of its own under the name, or under Now where that is on.

import { useEffect, useSyncExternalStore, type ReactNode } from "react";
import { ListCardChip, ListCardMark, ListCardTime, Tooltip } from "@armada/components";
import type { SettingValue } from "@armada/protocol";
import type { DropdownMenuEntry } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { saveSetting, useSetting } from "../../use-setting";
import type { Subject } from "./ship";

export const VIEW_FIELDS = [
  { id: "pr", label: "Active PRs" },
  { id: "issue", label: "Issues" },
  { id: "now", label: "Now" },
  { id: "time", label: "Time" },
  { id: "conflicts", label: "Conflicts" },
  { id: "status", label: "Status" },
  { id: "workflow", label: "Workflow" },
] as const;

export type ViewField = (typeof VIEW_FIELDS)[number]["id"];
export type RowView = Readonly<Record<ViewField, boolean>>;

export const DEFAULT_VIEW: RowView = { pr: false, issue: false, now: false, time: false, conflicts: true, status: true, workflow: false };

const KEY = "bridge.rowView";
const listeners = new Set<() => void>();
let held: RowView = DEFAULT_VIEW;
/** What Fleet last said, which a save it refused puts back. */
let kept: RowView = DEFAULT_VIEW;

const parse = (value: SettingValue | undefined): RowView => {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return DEFAULT_VIEW;
  const got = value as Partial<Record<ViewField, unknown>>;
  return Object.fromEntries(VIEW_FIELDS.map(({ id }) => [id, typeof got[id] === "boolean" ? got[id] : DEFAULT_VIEW[id]])) as RowView;
};

const same = (a: RowView, b: RowView) => VIEW_FIELDS.every(({ id }) => a[id] === b[id]);

function hold(next: RowView): void {
  if (same(held, next)) return;
  held = next;
  for (const notify of listeners) notify();
}

const subscribe = (notify: () => void) => {
  listeners.add(notify);
  return () => void listeners.delete(notify);
};

/** The choice in settings.json under `bridge.rowView`, read as Fleet publishes it; the default until Fleet has answered. */
export function useRowView(): RowView {
  const saved = useSetting(KEY);
  useEffect(() => {
    kept = parse(saved);
    hold(kept);
  }, [saved]);
  return useSyncExternalStore(subscribe, () => held);
}

/** Drawn now and saved; a save Fleet refuses puts the choice back as Fleet has it. */
export function toggleField(field: ViewField): void {
  const next = { ...held, [field]: !held[field] };
  hold(next);
  void saveSetting(KEY, next).then((outcome) => outcome.ok || hold(kept));
}

const FIELD_OF = new Map<string, ViewField>(VIEW_FIELDS.map(({ id }) => [`view:${id}`, id]));

/** The pick a menu item makes, where it is one of the View group's. */
export const viewFieldOf = (id: string): ViewField | undefined => FIELD_OF.get(id);

/** The View group of the row menu: a heading and seven checkable items. */
export const viewEntries = (view: RowView): DropdownMenuEntry[] => [
  { kind: "label", id: "view" , label: "View" },
  ...VIEW_FIELDS.map(({ id, label }): DropdownMenuEntry => ({ kind: "item", id: `view:${id}`, label, selected: view[id] })),
];

/** Each place in the card's corner, its width in px and its order. The other fields are on the card's lines. The places are the same for every row, so none shifts. */
const COLUMNS: readonly { field: "conflicts" | "status" | "tools" | "reported"; width: number }[] = [
  { field: "conflicts", width: 16 },
  { field: "tools", width: 16 },
  { field: "reported", width: 16 },
  { field: "status", width: 36 },
];
/** The gap between columns, `--space-1`. */
const GAP = 4;
/** What the default's two columns take of the row, gaps with them. */
const DEFAULT_TAKES = 16 + 36 + GAP * 2;
/** How much of the title's room the fields may take before the list rests wider to give it back. */
const TITLE_GIVES = 96;

/** The tools column is not a View field: it is kept while a row in the list has the mark, as the conflicts column is for the earlier-Job mark. */
const columnsOn = (view: RowView, earlier: boolean, tools: boolean, reported: boolean) =>
  COLUMNS.filter(({ field }) => (field === "conflicts" ? view.conflicts || earlier : field === "tools" ? tools : field === "reported" ? reported : view[field]));

/**
 * The row's columns: the title, then each field that is on. The conflicts column is kept while a row in the list carries the earlier-Job mark, which stands in the same place.
 * `clashRight` is where that column's own mark sits from the row's trailing edge, since the mark is drawn over the row and not in it.
 */
export function columnsOf(view: RowView, earlier: boolean, tools = false, reported = false): { clashRight: string; toolsRight: string; reportedRight: string } {
  const on = columnsOn(view, earlier, tools, reported);
  const after = on.slice(on.findIndex(({ field }) => field === "conflicts") + 1);
  // The tools mark is a button, so it too is drawn over the row, in its column's place.
  const afterTools = on.slice(on.findIndex(({ field }) => field === "tools") + 1);
  // The reported-a-bug mark is a button too, drawn over its column the same way.
  const afterReported = on.slice(on.findIndex(({ field }) => field === "reported") + 1);
  return {
    reportedRight: `calc(var(--space-3)${afterReported.map(({ field, width }) => ` + var(--deck-w-${field}, ${width}px) + ${GAP}px`).join("")})`,
    toolsRight: `calc(var(--space-3)${afterTools.map(({ field, width }) => ` + var(--deck-w-${field}, ${width}px) + ${GAP}px`).join("")})`,
    clashRight: `calc(var(--space-3)${after.map(({ field, width }) => ` + var(--deck-w-${field}, ${width}px) + ${GAP}px`).join("")})`,
  };
}

/** How much wider than its resting width the list is drawn for the fields that are on, in px, so the title keeps most of what it had. A column the person dragged is theirs and is left as dragged. */
export function restBonus(view: RowView, earlier: boolean, tools = false): number {
  const on = columnsOn(view, earlier, tools, false);
  const takes = on.reduce((sum, { width }) => sum + width + GAP, 0);
  return Math.max(0, takes - DEFAULT_TAKES - TITLE_GIVES);
}

const issueOf = (session: Session | undefined) => session?.attachments.find((one) => one.kind === "issue");

/** The step the Job's workflow is at, or the workflow's name where Fleet gives no step. Absent where the Job follows no workflow. */
const workflowOf = (session: Session | undefined): { name: string; step: string } | undefined => {
  const flow = session?.work?.workflow;
  if (flow === undefined) return undefined;
  return { name: flow.name, step: (flow.at === undefined ? undefined : flow.steps[flow.at]) ?? flow.name };
};

/**
 * What a row draws on its card: the corner's marks, the tag line and the lines under, each from the fields that are on. `status` and `hit` are the row's own: the state marks it drew before,
 * and what a search matched in place of its age.
 */
export function rowParts({ view, earlier, tools, reported, one, session, hit, status }: { view: RowView; earlier: boolean; tools: boolean; reported: boolean; one: Subject; session: Session | undefined; hit: { short: string; whole: string } | undefined; status: ReactNode }): { marks?: ReactNode; tag?: ReactNode; at?: ReactNode; lines?: ReactNode } {
  const issue = issueOf(session);
  const issueText = issue === undefined ? undefined : /^\d+$/.test(issue.number) ? `#${issue.number}` : issue.number;
  const flow = workflowOf(session);
  const open = one.pr !== undefined && one.pr.stage !== "landed" ? one.pr : undefined;
  const shownIssue = view.issue && issue !== undefined && issueText !== undefined;
  const shownPr = view.pr && open !== undefined;
  const at =
    !view.time ? undefined
    : hit !== undefined && hit.short !== "" ? <Tooltip label={hit.whole}><span className="deck-row__said" data-found="">{hit.short}</span></Tooltip>
    : one.at === undefined ? undefined
    : <ListCardTime at={one.at} said="Last turn" />;
  const shownFlow = view.workflow && flow !== undefined;
  return {
    // The clash, tools, reported and earlier marks are buttons, drawn over their empty places from `Deck.tsx`.
    ...(view.conflicts || earlier || tools || reported || view.status
      ? {
          marks: (
            <>
              {view.conflicts || earlier ? <ListCardMark /> : null}
              {tools ? <ListCardMark /> : null}
              {reported ? <ListCardMark /> : null}
              {view.status ? <ListCardMark wide>{status}</ListCardMark> : null}
            </>
          ),
        }
      : {}),
    ...(shownIssue || shownPr
      ? {
          tag: (
            <>
              {shownIssue ? <Tooltip label={`Issue ${issueText}: ${issue!.title}`}><span><ListCardChip>{issueText}</ListCardChip></span></Tooltip> : null}
              {shownPr ? <Tooltip label={`Pull request #${open!.number}`}><span><ListCardChip>#{open!.number}</ListCardChip></span></Tooltip> : null}
            </>
          ),
        }
      : {}),
    ...(at === undefined ? {} : { at }),
    ...(view.now || shownFlow
      ? {
          lines: (
            <>
              {shownFlow ? <Tooltip label={`${flow!.name}: ${flow!.step}`}><span className="deck-row__flow">{flow!.step}</span></Tooltip> : null}
              {view.now ? <span className="deck-row__says">{one.peek.at(-1) ?? ""}</span> : null}
            </>
          ),
        }
      : {}),
  };
}
