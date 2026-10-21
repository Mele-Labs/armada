// A pull request chip's menu, the merge line's and Progress's: Refresh, which has Fleet read the pull request from the forge now.
// It opens at the pointer on a right press, and under the chip on the ContextMenu key or Shift+F10. A chip whose window has no
// way to refresh has no menu.

import { useState, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import { DropdownMenu } from "@armada/components";

import { useRepositoryActs } from "./sessions-draft";

type At = { x: number; y: number; from: HTMLElement };

/** What a chip spreads onto its own element, and the menu to draw beside it. */
export type PullRequestMenu = {
  on: { onContextMenu?: (event: MouseEvent<HTMLElement>) => void; onKeyDown?: (event: KeyboardEvent<HTMLElement>) => void };
  menu: ReactNode;
};

export function usePullRequestMenu(manifestId: string | undefined, number: number): PullRequestMenu {
  const refresh = useRepositoryActs().refreshPullRequest;
  const [at, setAt] = useState<At | undefined>(undefined);
  if (refresh === undefined) return { on: {}, menu: null };
  const on: PullRequestMenu["on"] = {
    onContextMenu: (event) => {
      event.preventDefault();
      setAt({ x: event.clientX, y: event.clientY, from: event.currentTarget });
    },
    onKeyDown: (event) => {
      if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
      event.preventDefault();
      const box = event.currentTarget.getBoundingClientRect();
      setAt({ x: box.left + box.width / 2, y: box.bottom, from: event.currentTarget });
    },
  };
  const menu =
    at === undefined ? null : (
      <DropdownMenu
        triggerLabel={`Pull request #${number}`}
        at={{ x: at.x, y: at.y }}
        open
        entries={[
          { kind: "label", id: "heading", label: `Pull request #${number}` },
          { kind: "item", id: "refresh", label: "Refresh" },
        ]}
        onOpenChange={(open) => {
          if (open) return;
          setAt(undefined);
          at.from.focus();
        }}
        onSelect={(chosen) => {
          if (chosen === "refresh") void refresh(manifestId, number);
        }}
      />
    );
  return { on, menu };
}

/** The menu for a chip drawn inline, where a hook cannot be called per chip: `children` is given what to spread onto the chip. */
export function PullRequestMenuOn({ manifestId, number, children }: { manifestId: string | undefined; number: number; children: (on: PullRequestMenu["on"]) => ReactNode }) {
  const { on, menu } = usePullRequestMenu(manifestId, number);
  return (
    <>
      {children(on)}
      {menu}
    </>
  );
}
