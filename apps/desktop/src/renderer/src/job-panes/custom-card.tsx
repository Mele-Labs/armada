// The Mod Job that built a Custom tab, in the app's one hover card: its title and state, the tail of its thread, and a composer that messages it. The tab beside the card changes as the Job edits it.
// It opens from the tab's label or from the tab's menu ("Edit with its Job", which puts the caret in the composer), stays while the pointer or the focus is in it and while the Job works, and Esc puts it away.
// The card is placed against the pane, on the side with room, so it never covers the tab it edits.

import { useEffect, useRef, type ReactElement } from "react";
import { CircleCheck, ExternalLink, LoaderCircle, PanelTop } from "lucide-react";
import { Button, Inspect, InspectCard, SessionComposer, SessionThread, Tooltip } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { select } from "../cockpit/deck/store";
import { plain } from "../cockpit/deck/session-tiles";
import { searchFiles } from "../commands";
import { useNumbered } from "../numbered";
import { threadRowsOf } from "../session-thread-rows";
import { useSessionsDraft } from "../sessions-draft";

const NONE: readonly never[] = [];
const NOBODY = () => undefined;

/** The turns shown: the last few, which is what the Job is doing now. */
const TAIL = 6;

function Tail({ job }: { job: Session }) {
  const box = useRef<HTMLDivElement>(null);
  const rows = threadRowsOf(job).slice(-TAIL);
  // The newest turn stays in view as the Job writes.
  useEffect(() => {
    const el = box.current;
    if (el !== null) el.scrollTop = el.scrollHeight;
  });
  return (
    <div ref={box} className="job-card__thread">
      <SessionThread sessionId={job.id} rows={rows} working={job.turn.state === "working"} onAnswer={NOBODY} onOpenSession={NOBODY} />
    </div>
  );
}

function Message({ job, tabId }: { job: Session; tabId: string }) {
  const draft = useSessionsDraft();
  const numbered = useNumbered(job.manifestId);
  const keep = `custom-card:${tabId}`;
  return (
    <SessionComposer
      textOnly
      bare
      working={false}
      label="Message the Mod Job"
      model={null}
      effort={null}
      models={NONE}
      efforts={NONE}
      onTune={NOBODY}
      commands={NONE}
      taggable={NONE}
      searchFiles={(query) => searchFiles(query, job.manifestId)}
      {...(numbered === undefined ? {} : { numbered })}
      tags={NONE}
      onTags={NOBODY}
      drawn={NONE}
      onDraw={NOBODY}
      onRemoveDrawn={NOBODY}
      draftKey={keep}
      focusFor={keep}
      grows="scroller"
      onSend={async (sent) => (await draft?.send(job.id, plain(sent.text))) !== false}
    />
  );
}

/**
 * `children` is the tab's label, which the card hangs off. `summon` opens the card with the caret in its composer, as the tab's menu asks; `onSummoned` lets go of the ask. With no Job, or
 * while the first build is still the tab's own body, the label is drawn as it is.
 */
export function ModJobCard({ tabId, job, building, summon, onSummoned, children }: { tabId: string; job: Session | undefined; building: boolean; summon: boolean; onSummoned: () => void; children: ReactElement }) {
  if (job === undefined || building) return children;
  const working = job.turn.state === "working";
  const title = job.title ?? "Mod Job";
  const Glyph = working ? LoaderCircle : CircleCheck;
  return (
    <Inspect
      name={title}
      side="beside"
      around=".job-pane"
      pin={false}
      held={working}
      summoned={summon}
      onSummoned={onSummoned}
      to='[role="textbox"]'
      card={
        <InspectCard
          kind={{ Glyph: PanelTop, label: "Mod Job" }}
          width="wide"
          title={
            <span className="job-card__title">
              <Tooltip label={working ? "Working" : "Done"} asChild>
                <span className="job-card__state" role="img" aria-label={working ? "Working" : "Done"} data-working={working || undefined}>
                  <Glyph size={16} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
              <span>{title}</span>
            </span>
          }
        >
          <Tail job={job} />
          <Message job={job} tabId={tabId} />
          <Button variant="secondary" size="sm" onClick={() => select({ kind: "session", key: `session:${job.id}` })}>
            <ExternalLink size={16} strokeWidth={2} aria-hidden />
            Open Job
          </Button>
        </InspectCard>
      }
    >
      {children}
    </Inspect>
  );
}
