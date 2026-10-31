// The body of a Custom tab. Empty it is the composer the thread uses, where the tab is described; sent, it is the Mod Job's own thread while it builds the tab as a mod of
// kind `tab`; built, it is that mod's view. "Edit with a Job" opens the composer again above the view.

import { X } from "lucide-react";
import { Button, SessionComposer, SessionThread, Tooltip, type SentFromComposer } from "@armada/components";

import { searchFiles } from "../commands";
import { refusal } from "../annotate/send";
import { useNumbered } from "../numbered";
import { threadRowsOf } from "../session-thread-rows";
import { useSessions } from "../sessions-draft";
import { askToTell } from "../tell";
import { VIEWS } from "./custom-views";
import { setEditing, startBuilding, tabTitle, useCustom } from "./custom";

const NONE: readonly never[] = [];
const NOBODY = () => undefined;

/** The first line of the words, cut at a word past 40 characters: what Fleet titles a Mod Job by. */
function titleOf(said: string): string {
  const line = (said.split("\n").find((one) => one.trim() !== "") ?? "").trim();
  if (line.length <= 40) return line;
  const cut = line.slice(0, 40);
  return (cut.includes(" ") ? cut.slice(0, cut.lastIndexOf(" ")) : cut).trimEnd();
}

function Describe({ tabId, manifestId }: { tabId: string; manifestId: string | undefined }) {
  const numbered = useNumbered(manifestId);
  const keep = `custom:${tabId}`;
  const send = async (sent: SentFromComposer): Promise<boolean> => {
    const started = await window.armada.startModSession({ title: `Mod ${titleOf(sent.text)}`, said: sent.text });
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
      {...(numbered === undefined ? {} : { numbered })}
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

export function CustomBody({ tabId, manifestId, onOpenSession }: { tabId: string; manifestId: string | undefined; onOpenSession: (id: string) => void }) {
  const custom = useCustom(tabId, true);
  const job = useSessions().find((one) => one.id === custom.job);
  if (custom.stage === "building") {
    return (
      <div className="job-custom__thread">
        {job === undefined ? null : <SessionThread sessionId={job.id} rows={threadRowsOf(job)} working={job.turn.state === "working"} onAnswer={NOBODY} onOpenSession={onOpenSession} />}
      </div>
    );
  }
  if (custom.stage === "compose") {
    return (
      <div className="job-custom" data-stage="compose">
        <Describe tabId={tabId} manifestId={manifestId} />
      </div>
    );
  }
  const View = custom.mod === undefined ? undefined : VIEWS[custom.mod];
  return (
    <div className="job-custom" data-stage="built">
      {!custom.editing ? null : (
        <div className="job-custom__edit">
          <Describe tabId={tabId} manifestId={manifestId} />
          <Tooltip label="Close">
            <Button variant="ghost" size="sm" iconOnly aria-label={`Close the change to ${tabTitle(custom.mod ?? "")}`} onClick={() => setEditing(tabId, false)}>
              <X size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </div>
      )}
      {View === undefined ? null : <View />}
    </div>
  );
}
