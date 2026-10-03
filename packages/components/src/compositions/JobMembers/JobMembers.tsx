import type { LucideIcon } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { useCallback, useState } from "react";

import { GUIDE_MEMBER_LINK } from "../../guides";
import { Button } from "../../primitives/Button/Button";
import { GuideMark } from "../GuideMark/GuideMark";
import { Input } from "../../primitives/Input/Input";
import { StepBar } from "../StepBar/StepBar";
import type { TaskBarSegment } from "../StepBar/StepBar";

/**
 * How a member's work reaches the one before it.
 * `docs/concepts/landing.md` defines the three and nothing else may.
 */
export type JobMemberLink = "stacked" | "merged" | "published";

/** The button, and the word it produces. `design-system.md`, the verb table. */
export const DROP_MEMBER_LABEL = "Drop";

/**
 * A member's state, as the registry renders it.
 *
 * **Two shapes, because a status the registry has no verb or glyph for must
 * not be drawn as a badge with something invented in it.** `text` renders the
 * wire spelling and says what is missing; a blank cell hides both.
 */
export type JobMemberState =
  | { as: "badge"; status: string; icon: LucideIcon; label: string }
  | { as: "text"; wire: string; missing: string };

/** One Job landing under this one, as one row of the train. */
export type JobMemberRow = {
  id: string;
  /** Where it sits in the order, counting from one. */
  ordinal: number;
  title: string;
  state: JobMemberState;
  /**
   * Whether its pull request merged. **The merge, not a successful status** —
   * a Job whose gate hands off to a person finishes before anyone merges it.
   */
  landed?: boolean;
  /** How it reaches the member before it. Absent on the first, which has none. */
  link?: JobMemberLink;
  /** The branch its pull request targets, where it has one to target. */
  targets?: string;
  /** The address of its pull request. Absent until it has opened one. */
  pullRequest?: string;
  /** What that address is called on screen — `#1591`, never the whole URL. */
  pullRequestLabel?: string;
  /** What the pull request is — `merged`, `open`, `draft`. */
  pullRequestState?: string;
  branch?: string;
  /** The paths it writes. */
  scope?: readonly string[];
  /**
   * Its plan, one segment per task not dropped. **A bar, never a count in
   * words** — read down a column of members it says which one is near the end,
   * where `4 of 4` has to be converted first.
   */
  tasks?: readonly TaskBarSegment[];
  /** The same plan counted, ending the row — `9 tasks`. */
  tasksSaid?: string;
  /** Why somebody dropped it. Present only on a dropped member. */
  dropped?: string;
  /**
   * The refusal this member is holding a person up on, in one line. **The row
   * says a question is open; the column beside it is where it is answered** —
   * three inline answer blocks would be three decisions on one screen.
   */
  asked?: ReactNode;
  /** Open this member's own Job. */
  onOpen?: () => void;
  /**
   * Drop it: close its pull request, keep its branch, and rebase whatever was
   * stacked on it onto the one before. **Absent draws no control** — a member
   * that already landed or was already dropped has nothing to drop.
   */
  onDrop?: (reason: string) => void;
  /**
   * Open the pull request where it lives. **Bound by the caller and taking no
   * address**: what is sent is the member's Job id, so the string this row
   * drew never decides what opens. Nothing in Bridge navigates.
   */
  onOpenPullRequest?: () => void;
};

/** The line drawn between two members, which is how they are related. */
export type JobMemberJoin = {
  /** What the edge does, in one sentence naming both members. */
  said: ReactNode;
  /** Which of the three it is, in the wire's own word. */
  link: JobMemberLink;
};

export type JobMembersProps = {
  /** Every member, in the order they land. */
  members: readonly JobMemberRow[];
  /**
   * The line between one member and the next, one shorter than `members`:
   * `joins[0]` is drawn between the first member and the second. An entry left
   * out draws no line, which is a member whose edge nothing on the wire says.
   */
  joins?: readonly (JobMemberJoin | undefined)[];
  /** What the set is, beside its name — `three members, a pull request each`. */
  said?: ReactNode;
  /** How many members are holding a question, where any are. */
  waiting?: number;
  /**
   * What has to happen before this Job is finished, and how far along that is
   * — `Every member's pull request has merged. One of three is in.`
   */
  completeWhen: ReactNode;
  /** One segment per member, filled where that member's pull request merged. */
  completeBar?: readonly TaskBarSegment[];
  /** Why there are no members, where there are none. Absent draws nothing. */
  absent?: ReactNode;
  /**
   * The rows and the lines between them, with no card, no head and no foot.
   * **For a guide's figure**, which draws the relation rather than the region
   * — and which must hold no control, so the head's `?` is not rendered
   * rather than hidden.
   */
  bare?: boolean;
  /** A sentence the surface says once, briefly — `Dropped`. */
  onSaid?: (sentence: string) => void;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied?: (value: string) => void;
};

