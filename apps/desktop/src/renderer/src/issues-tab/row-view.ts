// What an issue's row draws, chosen by the person: a right press on a row or on the list opens a menu whose View group switches four fields on and off, as the Jobs list's does
// (`cockpit/deck/row-view.tsx`). Kept per person under a key of its own, so the two lists do not share a choice. The title is always drawn; the default adds the number on its own line
// and the state lamp.

import { useKept } from "@armada/components";
import type { DropdownMenuEntry } from "@armada/components";

export const ISSUE_FIELDS = [
  {
    id: "number",
    label: "Number",
  },
  {
    id: "labels",
    label: "Labels",
  },
  {
    id: "job",
    label: "Linked Job",
  },
  {
    id: "state",
    label: "State",
  },
] as const;

export type IssueField = (typeof ISSUE_FIELDS)[number]["id"];
export type IssueView = Readonly<Record<IssueField, boolean>>;

export const DEFAULT_ISSUE_VIEW: IssueView = {
  number: true,
  labels: false,
  job: false,
  state: true,
};

const KEEP = "issues:rowView";

/** What is kept, read back field by field so a stored shape that lacks one takes its default. */
const revive = (stored: unknown): IssueView => {
  const got =
    typeof stored === "object" && stored !== null
      ? (stored as Partial<Record<IssueField, unknown>>)
      : {};
  return Object.fromEntries(
    ISSUE_FIELDS.map(({ id }) => [
      id,
      typeof got[id] === "boolean" ? got[id] : DEFAULT_ISSUE_VIEW[id],
    ]),
  ) as IssueView;
};

/** The choice and the act that flips one field; the rows read it at once. */
export function useIssueView(): [
  IssueView,
  (field: IssueField) => void,
] {
  const [view, keep] = useKept<IssueView>(KEEP, DEFAULT_ISSUE_VIEW, {
    revive,
  });
  return [
    view,
    (field) =>
      keep((was) => ({
        ...was,
        [field]: !was[field],
      })),
  ];
}

const FIELD_OF = new Map<string, IssueField>(
  ISSUE_FIELDS.map(({ id }) => [
    `view:${id}`,
    id,
  ]),
);

/** The pick a menu item makes, where it is one of the View group's. */
export const issueFieldOf = (id: string): IssueField | undefined => FIELD_OF.get(id);

/** The View group of the menu: a heading and a checkable item for each field. */
export const issueViewEntries = (view: IssueView): DropdownMenuEntry[] => [
  {
    kind: "label",
    id: "view",
    label: "View",
  },
  ...ISSUE_FIELDS.map(
    ({ id, label }): DropdownMenuEntry => ({
      kind: "item",
      id: `view:${id}`,
      label,
      selected: view[id],
    }),
  ),
];
