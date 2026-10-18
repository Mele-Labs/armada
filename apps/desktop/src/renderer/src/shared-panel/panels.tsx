// The issue, the bay and the Check, in the pull request panel's shape: a head, a title, what it is in one line, its acts, and tabs.
// What each draws is what its read serves and nothing else; an absent fact leaves its slot empty.

import { useState } from "react";
import type { ReactNode } from "react";
import type { SheetBack } from "@armada/components";
import { Badge, Button, PrConversation, PrMark, Tooltip, agoOf, ranFor } from "@armada/components";
import { CircleCheck, CircleDot, DoorClosedLocked, DoorOpen, ExternalLink, Ghost, GitMerge, KeyRound, GitPullRequest, LoaderCircle, ShieldCheck, ShieldX, SquareTerminal } from "lucide-react";
import type { PullRequestEntry } from "@armada/protocol";

import { SidePanel } from "./side";
import { openInPanel, panelBack, closePanel, usePanel } from "./store";
import type { PanelItem } from "./store";
import { usePanelReads, useRead } from "./reads";
import type { BayRead, CheckRead, IssueRead } from "./reads";
import { askToOpenSession } from "../open-session";

type Of<K extends PanelItem["kind"]> = Extract<PanelItem, { kind: K }>;

function Open({ address, onOpenLink }: { address: string; onOpenLink: (address: string) => void }) {
  return (
    <span className="armada-pr-panel__open">
      <Tooltip label="Open in browser">
        <Button variant="ghost" size="sm" iconOnly aria-label="Open in browser" onClick={() => onOpenLink(address)}>
          <ExternalLink size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    </span>
  );
}

/** A row that opens something in this same panel. */
function Press({ columns, label, onPress, children }: { columns: "link" | "run"; label: string; onPress: () => void; children: ReactNode }) {
  return (
    <li className="armada-pr-row" data-press>
      <button type="button" className="armada-pr-row__button side-row" data-columns={columns} aria-label={label} onClick={onPress}>
        {children}
      </button>
    </li>
  );
}

/** The way back to the item this one was opened from, where it was opened from another. */
export function useBack(): SheetBack | undefined {
  const was = usePanel().back;
  return was === undefined ? undefined : { label: `Back to ${nameOf(was)}`, tooltip: `Back to ${nameOf(was)}`, onBack: panelBack };
}

const nameOf = (item: PanelItem): string => (item.kind === "pr" ? `#${item.number}` : item.kind === "issue" ? `#${item.number}` : item.kind === "bay" ? `slot-${item.slot}` : (item.name ?? "Check"));

// ---- An issue -----------------------------------------------------------------------------------------

export function IssuePanel({ item, onOpenLink }: { item: Of<"issue">; onOpenLink: (address: string) => void }) {
  const reads = usePanelReads();
  const [tab, setTab] = useState("conversation");
  const back = useBack();
  const read = useRead<IssueRead>(() => reads?.issue(item) ?? Promise.resolve(undefined), `${item.root ?? ""}#${item.number}`);
  if (read === undefined) return null;
  const entries: PullRequestEntry[] = read.comments.map((one) => ({ kind: "comment", author: one.author, at: one.at, body: one.body }));
  return (
    <SidePanel
      title={`Issue #${read.number}`}
      repository={read.repository}
      heading={read.title}
      meta={
        <>
          <Badge status={read.state === "open" ? "running" : "completed-success"} icon={read.state === "open" ? CircleDot : CircleCheck}>
            {read.state === "open" ? "Open" : "Closed"}
          </Badge>
          {read.labels.map((one) => (
            <Tooltip key={one} label="Label">
              <span className="armada-pr-panel__branch mono">{one}</span>
            </Tooltip>
          ))}
          <Tooltip label="Opened by">
            <span className="armada-pr-panel__who">{read.author}</span>
          </Tooltip>
          <Tooltip label="Opened">
            <span className="armada-pr-panel__who">{agoOf(read.openedAt)}</span>
          </Tooltip>
          {read.assignees.map((one) => (
            <Tooltip key={one} label="Assigned to">
              <span className="armada-pr-panel__who">{one}</span>
            </Tooltip>
          ))}
        </>
      }
      acts={<Open address={read.address} onOpenLink={onOpenLink} />}
      tabs={[
        { id: "conversation", label: "Conversation" },
        { id: "linked", label: "Pull requests" },
      ]}
      tab={tab}
      onTab={setTab}
      back={back}
      onClose={closePanel}
    >
      {tab === "conversation" ? <PrConversation page={{ author: read.author, created_at: read.openedAt, conversation: entries, checks: [], files: [], commits: [] }} description={read.body} /> : null}
      {tab === "linked" ? (
        <ul className="armada-pr-list armada-pr-rows" aria-label="Pull requests">
          {read.linked.map((one) => (
            <Press key={one.number} columns="link" label={`Pull request #${one.number} ${one.title}`} onPress={() => openInPanel({ kind: "pr", number: one.number, address: one.address, title: one.title })}>
              <PrMark Glyph={one.state === "merged" ? GitMerge : GitPullRequest} said={one.state === "merged" ? "Merged" : "Open"} tone={one.state === "merged" ? "success" : undefined} />
              <span className="armada-pr-row__name">{`#${one.number} ${one.title}`}</span>
              <span />
            </Press>
          ))}
        </ul>
      ) : null}
    </SidePanel>
  );
}

// ---- A bay ----------------------------------------------------------------------------------------------

