import { useEffect, useRef, useState } from "react";
import { Plus } from "lucide-react";
import type { KeyboardEvent } from "react";

import { useKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Popover } from "../../primitives/Popover/Popover";
import { ResponseEditor } from "../../primitives/Prose/ResponseEditor";
import { MAX_RESPONSES, actOnResponses } from "../../primitives/Prose/responses";
import type { ResponseAct } from "../../primitives/Prose/responses";
import { Select } from "../../primitives/Select/Select";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { QuickEdit, QuickKindMark, QuickPinnedMark } from "../SessionComposer/QuickBar";
import { THREAD_TEXT_SIZES, VOICE_ENGINES, actOnQuick, pinnedFirst, useThreadLook } from "./thread-look";
import type { QuickAct } from "./thread-look";
import type { QuickPhrase, ThreadLook, ThreadSpacing, ThreadTextSize, VoiceEngine, WhisperModel } from "./thread-look";

const SPACINGS: readonly { id: ThreadSpacing; said: string }[] = [
  { id: "compact", said: "Compact" },
  { id: "default", said: "Default" },
  { id: "roomy", said: "Roomy" },
];

export type ThreadSettingsProps = {
  look: ThreadLook;
  onChange: (change: (was: ThreadLook) => ThreadLook) => void;
  /** Where Talk hears from, and Whisper's model. Absent draws no Voice: the Aa panel leaves it to System. */
  voice?: { whisper: WhisperModel; onInstall: () => void };
  /** The first field takes the focus as the panel opens. */
  autoFocus?: boolean;
  /** What the new line's field is kept under, so a line half typed survives the panel closing. */
  keep: string;
  /** Drawn in the Aa panel, which takes its width from the form. */
  inPanel?: boolean;
  /** The Job the form is for. Absent, as in System, the quick bar has the one switch, for all Jobs. */
  jobId?: string;
};

/** Moves the line at `from` by `by`, within the list. */
function moved(list: readonly QuickPhrase[], from: number, by: number): QuickPhrase[] {
  const to = from + by;
  if (to < 0 || to >= list.length) return [...list];
  const next = [...list];
  const [one] = next.splice(from, 1);
  next.splice(to, 0, one!);
  return next;
}

/**
 * How a Job's thread reads, and the quick bar's lines. **One form for the Aa panel and System › Thread**, so the two never
 * differ; System adds Voice.
 */
