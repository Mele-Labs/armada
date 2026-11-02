import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type MouseEvent,
  type ReactNode,
} from "react";

import { Button } from "../Button/Button";
import { DropdownMenu } from "../DropdownMenu/DropdownMenu";
import type { DropdownMenuEntry } from "../DropdownMenu/DropdownMenu";
import { Tooltip } from "../Tooltip/Tooltip";
import { ResponseEditorLayer } from "./ResponseEditor";
import { MAX_RESPONSES } from "./responses";
import type { QuickResponse, ResponseAct } from "./responses";

/**
 * Where a line of an agent's words can be answered. Provided by a thread around an agent's message; without one
 * no line is hoverable and `Prose` draws as it always has. `marked` is the emoji the person already sent to a line.
 */
export type ProsePicker = {
  responses: readonly QuickResponse[];
  pick: (text: string, response: QuickResponse) => void;
  marked: (text: string) => string | undefined;
  /** A right press on a response offers Edit, Remove and Add response. Absent, a response has no menu. */
  onAct?: (act: ResponseAct) => void;
};
export const ProsePick = createContext<ProsePicker | null>(null);

/** Set inside a pickable list item, whose paragraphs are part of its line and not lines of their own. */
export const InPickLine = createContext(false);

/** The hast node `react-markdown` hands a component, as far as the line's words are read from it. */
export type LineNode = {
  type: string;
  value?: string;
  tagName?: string;
  children?: LineNode[];
};

/** The words of a line as plain text: formatting dropped, code kept, and a nested list left to its own lines. */
export function lineText(node: LineNode | undefined): string {
  const out: string[] = [];
  const walk = (one: LineNode, top: boolean) => {
    if (one.type === "text") out.push(one.value ?? "");
    if (one.type !== "element" && one.type !== "root") return;
    if (!top && (one.tagName === "ul" || one.tagName === "ol" || one.tagName === "input")) return;
    if (one.tagName === "br") out.push(" ");
    one.children?.forEach((child) => walk(child, false));
    if (one.tagName === "p") out.push(" ");
  };
  if (node !== undefined) walk(node, true);
  return out.join("").replace(/\s+/g, " ").trim();
}

/** How far past the line and its acts the pointer may stray before they go: leaving has to be clear, not a slip. */
const STRAY = 24;
/** How long the pointer must rest on another line before it takes the acts: crossing one on the way to the acts is not choosing it. */
const SETTLE_MS = 250;
/** How long the acts outlast the pointer leaving, so a quick overshoot does not lose them. */
const LINGER_MS = 300;

/** The one hot line: how to cool it. A line the pointer settles on cools the one before. */
let coolHot: (() => void) | null = null;
/** A line waiting to take the acts once the pointer has settled on it; reaching the hot line's acts calls it off. */
let settling: ReturnType<typeof setTimeout> | undefined;

/** Arrow keys walk the lines of the message the focus is in. */
function move(from: HTMLElement, by: 1 | -1): void {
  const message = from.closest(".armada-prose");
  if (message === null) return;
  const lines = [
    ...message.querySelectorAll<HTMLElement>("[data-pick]"),
  ];
  lines[lines.indexOf(from) + by]?.focus();
}

/**
 * A list item or a paragraph of an agent's message that can be answered: a wash on hover or focus, and the quick responses
 * over its right end. Enter sends the first, 1 to 6 send the nth, the arrows move between lines. What was sent keeps a quiet mark.
 * **A right press on a response offers Edit, Remove and Add response**, and the bar stays up while its menu or editor is open.
 */
