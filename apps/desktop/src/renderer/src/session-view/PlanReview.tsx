// The plan opened in full over the Cockpit, for review: every group open with its own paths and its tasks, the sketch drawn with it large
// and playing, and Approve at the foot as on the card in the thread. **Every group and task takes quick edits and notes in place** (`PlanEdit`),
// which collect as marks and go to the agent together as Send changes, beside Approve. Esc or Close shuts it, and approving shuts it too.
// **A plan recorded again opens as the diff the thread's card draws**, changed groups open and the rest one line, with Edit plan to reach the rows.

import { useState } from "react";
import { BadgeCheck } from "lucide-react";
import {
  Button,
  FindingNotes,
  JobCard,
  LandsInChip,
  PlanRevisionView,
  Sheet,
  SketchReading,
  Tooltip,
  anchorsOf,
  landingsOf,
  useFindingKeys,
  useUnchanged,
  type Finding,
  type JobCardIssue,
  type PlanRevision,
  type SessionState,
} from "@armada/components";
import { useAtFloor } from "@armada/shell";
import type { WorkPlan } from "@armada/screens/src/draft/sessions";

import { AddGroup, GroupEditor, SendChanges, type FindingPins, type PlanEditing } from "./PlanEdit";
import { OverTheApp } from "./OverTheApp";
import { viewOf } from "./plan-marks";

type PlanBodyProps = {
  plan: WorkPlan;
  revision?: PlanRevision;
  ed: PlanEditing;
  /** Edit plan: the rows are drawn in place of the diff against the version before. */
  editing: boolean;
  pullRequests?: {
    onOpen: (number: number) => void;
  };
  jobOf?: (group: string) =>
    | {
        title: string;
        state: SessionState;
        said: string;
        at?: string | number;
        onOpen: () => void;
      }
    | undefined;
  issueOf?: (group: string) => JobCardIssue | undefined;
  findings?: {
    findings: readonly Finding[];
    onSend: (id: string) => void;
    onDismiss: (id: string) => void;
  };
};

/**
 * The plan as it is read for review: every group with its paths and tasks, each taking quick edits and notes in place, and the sketch drawn with it.
 * **No frame of its own**: the plan's layer wraps it in a sheet with Approve and Send changes at the foot, and the Job's review sheet draws it
 * in its right pane with its own answers.
 */
