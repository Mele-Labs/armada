// The body of a Custom tab. Empty it is the composer the thread uses, where the tab is described; sent, it is the Mod Job's own thread while it builds the tab as a mod of
// kind `tab`; built, it is that mod's module, run in place (`tab-host.tsx`). A built tab is edited from its hover card (`custom-card.tsx`).

import { PanelTop } from "lucide-react";
import { SessionComposer, SessionThread, type SentFromComposer } from "@armada/components";

import { searchFiles } from "../commands";
import { refusal } from "../annotate/send";
import { useNumbered } from "../numbered";
import { threadRowsOf } from "../session-thread-rows";
import { useSessions } from "../sessions-draft";
import { askToTell } from "../tell";
import { TabMod } from "./tab-host";
import { startBuilding, useCustom } from "./custom";

const NONE: readonly never[] = [];
const NOBODY = () => undefined;

/** The first line of the words, cut at a word past 40 characters: what Fleet titles a Mod Job by. */
function titleOf(said: string): string {
  const line = (said.split("\n").find((one) => one.trim() !== "") ?? "").trim();
  if (line.length <= 40) return line;
  const cut = line.slice(0, 40);
  return (cut.includes(" ") ? cut.slice(0, cut.lastIndexOf(" ")) : cut).trimEnd();
}

function Describe({
  tabId,
  jobId,
  manifestId,
}: {
  tabId: string;
  jobId: string;
  manifestId: string | undefined;
}) {
  const numbered = useNumbered(manifestId);
  const keep = `custom:${tabId}`;
  const send = async (sent: SentFromComposer): Promise<boolean> => {
    const started = await window.armada.startModSession({
      title: `Mod ${titleOf(sent.text)}`,
      said: sent.text,
      aim: "tab",
      from_job: jobId,
      from_tab: tabId,
    });
    if (!started.ok) {
      askToTell(`Mod Job not started: ${refusal(started.outcome)}`);
      return false;
    }
    startBuilding(tabId, started.value.session_id);
    return true;
  };
  return (
    <SessionComposer
      textOnly
      bare
      working={false}
      label="Custom"
      model={null}
      effort={null}
      models={NONE}
      efforts={NONE}
      onTune={NOBODY}
      commands={NONE}
      taggable={NONE}
      searchFiles={(query) => searchFiles(query, manifestId)}
      {...(numbered === undefined
        ? {}
        : {
            numbered,
          })}
      tags={NONE}
      onTags={NOBODY}
      drawn={NONE}
      onDraw={NOBODY}
      onRemoveDrawn={NOBODY}
      draftKey={keep}
      focusFor={keep}
      grows="scroller"
      onSend={send}
    />
  );
}

export function CustomBody({
  tabId,
  jobId,
  manifestId,
  onOpenSession,
}: {
  tabId: string /** The Job the tab is open in. */;
  jobId: string;
  manifestId: string | undefined;
  onOpenSession: (id: string) => void;
}) {
  const custom = useCustom(tabId, true);
  const job = useSessions().find((one) => one.id === custom.job);
  if (custom.stage === "building") {
    return (
      <div className="job-custom__thread">
        {job === undefined ? null : (
          <SessionThread
            sessionId={job.id}
            rows={threadRowsOf(job)}
            working={job.turn.state === "working"}
            onAnswer={NOBODY}
            onOpenSession={onOpenSession}
          />
        )}
      </div>
    );
  }
  if (custom.stage === "compose") {
    return (
      <div className="job-custom" data-stage="compose">
        <section className="job-custom__card" aria-label="New tab">
          <PanelTop className="job-custom__glyph" size={32} strokeWidth={1.5} aria-hidden />
          <h3 className="job-custom__title">New tab</h3>
          <p className="job-custom__line">Builds a tab mod you can add to any pane</p>
          <div className="job-custom__box">
            <Describe tabId={tabId} jobId={jobId} manifestId={manifestId} />
          </div>
        </section>
      </div>
    );
  }
  return (
    <div className="job-custom" data-stage="built">
      {custom.mod === undefined ? null : <TabMod mod={custom.mod} jobId={jobId} tabId={tabId} />}
    </div>
  );
}
