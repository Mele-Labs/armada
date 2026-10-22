import type { LoadoutItem, McpServerState } from "@armada/protocol";

import { Prose } from "../../primitives/Prose/Prose";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { SkeletonText } from "../../primitives/Skeleton/Skeleton";
import { sourceSaid, toolsOf, type LoadoutRef } from "../../loadout";
import { StateMark } from "./LoadoutStateMark";

/** What one item says as the host read it: its text, why Fleet did not read it, or no answer. */
export type LoadoutItemRead =
  | { state: "reading" }
  | { state: "failed" }
  | { state: "not_read"; why: string }
  | { state: "read"; path?: string; format: "markdown" | "text"; body: string; truncated?: boolean };

/** The item open beside the Loadout. `read` is absent for an MCP server, which is drawn from the Loadout itself. */
export type LoadoutOpened = { item: LoadoutRef; read?: LoadoutItemRead };

export type LoadoutItemPanelProps = {
  opened: LoadoutOpened;
  /** The whole list, for an MCP server's tools. */
  items: readonly LoadoutItem[];
  /** An MCP server's state as a hosted Job's CLI reports it now, over what the agent reported. */
  state?: McpServerState;
  floor: boolean;
  onClose: () => void;
};

/**
 * A SKILL.md or an agent file opens on its front matter. Drawn as markdown that is a rule and a heading, so it is
 * handed to Prose as the fenced block it reads as.
 */
function fencedFrontMatter(body: string): string {
  const lines = body.split("\n");
  if (lines[0]?.replace(/\r$/, "") !== "---") return body;
  const close = lines.findIndex((line, at) => at >= 2 && line.trimEnd() === "---");
  if (close === -1) return body;
  const inner = lines.slice(1, close).join("\n").replace(/\r$/, "");
  const runs = inner.match(/`+/g) ?? [];
  const fence = "`".repeat(Math.max(3, ...runs.map((one) => one.length + 1)));
  return `${fence}yaml\n${inner}\n${fence}\n\n${lines.slice(close + 1).join("\n")}`;
}

const STATE_SAID: Partial<Record<NonNullable<LoadoutItem["state"]>, string>> = { loaded: "Loaded", deferred: "Deferred" };

function ServerTools({ server, items }: { server: LoadoutRef; items: readonly LoadoutItem[] }) {
  const tools = toolsOf(server, items);
  return (
    <section className="armada-loadout__group" aria-label="Tools">
      <h4 className="armada-loadout__eyebrow">Tools</h4>
      {tools.length === 0 ? (
        <p className="armada-loadout__none">None reported</p>
      ) : (
        <ul className="armada-loadout__rows">
          {tools.map((one) => (
            <li key={one.name} className="armada-loadout__row">
              <span className="armada-loadout__name">{one.name}</span>
              <span className="armada-loadout__source">{one.state === undefined ? "" : (STATE_SAID[one.state] ?? "")}</span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function Body({ read }: { read: LoadoutItemRead | undefined }) {
  if (read === undefined || read.state === "reading") return <SkeletonText label="Reading" widths={["70%", "90%", "55%", "80%"]} />;
  if (read.state === "failed") return <p className="armada-loadout__none">Not read. Press the row again.</p>;
  if (read.state === "not_read") return <p className="armada-loadout-item__why">{read.why}</p>;
  return (
    <>
      {read.truncated === true ? <p className="armada-loadout__none">Only the start is shown.</p> : null}
      {read.format === "markdown" ? <Prose text={fencedFrontMatter(read.body)} /> : <pre className="armada-loadout-item__text">{read.body}</pre>}
    </>
  );
}

/**
 * One Loadout item to the left of the Loadout, as the pull request's file opens to the left of its panel: a second
 * floating sheet that dims nothing. Its name heads it, where it came from and the path it was read from under that.
 * An MCP server lists its tools from the Loadout; every other kind draws what Fleet read.
 */
export function LoadoutItemPanel({ opened, items, state: live, floor, onClose }: LoadoutItemPanelProps) {
  const { item, read } = opened;
  const server = item.kind === "mcp_server";
  const path = !server && read?.state === "read" ? read.path : undefined;
  const state = server ? live ?? items.find((one) => one.kind === "mcp_server" && one.name === item.name && one.source === item.source)?.state : undefined;
  return (
    <Sheet
      open
      floating
      beside="session-loadout"
      kind="session-loadout-item"
      floor={floor}
      title={item.name}
      subtitle={
        <span className="armada-loadout-item__said">
          <span>{sourceSaid(item)}</span>
          <StateMark state={state} />
          {path === undefined ? null : <span className="armada-loadout-item__path">{path}</span>}
        </span>
      }
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
    >
      <div className="armada-loadout-item" aria-busy={!server && (read === undefined || read.state === "reading") ? true : undefined}>
        {server ? <ServerTools server={item} items={items} /> : <Body read={read} />}
      </div>
    </Sheet>
  );
}
