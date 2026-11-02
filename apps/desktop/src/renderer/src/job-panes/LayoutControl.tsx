// The Layout control in the title bar: an icon with the layout the Job is on in its tooltip and a mark on it once rearranged, and a menu to take another, to update the one it is on, to save it under a name, to reset it, or to pick the layout new Jobs open on.
// The layout rows are this Job's own choice and carry no tag: the one it is on is checked. The last row, New Jobs open on, trails the layout new Jobs open on and opens a submenu of every layout to change it.
// Save as turns the menu into a name field in place: ↵ saves, Esc leaves it.

import { PanelsTopLeft } from "lucide-react";
import { DropdownMenu, Tooltip, useKept, type DropdownMenuEntry } from "@armada/components";

import {
  nameTaken,
  resetLayout,
  saveLayoutAs,
  setLayoutForAll,
  takeLayout,
  updateLayout,
} from "./store";

type Props = {
  jobId: string;
  layout: string;
  edited: boolean;
  layouts: readonly string[];
  forAll: string;
  open: boolean;
  onOpen: (open: boolean) => void;
  naming: boolean;
  onNaming: (naming: boolean) => void;
};

export function LayoutControl({
  jobId,
  layout,
  edited,
  layouts,
  forAll,
  open,
  onOpen,
  naming,
  onNaming,
}: Props) {
  const [name, setName, clearName] = useKept(`job-layout-name:${jobId}`, "");
  const label = name.trim();
  const taken = label !== "" && nameTaken(label);

  const entries: DropdownMenuEntry[] = naming
    ? [
        {
          kind: "label",
          id: "naming",
          label: taken ? "A layout has that name." : "Save as",
        },
      ]
    : [
        ...layouts.map(
          (one): DropdownMenuEntry => ({
            kind: "item",
            id: `take:${one}`,
            label: one,
            selected: one === layout,
          }),
        ),
        {
          kind: "separator",
          id: "rule",
        },
        {
          kind: "item",
          id: "update",
          label: `Update ${layout}`,
          act: true,
          disabled: !edited,
        },
        {
          kind: "item",
          id: "save",
          label: "Save as…",
          act: true,
        },
        {
          kind: "item",
          id: "reset",
          label: "Reset",
          act: true,
          disabled: !edited,
        },
        {
          kind: "separator",
          id: "rule-new",
        },
        {
          kind: "item",
          id: "new",
          label: "New Jobs open on",
          detail: forAll,
          submenu: layouts.map((one) => ({
            id: `new:${one}`,
            label: one,
            selected: one === forAll,
          })),
        },
      ];

  const choose = (id: string) => {
    if (id.startsWith("take:")) takeLayout(jobId, id.slice("take:".length));
    else if (id === "update") updateLayout(jobId);
    else if (id === "save") {
      // The menu has just shut with the pick; it opens again as the field, which takes the focus as it opens.
      onNaming(true);
      window.setTimeout(() => onOpen(true), 0);
    } else if (id === "reset") resetLayout(jobId);
    else if (id.startsWith("new:")) setLayoutForAll(id.slice("new:".length));
  };

  // Leaving the menu any way but ↵ leaves the field; the name typed is kept.
  const change = (next: boolean) => {
    onOpen(next);
    if (!next && naming) onNaming(false);
  };

  const enter = () => {
    if (label === "" || taken) {
      // A name that cannot be saved keeps the field open, with its reason.
      onNaming(true);
      onOpen(true);
      return;
    }
    saveLayoutAs(jobId, label);
    clearName();
    onNaming(false);
  };

  const said = `Layout: ${layout}${edited ? " · edited" : ""}`;
  return (
    <Tooltip label={said} asChild>
      <span className="job-layout">
        <DropdownMenu
          icon={PanelsTopLeft}
          triggerLabel={said}
          entries={entries}
          open={open}
          onOpenChange={change}
          onSelect={choose}
          {...(naming
            ? {
                searchLabel: "Name",
                search: {
                  value: name,
                  onChange: setName,
                  onEnter: enter,
                },
              }
            : {})}
        />
        {edited ? <span className="job-layout__mark" aria-hidden="true" /> : null}
      </span>
    </Tooltip>
  );
}
