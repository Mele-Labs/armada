import { FilePenLine, GitCommitHorizontal, Power, ScanSearch } from "lucide-react";
import type { ReactNode } from "react";
import type { WorktreeSlot } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** A commit's short form: seven characters, as `git log --oneline` draws it. */
const short = (sha: string) => sha.slice(0, 7);

function Names({ label, items, mono = true }: { label: string; items: readonly string[]; mono?: boolean }) {
  if (items.length === 0) return null;
  return (
    <div className="armada-finding__group">
      <span className="armada-finding__label" aria-hidden>
        {label}
      </span>
      <ul className="armada-finding__names" aria-label={label} data-mono={mono ? "" : undefined}>
        {items.map((one, at) => (
          <li key={`${at}/${one}`}>{one}</li>
        ))}
      </ul>
    </div>
  );
}

/** A mark with no words, named by its tooltip and its accessible name. */
function Mark({ said, children }: { said: string; children: ReactNode }) {
  return (
    <Tooltip label={said}>
      <span className="armada-finding__mark" role="img" aria-label={said}>
        {children}
      </span>
    </Tooltip>
  );
}

/**
 * What a rescue Scout read of a stranded slot, on the bay: the live read while
 * it goes, the Finding when it ends. The summary and `why` are the Scout's;
 * the commits are the slot's own, not its.
 */
export function SlotFinding({ slot }: { slot: WorktreeSlot }) {
  const rescue = slot.rescue;
  if (rescue === undefined) return null;
  const reading = rescue.state === "reading";
  const commits = (slot.stranded?.commits ?? []).map((one) => `${short(one.sha)} ${one.subject}`);
  return (
    <section className="armada-finding" aria-label="Finding" aria-busy={reading || undefined}>
      <div className="armada-finding__meta">
        {reading ? (
          <Mark said="Reading">
            <ScanSearch className="armada-finding__reading" size={12} strokeWidth={2} aria-hidden />
          </Mark>
        ) : null}
        {rescue.state === "stopped" ? (
          <Mark said="Stopped">
            <Power size={12} strokeWidth={2} aria-hidden />
          </Mark>
        ) : null}
        <Tooltip label="Commit it read">
          <span className="armada-finding__commit" aria-label={`Commit it read: ${short(rescue.commit)}`}>
            <GitCommitHorizontal size={12} strokeWidth={2} aria-hidden />
            <span aria-hidden>{short(rescue.commit)}</span>
          </span>
        </Tooltip>
        {rescue.uncommitted ? (
          <Mark said="Uncommitted changes on top">
            <FilePenLine size={12} strokeWidth={2} aria-hidden />
          </Mark>
        ) : null}
        {rescue.cut === undefined || rescue.cut === 0 ? null : (
          <Tooltip label="Characters of the change cut before it read them">
            <span
              className="armada-finding__cut"
              aria-label={`Characters of the change cut before it read them: ${rescue.cut}`}
            >
              {rescue.cut} cut
            </span>
          </Tooltip>
        )}
      </div>
      {rescue.state === "failed" && rescue.why !== undefined ? (
        <p className="armada-finding__failed">{rescue.why}</p>
      ) : null}
      {rescue.summary === undefined ? null : <p className="armada-finding__summary">{rescue.summary}</p>}
      <div className="armada-finding__lists">
        <Names label="Read" items={rescue.read} />
        <Names label="Searched" items={rescue.searched} />
        <Names label="Commits" items={commits} mono={false} />
      </div>
    </section>
  );
}

/**
 * The Scrap's confirm, which names what goes with the checkout: every
 * uncommitted path, and the unpushed commits, the newest `unpushed` of
 * `stranded.commits`.
 */
export function ScrapConfirm({
  slot,
  onScrap,
  onCancel,
}: {
  slot: WorktreeSlot;
  onScrap: () => void;
  onCancel: () => void;
}) {
  const stranded = slot.stranded;
  const unpushed = (stranded?.commits ?? []).slice(0, stranded?.unpushed ?? 0);
  return (
    <div className="armada-finding armada-finding--confirm" role="group" aria-label={`Scrap slot-${slot.slot}`}>
      <div className="armada-finding__lists">
        <Names label="Uncommitted" items={stranded?.uncommitted ?? []} />
        <Names label="Unpushed" items={unpushed.map((one) => `${short(one.sha)} ${one.subject}`)} mono={false} />
      </div>
      <div className="armada-finding__choice">
        <Button variant="destructive" size="sm" onClick={onScrap}>
          Scrap
        </Button>
        <Button variant="ghost" size="sm" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
