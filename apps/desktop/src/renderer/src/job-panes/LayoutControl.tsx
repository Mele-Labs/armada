// The Layout control in the Job's header: the layout the Job is on, and a menu to take another, to update the one it is on, to save it under a name, to reset it, or to make it the layout every Job opens on.

import { useEffect, useRef } from "react";
import { Pin } from "lucide-react";
import { Dialog, DropdownMenu, Input, Tooltip, useKept, type DropdownMenuEntry } from "@armada/components";

import { DEFAULT_LAYOUT, nameTaken, resetLayout, saveLayoutAs, setLayoutForAll, takeLayout, updateLayout } from "./store";

type Props = { jobId: string; layout: string; edited: boolean; layouts: readonly string[]; forAll: string; open: boolean; onOpen: (open: boolean) => void; naming: boolean; onNaming: (naming: boolean) => void };

export function LayoutControl({ jobId, layout, edited, layouts, forAll, open, onOpen, naming, onNaming }: Props) {
  const [name, setName, clearName] = useKept(`job-layout-name:${jobId}`, "");
  const field = useRef<HTMLInputElement>(null);
  const label = name.trim();
  const taken = label !== "" && nameTaken(label);
  // The dialog gives its first focus to Cancel; the name is what is typed.
  useEffect(() => {
    if (!naming) return;
    const timer = window.setTimeout(() => field.current?.focus(), 0);
    return () => window.clearTimeout(timer);
  }, [naming]);

  const entries: DropdownMenuEntry[] = [
    ...layouts.map(
      (one): DropdownMenuEntry => ({
        kind: "item",
        id: `take:${one}`,
        label: one,
        selected: one === layout,
        ...(one === forAll
          ? {
              trail: (
                <Tooltip label="Used for all Jobs" asChild>
                  <span className="job-layout__pin" role="img" aria-label="Used for all Jobs">
                    <Pin size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              ),
            }
          : {}),
      }),
    ),
    { kind: "separator", id: "rule" },
    { kind: "item", id: "update", label: `Update ${layout}`, act: true, disabled: layout === DEFAULT_LAYOUT || !edited, ...(layout === DEFAULT_LAYOUT ? { hint: "Default stays as it is" } : {}) },
    { kind: "item", id: "save", label: "Save as…", act: true },
    { kind: "item", id: "reset", label: "Reset", act: true, disabled: !edited },
    { kind: "item", id: "all", label: "Use for all Jobs", act: true, disabled: layout === forAll },
  ];

  const choose = (id: string) => {
    if (id.startsWith("take:")) takeLayout(jobId, id.slice("take:".length));
    else if (id === "update") updateLayout(jobId);
    else if (id === "save") onNaming(true);
    else if (id === "reset") resetLayout(jobId);
    else if (id === "all") setLayoutForAll(jobId);
  };

  return (
    <>
      <DropdownMenu plain align="start" triggerLabel={`Layout: ${layout}${edited ? " · edited" : ""}`} triggerTooltip="Layout" entries={entries} open={open} onOpenChange={onOpen} onSelect={choose} />
      <Dialog
        open={naming}
        title="Save layout"
        tone="neutral"
        confirmLabel="Save"
        confirmDisabled={label === "" || taken}
        onCancel={() => onNaming(false)}
        onConfirm={() => {
          saveLayoutAs(jobId, label);
          clearName();
          onNaming(false);
        }}
      >
        <Input ref={field} label="Name" value={name} invalid={taken} message="A layout has that name." onChange={(event) => setName(event.target.value)} />
      </Dialog>
    </>
  );
}