export function PlanBody({
  plan,
  revision,
  ed,
  editing,
  pullRequests,
  jobOf,
  issueOf,
  findings,
}: PlanBodyProps) {
  const unchanged = useUnchanged(revision);
  // Marks already made are the rows' to show, so a plan holding any opens on them.
  const diff = revision !== undefined && !editing && ed.marks.length === 0;
  const groups = viewOf(plan, ed.marks);
  const keys = useFindingKeys(findings?.findings, {
    onSend: (id) => findings?.onSend(id),
    onDismiss: (id) => findings?.onDismiss(id),
  });
  const pins: FindingPins | undefined =
    findings === undefined
      ? undefined
      : {
          anchors: anchorsOf(
            findings.findings,
            groups.flatMap((group) => [
              group.id,
              ...group.tasks.map((task) => task.id),
            ]),
          ),
          onSend: keys.send,
          onDismiss: keys.dismiss,
        };
  return (
    <div className="sv-review">
      <div className="sv-review__groups" {...(pins === undefined ? {} : keys.bind)}>
        {diff ? (
          <PlanRevisionView revision={revision} unchanged={unchanged} />
        ) : (
          <>
            {pins === undefined ? null : (
              <FindingNotes
                findings={pins.anchors.whole}
                label="Findings on the plan"
                onSend={pins.onSend}
                onDismiss={pins.onDismiss}
              />
            )}
            {landingsOf(groups, pullRequests !== undefined).map((run) => {
              const editors = run.groups.map((group) => (
                <section
                  key={group.id}
                  className="sv-group sv-review__group"
                  data-open
                  aria-label={group.title}
                >
                  <GroupEditor
                    group={group}
                    ed={ed}
                    {...(pins === undefined
                      ? {}
                      : {
                          pins,
                        })}
                  />
                </section>
              ));
              // A group that goes to a Job of its own is that Job's card alone, its issue a chip in it: a placeholder until it is handed, then the Job. The Job plans its own
              // tasks, so the group has no editor here.
              if (run.own !== undefined) {
                const job = jobOf?.(run.own.id);
                const issue = issueOf?.(run.own.id);
                const chip =
                  issue === undefined
                    ? {}
                    : {
                        issue,
                      };
                return (
                  <div
                    key={run.key}
                    className="sv-review__landing"
                    role="group"
                    aria-label={run.own.title}
                  >
                    {job === undefined ? (
                      <JobCard placeholder={run.own.title} {...chip} />
                    ) : (
                      <JobCard
                        title={job.title}
                        state={job.state}
                        said={job.said}
                        handedFrom={run.own.title}
                        {...chip}
                        {...(job.at === undefined
                          ? {}
                          : {
                              at: job.at,
                            })}
                        onOpen={job.onOpen}
                      />
                    )}
                  </div>
                );
              }
              return run.landsIn === undefined ? (
                editors
              ) : (
                <div
                  key={run.key}
                  className="sv-review__landing"
                  role="group"
                  aria-label={`Lands in ${run.landsIn.pullRequest === undefined ? run.key : `#${run.landsIn.pullRequest.number}`}`}
                >
                  <LandsInChip landsIn={run.landsIn} onOpen={pullRequests?.onOpen} />
                  {editors}
                </div>
              );
            })}
            <AddGroup ed={ed} />
          </>
        )}
      </div>
      {plan.sketch === undefined ? null : (
        <div className="sv-review__sketch">
          <SketchReading scene={plan.sketch.scene} title={plan.sketch.title} />
        </div>
      )}
    </div>
  );
}

/**
 * The plan in the Job's review sheet: its summary first, as the thread's plan card reads it, then its body with the one switch the sheet's own head has no place for, Edit plan, which draws the rows in place of the diff against
 * the version before. Approve and Send back are the sheet's, not drawn here.
 */
export function PlanPane(props: Omit<PlanBodyProps, "editing">) {
  const [editing, setEditing] = useState(false);
  const { revision, plan, ed } = props;
  return (
    <div className="armada-job-review__fill">
      {plan.summary === undefined ? null : <p className="sv-blueprint__summary">{plan.summary}</p>}
      {revision === undefined || plan.approved || ed.marks.length > 0 ? null : (
        <div className="sv-review__foot">
          <Button variant="ghost" size="sm" onClick={() => setEditing(!editing)}>
            {editing ? "Show changes" : "Edit plan"}
          </Button>
        </div>
      )}
      <PlanBody {...props} editing={editing} />
    </div>
  );
}

export function PlanReview({
  open,
  plan,
  revision,
  ed,
  findings,
  pullRequests,
  jobOf,
  issueOf,
  onApprove,
  onClose,
}: {
  open: boolean;
  plan: WorkPlan;
  revision?: PlanRevision;
  ed: PlanEditing /** The Job opens pull requests: the groups stand under the chip of the one each lands in, and a chip's press opens its pull request. */;
  pullRequests?: {
    onOpen: (number: number) => void;
  } /** The Job a group was handed to, which heads the group that goes to a Job of its own in place of the placeholder. */;
  jobOf?: (group: string) =>
    | {
        title: string;
        state: SessionState;
        said: string;
        at?: string | number;
        onOpen: () => void;
      }
    | undefined /** The issue a group works, the chip on its Job's card. */;
  issueOf?: (
    group: string,
  ) =>
    | JobCardIssue
    | undefined /** A Judge's notes on the plan, pinned under the group or task each names, with the acts the card offers. */;
  findings?: {
    findings: readonly Finding[];
    onSend: (id: string) => void;
    onDismiss: (id: string) => void;
  };
  onApprove: () => void;
  onClose: () => void;
}) {
  const floor = useAtFloor();
  const [editing, setEditing] = useState(false);
  const marked = ed.marks.length > 0;
  const settled = plan.approved ? (
    <Tooltip label="Plan approved">
      <span className="sv-panel__mark" role="img" aria-label="Plan approved">
        <BadgeCheck size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  ) : undefined;
  const foot = (
    <div className="sv-review__foot">
      {revision === undefined || plan.approved || ed.marks.length > 0 ? null : (
        <Button variant="ghost" size="sm" onClick={() => setEditing(!editing)}>
          {editing ? "Show changes" : "Edit plan"}
        </Button>
      )}
      <SendChanges ed={ed} />
      {plan.approved ? null : (
        <Tooltip label={marked ? "Approve with these changes" : "Approve"}>
          <Button variant="primary" size="sm" onClick={onApprove}>
            Approve
          </Button>
        </Tooltip>
      )}
    </div>
  );
  const body = (
    <PlanBody
      plan={plan}
      editing={editing}
      ed={ed}
      {...(revision === undefined
        ? {}
        : {
            revision,
          })}
      {...(findings === undefined
        ? {}
        : {
            findings,
          })}
      {...(pullRequests === undefined
        ? {}
        : {
            pullRequests,
          })}
      {...(jobOf === undefined
        ? {}
        : {
            jobOf,
          })}
      {...(issueOf === undefined
        ? {}
        : {
            issueOf,
          })}
    />
  );
  return (
    <OverTheApp open={open}>
      <Sheet
        kind="plan-review"
        open={open}
        floating
        floor={floor}
        size="widest"
        title="Plan"
        controls={settled}
        footer={marked || !plan.approved ? foot : undefined}
        closeLabel="Close"
        closeBinding="Esc"
        onClose={onClose}
      >
        {body}
      </Sheet>
    </OverTheApp>
  );
}
