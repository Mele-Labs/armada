// Deferred: the follow-ups the agent kept for itself, in the Landing, folded under one label. A press opens them, and the button is a tab stop that Enter and Space press.
// Each row has Remind after its words, which sends the sentence back to the agent, then two thumbs that say whether Fleet was right to keep it: thumbs down takes it off the list,
// thumbs up keeps it and takes the thumbs away. A right press on the row offers the same two. Remind and the thumbs keep their columns down the list, empty where a row lacks them.
// The issues the Job filed for later stand in the same list, a row each that opens the issue as the Blueprint's chips do; they have no Remind or thumbs.

import { useState } from "react";
import { BellRing, ChevronDown, ThumbsDown, ThumbsUp } from "lucide-react";
import { Button, DropdownMenu, Tooltip } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { IssueRow, type Issue } from "./IssuesBlock";
import "./later-block.css";

type One = NonNullable<Session["later"]>[number];

export function LaterBlock({ later, issues, manifestId, onRemind, onConfirm, onDrop }: { later: NonNullable<Session["later"]>; issues: readonly Issue[]; manifestId: string | undefined; onRemind: (sentence: string) => void; onConfirm: (one: One) => void; onDrop: (one: One) => void }) {
  const [open, setOpen] = useState(false);
  const [menu, setMenu] = useState<{ id: string; x: number; y: number; from: HTMLElement | null } | undefined>(undefined);
  const menuOf = menu === undefined ? undefined : later.find((one) => one.id === menu.id);
  return (
    <section className="sv-later" data-open={open || undefined} aria-label="Deferred">
      <button type="button" className="sv-later__row" aria-expanded={open} onClick={() => setOpen((was) => !was)}>
        <span className="sv-later__label">Deferred</span>
        <ChevronDown className="sv-later__chevron" size={12} strokeWidth={2} aria-hidden />
      </button>
      {open ? (
        <ul className="sv-later__list">
          {later.map((one) => (
            <li
              key={one.id}
              className="sv-later__item"
              aria-label={one.text}
              onContextMenu={(event) => (event.preventDefault(), setMenu({ id: one.id, x: event.clientX, y: event.clientY, from: event.currentTarget.querySelector<HTMLElement>(".sv-later__remind button") }))}
              onKeyDown={(event) => {
                if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
                event.preventDefault();
                const box = event.currentTarget.querySelector<HTMLElement>(".sv-later__remind button")?.getBoundingClientRect() ?? event.currentTarget.getBoundingClientRect();
                setMenu({ id: one.id, x: box.left + box.width / 2, y: box.bottom, from: event.currentTarget.querySelector<HTMLElement>(".sv-later__remind button") });
              }}
            >
              <span className="sv-later__text">{one.text}</span>
              <span className="sv-later__remind">
                <Tooltip label="Remind">
                  <Button variant="ghost" size="sm" iconOnly aria-label={`Remind: ${one.text}`} onClick={() => onRemind(one.text)}>
                    <BellRing size={12} strokeWidth={2} aria-hidden />
                  </Button>
                </Tooltip>
              </span>
              <span className="sv-later__thumbs">
                {one.confirmed === true ? null : (
                  <>
                    <Tooltip label="Deferred">
                      <Button variant="ghost" size="sm" iconOnly aria-label={`Deferred: ${one.text}`} onClick={(event) => (event.currentTarget.closest("li")?.querySelector<HTMLElement>(".sv-later__remind button")?.focus(), onConfirm(one))}>
                        <ThumbsUp size={12} strokeWidth={2} aria-hidden />
                      </Button>
                    </Tooltip>
                    <Tooltip label="Not deferred">
                      <Button variant="ghost" size="sm" iconOnly aria-label={`Not deferred: ${one.text}`} onClick={() => onDrop(one)}>
                        <ThumbsDown size={12} strokeWidth={2} aria-hidden />
                      </Button>
                    </Tooltip>
                  </>
                )}
              </span>
            </li>
          ))}
          {issues.map((issue) => (
            <li key={issue.number} className="sv-later__item" data-kind="issue">
              <IssueRow issue={issue} manifestId={manifestId} />
            </li>
          ))}
        </ul>
      ) : null}
      {menu === undefined || menuOf === undefined ? null : (
        <DropdownMenu
          triggerLabel="Deferred"
          at={{ x: menu.x, y: menu.y }}
          open
          entries={[
            ...(menuOf.confirmed === true ? [] : [{ kind: "item" as const, id: "yes", label: "Deferred", icon: ThumbsUp }]),
            { kind: "item", id: "no", label: "Not deferred", icon: ThumbsDown },
          ]}
          onOpenChange={(next) => {
            if (next) return;
            setMenu(undefined);
            menu.from?.focus();
          }}
          onSelect={(id) => (id === "yes" ? onConfirm : onDrop)(menuOf)}
        />
      )}
    </section>
  );
}
