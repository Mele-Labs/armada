import { useEffect, useId, useState, type ReactNode } from "react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { Switch } from "../../primitives/Switch/Switch";
import { Textarea } from "../../primitives/Textarea/Textarea";

/** What a setting holds, as the field sends it back. */
export type SettingFieldValue = number | boolean | string | readonly string[];

/** The control a setting is drawn with, chosen from its kind, with the value in force. */
export type SettingFieldControl =
  /** `seconds` reads the figure out in minutes or hours beneath the field. */
  | { kind: "number"; value: number; min: number; max: number; unit?: string; seconds?: boolean }
  | { kind: "switch"; value: boolean }
  | { kind: "choice"; value: string; options: readonly string[] }
  | { kind: "text"; value: string }
  /** One entry a line. */
  | { kind: "list"; value: readonly string[] }
  /** Several lines of prose, whose way back is the text Armada ships. */
  | { kind: "prompt"; value: string };

/**
 * One setting from settings.json, drawn from its kind — **the generic row every setting but Bridge's
 * own panes is drawn with**, so a key Fleet adds needs no new component.
 *
 * A switch or a choice saves as it is pressed. A figure, a line of text, a list or a prompt is typed
 * first and saved with Save, so nothing goes to Fleet a keystroke at a time. A saved value is marked
 * Modified and offers Reset, which removes the key so the shipped value is back.
 */
export type SettingFieldProps = {
  /** The key in settings.json, `limits.dronesAtOnce`. Drawn mono under the title. */
  name: string;
  title: string;
  description: string;
  control: SettingFieldControl;
  /** The shipped value as a person reads it, `2` or `15 minutes`. Absent draws no line. */
  shipped?: string;
  /** settings.json holds a value for this key. Draws Modified and Reset. */
  modified: boolean;
  onSave: (value: SettingFieldValue) => void;
  /** Remove the key, so the shipped value is back. */
  onReset: () => void;
  /** The saved value is not in force until Fleet restarts. */
  restart?: boolean;
  /** The environment variable that wins over settings.json, `ARMADA_MODEL`. */
  overriddenBy?: string;
  /** Nothing can be sent: Fleet is not connected, or another save is out. */
  disabled?: boolean;
  /** This field's own save is out. */
  saving?: boolean;
  /** Fleet's own refusal, read exactly. */
  refused?: string;
};

/** `15 minutes`, `1 hour`, `90 seconds`. Whole units only; anything else is said in seconds. */
export function inWords(seconds: number): string {
  const plural = (n: number, unit: string) => `${n} ${unit}${n === 1 ? "" : "s"}`;
  if (seconds >= 3600 && seconds % 3600 === 0) return plural(seconds / 3600, "hour");
  if (seconds >= 60 && seconds % 60 === 0) return plural(seconds / 60, "minute");
  return plural(seconds, "second");
}

/** What a typed field holds as text: the list one entry a line. */
function asText(control: SettingFieldControl): string {
  if (control.kind === "list") return control.value.join("\n");
  return String(control.value);
}

