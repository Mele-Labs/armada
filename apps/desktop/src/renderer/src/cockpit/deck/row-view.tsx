// What a Job's row draws, chosen by the person under `&density=tight-type` (`flight-take.ts`): a right press on a row or on the list's empty room opens a menu whose View group
// switches seven fields on and off, saved in `localStorage` under one key. The default is what the row drew before: the Status and the Conflicts marks. Each field has a column of
// its own, in the menu's order, that is there for every row while it is on, so a row without the field leaves its place empty and nothing moves. Now is the exception: it is a second
// line, and rows grow by it. A mock of 26 Oct 2026 for the owner's note on the walk of `tight-type`.

import { useSyncExternalStore, type ReactNode } from "react";
import { Stamp, Tooltip } from "@armada/components";
import type { DropdownMenuEntry } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

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

const KEY = "armada.deck.rowView";
const listeners = new Set<() => void>();
let seen: string | null | undefined;
let held: RowView = DEFAULT_VIEW;

const parse = (text: string | null): RowView => {
  if (text === null) return DEFAULT_VIEW;
  try {
    const got = JSON.parse(text) as Partial<Record<ViewField, unknown>>;
    return Object.fromEntries(VIEW_FIELDS.map(({ id }) => [id, typeof got[id] === "boolean" ? got[id] : DEFAULT_VIEW[id]])) as RowView;
  } catch {
    return DEFAULT_VIEW;
  }
};

/** The saved choice, the same object until the stored text changes. */
function read(): RowView {
  let text: string | null = null;
  try {
    text = window.localStorage.getItem(KEY);
  } catch {
    // Storage is not offered: the default stands.
  }
  if (text !== seen) {
    seen = text;
    held = parse(text);
  }
  return held;
}

const subscribe = (notify: () => void) => {
  listeners.add(notify);
  window.addEventListener("storage", notify);
  return () => {
    listeners.delete(notify);
    window.removeEventListener("storage", notify);
  };
};

export const useRowView = (): RowView => useSyncExternalStore(subscribe, read);

export function toggleField(field: ViewField): void {
  const next = { ...read(), [field]: !read()[field] };
  try {
    window.localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    // Not saved; it holds until the page is left.
  }
  for (const notify of listeners) notify();
}

const FIELD_OF = new Map<string, ViewField>(VIEW_FIELDS.map(({ id }) => [`view:${id}`, id]));

/** The pick a menu item makes, where it is one of the View group's. */
export const viewFieldOf = (id: string): ViewField | undefined => FIELD_OF.get(id);

/** The View group of the row menu: a heading and seven checkable items. */
export const viewEntries = (view: RowView): DropdownMenuEntry[] => [
  { kind: "label", id: "view" , label: "View" },
  ...VIEW_FIELDS.map(({ id, label }): DropdownMenuEntry => ({ kind: "item", id: `view:${id}`, label, selected: view[id] })),
];

/** Each column's width in px and its place among the others, in the menu's order. Now is a line and has no column. The columns are the same for every row, so none shifts. */
const COLUMNS: readonly { field: Exclude<ViewField, "now">; width: number }[] = [
  { field: "pr", width: 48 },
  { field: "issue", width: 48 },
  { field: "time", width: 52 },
  { field: "conflicts", width: 16 },
  { field: "status", width: 36 },
  { field: "workflow", width: 64 },
];
/** The gap between columns, `--space-1`. */
const GAP = 4;
/** What the default's two columns take of the row, gaps with them. */
const DEFAULT_TAKES = 16 + 36 + GAP * 2;
/** How much of the title's room the fields may take before the list rests wider to give it back. */
const TITLE_GIVES = 96;

const columnsOn = (view: RowView, earlier: boolean) => COLUMNS.filter(({ field }) => (field === "conflicts" ? view.conflicts || earlier : view[field]));

/**
 * The row's columns: the title, then each field that is on. The conflicts column is kept while a row in the list carries the earlier-Job mark, which stands in the same place.
 * `clashRight` is where that column's own mark sits from the row's trailing edge, since the mark is drawn over the row and not in it.
 */
export function columnsOf(view: RowView, earlier: boolean): { template: string; clashRight: string } {
  const on = columnsOn(view, earlier);
  const after = on.slice(on.findIndex(({ field }) => field === "conflicts") + 1);
  return {
    template: ["minmax(0, 1fr)", ...on.map(({ width }) => `${width}px`)].join(" "),
    clashRight: `calc(var(--space-3) + ${after.reduce((sum, { width }) => sum + width + GAP, 0)}px)`,
  };
}

/** How much wider than its resting width the list is drawn for the fields that are on, in px, so the title keeps most of what it had. A column the person dragged is theirs and is left as dragged. */
export function restBonus(view: RowView, earlier: boolean): number {
  const on = columnsOn(view, earlier);
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

const cell = (field: ViewField, content: ReactNode) => (
  <span className="deck-row__cell" data-field={field}>
    {content}
  </span>
);

/**
 * The cells of a row after its title, one for each field that is on, and Now's line. `status` and `hit` are the row's own: the state marks it drew before, and what a search
 * matched in place of its age.
 */
export function RowFields({ view, earlier, one, session, hit, status }: { view: RowView; earlier: boolean; one: Subject; session: Session | undefined; hit: { short: string; whole: string } | undefined; status: ReactNode }) {
  const issue = issueOf(session);
  const issueText = issue === undefined ? undefined : /^\d+$/.test(issue.number) ? `#${issue.number}` : issue.number;
  const flow = workflowOf(session);
  const open = one.pr !== undefined && one.pr.stage !== "landed" ? one.pr : undefined;
  return (
    <>
      {view.pr ? cell("pr", open === undefined ? null : <Tooltip label={`Pull request #${open.number}`}><span className="deck-row__chip">#{open.number}</span></Tooltip>) : null}
      {view.issue ? cell("issue", issue === undefined || issueText === undefined ? null : <Tooltip label={`Issue ${issueText}: ${issue.title}`}><span className="deck-row__issue">{issueText}</span></Tooltip>) : null}
      {view.time
        ? cell("time", hit !== undefined && hit.short !== "" ? <Tooltip label={hit.whole}><span className="deck-row__said" data-found="">{hit.short}</span></Tooltip> : one.at === undefined ? null : <Tooltip label="Last turn"><span className="deck-row__said"><Stamp still first="age" at={one.at} /></span></Tooltip>)
        : null}
      {view.conflicts || earlier ? cell("conflicts", null) : null}
      {view.status ? cell("status", status) : null}
      {view.workflow ? cell("workflow", flow === undefined ? null : <Tooltip label={`${flow.name}: ${flow.step}`}><span className="deck-row__flow">{flow.step}</span></Tooltip>) : null}
      {view.now ? <span className="deck-row__says">{one.peek.at(-1) ?? ""}</span> : null}
    </>
  );
}
