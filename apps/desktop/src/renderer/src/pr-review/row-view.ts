// What a pull request's row draws, chosen by the person: a right press on a row or on the list opens a menu whose View group switches five fields on and off, as the Issues list's does
// (`issues-tab/row-view.ts`). Kept per person under a key of its own, so the lists do not share a choice. The title is always drawn; the default adds the number on its own line
// and the state marks.

import { useKept } from "@armada/components";
import type { DropdownMenuEntry } from "@armada/components";

export const REVIEW_FIELDS = [
  {
    id: "number",
    label: "Number",
  },
  {
    id: "state",
    label: "State",
  },
  {
    id: "author",
    label: "Author",
  },
  {
    id: "repository",
    label: "Repository",
  },
  {
    id: "reason",
    label: "Reason",
  },
] as const;

export type ReviewField = (typeof REVIEW_FIELDS)[number]["id"];
export type ReviewView = Readonly<Record<ReviewField, boolean>>;

export const DEFAULT_REVIEW_VIEW: ReviewView = {
  number: true,
  state: true,
  author: false,
  repository: false,
  reason: false,
};

const KEEP = "pr-review:rowView";

/** What is kept, read back field by field so a stored shape that lacks one takes its default. */
const revive = (stored: unknown): ReviewView => {
  const got =
    typeof stored === "object" && stored !== null
      ? (stored as Partial<Record<ReviewField, unknown>>)
      : {};
  return Object.fromEntries(
    REVIEW_FIELDS.map(({ id }) => [
      id,
      typeof got[id] === "boolean" ? got[id] : DEFAULT_REVIEW_VIEW[id],
    ]),
  ) as ReviewView;
};

/** The choice and the act that flips one field; the rows read it at once. */
export function useReviewView(): [
  ReviewView,
  (field: ReviewField) => void,
] {
  const [view, keep] = useKept<ReviewView>(KEEP, DEFAULT_REVIEW_VIEW, {
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

const FIELD_OF = new Map<string, ReviewField>(
  REVIEW_FIELDS.map(({ id }) => [
    `view:${id}`,
    id,
  ]),
);

/** The pick a menu item makes, where it is one of the View group's. */
export const reviewFieldOf = (id: string): ReviewField | undefined => FIELD_OF.get(id);

/** The View group of the menu: a heading and a checkable item for each field. */
export const reviewViewEntries = (view: ReviewView): DropdownMenuEntry[] => [
  {
    kind: "label",
    id: "view",
    label: "View",
  },
  ...REVIEW_FIELDS.map(
    ({ id, label }): DropdownMenuEntry => ({
      kind: "item",
      id: `view:${id}`,
      label,
      selected: view[id],
    }),
  ),
];