/** The name of the region, so the band and the label cannot drift apart. */
export const TRAIN_LABEL = "The train";

/** What the footer's own label says. */
export const COMPLETE_WHEN_LABEL = "Complete when";

/**
 * A Job whose members are Jobs — several pull requests, landing in a set order.
 *
 * **One member is one row, not a card of labelled fields.** The board puts the
 * mark, the number, the title, the pull request, the branch, the scope, a
 * progress bar and the state on two lines inside one box, so three members and
 * the decision beside them fit on one screen.
 *
 * **The order is the point**, so the number leads each row and the line drawn
 * between two rows says what the edge between them does.
 *
 * **Landed is the merge**, never `completed_success`: a Job handed off to a
 * person finishes while its pull request is still open.
 */
export function JobMembers({
  members,
  joins = [],
  said,
  waiting,
  completeWhen,
  completeBar,
  absent,
  bare,
  onSaid,
  onCopied,
}: JobMembersProps) {
  if (members.length === 0) {
    return absent === undefined ? null : (
      <p className="armada-members__absent" role="note">
        {absent}
      </p>
    );
  }
  return (
    <section className="armada-members" data-bare={bare || undefined} aria-label={TRAIN_LABEL}>
      {bare ? null : (
      <div className="armada-members__head">
        <h3 className="armada-members__name">{TRAIN_LABEL}</h3>
        {/* What the three links are is true of a member that never ran, so
            the joins say which one they have and this says what they mean. */}
        <GuideMark guide={GUIDE_MEMBER_LINK} />
        {said === undefined ? null : <p className="armada-members__said">{said}</p>}
        <span className="armada-members__head-spacer" />
        {waiting === undefined || waiting === 0 ? null : (
          <span className="armada-members__waiting">{`${waiting} waiting on you`}</span>
        )}
      </div>
      )}

      {/* The order is the fact the list's own name has to carry. */}
      <ol className="armada-members__list" aria-label="Pull requests, in the order they land">
        {members.map((member, at) => {
          const join = at === 0 ? undefined : joins[at - 1];
          return (
            <li
              className="armada-members__entry"
              key={member.id}
              aria-label={`${member.ordinal}. ${member.title}`}
            >
              {join === undefined ? null : (
                <div className="armada-members__join">
                  <span className="armada-members__join-rule" aria-hidden />
                  <p className="armada-members__join-said">
                    {join.said} <span className="armada-members__join-link">{join.link}</span>
                  </p>
                </div>
              )}
              <Member member={member} onSaid={onSaid} onCopied={onCopied} />
            </li>
          );
        })}
      </ol>

      {bare ? null : <span className="armada-members__list-spacer" />}

      {/* The total closes the region rather than heading it: a reader who has
          just counted three rows wants it, and one who has not is being asked
          to hold a promise. */}
      {bare ? null : (
      <div className="armada-members__complete">
        <span className="armada-members__complete-label">{COMPLETE_WHEN_LABEL}</span>
        <p className="armada-members__complete-said">{completeWhen}</p>
        {completeBar === undefined ? null : (
          <StepBar
            tasks={completeBar}
            {...(typeof completeWhen === "string" ? { label: completeWhen } : {})}
          />
        )}
      </div>
      )}
    </section>
  );
}

