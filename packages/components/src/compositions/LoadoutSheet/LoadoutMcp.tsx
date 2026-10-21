import { useEffect, useRef } from "react";
import type { LoadoutItem, McpServerAct as McpAct, McpServerState } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { loadoutKeyOf, type LoadoutRef } from "../../loadout";
import { StateMark } from "./LoadoutStateMark";

/** One server as the CLI reports it now, and what the host knows of the person's acts on it. */
export type McpServerRow = {
  name: string;
  state: McpServerState;
  /** The CLI's words for a failure. */
  error?: string;
  tools: number;
  /** The act sent and not answered yet. */
  acting?: McpAct;
  /** Fleet's words for the last act refused on this row, or why its sign-in page did not open. */
  refused?: string;
  /** Its sign-in page was opened: while it still needs sign-in, the row offers Reconnect, since a claude.ai page sends nothing back. */
  signedIn?: true;
};

/**
 * A hosted Job's MCP servers, live. `servers` is absent while the first read is out, and the group draws the agent's
 * own report meanwhile. `said` is Fleet's words for a read it refused.
 */
export type McpGroup = { servers?: readonly McpServerRow[]; said?: string; onAct: (server: string, act: McpAct) => void };

/** How a row opens its item: which one is open, and the press. Absent draws the rows as text. */
export type Opens = { open: string | undefined; onOpen: (item: LoadoutRef) => void } | undefined;

const ACT_SAID: Record<McpAct, { label: string; underway: string }> = {
  reconnect: { label: "Reconnect", underway: "Reconnecting" },
  turn_on: { label: "Turn on", underway: "Turning on" },
  turn_off: { label: "Turn off", underway: "Turning off" },
  sign_in: { label: "Sign in", underway: "Signing in" },
};

/** The one act a row offers for where its server stands. A server still connecting offers none. */
export function mcpActOf(row: Pick<McpServerRow, "state" | "signedIn">): McpAct | undefined {
  switch (row.state) {
    case "connected":
      return "turn_off";
    case "failed":
      return "reconnect";
    case "needs_sign_in":
      return row.signedIn === true ? "reconnect" : "sign_in";
    case "off":
      return "turn_on";
    case "connecting":
      return undefined;
  }
}

function stateSaid(row: McpServerRow): string {
  switch (row.state) {
    case "connected":
      return row.tools === 1 ? "1 tool" : `${row.tools} tools`;
    case "failed":
      return row.error ?? "failed";
    case "connecting":
      return "connecting";
    case "needs_sign_in":
      return "needs sign-in";
    case "off":
      return "off";
  }
}

/** The Loadout item a live row stands for, so a press opens the same panel the agent's report would. */
export function mcpRefOf(name: string, items: readonly LoadoutItem[]): LoadoutRef {
  const reported = items.find((one) => one.kind === "mcp_server" && one.name === name);
  return reported === undefined ? { kind: "mcp_server", name, source: "mcp", from: name } : { kind: "mcp_server", name, source: reported.source, ...(reported.from === undefined ? {} : { from: reported.from }) };
}

function McpRow({ row, item, opens, onAct }: { row: McpServerRow; item: LoadoutRef; opens: Opens; onAct: McpGroup["onAct"] }) {
  const act = row.acting ?? mcpActOf(row);
  const said = (
    <>
      <span className="armada-loadout__name">{row.name}</span>
      <StateMark state={row.state} />
      <span className="armada-loadout__said">{stateSaid(row)}</span>
    </>
  );
  return (
    <li className="armada-loadout__item armada-loadout__mcp">
      {opens === undefined ? (
        <span className="armada-loadout__row armada-loadout__mcp-said">{said}</span>
      ) : (
        <button type="button" className="armada-loadout__row armada-loadout__press" aria-current={opens.open === loadoutKeyOf(item) ? "true" : undefined} onClick={() => opens.onOpen(item)}>
          {said}
        </button>
      )}
      {act === undefined ? null : (
        <Button variant="secondary" size="sm" pending={row.acting !== undefined} {...(row.refused === undefined ? {} : { answer: "refused" as const })} onClick={() => onAct(row.name, act)}>
          {row.acting === undefined ? ACT_SAID[act].label : ACT_SAID[act].underway}
        </Button>
      )}
      {row.refused === undefined ? null : <p className="armada-loadout__refused">{row.refused}</p>}
    </li>
  );
}

/** Scrolls the group into view when the host asked for it, as typing `/mcp` does. */
export function useRevealed(reveal: boolean) {
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    if (reveal) ref.current?.scrollIntoView({ block: "start" });
  }, [reveal]);
  return ref;
}

/** A hosted Job's MCP servers, each with the one act its state takes. Connected draws no mark, and nothing is counted. */
export function McpServers({ servers, items, said, opens, onAct, reveal }: { servers: readonly McpServerRow[]; items: readonly LoadoutItem[]; said: string | undefined; opens: Opens; onAct: McpGroup["onAct"]; reveal: boolean }) {
  const ref = useRevealed(reveal);
  return (
    <section ref={ref} className="armada-loadout__group" aria-label="MCP servers">
      <h4 className="armada-loadout__eyebrow">MCP servers</h4>
      {said === undefined ? null : <p className="armada-loadout__none">{said}</p>}
      {servers.length === 0 ? (
        <p className="armada-loadout__none">None reported</p>
      ) : (
        <ul className="armada-loadout__rows">
          {servers.map((row) => (
            <McpRow key={row.name} row={row} item={mcpRefOf(row.name, items)} opens={opens} onAct={onAct} />
          ))}
        </ul>
      )}
    </section>
  );
}
