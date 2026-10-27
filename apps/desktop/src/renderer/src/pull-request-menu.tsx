// A pull request chip's menu, wherever a chip is drawn: Open in browser and Copy link where the chip knows its address, Refresh,
// which has Fleet read the pull request from the forge now, and Teach where the chip is in a Job. It opens at the pointer on a right press, and under the chip on the ContextMenu key or
// Shift+F10. A chip with none of these to offer has no menu.

import { useState, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import { DropdownMenu } from "@armada/components";
import { ExternalLink, GraduationCap, Link } from "lucide-react";

import { openLink } from "./commands";
import { useRepositoryActs } from "./sessions-draft";
import { useTeach } from "./teach";

type At = { x: number; y: number; from: HTMLElement };

/** What a chip spreads onto its own element, and the menu to draw beside it. */
export type PullRequestMenu = {
  on: { onContextMenu?: (event: MouseEvent<HTMLElement>) => void; onKeyDown?: (event: KeyboardEvent<HTMLElement>) => void };
  menu: ReactNode;
};

/** `address` is where the pull request is on the forge; a chip that does not hold it offers no link. */
export function usePullRequestMenu(manifestId: string | undefined, number: number, title?: string, address?: string): PullRequestMenu {
  const refresh = useRepositoryActs().refreshPullRequest;
  const teach = useTeach();
  const [at, setAt] = useState<At | undefined>(undefined);
  if (refresh === undefined && teach === undefined && address === undefined) return { on: {}, menu: null };
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
          ...(address === undefined
            ? []
            : [
                { kind: "item" as const, id: "browser", label: "Open in browser", icon: ExternalLink },
                { kind: "item" as const, id: "copy", label: "Copy link", icon: Link },
              ]),
          ...(refresh === undefined ? [] : [{ kind: "item" as const, id: "refresh", label: "Refresh" }]),
          ...(teach === undefined ? [] : [{ kind: "item" as const, id: "teach", label: "Teach", icon: GraduationCap }]),
        ]}
        onOpenChange={(open) => {
          if (open) return;
          setAt(undefined);
          at.from.focus();
        }}
        onSelect={(chosen) => {
          if (chosen === "browser" && address !== undefined) void openLink(address);
          else if (chosen === "copy" && address !== undefined) void navigator.clipboard.writeText(address);
          else if (chosen === "refresh") void refresh?.(manifestId, number);
          else if (chosen === "teach") teach?.("pull_request", title === undefined ? `#${number}` : `#${number} ${title}`);
        }}
      />
    );
  return { on, menu };
}

/** The menu for a chip drawn inline, where a hook cannot be called per chip: `children` is given what to spread onto the chip. */
export function PullRequestMenuOn({ manifestId, number, title, address, children }: { manifestId: string | undefined; number: number; title?: string; address?: string; children: (on: PullRequestMenu["on"]) => ReactNode }) {
  const { on, menu } = usePullRequestMenu(manifestId, number, title, address);
  return (
    <>
      {children(on)}
      {menu}
    </>
  );
}
