import { Reply } from "lucide-react";
import { useRef, useState } from "react";
import type { KeyboardEvent, MouseEvent, ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import { responseEntries } from "../../primitives/Prose/PickLine";
import { ResponseEditorLayer } from "../../primitives/Prose/ResponseEditor";
import { CopyAct } from "../../primitives/Prose/RunnableBlock";
import type { QuickResponse, ResponseAct } from "../../primitives/Prose/responses";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Under an agent's message in the spacious take, shown while the pointer is over it or a key has reached it: Copy the
 * message, Reply with it quoted, the quick responses, and when it was said. A right press on a response offers Edit, Add
 * response and Remove, and the bar stays up while its menu or editor is open.
 */
export function MessageBar({
  text,
  onReply,
  responses,
  onPick,
  onAct,
  marked,
  at,
}: {
  /** The message's own words, as the agent wrote them. */
  text: string;
  onReply?: () => void;
  responses: readonly QuickResponse[];
  onPick?: (response: QuickResponse) => void;
  /** Absent, a response has no menu. */
  onAct?: (act: ResponseAct) => void;
  /** The emoji he already sent to this message. */
  marked?: string;
  at: ReactNode;
}) {
  const bar = useRef<HTMLDivElement>(null);
  const [menu, setMenu] = useState<
    | {
        x: number;
        y: number;
        id: string;
      }
    | undefined
  >(undefined);
  const [editing, setEditing] = useState<
    | {
        id: string | undefined;
      }
    | undefined
  >(undefined);
  const menuFor = menu === undefined ? undefined : responses.find((one) => one.id === menu.id);
  const edited = editing === undefined ? undefined : responses.find((one) => one.id === editing.id);
  const alone = responses.length <= 1;
  const openMenu = (response: QuickResponse) => ({
    onContextMenu: (event: MouseEvent<HTMLElement>) => {
      event.preventDefault();
      setMenu({
        x: event.clientX,
        y: event.clientY,
        id: response.id,
      });
    },
    onKeyDown: (event: KeyboardEvent<HTMLElement>) => {
      if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
      event.preventDefault();
      const box = event.currentTarget.getBoundingClientRect();
      setMenu({
        x: box.left + box.width / 2,
        y: box.bottom,
        id: response.id,
      });
    },
  });
  return (
    <div
      ref={bar}
      className="armada-message-bar"
      role="toolbar"
      aria-label="Message"
      data-held={menu !== undefined || editing !== undefined || undefined}
    >
      <CopyAct text={text} label="Copy message" />
      {onReply === undefined ? null : (
        <Tooltip label="Reply">
          <Button variant="ghost" size="sm" iconOnly aria-label="Reply" onClick={onReply}>
            <Reply size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
      {onPick === undefined || responses.length === 0 ? null : (
        <span className="armada-message-bar__responses">
          {responses.map((response) => (
            <Tooltip key={response.id} label={response.name}>
              <Button
                variant="ghost"
                size="sm"
                iconOnly
                aria-label={response.name}
                aria-pressed={marked === response.emoji}
                aria-haspopup={onAct === undefined ? undefined : "menu"}
                {...(onAct === undefined ? {} : openMenu(response))}
                onClick={() => onPick(response)}
              >
                <span className="armada-message-bar__emoji" aria-hidden>
                  {response.emoji}
                </span>
              </Button>
            </Tooltip>
          ))}
        </span>
      )}
      {at}
      {menu === undefined || menuFor === undefined || onAct === undefined ? null : (
        <DropdownMenu
          triggerLabel={`Quick response ${menuFor.name}`}
          at={menu}
          open
          entries={responseEntries(responses.length)}
          onOpenChange={(open) => open || setMenu(undefined)}
          onSelect={(chosen) => {
            setMenu(undefined);
            if (chosen === "edit")
              setEditing({
                id: menuFor.id,
              });
            else if (chosen === "add")
              setEditing({
                id: undefined,
              });
            else if (chosen === "remove")
              onAct({
                act: "remove",
                id: menuFor.id,
              });
          }}
        />
      )}
      {editing === undefined || onAct === undefined ? null : (
        <ResponseEditorLayer
          bar={bar}
          line={bar}
          label={edited === undefined ? "Add response" : `Edit ${edited.name}`}
          keep="quick-response"
          {...(edited === undefined
            ? {}
            : {
                response: edited,
              })}
          onSave={(emoji, name) => {
            onAct(
              edited === undefined
                ? {
                    act: "add",
                    emoji,
                    name,
                  }
                : {
                    act: "edit",
                    id: edited.id,
                    emoji,
                    name,
                  },
            );
            setEditing(undefined);
          }}
          {...(edited === undefined || alone
            ? {}
            : {
                onRemove: () => {
                  onAct({
                    act: "remove",
                    id: edited.id,
                  });
                  setEditing(undefined);
                },
              })}
          onClose={() => setEditing(undefined)}
        />
      )}
    </div>
  );
}
