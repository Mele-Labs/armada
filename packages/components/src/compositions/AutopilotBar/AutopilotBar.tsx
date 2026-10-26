// One repository's Autopilot, on every Cockpit filter so Pause is one press away: the switch beside its
// name, what it is doing, its caps used of their limit, and Pause, Resume and Stop. Off, the switch opens
// the start form, and nothing starts until the person has seen what Fleet read, and the words it read each
// setting from, and pressed Start. A reached cap pauses it, and Edit caps raises it in the same fields.
// #2092, #2105, `docs/concepts/autopilot.md`.

import { useEffect, useRef, useState, type CSSProperties, type FormEvent } from "react";
import { Plane } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { Switch } from "../../primitives/Switch/Switch";
import { Textarea } from "../../primitives/Textarea/Textarea";

/** One cap, used of its limit. `fill` is 0 to 1; `reached` lights it. `at` is an RFC 3339 instant drawn as a stamp before `value`. */
export type AutopilotCap = { label: string; value: string; at?: string; fill: number; reached: boolean };

/** The four settings as the start form holds them, each a field's text. */
export type AutopilotFields = { atOnce: string; until: string; spend: string; sessions: string };

/** What Fleet read the sentence as: the fields, and the person's own words each was read from, where it said any. */
export type AutopilotReading = { fields: AutopilotFields; quoted: Readonly<Partial<Record<keyof AutopilotFields, string>>> };

export type AutopilotPress = "pause" | "resume" | "stop" | "read" | "start" | "save";

export type AutopilotBarProps = {
  /** The repository it works, named where several bars are drawn: on All. */
  repository?: string | undefined;
  /** `off` where there is none, or the last one ended. */
  state: "off" | "running" | "paused";
  /** The person's sentence, while it runs. */
  instruction?: string | undefined;
  /** Which cap paused it, where one did: `Session cap reached`. The state says it, and Resume waits for it to be raised. */
  capped?: string | undefined;
  /** Why its last pass picked nothing, in Fleet's words. */
  fault?: string | undefined;
  caps: readonly AutopilotCap[];
  /** What Stop does to this run's picks, named. */
  stopping: string;
  /** The press Fleet has not answered. */
  pending?: AutopilotPress | undefined;
  /** What Fleet refused, said where the press was. */
  refused?: string | undefined;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  /** The start form is open: the switch was turned on, or the one bar sent a sentence here. */
  starting: boolean;
  onStarting: (open: boolean) => void;
  sentence: string;
  onSentence: (sentence: string) => void;
  /** What Fleet read the sentence as. Absent until it is read. */
  reading?: AutopilotReading | undefined;
  onRead: () => void;
  onStart: (fields: AutopilotFields) => void;
  /** Edit caps is open, holding `current`: what it runs with now. */
  editing: boolean;
  onEditing: (open: boolean) => void;
  current?: AutopilotFields | undefined;
  onSave: (fields: AutopilotFields) => void;
};

const STATE: Record<AutopilotBarProps["state"], string> = { off: "Off", running: "Running", paused: "Paused · started Jobs finish" };

/** The state as the bar says it: a cap that paused it is named in place of what Pause means. */
const stateOf = (state: AutopilotBarProps["state"], capped: string | undefined) => (state === "paused" && capped !== undefined ? `Paused · ${capped}` : STATE[state]);

const FIELDS: readonly (readonly [keyof AutopilotFields, string, string])[] = [
  ["atOnce", "At once", "1"],
  ["until", "Until", "no time set"],
  ["spend", "Spend, in dollars", "no cap"],
  ["sessions", "Jobs", "no cap"],
];

function Caps({ caps }: { caps: readonly AutopilotCap[] }) {
  return (
    <dl className="armada-autopilot__caps">
      {caps.map((cap) => (
        <div key={cap.label} className="armada-autopilot__cap" data-reached={cap.reached || undefined}>
          <dt>{cap.label}</dt>
          <dd>
            {cap.at === undefined ? null : <Stamp at={cap.at} />}
            {cap.at !== undefined && cap.value !== "" ? " · " : null}
            {cap.value}
            {cap.reached ? <span className="armada-autopilot__reached"> · reached</span> : null}
          </dd>
          <dd className="armada-autopilot__meter" aria-hidden="true" style={{ ["--fill" as string]: `${Math.round(Math.min(1, Math.max(0, cap.fill)) * 100)}%` } as CSSProperties} />
        </div>
      ))}
    </dl>
  );
}

/** The four settings as fields, each with the words it was read from where the form is a reading. */
function Fields({ fields, onChange, quoted }: { fields: AutopilotFields; onChange: (fields: AutopilotFields) => void; quoted?: AutopilotReading["quoted"] }) {
  return (
    <span className="armada-autopilot__fields">
      {FIELDS.map(([key, label, placeholder]) => (
        <span key={key} className="armada-autopilot__field">
          <Input label={label} mono value={fields[key]} placeholder={placeholder} onChange={(event) => onChange({ ...fields, [key]: event.currentTarget.value })} />
          {quoted === undefined ? null : <span className="armada-autopilot__said">{quoted[key] === undefined ? "not said" : `“${quoted[key]}”`}</span>}
        </span>
      ))}
    </span>
  );
}

export { Fields as AutopilotFieldInputs };