export function PickLine({
  as: Tag,
  className,
  text,
  children,
  ...rest
}: {
  as: "li" | "p";
  className: string;
  text: string;
  children?: ReactNode;
  [data: `data-${string}`]: unknown;
}) {
  const picker = useContext(ProsePick);
  // Whether this line holds the acts. Read from the pointer's own events, so the nearest line wins over a list inside it.
  const [hot, setHot] = useState(false);
  const line = useRef<HTMLElement | null>(null);
  const acts = useRef<HTMLSpanElement | null>(null);
  // Whether the pointer is over this line now, for a line still settling.
  const over = useRef(false);
  // The menu or the editor a right press opened: the acts are not put away while either is up.
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
  const holding = useRef(false);
  holding.current = menu !== undefined || editing !== undefined;
  // While hot, the acts stay until the pointer is clearly away from the line and from them, and a moment after.
  useEffect(() => {
    if (!hot) return;
    let leaving: ReturnType<typeof setTimeout> | undefined;
    const cool = () => {
      clearTimeout(leaving);
      setHot(false);
    };
    coolHot = cool;
    const within = (one: Element | null, by: number, event: PointerEvent) => {
      if (one === null) return false;
      const box = one.getBoundingClientRect();
      return (
        event.clientX >= box.left - by &&
        event.clientX <= box.right + by &&
        event.clientY >= box.top - by &&
        event.clientY <= box.bottom + by
      );
    };
    const onMove = (event: PointerEvent) => {
      if (within(acts.current, STRAY, event)) clearTimeout(settling);
      if (within(line.current, STRAY, event) || within(acts.current, STRAY, event)) {
        clearTimeout(leaving);
        leaving = undefined;
      } else if (leaving === undefined) {
        leaving = setTimeout(() => {
          leaving = undefined;
          if (!holding.current) cool();
        }, LINGER_MS);
      }
    };
    document.addEventListener("pointermove", onMove);
    return () => {
      clearTimeout(leaving);
      document.removeEventListener("pointermove", onMove);
      if (coolHot === cool) coolHot = null;
    };
  }, [
    hot,
  ]);
  // `rest` is the `data-line` a source view puts on a block.
  if (picker === null || text === "")
    return (
      <Tag className={className} {...rest}>
        {children}
      </Tag>
    );
  const marked = picker.marked(text);
  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.target !== event.currentTarget || event.metaKey || event.ctrlKey || event.altKey)
      return;
    const response =
      event.key === "Enter"
        ? picker.responses[0]
        : /^[1-9]$/.test(event.key)
          ? picker.responses[Number(event.key) - 1]
          : undefined;
    if (response !== undefined) {
      event.preventDefault();
      picker.pick(text, response);
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      move(event.currentTarget, event.key === "ArrowDown" ? 1 : -1);
    }
  };
  const markName =
    marked === undefined
      ? undefined
      : (picker.responses.find((one) => one.emoji === marked)?.name ?? "Sent");
  const edited =
    editing === undefined ? undefined : picker.responses.find((one) => one.id === editing.id);
  const full = picker.responses.length >= MAX_RESPONSES;
  const alone = picker.responses.length <= 1;
  const entries: DropdownMenuEntry[] = [
    {
      kind: "item",
      id: "edit",
      label: "Edit",
    },
    {
      kind: "item",
      id: "add",
      label: "Add response",
      disabled: full,
      ...(full
        ? {
            hint: "Six at most",
          }
        : {}),
    },
    {
      kind: "separator",
      id: "rule",
    },
    {
      kind: "item",
      id: "remove",
      label: "Remove",
      danger: true,
      disabled: alone,
      ...(alone
        ? {
            hint: "One stays",
          }
        : {}),
    },
  ];
  const openMenu =
    picker.onAct === undefined
      ? undefined
      : (response: QuickResponse) => ({
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
  const menuFor =
    menu === undefined ? undefined : picker.responses.find((one) => one.id === menu.id);
  return (
    <Tag
      ref={(node: HTMLElement | null) => {
        line.current = node;
      }}
      className={className}
      {...rest}
      data-pick=""
      data-hot={hot || undefined}
      data-marked={marked}
      tabIndex={0}
      onPointerOver={(event) => {
        const mine = (event.target as Element).closest("[data-pick]") === event.currentTarget;
        over.current = mine;
        if (hot || !mine || acts.current?.contains(event.target as Node)) return;
        const take = () => {
          coolHot?.();
          setHot(true);
        };
        clearTimeout(settling);
        // Nothing hot: take the acts at once. Another line hot: only once the pointer rests here.
        if (coolHot === null) take();
        else settling = setTimeout(() => over.current && take(), SETTLE_MS);
      }}
      onPointerOut={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) over.current = false;
      }}
      aria-keyshortcuts={[
        "Enter",
        ...picker.responses.map((_, at) => String(at + 1)),
      ].join(" ")}
      onKeyDown={onKeyDown}
    >
      <InPickLine.Provider value>{children}</InPickLine.Provider>
      {marked === undefined ? null : (
        <Tooltip label={markName!}>
          <span className="armada-prose__pick-mark" role="img" aria-label={markName!}>
            {marked}
          </span>
        </Tooltip>
      )}
      <span
        ref={acts}
        className="armada-prose__pick-acts"
        role="group"
        aria-label={`Quick responses: ${text}`}
      >
        {picker.responses.map((response) => (
          <Tooltip key={response.id} label={response.name}>
            <Button
              variant="ghost"
              size="sm"
              iconOnly
              aria-label={response.name}
              tabIndex={-1}
              aria-haspopup={picker.onAct === undefined ? undefined : "menu"}
              {...(openMenu?.(response) ?? {})}
              onClick={(event) => {
                picker.pick(text, response);
                setHot(false);
                // A press leaves focus on the button, which would hold the acts up after the pointer has gone.
                event.currentTarget.blur();
              }}
            >
              <span className="armada-prose__pick-emoji" aria-hidden>
                {response.emoji}
              </span>
            </Button>
          </Tooltip>
        ))}
      </span>
      {menu === undefined || menuFor === undefined || picker.onAct === undefined ? null : (
        <DropdownMenu
          triggerLabel={`Quick response ${menuFor.name}`}
          at={menu}
          open
          entries={entries}
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
              picker.onAct?.({
                act: "remove",
                id: menuFor.id,
              });
          }}
        />
      )}
      {editing === undefined || picker.onAct === undefined ? null : (
        <ResponseEditorLayer
          bar={acts}
          line={line}
          label={edited === undefined ? "Add response" : `Edit ${edited.name}`}
          keep="quick-response"
          {...(edited === undefined
            ? {}
            : {
                response: edited,
              })}
          onSave={(emoji, name) => {
            picker.onAct?.(
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
                  picker.onAct?.({
                    act: "remove",
                    id: edited.id,
                  });
                  setEditing(undefined);
                },
              })}
          onClose={() => setEditing(undefined)}
        />
      )}
    </Tag>
  );
}
