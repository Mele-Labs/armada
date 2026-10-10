//! Every prompt piece Fleet ships beyond `super`'s own list: the key and
//! the words that ship. Generated from one table with the config entries.

use config::settings as keys;
use config::settings::Words;

pub(super) fn drone() -> Vec<(Words, &'static str)> {
    vec![
        (keys::PROMPT_DRONE_NOTEKEEPING, crate::briefing::NOTEKEEPING),
        (keys::PROMPT_DRONE_JOB_BRIEF, crate::briefing::JOB_BRIEF),
        (
            keys::PROMPT_DRONE_JOB_ATTACHMENTS,
            crate::briefing::JOB_ATTACHMENTS,
        ),
        (
            keys::PROMPT_DRONE_JOB_DONE_WHEN,
            crate::briefing::JOB_DONE_WHEN,
        ),
        (
            keys::PROMPT_DRONE_WHERE_YOU_ARE,
            crate::briefing::WHERE_YOU_ARE,
        ),
        (keys::PROMPT_DRONE_YOU_ARE_ON, crate::briefing::YOU_ARE_ON),
        (keys::PROMPT_DRONE_PART_DONE, crate::briefing::PART_DONE),
        (keys::PROMPT_DRONE_PART_HERE, crate::briefing::PART_HERE),
        (
            keys::PROMPT_DRONE_PART_NOT_YOURS,
            crate::briefing::PART_NOT_YOURS,
        ),
        (keys::PROMPT_DRONE_PART_STOP, crate::briefing::PART_STOP),
        (keys::PROMPT_DRONE_PARTS_AFTER, crate::briefing::PARTS_AFTER),
        (keys::PROMPT_DRONE_STEP, crate::briefing::STEP),
        (
            keys::PROMPT_DRONE_FOR_THIS_PART,
            crate::briefing::FOR_THIS_PART,
        ),
        (keys::PROMPT_DRONE_AGAIN, crate::briefing::AGAIN),
        (
            keys::PROMPT_DRONE_AGAIN_ANSWER,
            crate::briefing::AGAIN_ANSWER,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_NOTHING_CITED,
            crate::briefing::AGAIN_NOTHING_CITED,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_NO_VERDICT,
            crate::briefing::AGAIN_NO_VERDICT,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_GATE_FAILURE,
            crate::briefing::AGAIN_GATE_FAILURE,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_SUSPECT,
            crate::briefing::AGAIN_SUSPECT,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_UNDECIDED,
            crate::briefing::AGAIN_UNDECIDED,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_THRASHING,
            crate::briefing::AGAIN_THRASHING,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_KILLED,
            crate::briefing::AGAIN_KILLED,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_RUN_ENDED,
            crate::briefing::AGAIN_RUN_ENDED,
        ),
        (keys::PROMPT_DRONE_AGAIN_GONE, crate::briefing::AGAIN_GONE),
        (
            keys::PROMPT_DRONE_AGAIN_SCOPE_REFUSED,
            crate::briefing::AGAIN_SCOPE_REFUSED,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_NO_REPORT,
            crate::briefing::AGAIN_NO_REPORT,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_CHECK_TIMEOUT,
            crate::briefing::AGAIN_CHECK_TIMEOUT,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_TOO_LARGE,
            crate::briefing::AGAIN_TOO_LARGE,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_BLOCKED,
            crate::briefing::AGAIN_BLOCKED,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_ASK_UNANSWERED,
            crate::briefing::AGAIN_ASK_UNANSWERED,
        ),
        (
            keys::PROMPT_DRONE_AGAIN_LOOP_CAP,
            crate::briefing::AGAIN_LOOP_CAP,
        ),
        (keys::PROMPT_DRONE_ASKS_USED, crate::briefing::ASKS_USED),
        (keys::PROMPT_DRONE_ASKS_ALL, crate::briefing::ASKS_ALL),
        (keys::PROMPT_DRONE_ASKS_LEFT, crate::briefing::ASKS_LEFT),
        (keys::PROMPT_DRONE_DELIVERS, crate::terms::DELIVERS),
        (
            keys::PROMPT_DRONE_RECORDS_PLAN_TOO,
            crate::terms::RECORDS_PLAN_TOO,
        ),
        (keys::PROMPT_DRONE_RECORDS_PLAN, crate::terms::RECORDS_PLAN),
        (keys::PROMPT_DRONE_CAPTURED, crate::terms::CAPTURED),
        (keys::PROMPT_DRONE_CHECKS, crate::terms::CHECKS),
        (keys::PROMPT_DRONE_CHECKS_ASK, crate::terms::CHECKS_ASK),
        (
            keys::PROMPT_DRONE_CHECKS_WHOLE_ALL,
            crate::terms::CHECKS_WHOLE_ALL,
        ),
        (
            keys::PROMPT_DRONE_CHECKS_WHOLE_EXCEPT,
            crate::terms::CHECKS_WHOLE_EXCEPT,
        ),
        (
            keys::PROMPT_DRONE_CHECKS_SKIPPED,
            crate::terms::CHECKS_SKIPPED,
        ),
        (keys::PROMPT_DRONE_CHECKS_WAIT, crate::terms::CHECKS_WAIT),
        (
            keys::PROMPT_DRONE_CHECKS_AT_GATE,
            crate::terms::CHECKS_AT_GATE,
        ),
        (
            keys::PROMPT_DRONE_CHECKS_AT_HANDOFF,
            crate::terms::CHECKS_AT_HANDOFF,
        ),
        (keys::PROMPT_DRONE_CHECKS_LATER, crate::terms::CHECKS_LATER),
        (
            keys::PROMPT_DRONE_CHECKS_NOT_A_VERDICT,
            crate::terms::CHECKS_NOT_A_VERDICT,
        ),
        (keys::PROMPT_DRONE_CHECKS_LIMIT, crate::terms::CHECKS_LIMIT),
        (keys::PROMPT_DRONE_DECLARE, crate::terms::DECLARE),
        (
            keys::PROMPT_DRONE_DECLARE_COMPARED,
            crate::terms::DECLARE_COMPARED,
        ),
        (
            keys::PROMPT_DRONE_DECLARE_EXCLUDED,
            crate::terms::DECLARE_EXCLUDED,
        ),
        (keys::PROMPT_DRONE_REDECLARE, crate::terms::REDECLARE),
        (
            keys::PROMPT_DRONE_REDECLARE_WHAT,
            crate::terms::REDECLARE_WHAT,
        ),
        (keys::PROMPT_DRONE_SPLIT_DECIDES, crate::terms::DECIDES_IT),
        (keys::PROMPT_DRONE_SPLIT_PROPOSES, crate::terms::PROPOSES_IT),
        (keys::PROMPT_DRONE_REVIEW, crate::review_term::REVIEW),
        (keys::PROMPT_CROSSING_PRODUCED, crate::crossing::PRODUCED),
        (
            keys::PROMPT_CROSSING_PRODUCED_UNRECORDED,
            crate::crossing::PRODUCED_UNRECORDED,
        ),
        (
            keys::PROMPT_CROSSING_PRODUCED_FILE,
            crate::crossing::PRODUCED_FILE,
        ),
        (
            keys::PROMPT_CROSSING_PRODUCED_ON_BRANCH,
            crate::crossing::PRODUCED_ON_BRANCH,
        ),
        (
            keys::PROMPT_CROSSING_NOT_CLAIMED,
            crate::crossing::NOT_CLAIMED,
        ),
        (keys::PROMPT_CROSSING_SETTLED, crate::crossing::SETTLED),
        (
            keys::PROMPT_CROSSING_SETTLED_BY_CHECKS,
            crate::crossing::SETTLED_BY_CHECKS,
        ),
        (
            keys::PROMPT_CROSSING_SETTLED_BY_PERSON,
            crate::crossing::SETTLED_BY_PERSON,
        ),
        (keys::PROMPT_CROSSING_BRANCH, crate::crossing::BRANCH),
        (
            keys::PROMPT_CROSSING_BRANCH_UP_TO_DATE,
            crate::crossing::BRANCH_UP_TO_DATE,
        ),
        (
            keys::PROMPT_CROSSING_BRANCH_CONFLICTED,
            crate::crossing::BRANCH_CONFLICTED,
        ),
        (
            keys::PROMPT_CROSSING_BRANCH_NOT_MOVED,
            crate::crossing::BRANCH_NOT_MOVED,
        ),
        (keys::PROMPT_CROSSING_PLAN, crate::crossing::PLAN),
        (
            keys::PROMPT_CROSSING_PLAN_FOLLOW,
            crate::crossing::PLAN_FOLLOW,
        ),
        (keys::PROMPT_CROSSING_PLAN_READ, crate::crossing::PLAN_READ),
        (keys::PROMPT_CROSSING_PLAN_TASK, crate::crossing::PLAN_TASK),
        (
            keys::PROMPT_CROSSING_DISPATCHED,
            crate::crossing::DISPATCHED,
        ),
        (keys::PROMPT_CROSSING_SENT_BACK, crate::crossing::SENT_BACK),
        (
            keys::PROMPT_CROSSING_SENT_BACK_UNRECORDED,
            crate::crossing::SENT_BACK_UNRECORDED,
        ),
        (
            keys::PROMPT_CROSSING_SENT_BACK_FILE,
            crate::crossing::SENT_BACK_FILE,
        ),
        (
            keys::PROMPT_CROSSING_SENT_BACK_ADDRESS,
            crate::crossing::SENT_BACK_ADDRESS,
        ),
        (keys::PROMPT_CROSSING_DISMISSED, crate::crossing::DISMISSED),
        (
            keys::PROMPT_CROSSING_DISMISSED_ROW,
            crate::crossing::DISMISSED_ROW,
        ),
        (keys::PROMPT_FIX_HELD_OFF, crate::fixing::HELD_OFF),
        (
            keys::PROMPT_FIX_HELD_OFF_FIXING,
            crate::fixing::HELD_OFF_FIXING,
        ),
        (
            keys::PROMPT_FIX_HELD_OFF_LANDED,
            crate::fixing::HELD_OFF_LANDED,
        ),
        (keys::PROMPT_FIX_FIXING, crate::fixing::FIX_FIXING),
        (
            keys::PROMPT_FIX_FIXING_HOLDS,
            crate::fixing::FIX_FIXING_HOLDS,
        ),
        (keys::PROMPT_FIX_LANDED, crate::fixing::FIX_LANDED),
        (
            keys::PROMPT_FIX_LANDED_HOLDS,
            crate::fixing::FIX_LANDED_HOLDS,
        ),
        (
            keys::PROMPT_FIX_IN_YOUR_COPY,
            crate::fixing::FIX_IN_YOUR_COPY,
        ),
        (keys::PROMPT_FIX_GONE, crate::fixing::FIX_GONE),
        (keys::PROMPT_FIX_GONE_HOLDS, crate::fixing::FIX_GONE_HOLDS),
        (keys::PROMPT_PEERS, crate::peers::HEADING),
        (keys::PROMPT_PEERS_SHARING, crate::peers::SHARING),
        (keys::PROMPT_PEERS_AHEAD, crate::peers::AHEAD),
        (keys::PROMPT_PEERS_FIX, crate::peers::FIX),
        (keys::PROMPT_PEERS_MORE, crate::peers::MORE),
        (keys::PROMPT_PEERS_CLAIMED, crate::peers::CLAIMED),
        (keys::PROMPT_PEERS_LANDED, crate::peers::LANDED),
        (keys::PROMPT_PEERS_NOTE, crate::peers::NOTE),
        (keys::PROMPT_PEERS_AHEAD_ON, crate::peers::AHEAD_ON),
        (keys::PROMPT_PEERS_LANDED_LATER, crate::peers::LANDED_LATER),
        (keys::PROMPT_PEERS_LANDED_NOW, crate::peers::LANDED_NOW),
        (keys::PROMPT_PEERS_NEXT_NUMBER, crate::peers::NEXT_NUMBER),
        (keys::PROMPT_PEERS_CARRY_ON, crate::peers::CARRY_ON),
        (keys::PROMPT_PLAN_ADDED, crate::work_plan::ADDED),
        (keys::PROMPT_PLAN_DROPPED, crate::work_plan::DROPPED),
        (keys::PROMPT_PLAN_GROUP_ROUND, crate::work_plan::GROUP_ROUND),
        (keys::PROMPT_HEAL_INDEX, crate::healing::INDEX),
        (keys::PROMPT_HEAL_INSTALL, crate::healing::INSTALL),
        (keys::PROMPT_SIDE_SKILL, crate::side_run::SKILL),
        (keys::PROMPT_SIDE_CONTEXT, crate::side_run::CONTEXT),
        (
            keys::PROMPT_SIDE_CONTEXT_ON_BRANCH,
            crate::side_run::CONTEXT_ON_BRANCH,
        ),
        (keys::PROMPT_SIDE_BEFORE_STEP, crate::side_run::BEFORE_STEP),
        (keys::PROMPT_SIDE_AFTER_STEP, crate::side_run::AFTER_STEP),
        (keys::PROMPT_SIDE_AFTER_PR, crate::side_run::AFTER_PR),
        (keys::PROMPT_SIDE_REST, crate::side_run::REST),
        (keys::PROMPT_HELM_VOICE, crate::helm::VOICE),
        (keys::PROMPT_POKE_QUIET, crate::silence::POKE_QUIET),
        (
            keys::PROMPT_POKE_BACKGROUND,
            crate::silence::POKE_BACKGROUND,
        ),
        (keys::PROMPT_INTERRUPT, crate::converging::INTERRUPT),
        (keys::PROMPT_PERMIT_JOB, crate::permitting::PERMIT_JOB),
        (
            keys::PROMPT_PERMIT_REPOSITORY,
            crate::permitting::PERMIT_REPOSITORY,
        ),
        (
            keys::PROMPT_PERMIT_MACHINE,
            crate::permitting::PERMIT_MACHINE,
        ),
        (
            keys::PROMPT_PERMIT_REJECTED,
            crate::permitting::PERMIT_REJECTED,
        ),
        (
            keys::PROMPT_PERMIT_REJECTED_NOTE,
            crate::permitting::PERMIT_REJECTED_NOTE,
        ),
        (
            keys::PROMPT_PILOT_OPENING,
            crate::session_host::PILOT_OPENING,
        ),
        (
            keys::PROMPT_PILOT_WORKTREE,
            crate::session_host::PILOT_WORKTREE,
        ),
        (
            keys::PROMPT_PILOT_RESTART,
            crate::session_host::PILOT_RESTART,
        ),
        (
            keys::PROMPT_PILOT_FOR_GOOD,
            crate::session_host::PILOT_FOR_GOOD,
        ),
        (
            keys::PROMPT_PILOT_STOPPED,
            crate::session_host::PILOT_STOPPED,
        ),
        (
            keys::PROMPT_PILOT_JUDGE_REFUSED,
            crate::session_host::PILOT_JUDGE_REFUSED,
        ),
        (
            keys::PROMPT_PILOT_CHECKS_FAILED,
            crate::session_host::PILOT_CHECKS_FAILED,
        ),
        (
            keys::PROMPT_PILOT_OUTSIDE_PLAN,
            crate::session_host::PILOT_OUTSIDE_PLAN,
        ),
        (
            keys::PROMPT_PILOT_CHANGED,
            crate::session_host::PILOT_CHANGED,
        ),
        (
            keys::PROMPT_PILOT_NARRATIVE,
            crate::session_host::PILOT_NARRATIVE,
        ),
        (keys::PROMPT_PILOT_RECORD, crate::session_host::PILOT_RECORD),
    ]
}