function StartForm({ sentence, onSentence, reading, onRead, onStart, pending }: Pick<AutopilotBarProps, "sentence" | "onSentence" | "reading" | "onRead" | "onStart" | "pending">) {
  const [fields, setFields] = useState<AutopilotFields | undefined>(reading?.fields);
  // A new reading replaces whatever was typed over the last one.
  useEffect(() => setFields(reading?.fields), [reading]);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (fields === undefined) return onRead();
    onStart(fields);
  };
  return (
    <form className="armada-autopilot__start" aria-label="Start Autopilot" onSubmit={submit}>
      <Textarea
        label="Tell Autopilot what to work on, in your own words"
        rows={2}
        value={sentence}
        onChange={(event) => onSentence(event.currentTarget.value)}
      />
      {fields === undefined || reading === undefined ? (
        <span className="armada-autopilot__foot">
          <span>Fleet reads it as settings for you to check. Nothing starts until you press Start.</span>
          <Button variant="secondary" size="sm" type="submit" disabled={sentence.trim() === ""} pending={pending === "read"}>
            {pending === "read" ? "Reading…" : "Read it"}
          </Button>
        </span>
      ) : (
        <>
          <span className="armada-autopilot__read">
            Fleet read this as:
            <Button variant="ghost" size="sm" onClick={onRead} disabled={sentence.trim() === "" || pending !== undefined} pending={pending === "read"}>
              Read again
            </Button>
          </span>
          <Fields fields={fields} onChange={setFields} quoted={reading.quoted} />
          <span className="armada-autopilot__foot">
            <span>Clear picks start on their own. Picks it is unsure of are set aside for you. A cap pauses the next pick, never a Job that is running.</span>
            <Button variant="primary" size="sm" type="submit" disabled={pending !== undefined && pending !== "start"} pending={pending === "start"}>
              Start Autopilot
            </Button>
          </span>
        </>
      )}
    </form>
  );
}

/** The settings it runs with, in the start form's fields, saved in its state: never a resume. */
function EditForm({ current, onSave, onEditing, pending, capped }: Pick<AutopilotBarProps, "onSave" | "onEditing" | "pending" | "capped"> & { current: AutopilotFields }) {
  const [fields, setFields] = useState(current);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    onSave(fields);
  };
  return (
    <form className="armada-autopilot__start" aria-label="Edit caps" onSubmit={submit}>
      <Fields fields={fields} onChange={setFields} />
      <span className="armada-autopilot__foot">
        <span>{capped === undefined ? "The new caps hold from its next pick." : "Raise the cap that paused it and save, then resume."}</span>
        <Button variant="ghost" size="sm" onClick={() => onEditing(false)} disabled={pending === "save"}>
          Cancel
        </Button>
        <Button variant="primary" size="sm" type="submit" disabled={pending !== undefined && pending !== "save"} pending={pending === "save"}>
          Save caps
        </Button>
      </span>
    </form>
  );
}

export function AutopilotBar(props: AutopilotBarProps) {
  const { repository, state, instruction, capped, fault, caps, stopping, pending, refused, onPause, onResume, onStop, starting, onStarting, editing, onEditing, current } = props;
  const [confirming, setConfirming] = useState(false);
  const on = state !== "off";
  // Stop answered: the question it asked has nothing left to ask.
  useEffect(() => {
    if (!on) setConfirming(false);
  }, [on]);
  const keep = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (confirming) keep.current?.focus();
  }, [confirming]);
  const flip = () => (on ? setConfirming(true) : onStarting(!starting));
  return (
    <section className="armada-autopilot" aria-label="Autopilot" data-state={state}>
      <div className="armada-autopilot__head">
        <span className="armada-autopilot__name">
          <Switch size="sm" checked={on || starting} onChange={flip}>
            <span className="armada-autopilot__label">
              <Plane size={16} aria-hidden="true" />
              Autopilot
              {repository === undefined ? null : <span className="armada-autopilot__repository">{repository}</span>}
            </span>
          </Switch>
        </span>
        <span className="armada-autopilot__state" role="status">
          {stateOf(state, capped)}
        </span>
        {on && instruction !== undefined ? <span className="armada-autopilot__instruction">“{instruction}”</span> : null}
        {on ? (
          <span className="armada-autopilot__acts">
            {editing || current === undefined ? null : (
              <Button variant="ghost" size="sm" onClick={() => onEditing(true)} disabled={pending !== undefined}>
                Edit caps
              </Button>
            )}
            {state === "paused" ? (
              <Button variant="secondary" size="sm" onClick={onResume} disabled={pending !== undefined && pending !== "resume"} pending={pending === "resume"}>
                Resume
              </Button>
            ) : (
              <Button variant="secondary" size="sm" onClick={onPause} disabled={pending !== undefined && pending !== "pause"} pending={pending === "pause"}>
                Pause
              </Button>
            )}
            <Button variant="destructive" size="sm" onClick={() => setConfirming(true)} disabled={pending !== undefined || confirming}>
              Stop
            </Button>
          </span>
        ) : null}
      </div>
      {on && confirming ? (
        <div className="armada-autopilot__confirm" role="group" aria-label="Stop Autopilot">
          <span>{stopping}</span>
          <Button ref={keep} variant="secondary" size="sm" onClick={() => setConfirming(false)}>
            {state === "paused" ? "Keep it paused" : "Keep running"}
          </Button>
          <Button variant="destructive" size="sm" onClick={onStop} pending={pending === "stop"}>
            Stop Autopilot
          </Button>
        </div>
      ) : null}
      {refused === undefined ? null : (
        <p className="armada-autopilot__refused" role="alert">
          {refused}
        </p>
      )}
      {on && fault !== undefined ? <p className="armada-autopilot__fault">Its last pass picked nothing. {fault}</p> : null}
      {on ? <Caps caps={caps} /> : null}
      {on && editing && current !== undefined ? <EditForm {...props} current={current} /> : null}
      {!on && starting ? <StartForm {...props} /> : null}
    </section>
  );
}
