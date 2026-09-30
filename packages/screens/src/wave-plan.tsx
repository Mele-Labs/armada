// The wave on the Plan destination, and one Job of it read in a panel. `#1544`.
//
// **The panel is Plan's task panel's frame** (owner, 30 Sep 2026): the same
// floating `Sheet` that dims the work area, its head the Job's title with its
// state tag and Close in the corner, resized by the same handle. What the Job
// is asking you sits at its top, so a line in Needs you and its answer are one
// press apart.

import { Badge, Button, HoldButton, RowLink, Sheet, Tooltip, type SheetBack } from "@armada/components";
import { JOB_LIFECYCLE, JOB_STATUS } from "@armada/components/src/generated/vocabulary";
import { useCallback, useState, type ReactNode } from "react";

import type { WaveJobView, WaveView } from "./draft/wave";
import { useTaskWidth } from "./task-width";
import { JobAnswer, WaveRegion, blocksOf, waitsOf, type WaveRegionProps } from "./tab-wave";
import { waveSpentSaid } from "./wave";

/** What dropping a Job from the wave does, said where a person is about to do it. */
const DROP_SAID =
  "The Job ends at killed, which is terminal and carries no verdict. Nothing resumes it, " +
  "anything its Drone wrote stays on its branch, and the rest of the wave carries on.";

export type WavePlanProps = WaveRegionProps & {
  /** The window is at `--window-floor`, so the panel goes flush. */
  floor: boolean;
  /**
   * Drop one Job from the wave, held rather than pressed.
   *
   * **`kill_job` on that Job, by the name this surface gives it.** `killed` is
   * the registry's own "cleared from the Board — an operator act, carrying no
   * verdict", which is exactly what dropping one is. No second act is minted.
   */
  onDropFromWave: (jobId: string) => void;
};

/** One labelled part of the panel's body, on the task panel's own field. */
function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="armada-task-sheet__field" aria-label={label}>
      <h3 className="armada-task-sheet__label">{label}</h3>
      {children}
    </section>
  );
}

/** A mark is 12px at stroke 2, `TaskMark`'s own geometry. */
const MARK_ICON = 12;
const MARK_STROKE = 2;

/**
 * The Jobs one Job waits for, or that wait for it, each a line that opens its
 * own panel — `RowLink`, the one treatment a press to another row takes: the
 * status glyph, the handle, and the status's verb at its end.
 */
function Related({
  label,
  jobs,
  onOpen,
}: {
  label: string;
  jobs: readonly WaveJobView[];
  onOpen: (jobId: string) => void;
}) {
  if (jobs.length === 0) return null;
  return (
    <Field label={label}>
      <ul className="armada-wave__related">
        {jobs.map((one) => {
          const rendering = JOB_STATUS[one.status];
          const Icon = rendering?.icon ?? undefined;
          return (
            <li key={one.job}>
              <Tooltip label={one.title}>
                <RowLink
                  mono
                  {...(Icon == null
                    ? {}
                    : { mark: <Icon size={MARK_ICON} strokeWidth={MARK_STROKE} aria-hidden /> })}
                  says={rendering?.verb ?? one.status}
                  {...(one.status === "completed_success" ? { tone: "passed" as const } : {})}
                  {...(one.status === "completed_failed" ? { tone: "failed" as const } : {})}
                  label={`${one.title}, ${rendering?.verb ?? one.status}`}
                  onOpen={() => onOpen(one.job)}
                >
                  {one.handle ?? one.title}
                </RowLink>
              </Tooltip>
            </li>
          );
        })}
      </ul>
    </Field>
  );
}