function Member({
  member,
  onSaid,
  onCopied,
}: {
  member: JobMemberRow;
  onSaid?: (sentence: string) => void;
  onCopied?: (value: string) => void;
}) {
  const copy = useCallback(
    (event: MouseEvent<HTMLElement>, value: string) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(() => onCopied?.(value));
    },
    [onCopied],
  );
  const address = member.pullRequest;
  const branch = member.branch;
  const state = member.state;
  const status = state.as === "badge" ? state.status : undefined;
  // The trigger sits on the row and the form under it, so a row that offers a
  // drop is the same two lines tall as one that does not.
  const [dropping, setDropping] = useState(false);

  return (
    <div
      className="armada-members__member"
      data-dropped={member.dropped === undefined ? undefined : true}
      data-asking={member.asked === undefined ? undefined : true}
    >
      <div className="armada-members__line">
        {/* The state as a square of its own hue, so it reads down the left
            edge of the list. Hue by inline token reference, `Badge`'s own
            rule: the stem is data, so it cannot be a stylesheet's selector. */}
        <span
          className="armada-members__mark"
          data-status={status}
          aria-hidden
          {...(status === undefined ? {} : { style: { background: `var(--status-${status})` } })}
        />
        <span className="armada-members__ordinal">{`Member ${member.ordinal}`}</span>
        {member.onOpen === undefined ? (
          <span className="armada-members__title">{member.title}</span>
        ) : (
          <button type="button" className="armada-members__open" onClick={member.onOpen}>
            {member.title}
          </button>
        )}
        {address === undefined ? null : (
          <a
            href={address}
            className="armada-members__pr"
            data-status={status}
            {...(status === undefined
              ? {}
              : {
                  style: {
                    color: `var(--status-${status})`,
                    background: `var(--status-${status}-bg)`,
                  },
                })}
            onClick={(event) => {
              event.preventDefault();
              member.onOpenPullRequest?.();
            }}
          >
            <span className="armada-members__pr-number">{member.pullRequestLabel ?? address}</span>
            {member.pullRequestState === undefined ? null : (
              <span className="armada-members__pr-state">{member.pullRequestState}</span>
            )}
          </a>
        )}
        {member.onDrop === undefined || member.dropped !== undefined || dropping ? null : (
          <button
            type="button"
            className="armada-members__drop-open"
            onClick={() => setDropping(true)}
          >
            {`${DROP_MEMBER_LABEL}…`}
          </button>
        )}
      </div>

      <div className="armada-members__line">
        {branch === undefined ? null : (
          <button
            type="button"
            className="armada-members__branch"
            title="Copy"
            onClick={(event) => copy(event, branch)}
          >
            {branch}
          </button>
        )}
        {member.targets === undefined ? null : (
          <span className="armada-members__targets">{`→ ${member.targets}`}</span>
        )}
        {member.scope?.map((path) => (
          <span className="armada-members__scope" key={path}>
            {path}
          </span>
        ))}
        <span className="armada-members__line-spacer" />
        {member.tasks === undefined || member.tasks.length === 0 ? null : (
          <StepBar tasks={member.tasks} />
        )}
        {member.tasksSaid === undefined ? null : (
          <span className="armada-members__tasks">{member.tasksSaid}</span>
        )}
        <span
          className="armada-members__state"
          data-status={status}
          {...(status === undefined ? {} : { style: { color: `var(--status-${status})` } })}
          {...(state.as === "text" ? { title: state.missing } : {})}
        >
          {state.as === "badge" ? state.label : state.wire}
        </span>
      </div>

      {member.asked === undefined ? null : (
        <p className="armada-members__asked">{member.asked}</p>
      )}
      {member.dropped === undefined ? null : (
        <p className="armada-members__dropped">{`Dropped. ${member.dropped} Its branch is kept.`}</p>
      )}
      {dropping ? (
        <DropMember member={member} onSaid={onSaid} onClose={() => setDropping(false)} />
      ) : null}
    </div>
  );
}

/**
 * Drop a member: close its pull request, keep its branch, and rebase whatever
 * was stacked on it onto the one before.
 *
 * **The reason is inline, not a dialog** — the plan's own drop settled that: a
 * modal over the list would hide the member the reason is about.
 */
function DropMember({
  member,
  onSaid,
  onClose,
}: {
  member: JobMemberRow;
  onSaid?: (sentence: string) => void;
  onClose: () => void;
}) {
  const [reason, setReason] = useState("");
  // A pristine field is a hint, never an error: a red border on a field
  // nobody has touched reads as a mistake that already happened.
  const [attemptedEmpty, setAttemptedEmpty] = useState(false);
  const blank = reason.trim() === "";

  function drop(): void {
    if (blank) {
      setAttemptedEmpty(true);
      return;
    }
    member.onDrop?.(reason.trim());
    onSaid?.("Dropped");
    onClose();
  }

  return (
    <div className="armada-members__drop">
      <p className="armada-members__drop-said">
        Its pull request is closed and its branch is kept. Whatever is stacked on it rebases onto
        the one before.
      </p>
      <Input
        label="Reason"
        value={reason}
        invalid={attemptedEmpty && blank}
        onChange={(event) => setReason(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") onClose();
          if (event.key === "Enter") drop();
        }}
      />
      {blank ? (
        <span className="armada-members__hint" data-tone={attemptedEmpty ? "error" : "muted"}>
          A reason is needed.
        </span>
      ) : null}
      <div className="armada-members__acts">
        <Button variant="secondary" size="sm" ground="sunken" onClick={onClose}>
          Cancel
        </Button>
        <Button variant="secondary" size="sm" ground="sunken" disabled={blank} onClick={drop}>
          {DROP_MEMBER_LABEL}
        </Button>
      </div>
    </div>
  );
}
