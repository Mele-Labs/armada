import { Bot, CircleDot, FileDiff, Megaphone, Scale, ShieldCheck, ShieldX } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What is asking, where the canvas would be: the asker's own live output, what it changed so far
 * with the file it is asking about marked, and for a Judge the work product beside the checks it is
 * reading. **The Now panel beside it lists everything else that runs**, so this draws only the one.
 */
export type NowAskerFile = {
  path: string;
  change: "added" | "changed" | "removed";
  /** The file or part the question is about. */
  asking?: boolean;
  /** The diff so far: lines led by `+`, `-` or a space. */
  diff?: readonly string[];
};

export type NowAskerCheck = { name: string; state: "passed" | "failed" | "running"; tail?: readonly string[] };

export type NowAsker = {
  /** Named after its step: `Plan Drone`, `Judge on Implement`. */
  name: string;
  of: "drone" | "judge";
  step?: string;
  /** `running` pulses; `waiting` is a Drone stopped on the question. */
  state: "running" | "waiting";
  /** Its last actions, oldest first. */
  actions: readonly string[];
  /** The tail of its output, oldest first. */
  tail: readonly string[];
  changed?: readonly NowAskerFile[];
  /** A Judge's read: the work product, and the checks it is reading against. */
  product?: { title: string; lines: readonly string[] };
  checks?: readonly NowAskerCheck[];
};

export type AskerViewProps = {
  asker: NowAsker;
  /** A press on a file. Absent draws files as plain rows. */
  onOpenFile?: (path: string) => void;
};

const CHANGE: Record<NowAskerFile["change"], { Glyph: LucideIcon; said: string }> = {
  // One glyph for all three: `file-diff` is a produced file's change, and the hue and the tooltip say which.
  added: { Glyph: FileDiff, said: "Added" },
  changed: { Glyph: FileDiff, said: "Changed" },
  removed: { Glyph: FileDiff, said: "Removed" },
};

const CHECK: Record<NowAskerCheck["state"], { Glyph: LucideIcon; said: string }> = {
  passed: { Glyph: ShieldCheck, said: "Passed" },
  failed: { Glyph: ShieldX, said: "Failed" },
  running: { Glyph: CircleDot, said: "Running" },
};

function Mark({ Glyph, said, state, pulsing }: { Glyph: LucideIcon; said: string; state?: string; pulsing?: boolean }) {
  return (
    <Tooltip label={said}>
      <span className="armada-asker__mark" data-state={state} data-pulsing={pulsing || undefined} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

function Frame({ label, children, tone }: { label: string; children: React.ReactNode; tone?: string }) {
  return (
    <section className="armada-asker__frame" data-tone={tone} aria-label={label}>
      <h4 className="armada-asker__band">{label}</h4>
      {children}
    </section>
  );
}

export function AskerView({ asker, onOpenFile }: AskerViewProps) {
  const Who = asker.of === "judge" ? Scale : Bot;
  const live = asker.state === "running";
  return (
    <figure className="armada-asker" role="group" aria-label="Asker" data-of={asker.of}>
      <Frame label={asker.name} tone="live">
        <div className="armada-asker__who">
          <Mark Glyph={Who} said={asker.of === "judge" ? "Judge" : "Drone"} />
          {asker.step === undefined ? null : <span className="armada-asker__step">{asker.step}</span>}
          <Mark Glyph={live ? CircleDot : Megaphone} said={live ? "Running" : "Asks you"} state={live ? "running" : "waiting"} pulsing={live} />
        </div>
        <ol className="armada-asker__actions" aria-label="Last actions">
          {asker.actions.map((line, at) => (
            <li key={`${String(at)}-${line}`}>{line}</li>
          ))}
        </ol>
        <pre className="armada-asker__tail" aria-label="Output">
          {asker.tail.join("\n")}
        </pre>
      </Frame>
      {asker.product === undefined && asker.checks === undefined ? null : (
        <>
          {asker.product === undefined ? null : (
            <Frame label="Work product">
              <p className="armada-asker__product">{asker.product.title}</p>
              <pre className="armada-asker__tail">{asker.product.lines.join("\n")}</pre>
            </Frame>
          )}
          {asker.checks === undefined ? null : (
            <Frame label="Checks it is reading">
              <ul className="armada-asker__rows">
                {asker.checks.map((check) => {
                  const { Glyph, said } = CHECK[check.state];
                  return (
                    <li key={check.name} className="armada-asker__row">
                      <div className="armada-asker__line">
                        <Mark Glyph={Glyph} said={said} state={check.state} pulsing={check.state === "running"} />
                        <span className="armada-asker__path">{check.name}</span>
                      </div>
                      {check.tail === undefined ? null : <pre className="armada-asker__diff">{check.tail.join("\n")}</pre>}
                    </li>
                  );
                })}
              </ul>
            </Frame>
          )}
        </>
      )}
      {asker.changed === undefined ? null : (
        <Frame label="Changed so far">
          <ul className="armada-asker__rows">
            {asker.changed.map((file) => {
              const { Glyph, said } = CHANGE[file.change];
              const name = `${said}, ${file.path}${file.asking === true ? ", asked about" : ""}`;
              const body = (
                <>
                  <Mark Glyph={Glyph} said={said} state={file.change} />
                  <span className="armada-asker__path">{file.path}</span>
                  {file.asking !== true ? null : <Mark Glyph={Megaphone} said="Asking about this" state="asking" />}
                </>
              );
              return (
                <li key={file.path} className="armada-asker__row" data-asking={file.asking || undefined}>
                  {onOpenFile === undefined ? (
                    <div className="armada-asker__line" role="group" aria-label={name}>
                      {body}
                    </div>
                  ) : (
                    <button type="button" className="armada-asker__line" aria-label={`Open ${name}`} onClick={() => onOpenFile(file.path)}>
                      {body}
                    </button>
                  )}
                  {file.diff === undefined ? null : (
                    <pre className="armada-asker__diff" aria-label={`Diff of ${file.path}`}>
                      {file.diff.map((line, at) => (
                        <span key={`${String(at)}-${line}`} className="armada-asker__code" data-sign={line.charAt(0) === "+" || line.charAt(0) === "-" ? line.charAt(0) : undefined}>
                          {line}
                          {"\n"}
                        </span>
                      ))}
                    </pre>
                  )}
                </li>
              );
            })}
          </ul>
        </Frame>
      )}
    </figure>
  );
}