export function ThreadSettings({ look, onChange, voice, autoFocus = false, keep, inPanel = false, jobId }: ThreadSettingsProps) {
  const first = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (autoFocus) first.current?.focus();
  }, [autoFocus]);
  const [adding, setAdding, clearAdding] = useKept<string>(keep, "");
  const [editing, setEditing] = useState<string | undefined>(undefined);
  const act = (what: QuickAct) => onChange((was) => actOnQuick(was, what));
  // The response being edited: its id, or `new` for the blank editor. Under its button, as a block, since the Aa panel is a layer already.
  const [editingResponse, setEditingResponse] = useState<string | undefined>(undefined);
  const row = useRef<HTMLDivElement>(null);
  const refocus = useRef<string | undefined>(undefined);
  const actOnResponse = (what: ResponseAct) => onChange((was) => ({ ...was, responses: actOnResponses(was.responses, what) }));
  // Moving a response moves its button in the document, which drops the focus; put it back where the key left it.
  useEffect(() => {
    if (refocus.current === undefined) return;
    row.current?.querySelector<HTMLElement>(`[data-response="${refocus.current}"]`)?.focus();
    refocus.current = undefined;
  }, [look.responses]);
  const set = <K extends keyof ThreadLook>(key: K, value: ThreadLook[K]) => onChange((was) => ({ ...was, [key]: value }));
  const add = () => {
    const text = adding.trim();
    if (text === "") return;
    onChange((was) => ({ ...was, quick: [...was.quick, { id: `saved:${Date.now()}`, text, kind: "saved" }] }));
    clearAdding();
  };
  // ⌥↑ and ⌥↓ move a line from anywhere on its row.
  const onRowKey = (at: number) => (event: KeyboardEvent) => {
    if (!event.altKey || (event.key !== "ArrowUp" && event.key !== "ArrowDown")) return;
    event.preventDefault();
    onChange((was) => ({ ...was, quick: pinnedFirst(moved(was.quick, at, event.key === "ArrowUp" ? -1 : 1)) }));
  };
  return (
    <div className="armada-thread-settings" data-in={inPanel ? "panel" : undefined}>
      <Input ref={first} label="Agent name" value={look.agentName} onChange={(event) => set("agentName", event.currentTarget.value)} />
      <Switch checked={look.showAgentName} onChange={(event) => set("showAgentName", event.currentTarget.checked)}>
        Show agent name
      </Switch>
      <Switch checked={look.showTimes} onChange={(event) => set("showTimes", event.currentTarget.checked)}>
        Show times
      </Switch>
      <div className="armada-thread-settings__pair">
        <Select label="Text size" value={look.textSize} onChange={(event) => set("textSize", event.currentTarget.value as ThreadTextSize)}>
          {THREAD_TEXT_SIZES.map((one) => (
            <option key={one.size} value={one.size}>
              {one.said}
            </option>
          ))}
        </Select>
        <Select label="Spacing" value={look.spacing} onChange={(event) => set("spacing", event.currentTarget.value as ThreadSpacing)}>
          {SPACINGS.map((one) => (
            <option key={one.id} value={one.id}>
              {one.said}
            </option>
          ))}
        </Select>
      </div>
      <section className="armada-thread-settings__responses" aria-label="Quick responses">
        <h3 className="armada-thread-settings__head">Quick responses</h3>
        <div ref={row} className="armada-thread-settings__emojis">
          {look.responses.map((one) => (
            <Tooltip key={one.id} label={one.name}>
              <Button
                variant="ghost"
                size="sm"
                iconOnly
                aria-label={`Edit ${one.name}`}
                aria-expanded={editingResponse === one.id}
                aria-keyshortcuts="Alt+ArrowLeft Alt+ArrowRight"
                data-response={one.id}
                onClick={() => setEditingResponse(editingResponse === one.id ? undefined : one.id)}
                onKeyDown={(event) => {
                  if (!event.altKey || (event.key !== "ArrowLeft" && event.key !== "ArrowRight")) return;
                  event.preventDefault();
                  refocus.current = one.id;
                  actOnResponse({ act: "move", id: one.id, by: event.key === "ArrowLeft" ? -1 : 1 });
                }}
              >
                <span className="armada-prose__pick-emoji" aria-hidden>{one.emoji}</span>
              </Button>
            </Tooltip>
          ))}
          <Tooltip label="Add response">
            <Button variant="ghost" size="sm" iconOnly aria-label="Add response" disabled={look.responses.length >= MAX_RESPONSES} aria-expanded={editingResponse === "new"} onClick={() => setEditingResponse(editingResponse === "new" ? undefined : "new")}>
              <Plus size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </div>
        {editingResponse === undefined ? null : (
          <div className="armada-thread-settings__editor">
            <ResponseEditor
              key={editingResponse}
              keep={`${keep}:response`}
              {...(editingResponse === "new" ? {} : { response: look.responses.find((one) => one.id === editingResponse)! })}
              onSave={(emoji, name) => {
                actOnResponse(editingResponse === "new" ? { act: "add", emoji, name } : { act: "edit", id: editingResponse, emoji, name });
                setEditingResponse(undefined);
              }}
              {...(editingResponse === "new" || look.responses.length <= 1 ? {} : { onRemove: () => { actOnResponse({ act: "remove", id: editingResponse }); setEditingResponse(undefined); } })}
              onClose={() => setEditingResponse(undefined)}
            />
          </div>
        )}
      </section>
      <section className="armada-thread-settings__quick" aria-label="Quick replies">
        <h3 className="armada-thread-settings__head">Quick replies</h3>
        {jobId === undefined ? null : (
          <Switch
            checked={!look.quickOff && !look.quickOffFor.includes(jobId)}
            disabled={look.quickOff}
            onChange={(event) => {
              const on = event.currentTarget.checked;
              onChange((was) => ({ ...was, quickOffFor: on ? was.quickOffFor.filter((one) => one !== jobId) : [...was.quickOffFor.filter((one) => one !== jobId), jobId] }));
            }}
          >
            In this Job
          </Switch>
        )}
        <Switch checked={!look.quickOff} onChange={(event) => set("quickOff", !event.currentTarget.checked)}>
          In all Jobs
        </Switch>
        <ol className="armada-thread-settings__lines">
          {look.quick.map((one, at) => (
            <li key={one.id} className="armada-thread-settings__line" onKeyDown={onRowKey(at)}>
              <QuickKindMark kind={one.kind} />
              {one.pinned === true ? <QuickPinnedMark /> : null}
              {editing === one.id ? (
                <QuickEdit
                  text={one.text}
                  label={`Edit ${one.text}`}
                  onDone={(text) => {
                    setEditing(undefined);
                    if (text !== undefined) act({ act: "edit", id: one.id, text });
                  }}
                />
              ) : (
                <>
                  <span className="armada-thread-settings__text">{one.text}</span>
                  <span className="armada-thread-settings__acts">
                    {one.kind === "learnt" ? (
                      <Button size="sm" variant="ghost" aria-label={`Save ${one.text}`} onClick={() => onChange((was) => ({ ...was, quick: was.quick.map((it) => (it.id === one.id ? { ...it, kind: "saved" } : it)) }))}>
                        Save
                      </Button>
                    ) : null}
                    <Button size="sm" variant="ghost" aria-label={`Edit ${one.text}`} onClick={() => setEditing(one.id)}>
                      Edit
                    </Button>
                    <Button size="sm" variant="ghost" aria-label={`${one.pinned === true ? "Unpin" : "Pin"} ${one.text}`} onClick={() => act({ act: one.pinned === true ? "unpin" : "pin", id: one.id })}>
                      {one.pinned === true ? "Unpin" : "Pin"}
                    </Button>
                    <Button size="sm" variant="ghost" aria-label={`Move ${one.text} up`} aria-keyshortcuts="Alt+ArrowUp" disabled={at === 0} onClick={() => onChange((was) => ({ ...was, quick: pinnedFirst(moved(was.quick, at, -1)) }))}>
                      Up
                    </Button>
                    <Button size="sm" variant="ghost" aria-label={`Move ${one.text} down`} aria-keyshortcuts="Alt+ArrowDown" disabled={at === look.quick.length - 1} onClick={() => onChange((was) => ({ ...was, quick: pinnedFirst(moved(was.quick, at, 1)) }))}>
                      Down
                    </Button>
                    <Button size="sm" variant="ghost" aria-label={`Delete ${one.text}`} onClick={() => act({ act: "delete", id: one.id })}>
                      Delete
                    </Button>
                  </span>
                </>
              )}
            </li>
          ))}
        </ol>
        <form
          className="armada-thread-settings__add"
          onSubmit={(event) => {
            event.preventDefault();
            add();
          }}
        >
          <Input aria-label="New quick reply" value={adding} onChange={(event) => setAdding(event.currentTarget.value)} />
          <Button type="submit" size="sm" variant="secondary" disabled={adding.trim() === ""}>
            Add
          </Button>
        </form>
        <Switch checked={look.learn} onChange={(event) => set("learn", event.currentTarget.checked)}>
          Learn from my messages
        </Switch>
      </section>
      {voice === undefined ? null : (
        <section className="armada-thread-settings__voice" aria-label="Voice">
          <Select label="Voice" value={look.voice} onChange={(event) => set("voice", event.currentTarget.value as VoiceEngine)}>
            {VOICE_ENGINES.map((one) => (
              <option key={one.id} value={one.id}>
                {one.said}
              </option>
            ))}
          </Select>
          {look.voice !== "whisper" ? null : voice.whisper.state === "absent" ? (
            <div className="armada-thread-settings__model">
              <span className="armada-thread-settings__model-name">Whisper model</span>
              <Button size="sm" variant="secondary" onClick={voice.onInstall}>
                Install
              </Button>
            </div>
          ) : voice.whisper.state === "installing" ? (
            <div className="armada-thread-settings__model">
              <span className="armada-thread-settings__model-name">Whisper model</span>
              <span className="armada-thread-settings__bar" role="progressbar" aria-label="Installing Whisper model" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(voice.whisper.progress * 100)}>
                <span className="armada-thread-settings__fill" style={{ inlineSize: `${Math.round(voice.whisper.progress * 100)}%` }} />
              </span>
            </div>
          ) : (
            <div className="armada-thread-settings__model">
              <span className="armada-thread-settings__model-name">Whisper model</span>
              <span className="armada-thread-settings__installed">Installed</span>
            </div>
          )}
        </section>
      )}
    </div>
  );
}

/**
 * Aa, in a Job's head: the thread's look in a panel, changed live. Esc or a press outside puts it away. Draws nothing where the
 * window holds no look (`ThreadLookContext`).
 */
export function ThreadLookButton({ jobId }: { jobId?: string }) {
  const source = useThreadLook();
  if (source === null) return null;
  return (
    <Popover
      align="end"
      label="Thread look"
      trigger={
        <Tooltip label="Thread look">
          <Button variant="ghost" size="sm" aria-label="Thread look" aria-haspopup="dialog" className="armada-thread-look__button">
            Aa
          </Button>
        </Tooltip>
      }
    >
      <ThreadSettings look={source.look} onChange={source.setLook} autoFocus keep="thread-look:new-line" inPanel {...(jobId === undefined ? {} : { jobId })} />
    </Popover>
  );
}
