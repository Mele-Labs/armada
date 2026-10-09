// The pane on the right of Command Central and the Running board: the thing picked, with the
// context to answer it. In order: its title, what the Job was asked for, where it is in its
// workflow, the words leading up to it, then the question and its acts. A Session or the merge line
// has no steps, so it says what it is doing instead. Mock only, as the Dashboard is.

import { useEffect, useState, type ReactNode } from "react";
import { CircleDot, Workflow } from "lucide-react";
import { Button, JobDiffSheet, NowPanel, UnifiedDiff, railOfPatch, type DiffFile, type NowPanelProps } from "@armada/components";
import type { AboutFiles, AboutLink } from "@armada/jobs/draft/calls";

import type { BridgeState } from "../../shared/bridge";
import { age, type Item } from "./Dashboard";
import { sessionIdOf } from "./cockpit/waiting";
import { SessionMini } from "./sessions";
import { JobMarks, hasMarks } from "@armada/screens";

const NOTE = { ask: "asking you", issue: "went wrong", bad: "ended badly", running: "running", queued: "queued", ok: "done" } as const;

function Section({ head, children }: { head: string; children: ReactNode }) {
  return (
    <section className="armada-call__section" aria-label={head}>
      <h3 className="armada-call__label">{head}</h3>
      {children}
    </section>
  );
}

/** A Job or an issue, as a chip: its mark, its number and its title, pressed to open it. */
function LinkChip({ link, onOpenJob, onOpenLink }: { link: AboutLink; onOpenJob?: ((id: string) => void) | undefined; onOpenLink?: ((url: string) => void) | undefined }) {
  const Icon = link.kind === "job" ? Workflow : CircleDot;
  const open = link.kind === "job" ? (link.jobId === undefined ? undefined : () => onOpenJob?.(link.jobId!)) : link.url === undefined ? undefined : () => onOpenLink?.(link.url!);
  return (
    <button type="button" className="armada-linkchip" data-kind={link.kind} onClick={open} disabled={open === undefined}>
      <Icon size={14} aria-hidden="true" />
      <span className="armada-linkchip__number">#{link.number}</span>
      <span className="armada-linkchip__title">{link.title}</span>
    </button>
  );
}

/** Files as chips, directory receding and the name kept; a press opens the file's patch in a panel. */
function FileChips({ files, onRead }: { files: AboutFiles["files"]; onRead: (path: string) => void }) {
  return (
    <span className="armada-filechips">
      {files.map((file) => {
        const cut = file.path.lastIndexOf("/") + 1;
        return (
          <button key={file.path} type="button" className="armada-filechip" title={file.path} onClick={() => onRead(file.path)}>
            {/* The directory is laid right to left so its start is what clips; the mark keeps its slash at the end. */}
            <span className="armada-filechip__dir">{`${file.path.slice(0, cut)}\u200E`}</span>
            <span className="armada-filechip__base">{file.path.slice(cut)}</span>
          </button>
        );
      })}
    </span>
  );
}

/** The Job's workflow as a stepper, each step named under its pip, the current one in the call's hue. */
function Stepper({ item, workflows }: { item: Item; workflows: BridgeState["holds"]["workflows"] }) {
  const job = item.job!;
  const steps = workflows.find((one) => one.id === job.workflow_id)?.steps ?? [];
  if (steps.length === 0) return null;
  const at = Math.max(0, steps.findIndex((step) => step.step_id === job.current_step_id));
  return (
    <ol className="armada-steps" aria-label="Steps" style={{ ["--steps" as string]: steps.length }}>
      {steps.map((step, index) => (
        <li key={step.step_id} className="armada-steps__step" data-step={index < at ? "done" : index === at ? "current" : "ahead"}>
          <span className="armada-steps__pip" aria-hidden="true" />
          <span className="armada-steps__name">{step.label}</span>
          {index === at ? <span className="armada-steps__note">{NOTE[item.hue]}</span> : null}
        </li>
      ))}
    </ol>
  );
}