/** One Job of the wave, read whole, with what it asks of you at the top. */
function JobSheet({
  wave,
  job,
  region,
  floor,
  width,
  onResize,
  back,
  onOpenRelated,
  onDrop,
  onClose,
}: {
  wave: WaveView;
  job: WaveJobView;
  region: WaveRegionProps;
  floor: boolean;
  width: number | undefined;
  onResize: (width: number) => void;
  back: SheetBack | undefined;
  onOpenRelated: (jobId: string) => void;
  onDrop: () => void;
  onClose: () => void;
}) {
  const rendering = JOB_STATUS[job.status];
  const spent = waveSpentSaid(job);
  return (
    <Sheet
      open
      floating
      {...(width === undefined ? {} : { width })}
      onResize={onResize}
      floor={floor}
      title={job.title}
      subtitle={
        rendering?.badgeStatus == null || rendering.icon == null ? (
          job.status
        ) : (
          <Badge status={rendering.badgeStatus} icon={rendering.icon}>
            {rendering.verb}
          </Badge>
        )
      }
      back={back}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
    >
      <div className="armada-task-sheet__body">
        <JobAnswer
          jobId={job.job}
          board={region.board}
          questions={region.questions}
          repositories={region.repositories}
          now={region.now}
          stale={region.stale}
          acting={region.acting}
          onAnswerJudge={region.onAnswerJudge}
          onAnswerCommand={region.onAnswerCommand}
        />
        {job.brief === undefined ? null : <Field label="Brief">{job.brief}</Field>}
        {job.expects === undefined || job.expects.length === 0 ? null : (
          <Field label="Expects">
            <ul className="armada-wave__expects">
              {job.expects.map((one) => (
                <li key={one}>{one}</li>
              ))}
            </ul>
          </Field>
        )}
        <Related label="Waits for" jobs={waitsOf(wave, job)} onOpen={onOpenRelated} />
        <Related label="Blocks" jobs={blocksOf(wave, job)} onOpen={onOpenRelated} />
        {spent === undefined ? null : <Field label="Spent">{spent}</Field>}
        <div className="armada-task-sheet__acts">
          <Button size="sm" ground="sunken" onClick={() => region.onOpenJob(job.job)}>
            Open job
          </Button>
          {/* **Only on a Job that has not ended.** Killing one that has is no
              act at all, and a button that does nothing is a promise. */}
          {JOB_LIFECYCLE[job.status]?.terminal !== false ? null : (
            <HoldButton
              askLabel="Drop from the wave"
              description={DROP_SAID}
              disabled={region.stale || region.acting}
              onAsk={onDrop}
              onCommit={onDrop}
            >
              Hold to drop from the wave
            </HoldButton>
          )}
        </div>
      </div>
    </Sheet>
  );
}

export function WavePlan({ floor, onDropFromWave, ...region }: WavePlanProps) {
  // Which Job the panel is on. **This destination's own state**: a panel that
  // survived leaving the Plan would open over a board nobody is reading.
  const [open, setOpen] = useState<string | null>(null);
  // The Jobs the open one was reached from, pressed in Waits for or Blocks —
  // the group-to-task step back, on the same terms: Back and Close land on the
  // one before, and it never leaves the plan.
  const [from, setFrom] = useState<readonly string[]>([]);
  const [width, resize] = useTaskWidth();
  // **Stable across a tick of `now`**: the region memoises the canvas's nodes
  // on it, and a fresh function each second left every card hidden.
  const opening = useCallback((jobId: string) => {
    setFrom([]);
    setOpen(jobId);
  }, []);

  const wave = region.draft?.wave ?? undefined;
  if (wave === undefined) return <WaveRegion {...region} />;

  const byId = new Map(wave.jobs.map((one) => [one.job, one]));
  const reading = open === null ? undefined : byId.get(open);
  const before = from.length === 0 ? undefined : byId.get(from[from.length - 1]!);
  const goBack = () => {
    setOpen(from[from.length - 1] ?? null);
    setFrom(from.slice(0, -1));
  };
  const back: SheetBack | undefined =
    before === undefined
      ? undefined
      : { label: `Back to ${before.title}`, tooltip: `Back to ${before.title}`, onBack: goBack };

  return (
    <>
      <WaveRegion {...region} onOpenJob={opening} />
      {reading === undefined ? null : (
        <JobSheet
          wave={wave}
          job={reading}
          region={region}
          floor={floor}
          width={width}
          onResize={resize}
          back={back}
          onOpenRelated={(jobId) => {
            setFrom([...from, reading.job]);
            setOpen(jobId);
          }}
          onDrop={() => {
            onDropFromWave(reading.job);
            setOpen(null);
            setFrom([]);
          }}
          onClose={before === undefined ? () => setOpen(null) : goBack}
        />
      )}
    </>
  );
}