export function SettingField({
  name,
  title,
  description,
  control,
  shipped,
  modified,
  onSave,
  onReset,
  restart = false,
  overriddenBy,
  disabled = false,
  saving = false,
  refused,
}: SettingFieldProps) {
  const id = useId();
  const meansId = `${id}-means`;
  const [typed, setTyped] = useState<string | null>(null);
  const held = asText(control);
  // A republished value is the answer to a save: what was typed has gone, so the field reads it.
  useEffect(() => setTyped(null), [held]);
  const off = disabled || saving;
  const field = typed ?? held;
  const dirty = field !== held;

  const asked = Number(field);
  const inRange =
    control.kind !== "number" || (field.trim() !== "" && Number.isInteger(asked) && asked >= control.min && asked <= control.max);

  function save(): void {
    if (control.kind === "number") onSave(asked);
    else if (control.kind === "list") onSave(field.split("\n").map((one) => one.trim()).filter((one) => one !== ""));
    else onSave(field);
  }

  const resetLabel = control.kind === "prompt" ? "Reset to shipped" : "Reset";
  const meta = (
    <div className="armada-setting-field__meta">
      <code className="armada-setting-field__key">{name}</code>
      {modified ? <span className="armada-setting-field__modified">Modified</span> : null}
      {modified ? (
        <Button variant="ghost" size="sm" disabled={off} aria-label={`${resetLabel}: ${title}`} onClick={onReset}>
          {resetLabel}
        </Button>
      ) : null}
    </div>
  );

  const notes: ReactNode[] = [];
  if (restart) notes.push(<p key="restart" className="armada-setting-field__note">Restart Fleet to apply.</p>);
  if (overriddenBy !== undefined) {
    notes.push(
      <p key="env" className="armada-setting-field__note">
        Overridden by <code>${overriddenBy}</code> where Fleet runs, so the value here is not the one in force.
      </p>,
    );
  }

  if (control.kind === "switch") {
    return (
      <div className="armada-setting-field" data-modified={modified || undefined}>
        <Switch checked={control.value} disabled={off} description={description} onChange={(event) => onSave(event.currentTarget.checked)}>
          {title}
        </Switch>
        {meta}
        {shipped === undefined ? null : <p className="armada-setting-field__ships">{`Ships ${shipped}.`}</p>}
        {notes}
        {refused === undefined ? null : <p className="armada-setting-field__refused" role="alert">{refused}</p>}
      </div>
    );
  }

  const typedField =
    control.kind === "choice" ? (
      <Select id={id} aria-describedby={meansId} value={control.value} disabled={off} onChange={(event) => onSave(event.target.value)}>
        {control.options.map((one) => (
          <option key={one} value={one}>
            {one}
          </option>
        ))}
      </Select>
    ) : control.kind === "prompt" || control.kind === "list" ? (
      <Textarea
        id={id}
        aria-describedby={meansId}
        rows={control.kind === "prompt" ? 8 : 4}
        grow={control.kind === "prompt"}
        keep={false}
        value={field}
        disabled={off}
        onChange={(event) => setTyped(event.target.value)}
      />
    ) : (
      <Input
        id={id}
        mono
        aria-describedby={meansId}
        {...(control.kind === "number" ? { inputMode: "numeric" as const, trailing: control.seconds ? "s" : control.unit } : {})}
        value={field}
        disabled={off}
        invalid={(dirty && !inRange) || refused !== undefined}
        {...(control.kind === "number" && dirty && !inRange
          ? { message: `Between ${control.min} and ${control.max}.` }
          : refused !== undefined
            ? { message: refused }
            : {})}
        onChange={(event) => setTyped(event.target.value)}
      />
    );

  return (
    <div className="armada-setting-field" data-modified={modified || undefined}>
      <label className="armada-setting-field__title" htmlFor={id}>
        {title}
      </label>
      {meta}
      <p className="armada-setting-field__means" id={meansId}>
        {description}
      </p>
      <div className="armada-setting-field__row" data-wide={control.kind === "prompt" || control.kind === "list" || undefined}>
        {typedField}
        {control.kind === "choice" ? null : (
          <Button
            variant="secondary"
            size="sm"
            ground="sunken"
            pending={saving}
            disabled={off || !dirty || !inRange}
            aria-label={`Save ${title}`}
            onClick={save}
          >
            {saving ? "Saving…" : "Save"}
          </Button>
        )}
      </div>
      {control.kind === "number" && control.seconds && inRange ? (
        <p className="armada-setting-field__ships">{inWords(asked)}</p>
      ) : null}
      {shipped === undefined ? null : <p className="armada-setting-field__ships">{`Ships at ${shipped}.`}</p>}
      {notes}
      {refused !== undefined && control.kind !== "number" && control.kind !== "text" ? (
        <p className="armada-setting-field__refused" role="alert">
          {refused}
        </p>
      ) : null}
    </div>
  );
}

/**
 * A section's settings under the line that says when a change counts, and why nothing can be sent
 * where nothing can. The rows are `SettingField`s.
 */
export function SettingFieldList({ lead, note, children }: { lead: ReactNode; note?: ReactNode; children: ReactNode }) {
  return (
    <div className="armada-setting-fields">
      <div className="armada-setting-fields__opening">
        <p className="armada-setting-fields__lead">{lead}</p>
        {note === undefined ? null : <p className="armada-setting-fields__lead">{note}</p>}
      </div>
      {children}
    </div>
  );
}

/** Why Fleet refused settings.json: the key at fault, where there is one, and why. */
export type SettingsFileRefusal = { key?: string | null; reason: string };

/**
 * settings.json itself, above every section: **a refusal first**, since every value below is the
 * last good one while it stands, then the file's path and the way to open it in the editor.
 */
export type SettingsFileBarProps = {
  /** Where the file is, or `null` before Fleet has said. Off until it has. */
  path: string | null;
  refused: SettingsFileRefusal | null;
  /** The press is out. */
  opening?: boolean;
  /** Why the last press did not open the file. */
  said?: string;
  onOpen: () => void;
};

export function SettingsFileBar({ path, refused, opening = false, said, onOpen }: SettingsFileBarProps) {
  return (
    <div className="armada-setting-fields__opening">
      {refused === null ? null : (
        <Alert tone="caution" title="Fleet refused settings.json">
          {refused.key == null ? null : (
            <>
              <code>{refused.key}</code>
              {": "}
            </>
          )}
          {`${refused.reason} The last settings that read cleanly stay in force until the file is fixed.`}
        </Alert>
      )}
      <div className="armada-setting-fields__file">
        <code className="armada-setting-field__key">{path ?? "settings.json"}</code>
        <Button variant="secondary" size="sm" ground="card" pending={opening} disabled={path === null} onClick={onOpen}>
          Open settings.json
        </Button>
      </div>
      {said === undefined ? null : (
        <p className="armada-setting-fields__lead" role="status">
          {said}
        </p>
      )}
    </div>
  );
}