export function CallPane({
  item,
  now,
  workflows,
  onDone,
  onOpenSession,
  onOpenJob,
  onOpenLink,
  nowPanel,
}: {
  item: Item;
  now: number;
  workflows: BridgeState["holds"]["workflows"];
  onDone: () => void;
  /** Opens the whole Session, from its miniature. */
  onOpenSession?: (id: string) => void;
  /** Opens a Job a chip names. */
  onOpenJob?: (id: string) => void;
  /** Opens an issue's address. */
  onOpenLink?: (url: string) => void;
  /** What the Job is doing now, drawn in place of where it is in its workflow. Mock only. */
  nowPanel?: Omit<NowPanelProps, "onHide">;
}) {
  const [reading, setReading] = useState<string>();
  const allFiles = (item.about ?? []).flatMap(([, value]) => (typeof value === "object" && "files" in value ? value.files : []));
  const patch: DiffFile[] = allFiles.map((file) => ({ path: file.path, lines: [...file.lines] }));
  const sessionId = sessionIdOf(item.key);
  const Icon = item.icon;
  const [at, setAt] = useState(0);
  const [picked, setPicked] = useState<string>();
  useEffect(() => (setAt(0), setPicked(undefined)), [item.key]);
  const decision = item.decisions?.[at];
  const asking = item.hue === "ask";
  const hasSteps = item.job !== undefined && (workflows.find((one) => one.id === item.job!.workflow_id)?.steps ?? []).length > 0;
  return (
    <article className="armada-call" data-hue={item.hue} aria-label={item.title} key={item.key}>
      <header className="armada-call__band">
        <Icon size={16} aria-hidden="true" />
        <span>{item.kind}</span>
        <span className="armada-call__where">{item.where}</span>
        <span className="armada-call__age">{age(item.at, now)}</span>
      </header>
      {sessionId !== undefined && onOpenSession !== undefined ? (
        <div className="armada-call__body" data-session>
          <div className="armada-call__session-head">
            <h2 className="armada-call__title">{item.title}</h2>
            <Button variant="ghost" size="sm" onClick={() => onOpenSession(sessionId)}>Open Session</Button>
          </div>
          <SessionMini sessionId={sessionId} onOpen={onOpenSession} />
        </div>
      ) : nowPanel !== undefined ? (
        <div className="armada-call__body">
          <h2 className="armada-call__title">{item.title}</h2>
          <NowPanel {...nowPanel} />
          <div className="armada-call__acts">{item.acts(onDone)}</div>
        </div>
      ) : (
      <div className="armada-call__body">
        <h2 className="armada-call__title">
          {item.title}
          {item.job === undefined || !hasMarks(item.job, now) ? null : (
            <span className="armada-call__marks">
              <JobMarks job={item.job} now={now} />
            </span>
          )}
        </h2>
        {item.about === undefined || item.about.length === 0 ? null : (
          <dl className="armada-call__about" aria-label="Where it lives">
            {item.about.map(([term, value]) => (
              <div key={term}>
                <dt>{term}</dt>
                <dd>
                  {typeof value === "string" ? (
                    value
                  ) : "files" in value ? (
                    <FileChips files={value.files} onRead={setReading} />
                  ) : (
                    <LinkChip link={value} onOpenJob={onOpenJob} onOpenLink={onOpenLink} />
                  )}
                </dd>
              </div>
            ))}
          </dl>
        )}
        {item.request === undefined ? null : (
          <Section head="What was asked for">
            <p className="armada-call__request">{item.request}</p>
          </Section>
        )}
        {hasSteps ? (
          <Section head="Where it is">
            <Stepper item={item} workflows={workflows} />
          </Section>
        ) : item.doing === undefined ? null : (
          <Section head="What it is doing">
            <p className="armada-call__doing">{item.doing}</p>
          </Section>
        )}
        {item.context === undefined || item.context.length === 0 ? null : (
          <Section head={item.contextHead ?? "Why it asks"}>
            <pre className="armada-call__said">{item.context.join("\n")}</pre>
          </Section>
        )}
        {decision === undefined ? (
          <>
            <p className="armada-call__ask" data-quiet={asking ? undefined : ""}>{item.fact}</p>
            {item.body.length === 0 || item.job !== undefined || item.context !== undefined ? null : (
              <dl className="armada-call__facts">
                {item.body.map(([term, value]) => (
                  <div key={term}>
                    <dt>{term}</dt>
                    <dd>{value}</dd>
                  </div>
                ))}
              </dl>
            )}
            <div className="armada-call__acts">{item.acts(onDone)}</div>
          </>
        ) : (
          <>
            <p className="armada-call__ask">{decision.question}</p>
            <div className="armada-call__tiles" role="radiogroup" aria-label={decision.question}>
              {decision.options.map((option) => (
                <button
                  key={option.id}
                  type="button"
                  role="radio"
                  aria-checked={picked === option.id}
                  className="armada-call__tile"
                  onClick={() => setPicked(option.id)}
                >
                  {option.label}
                </button>
              ))}
            </div>
            <div className="armada-call__acts">
              <Button
                variant="primary"
                disabled={picked === undefined}
                onClick={() => (at + 1 < item.decisions!.length ? (setAt(at + 1), setPicked(undefined)) : onDone())}
              >
                Answer
              </Button>
            </div>
          </>
        )}
      </div>
      )}
      {patch.length === 0 ? null : (
        <JobDiffSheet
          open={reading !== undefined}
          branch={item.where}
          files={railOfPatch(patch)}
          {...(reading === undefined ? {} : { selected: reading })}
          onSelect={setReading}
          onClose={() => setReading(undefined)}
        >
          <UnifiedDiff files={patch.filter((file) => file.path === reading)} />
        </JobDiffSheet>
      )}
    </article>
  );
}