const BAY = {
  leased: { status: "running", Glyph: KeyRound },
  free: { status: "completed-success", Glyph: DoorOpen },
  stranded: { status: "awaiting-review", Glyph: Ghost },
  closed: { status: "not-started", Glyph: DoorClosedLocked },
} as const;

export function BayPanel({ item }: { item: Of<"bay"> }) {
  const reads = usePanelReads();
  const [tab, setTab] = useState("facts");
  const back = useBack();
  const read = useRead<BayRead>(() => reads?.bay(item) ?? Promise.resolve(undefined), `${item.manifestId}/${item.slot}`);
  if (read === undefined) return null;
  const facts: readonly (readonly [string, ReactNode, boolean?])[] = [
    ["Holder", read.holder === undefined ? undefined : <HolderLink id={read.holder.id} title={read.holder.title} />],
    ["Branch", read.branch, true],
    ["Held for", read.heldFor],
    ["Base", read.base, true],
    ["Behind", read.behind > 0 ? `${read.behind} commits` : undefined],
    ["Checkout", read.warm ? "Warm" : "Cold"],
    ["Uncommitted", read.uncommitted.length === 0 ? undefined : read.uncommitted.join("\n"), true],
    ["Unpushed", read.unpushed.length === 0 ? undefined : read.unpushed.join("\n"), true],
    ["Path", read.path, true],
  ];
  return (
    <SidePanel
      title={`Bay ${read.name}`}
      repository={read.repository}
      heading={read.name}
      meta={
        <Badge status={BAY[read.state].status} icon={BAY[read.state].Glyph}>
          {read.word}
        </Badge>
      }
      tabs={[
        { id: "facts", label: "Facts" },
        { id: "history", label: "History" },
      ]}
      tab={tab}
      onTab={setTab}
      back={back}
      onClose={closePanel}
    >
      {tab === "facts" ? (
        <dl className="side-facts" aria-label="Facts">
          {facts
            .filter(([, value]) => value !== undefined && value !== "")
            .map(([term, value, mono]) => (
              <div key={term} className="side-facts__row">
                <dt>{term}</dt>
                <dd data-mono={mono || undefined}>{value}</dd>
              </div>
            ))}
        </dl>
      ) : null}
      {tab === "history" ? (
        <ul className="armada-pr-list armada-pr-rows" aria-label="History">
          {read.history.map((one, at) => (
            <li key={`${at}:${one.who}`} className="armada-pr-row" data-columns="commit">
              <Tooltip label="Held">
                <span className="armada-pr-row__sha mono">{one.span}</span>
              </Tooltip>
              <span className="armada-pr-row__name">{one.who}</span>
              {one.branch === undefined ? <span /> : <span className="armada-pr-row__author mono">{one.branch}</span>}
            </li>
          ))}
        </ul>
      ) : null}
    </SidePanel>
  );
}

function HolderLink({ id, title }: { id: string; title: string }) {
  return (
    <Button variant="ghost" size="sm" aria-label={`Job: ${title}`} onClick={() => (closePanel(), askToOpenSession(id))}>
      <SquareTerminal size={16} strokeWidth={2} aria-hidden />
      {title}
    </Button>
  );
}

// ---- A Check --------------------------------------------------------------------------------------------

const CHECK = {
  passed: { Glyph: ShieldCheck, said: "Passed", tone: "success", status: "completed-success" },
  failed: { Glyph: ShieldX, said: "Failed", tone: "failed", status: "completed-failed" },
  running: { Glyph: LoaderCircle, said: "Running", tone: "running", status: "running" },
} as const;

export function CheckPanel({ item }: { item: Of<"check"> }) {
  const reads = usePanelReads();
  const [tab, setTab] = useState("runs");
  const back = useBack();
  const read = useRead<CheckRead>(() => reads?.check(item) ?? Promise.resolve(undefined), item.id);
  if (read === undefined) return null;
  const now = CHECK[read.state];
  return (
    <SidePanel
      title={`Check ${read.name}`}
      repository={read.repository}
      heading={read.name}
      meta={
        <Badge status={now.status} icon={now.Glyph}>
          {now.said}
        </Badge>
      }
      tabs={[
        { id: "runs", label: "Runs" },
        { id: "log", label: "Log" },
      ]}
      tab={tab}
      onTab={setTab}
      back={back}
      onClose={closePanel}
    >
      {tab === "runs" ? (
        <ul className="armada-pr-list armada-pr-rows" aria-label="Runs">
          {read.runs.map((one, at) =>
            one.pull === undefined ? (
              <li key={`${at}:${one.on}`} className="armada-pr-row" data-columns="run">
                <RunCells run={one} />
              </li>
            ) : (
              <Press key={`${at}:${one.on}`} columns="run" label={`Pull request #${one.pull.number}`} onPress={() => openInPanel({ kind: "pr", number: one.pull!.number, address: one.pull!.address })}>
                <RunCells run={one} />
              </Press>
            ),
          )}
        </ul>
      ) : null}
      {tab === "log" ? (
        <section className="side-log" aria-label={`Log of ${read.log.run}`}>
          <pre className="side-log__lines mono">{read.log.lines.join("\n")}</pre>
        </section>
      ) : null}
    </SidePanel>
  );
}

function RunCells({ run }: { run: CheckRead["runs"][number] }) {
  const state = CHECK[run.state];
  return (
    <>
      <PrMark Glyph={state.Glyph} said={state.said} tone={state.tone} />
      <span className="armada-pr-row__name mono">{run.on}</span>
      {run.seconds === undefined ? <span /> : <span className="armada-pr-row__figure mono">{ranFor(run.seconds)}</span>}
      <span className="armada-pr-row__figure">{run.ago}</span>
    </>
  );
}
