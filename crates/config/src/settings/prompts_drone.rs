//! The pieces of a Drone's brief and turns, each editable. What ships is Fleet's.

use super::prompts::{prompt, PromptSection};
use super::{Entry, Words};

pub const PROMPT_DRONE_NOTEKEEPING: Words = Words("prompts.droneNotekeeping");
pub const PROMPT_DRONE_JOB_BRIEF: Words = Words("prompts.droneJobBrief");
pub const PROMPT_DRONE_JOB_ATTACHMENTS: Words = Words("prompts.droneJobAttachments");
pub const PROMPT_DRONE_JOB_DONE_WHEN: Words = Words("prompts.droneJobDoneWhen");
pub const PROMPT_DRONE_WHERE_YOU_ARE: Words = Words("prompts.droneWhereYouAre");
pub const PROMPT_DRONE_YOU_ARE_ON: Words = Words("prompts.droneYouAreOn");
pub const PROMPT_DRONE_PART_DONE: Words = Words("prompts.dronePartDone");
pub const PROMPT_DRONE_PART_HERE: Words = Words("prompts.dronePartHere");
pub const PROMPT_DRONE_PART_NOT_YOURS: Words = Words("prompts.dronePartNotYours");
pub const PROMPT_DRONE_PART_STOP: Words = Words("prompts.dronePartStop");
pub const PROMPT_DRONE_PARTS_AFTER: Words = Words("prompts.dronePartsAfter");
pub const PROMPT_DRONE_STEP: Words = Words("prompts.droneStep");
pub const PROMPT_DRONE_FOR_THIS_PART: Words = Words("prompts.droneForThisPart");
pub const PROMPT_DRONE_AGAIN: Words = Words("prompts.droneAgain");
pub const PROMPT_DRONE_AGAIN_ANSWER: Words = Words("prompts.droneAgainAnswer");
pub const PROMPT_DRONE_AGAIN_NOTHING_CITED: Words = Words("prompts.droneAgainNothingCited");
pub const PROMPT_DRONE_AGAIN_NO_VERDICT: Words = Words("prompts.droneAgainNoVerdict");
pub const PROMPT_DRONE_AGAIN_GATE_FAILURE: Words = Words("prompts.droneAgainGateFailure");
pub const PROMPT_DRONE_AGAIN_SUSPECT: Words = Words("prompts.droneAgainSuspect");
pub const PROMPT_DRONE_AGAIN_UNDECIDED: Words = Words("prompts.droneAgainUndecided");
pub const PROMPT_DRONE_AGAIN_THRASHING: Words = Words("prompts.droneAgainThrashing");
pub const PROMPT_DRONE_AGAIN_KILLED: Words = Words("prompts.droneAgainKilled");
pub const PROMPT_DRONE_AGAIN_RUN_ENDED: Words = Words("prompts.droneAgainRunEnded");
pub const PROMPT_DRONE_AGAIN_GONE: Words = Words("prompts.droneAgainGone");
pub const PROMPT_DRONE_AGAIN_SCOPE_REFUSED: Words = Words("prompts.droneAgainScopeRefused");
pub const PROMPT_DRONE_AGAIN_NO_REPORT: Words = Words("prompts.droneAgainNoReport");
pub const PROMPT_DRONE_AGAIN_CHECK_TIMEOUT: Words = Words("prompts.droneAgainCheckTimeout");
pub const PROMPT_DRONE_AGAIN_TOO_LARGE: Words = Words("prompts.droneAgainTooLarge");
pub const PROMPT_DRONE_AGAIN_BLOCKED: Words = Words("prompts.droneAgainBlocked");
pub const PROMPT_DRONE_AGAIN_ASK_UNANSWERED: Words = Words("prompts.droneAgainAskUnanswered");
pub const PROMPT_DRONE_AGAIN_LOOP_CAP: Words = Words("prompts.droneAgainLoopCap");
pub const PROMPT_DRONE_ASKS_USED: Words = Words("prompts.droneAsksUsed");
pub const PROMPT_DRONE_ASKS_ALL: Words = Words("prompts.droneAsksAll");
pub const PROMPT_DRONE_ASKS_LEFT: Words = Words("prompts.droneAsksLeft");
pub const PROMPT_DRONE_DELIVERS: Words = Words("prompts.droneDelivers");
pub const PROMPT_DRONE_RECORDS_PLAN_TOO: Words = Words("prompts.droneRecordsPlanToo");
pub const PROMPT_DRONE_RECORDS_PLAN: Words = Words("prompts.droneRecordsPlan");
pub const PROMPT_DRONE_CAPTURED: Words = Words("prompts.droneCaptured");
pub const PROMPT_DRONE_CHECKS: Words = Words("prompts.droneChecks");
pub const PROMPT_DRONE_CHECKS_ASK: Words = Words("prompts.droneChecksAsk");
pub const PROMPT_DRONE_CHECKS_WHOLE_ALL: Words = Words("prompts.droneChecksWholeAll");
pub const PROMPT_DRONE_CHECKS_WHOLE_EXCEPT: Words = Words("prompts.droneChecksWholeExcept");
pub const PROMPT_DRONE_CHECKS_SKIPPED: Words = Words("prompts.droneChecksSkipped");
pub const PROMPT_DRONE_CHECKS_WAIT: Words = Words("prompts.droneChecksWait");
pub const PROMPT_DRONE_CHECKS_AT_GATE: Words = Words("prompts.droneChecksAtGate");
pub const PROMPT_DRONE_CHECKS_AT_HANDOFF: Words = Words("prompts.droneChecksAtHandoff");
pub const PROMPT_DRONE_CHECKS_LATER: Words = Words("prompts.droneChecksLater");
pub const PROMPT_DRONE_CHECKS_NOT_A_VERDICT: Words = Words("prompts.droneChecksNotAVerdict");
pub const PROMPT_DRONE_CHECKS_LIMIT: Words = Words("prompts.droneChecksLimit");
pub const PROMPT_DRONE_DECLARE: Words = Words("prompts.droneDeclare");
pub const PROMPT_DRONE_DECLARE_COMPARED: Words = Words("prompts.droneDeclareCompared");
pub const PROMPT_DRONE_DECLARE_EXCLUDED: Words = Words("prompts.droneDeclareExcluded");
pub const PROMPT_DRONE_REDECLARE: Words = Words("prompts.droneRedeclare");
pub const PROMPT_DRONE_REDECLARE_WHAT: Words = Words("prompts.droneRedeclareWhat");
pub const PROMPT_DRONE_SPLIT_DECIDES: Words = Words("prompts.droneSplitDecides");
pub const PROMPT_DRONE_SPLIT_PROPOSES: Words = Words("prompts.droneSplitProposes");
pub const PROMPT_DRONE_REVIEW: Words = Words("prompts.droneReview");
pub const PROMPT_CROSSING_PRODUCED: Words = Words("prompts.crossingProduced");
pub const PROMPT_CROSSING_PRODUCED_UNRECORDED: Words = Words("prompts.crossingProducedUnrecorded");
pub const PROMPT_CROSSING_PRODUCED_FILE: Words = Words("prompts.crossingProducedFile");
pub const PROMPT_CROSSING_PRODUCED_ON_BRANCH: Words = Words("prompts.crossingProducedOnBranch");
pub const PROMPT_CROSSING_NOT_CLAIMED: Words = Words("prompts.crossingNotClaimed");
pub const PROMPT_CROSSING_SETTLED: Words = Words("prompts.crossingSettled");
pub const PROMPT_CROSSING_SETTLED_BY_CHECKS: Words = Words("prompts.crossingSettledByChecks");
pub const PROMPT_CROSSING_SETTLED_BY_PERSON: Words = Words("prompts.crossingSettledByPerson");
pub const PROMPT_CROSSING_BRANCH: Words = Words("prompts.crossingBranch");
pub const PROMPT_CROSSING_BRANCH_UP_TO_DATE: Words = Words("prompts.crossingBranchUpToDate");
pub const PROMPT_CROSSING_BRANCH_CONFLICTED: Words = Words("prompts.crossingBranchConflicted");
pub const PROMPT_CROSSING_BRANCH_NOT_MOVED: Words = Words("prompts.crossingBranchNotMoved");
pub const PROMPT_CROSSING_PLAN: Words = Words("prompts.crossingPlan");
pub const PROMPT_CROSSING_PLAN_FOLLOW: Words = Words("prompts.crossingPlanFollow");
pub const PROMPT_CROSSING_PLAN_READ: Words = Words("prompts.crossingPlanRead");
pub const PROMPT_CROSSING_PLAN_TASK: Words = Words("prompts.crossingPlanTask");
pub const PROMPT_CROSSING_DISPATCHED: Words = Words("prompts.crossingDispatched");
pub const PROMPT_CROSSING_SENT_BACK: Words = Words("prompts.crossingSentBack");
pub const PROMPT_CROSSING_SENT_BACK_UNRECORDED: Words = Words("prompts.crossingSentBackUnrecorded");
pub const PROMPT_CROSSING_SENT_BACK_FILE: Words = Words("prompts.crossingSentBackFile");
pub const PROMPT_CROSSING_SENT_BACK_ADDRESS: Words = Words("prompts.crossingSentBackAddress");
pub const PROMPT_CROSSING_DISMISSED: Words = Words("prompts.crossingDismissed");
pub const PROMPT_CROSSING_DISMISSED_ROW: Words = Words("prompts.crossingDismissedRow");
pub const PROMPT_FIX_HELD_OFF: Words = Words("prompts.fixHeldOff");
pub const PROMPT_FIX_HELD_OFF_FIXING: Words = Words("prompts.fixHeldOffFixing");
pub const PROMPT_FIX_HELD_OFF_LANDED: Words = Words("prompts.fixHeldOffLanded");
pub const PROMPT_FIX_FIXING: Words = Words("prompts.fixFixing");
pub const PROMPT_FIX_FIXING_HOLDS: Words = Words("prompts.fixFixingHolds");
pub const PROMPT_FIX_LANDED: Words = Words("prompts.fixLanded");
pub const PROMPT_FIX_LANDED_HOLDS: Words = Words("prompts.fixLandedHolds");
pub const PROMPT_FIX_IN_YOUR_COPY: Words = Words("prompts.fixInYourCopy");
pub const PROMPT_FIX_GONE: Words = Words("prompts.fixGone");
pub const PROMPT_FIX_GONE_HOLDS: Words = Words("prompts.fixGoneHolds");
pub const PROMPT_PEERS: Words = Words("prompts.peers");
pub const PROMPT_PEERS_SHARING: Words = Words("prompts.peersSharing");
pub const PROMPT_PEERS_AHEAD: Words = Words("prompts.peersAhead");
pub const PROMPT_PEERS_FIX: Words = Words("prompts.peersFix");
pub const PROMPT_PEERS_MORE: Words = Words("prompts.peersMore");
pub const PROMPT_PEERS_CLAIMED: Words = Words("prompts.peersClaimed");
pub const PROMPT_PEERS_LANDED: Words = Words("prompts.peersLanded");
pub const PROMPT_PEERS_NOTE: Words = Words("prompts.peersNote");
pub const PROMPT_PEERS_AHEAD_ON: Words = Words("prompts.peersAheadOn");
pub const PROMPT_PEERS_LANDED_LATER: Words = Words("prompts.peersLandedLater");
pub const PROMPT_PEERS_LANDED_NOW: Words = Words("prompts.peersLandedNow");
pub const PROMPT_PEERS_NEXT_NUMBER: Words = Words("prompts.peersNextNumber");
pub const PROMPT_PEERS_CARRY_ON: Words = Words("prompts.peersCarryOn");
pub const PROMPT_PLAN_ADDED: Words = Words("prompts.planAdded");
pub const PROMPT_PLAN_DROPPED: Words = Words("prompts.planDropped");
pub const PROMPT_PLAN_GROUP_ROUND: Words = Words("prompts.planGroupRound");
pub const PROMPT_HEAL_INDEX: Words = Words("prompts.healIndex");
pub const PROMPT_HEAL_INSTALL: Words = Words("prompts.healInstall");
pub const PROMPT_SIDE_SKILL: Words = Words("prompts.sideSkill");
pub const PROMPT_SIDE_CONTEXT: Words = Words("prompts.sideContext");
pub const PROMPT_SIDE_CONTEXT_ON_BRANCH: Words = Words("prompts.sideContextOnBranch");
pub const PROMPT_SIDE_BEFORE_STEP: Words = Words("prompts.sideBeforeStep");
pub const PROMPT_SIDE_AFTER_STEP: Words = Words("prompts.sideAfterStep");
pub const PROMPT_SIDE_AFTER_PR: Words = Words("prompts.sideAfterPr");
pub const PROMPT_SIDE_REST: Words = Words("prompts.sideRest");
pub const PROMPT_HELM_VOICE: Words = Words("prompts.helmVoice");
pub const PROMPT_POKE_QUIET: Words = Words("prompts.pokeQuiet");
pub const PROMPT_POKE_BACKGROUND: Words = Words("prompts.pokeBackground");
pub const PROMPT_INTERRUPT: Words = Words("prompts.interrupt");
pub const PROMPT_PERMIT_JOB: Words = Words("prompts.permitJob");
pub const PROMPT_PERMIT_REPOSITORY: Words = Words("prompts.permitRepository");
pub const PROMPT_PERMIT_MACHINE: Words = Words("prompts.permitMachine");
pub const PROMPT_PERMIT_REJECTED: Words = Words("prompts.permitRejected");
pub const PROMPT_PERMIT_REJECTED_NOTE: Words = Words("prompts.permitRejectedNote");
pub const PROMPT_PILOT_OPENING: Words = Words("prompts.pilotOpening");
pub const PROMPT_PILOT_WORKTREE: Words = Words("prompts.pilotWorktree");
pub const PROMPT_PILOT_RESTART: Words = Words("prompts.pilotRestart");
pub const PROMPT_PILOT_FOR_GOOD: Words = Words("prompts.pilotForGood");
pub const PROMPT_PILOT_STOPPED: Words = Words("prompts.pilotStopped");
pub const PROMPT_PILOT_JUDGE_REFUSED: Words = Words("prompts.pilotJudgeRefused");
pub const PROMPT_PILOT_CHECKS_FAILED: Words = Words("prompts.pilotChecksFailed");
pub const PROMPT_PILOT_OUTSIDE_PLAN: Words = Words("prompts.pilotOutsidePlan");
pub const PROMPT_PILOT_CHANGED: Words = Words("prompts.pilotChanged");
pub const PROMPT_PILOT_NARRATIVE: Words = Words("prompts.pilotNarrative");
pub const PROMPT_PILOT_RECORD: Words = Words("prompts.pilotRecord");

pub(super) const DRONE: &[Entry] = &[
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_NOTEKEEPING, "Notes it keeps", "Where a Drone keeps files it writes for itself. {job} is the Job's id.", &["job"]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_JOB_BRIEF, "The Job", "The block naming the Job. {title} is its title; what the person wrote follows it.", &["title"]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_JOB_ATTACHMENTS, "Attached files", "The line the Job's attached files follow.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_JOB_DONE_WHEN, "Done when", "The line the Job's acceptance criteria follow.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_WHERE_YOU_ARE, "Where it is", "The block above the list of parts. {parts} is how many there are.", &["parts"]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_YOU_ARE_ON, "Its part", "The sentence saying which part a Drone is on. {part} is its number.", &["part"]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_PART_DONE, "A part done", "How a part before this one is marked in the list.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_PART_HERE, "This part", "How the part a Drone is on is marked in the list.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_PART_NOT_YOURS, "A part after", "How a part after this one is marked in the list.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_PART_STOP, "Stop at this part", "The line under the part a Drone is on.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_PARTS_AFTER, "The parts after", "What a Drone is told about the parts after its own.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_STEP, "The step", "The block naming the step. {step} is its label.", &["step"]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_FOR_THIS_PART, "A note for this part", "The block carrying what the approver left for this part. {note} is their words.", &["note"]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN, "Why again", "The block a restarted Drone reads first. {why} is one of the reasons below.", &["why"]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_ANSWER, "Answer what was cited", "What a restarted Drone is told where something was cited against the last attempt.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_NOTHING_CITED, "Nothing cited", "What a restarted Drone is told where nothing was cited.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_NO_VERDICT, "Why again, no verdict", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_GATE_FAILURE, "Why again, checks failed", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_SUSPECT, "Why again, evidence not accepted", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_UNDECIDED, "Why again, never checked", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_THRASHING, "Why again, stopped mid-run", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_KILLED, "Why again, ended by a person", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_RUN_ENDED, "Why again, said its run was over", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_GONE, "Why again, found gone", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_SCOPE_REFUSED, "Why again, scope refused", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_NO_REPORT, "Why again, did not report", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_CHECK_TIMEOUT, "Why again, a check timed out", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_TOO_LARGE, "Why again, evidence too large", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_BLOCKED, "Why again, refused a tool", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_ASK_UNANSWERED, "Why again, nobody answered", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::DroneBrief, PROMPT_DRONE_AGAIN_LOOP_CAP, "Why again, out of rounds", "One reason a restarted Drone is given for the last attempt stopping.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_ASKS_USED, "Whole runs used up", "What a Drone is told when it has asked for every check as often as it may. {allowed} is how often.", &["allowed"]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_ASKS_ALL, "Whole runs it may ask", "What a Drone is told about whole runs before it has asked for any. {allowed} is how many.", &["allowed"]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_ASKS_LEFT, "Whole runs left", "What a Drone is told about whole runs part-way. {allowed} is how many and {left} how many remain.", &["allowed", "left"]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_DELIVERS, "The file a part delivers", "The block naming the file a part writes. {target} is its path.", &["target"]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_RECORDS_PLAN_TOO, "Recording the plan too", "The block for a part that records the Job's plan beside its own file.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_RECORDS_PLAN, "Recording the plan", "The block for a part whose product is the Job's plan.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_CAPTURED, "A captured part", "The block for a part whose shown_by is run as a spec.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_CHECKS, "The checks", "The heading the checks that gate a part follow.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_CHECKS_ASK, "Asking for checks", "How a Drone asks for checks. {whole} is one of the two sentences below.", &["whole"]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_CHECKS_WHOLE_ALL, "An ask runs all", "What an ask naming no check runs, where it runs every one.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_CHECKS_WHOLE_EXCEPT, "An ask runs most", "What an ask naming no check runs, where some run only later.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_CHECKS_SKIPPED, "Checks it skips", "What a Drone is told where a check covers only some files.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_CHECKS_WAIT, "Waiting for checks", "How a Drone is told results arrive.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_CHECKS_AT_GATE, "Checks only at submit", "The checks an ask does not run. {checks} names them and {runs} is \"it runs\" or \"they run\".", &["checks", "runs"]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_CHECKS_AT_HANDOFF, "Checks before handoff", "The checks that run last. {checks} names them and {runs} is \"it runs\" or \"they run\".", &["checks", "runs"]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_CHECKS_LATER, "Checks held for later", "The checks not run on this part. {checks} names them, {is} is \"is\" or \"are\" and {runs} is \"It runs\" or \"They run\".", &["checks", "is", "runs"]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_CHECKS_NOT_A_VERDICT, "An ask is not a verdict", "What a Drone is told an ask decides.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_CHECKS_LIMIT, "A limit on asking", "What a Drone is told about the limit where the number is not known.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_DECLARE, "Declaring scope", "The block asking a Drone to declare where its work is.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_DECLARE_COMPARED, "Scope compared", "What a Drone is told where files changed outside its scope are compared.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_DECLARE_EXCLUDED, "Paths to stay out of", "The line the paths a part stays out of follow.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_REDECLARE, "Outside what it declared", "The line the files changed outside a declared scope follow.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_REDECLARE_WHAT, "What to do about it", "What a Drone is told to do about files outside its scope.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_SPLIT_DECIDES, "Deciding a split", "The block for a part whose plan the next part makes Jobs of.", &[]),
    prompt(PromptSection::StepTerms, PROMPT_DRONE_SPLIT_PROPOSES, "Proposing a split", "The block for a part that dispatches Jobs itself.", &[]),
    prompt(PromptSection::Review, PROMPT_DRONE_REVIEW, "Writing a review", "The block for a part whose product is Armada's review of a change.", &[]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_PRODUCED, "What the part before produced", "What a Drone is told the part before claimed. {part} is its number and {claimed} its words.", &["part", "claimed"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_PRODUCED_UNRECORDED, "Nothing recorded before", "The same, where the part before left no claim. {part} is its number.", &["part"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_PRODUCED_FILE, "The file the part before wrote", "Where the part before wrote its finding. {file} is the path.", &["file"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_PRODUCED_ON_BRANCH, "Its work is on the branch", "Where the part before wrote no file.", &[]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_NOT_CLAIMED, "What the part before did not claim", "{part} is its number and {not_claimed} its words.", &["part", "not_claimed"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_SETTLED, "The part before is settled", "The block saying the part before is closed. {how} is one of the two sentences below.", &["how"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_SETTLED_BY_CHECKS, "Settled by its checks", "{step} is the part before's label.", &["step"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_SETTLED_BY_PERSON, "Settled by a person", "{step} is the part before's label.", &["step"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_BRANCH, "The branch it is on", "The block about the catch-up onto the base. {what_happened} is one of the three below.", &["what_happened"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_BRANCH_UP_TO_DATE, "Brought up to date", "{base} is the base branch and {commits} how many commits it moved.", &["base", "commits"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_BRANCH_CONFLICTED, "Left with conflicts", "{base} is the base branch and {files} the files with markers.", &["base", "files"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_BRANCH_NOT_MOVED, "Could not follow", "{base} is the base branch.", &["base"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_PLAN, "The plan", "The block carrying the Job's plan. {plan} is the plan.", &["plan"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_PLAN_FOLLOW, "Keeping the plan current", "What a Drone that follows the plan is told under it.", &[]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_PLAN_READ, "Reading the plan", "What a Drone that only reads the plan is told under it.", &[]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_PLAN_TASK, "Its task", "The block naming a task's Drone's one task. {task} is its id and {title} its title.", &["task", "title"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_DISPATCHED, "The Jobs it dispatched", "The block the Jobs a part dispatched follow.", &[]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_SENT_BACK, "Sent back", "What a Drone is told a later part found. {part} is its number and {found} its words.", &["part", "found"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_SENT_BACK_UNRECORDED, "Sent back, nothing recorded", "The same, where the later part left no record. {part} is its number.", &["part"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_SENT_BACK_FILE, "What sent it back, in a file", "Where the whole finding is. {file} is the path.", &["file"]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_SENT_BACK_ADDRESS, "Address what was found", "Where the later part wrote no file.", &[]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_DISMISSED, "What a person ruled out", "The block the findings a person dismissed follow.", &[]),
    prompt(PromptSection::Crossing, PROMPT_CROSSING_DISMISSED_ROW, "One finding ruled out", "{finding} is the finding and {reason} why it was dismissed.", &["finding", "reason"]),
    prompt(PromptSection::AnotherJobsFix, PROMPT_FIX_HELD_OFF, "Files another Job is fixing", "The block the files another Job's fix holds follow.", &[]),
    prompt(PromptSection::AnotherJobsFix, PROMPT_FIX_HELD_OFF_FIXING, "Held, being fixed", "{test} is the test being fixed.", &["test"]),
    prompt(PromptSection::AnotherJobsFix, PROMPT_FIX_HELD_OFF_LANDED, "Held, fix landed", "{test} is the test that was fixed.", &["test"]),
    prompt(PromptSection::AnotherJobsFix, PROMPT_FIX_FIXING, "A fix is fixing", "Where a fix for a test this Job failed on stands. {test} are filled by Fleet.", &["test"]),
    prompt(PromptSection::AnotherJobsFix, PROMPT_FIX_FIXING_HOLDS, "A fix is fixing, files held", "Where a fix for a test this Job failed on stands. {test} and {files} are filled by Fleet.", &["test", "files"]),
    prompt(PromptSection::AnotherJobsFix, PROMPT_FIX_LANDED, "A fix landed", "Where a fix for a test this Job failed on stands. {test} are filled by Fleet.", &["test"]),
    prompt(PromptSection::AnotherJobsFix, PROMPT_FIX_LANDED_HOLDS, "A fix landed, files held", "Where a fix for a test this Job failed on stands. {test} and {files} are filled by Fleet.", &["test", "files"]),
    prompt(PromptSection::AnotherJobsFix, PROMPT_FIX_IN_YOUR_COPY, "A fix in your copy", "Where a fix for a test this Job failed on stands. {test} and {files} are filled by Fleet.", &["test", "files"]),
    prompt(PromptSection::AnotherJobsFix, PROMPT_FIX_GONE, "A fix ended", "Where a fix for a test this Job failed on stands. {test} are filled by Fleet.", &["test"]),
    prompt(PromptSection::AnotherJobsFix, PROMPT_FIX_GONE_HOLDS, "A fix ended, files freed", "Where a fix for a test this Job failed on stands. {test} and {files} are filled by Fleet.", &["test", "files"]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS, "Other Jobs writing here", "The heading of the turn about other Jobs.", &[]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS_SHARING, "Shared files", "What a Drone is told where other Jobs change its files.", &[]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS_AHEAD, "A need ahead", "What a Drone is told where another declared a need first.", &[]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS_FIX, "A test another Job fixes", "What a Drone is told where a failing test is another Job's.", &[]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS_MORE, "More items", "The line closing a long list. {count} is how many more.", &["count"]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS_CLAIMED, "A Job claimed files", "{title}, {handle} and {paths} are filled by Fleet.", &["title", "handle", "paths"]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS_LANDED, "A Job landed", "{title}, {handle} and {paths} are filled by Fleet.", &["title", "handle", "paths"]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS_NOTE, "A note from a Job", "The line a note from another Job follows. {title} and {handle} name it.", &["title", "handle"]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS_AHEAD_ON, "Ahead on a path", "{path} is the path and {ahead} the needs ahead.", &["path", "ahead"]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS_LANDED_LATER, "What landed, later", "Where what landed reaches a live Drone.", &[]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS_LANDED_NOW, "What landed, now", "Where what landed reaches a new Drone.", &[]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS_NEXT_NUMBER, "The next number", "What a Drone is told about shared numbering, and leave_note.", &[]),
    prompt(PromptSection::OtherJobs, PROMPT_PEERS_CARRY_ON, "Carry on", "The line closing the turn.", &[]),
    prompt(PromptSection::PlanChanges, PROMPT_PLAN_ADDED, "A task added", "The turn a working Drone gets when a person adds a task. {id} and {title} name it.", &["id", "title"]),
    prompt(PromptSection::PlanChanges, PROMPT_PLAN_DROPPED, "A task dropped", "The turn when a person drops a task. {id}, {title} and {reason} are filled by Fleet.", &["id", "title", "reason"]),
    prompt(PromptSection::PlanChanges, PROMPT_PLAN_GROUP_ROUND, "The group goes round", "The turn when a group's checks fail. {group} names it and {tasks} lists its tasks.", &["group", "tasks"]),
    prompt(PromptSection::WorktreeRepair, PROMPT_HEAL_INDEX, "Repairing the index", "The body under REPAIR THE INDEX. {paths} names the unmerged paths.", &["paths"]),
    prompt(PromptSection::WorktreeRepair, PROMPT_HEAL_INSTALL, "Repairing the install", "The body under REPAIR THE INSTALL. {printed} is what the check printed and {bootstrap} the commands to run.", &["printed", "bootstrap"]),
    prompt(PromptSection::SideRuns, PROMPT_SIDE_SKILL, "Running a skill", "The body under RUN THE SKILL. {skill} is the skill and {context} the sentence below.", &["skill", "context"]),
    prompt(PromptSection::SideRuns, PROMPT_SIDE_CONTEXT, "A side run's Job", "{job} is the Job's title and {place} one of the three below.", &["job", "place"]),
    prompt(PromptSection::SideRuns, PROMPT_SIDE_CONTEXT_ON_BRANCH, "A side run's Job and branch", "{job}, {branch} and {place} are filled by Fleet.", &["job", "branch", "place"]),
    prompt(PromptSection::SideRuns, PROMPT_SIDE_BEFORE_STEP, "Before a step", "{step} is the step.", &["step"]),
    prompt(PromptSection::SideRuns, PROMPT_SIDE_AFTER_STEP, "After a step", "{step} is the step.", &["step"]),
    prompt(PromptSection::SideRuns, PROMPT_SIDE_AFTER_PR, "After the pull request", "When a side run fires after the pull request opens.", &[]),
    prompt(PromptSection::SideRuns, PROMPT_SIDE_REST, "How a side run ends", "The last paragraph of every side run.", &[]),
    prompt(PromptSection::Helm, PROMPT_HELM_VOICE, "Voice", "The block Machine Voice is given in. {voice} is the setting.", &["voice"]),
    prompt(PromptSection::InjectedTurns, PROMPT_POKE_QUIET, "Poke: a quiet Drone", "What a Drone that has sent nothing for a while is told. {quiet} is how long, as \"12 minutes\".", &["quiet"]),
    prompt(PromptSection::InjectedTurns, PROMPT_POKE_BACKGROUND, "Poke: waiting on background work", "What a Drone that ended its turn waiting on work it put in the background is told.", &[]),
    prompt(PromptSection::InjectedTurns, PROMPT_INTERRUPT, "Force-interrupt directive", "What a Drone whose work has stopped moving is told to do now. {expected} and {produced} are the finding.", &["expected", "produced"]),
    prompt(PromptSection::InjectedTurns, PROMPT_PERMIT_JOB, "Permission: allowed for this Job", "A person allowed a command for this Job. {command} is the command.", &["command"]),
    prompt(PromptSection::InjectedTurns, PROMPT_PERMIT_REPOSITORY, "Permission: allowed in this repository", "A person allowed a command in this repository. {rule} is what was kept and {command} what the Drone ran.", &["rule", "command"]),
    prompt(PromptSection::InjectedTurns, PROMPT_PERMIT_MACHINE, "Permission: allowed on this machine", "A person allowed a command on this machine. {rule} is what was kept and {command} what the Drone ran.", &["rule", "command"]),
    prompt(PromptSection::InjectedTurns, PROMPT_PERMIT_REJECTED, "Permission: refused", "A person said no to a command. {command} is the command.", &["command"]),
    prompt(PromptSection::InjectedTurns, PROMPT_PERMIT_REJECTED_NOTE, "Permission: the person's words", "The line a person's own words follow when they said no with a note.", &[]),
    prompt(PromptSection::InjectedTurns, PROMPT_PILOT_OPENING, "Pilot handoff: opening", "What a piloted session reads first. {title} and {id} name the Job.", &["title", "id"]),
    prompt(PromptSection::InjectedTurns, PROMPT_PILOT_WORKTREE, "Pilot handoff: the worktree", "{path} and {branch} say where the work is.", &["path", "branch"]),
    prompt(PromptSection::InjectedTurns, PROMPT_PILOT_RESTART, "Pilot handoff: to restart the step", "Where the Job was taken over to restart its step.", &[]),
    prompt(PromptSection::InjectedTurns, PROMPT_PILOT_FOR_GOOD, "Pilot handoff: for good", "Where the Job was taken over for good.", &[]),
    prompt(PromptSection::InjectedTurns, PROMPT_PILOT_STOPPED, "Pilot handoff: where it stopped", "{step} is the step, {label} its name in quotes or nothing, {trigger} why in brackets or nothing, and {runs} how many runs.", &["step", "label", "trigger", "runs"]),
    prompt(PromptSection::InjectedTurns, PROMPT_PILOT_JUDGE_REFUSED, "Pilot handoff: what the Judge refused", "The line the refused criteria follow.", &[]),
    prompt(PromptSection::InjectedTurns, PROMPT_PILOT_CHECKS_FAILED, "Pilot handoff: checks that failed", "The line the failed checks follow.", &[]),
    prompt(PromptSection::InjectedTurns, PROMPT_PILOT_OUTSIDE_PLAN, "Pilot handoff: outside the plan", "{files} are the files changed outside the declared plan.", &["files"]),
    prompt(PromptSection::InjectedTurns, PROMPT_PILOT_CHANGED, "Pilot handoff: what changed", "{files} are the files the worktree changes.", &["files"]),
    prompt(PromptSection::InjectedTurns, PROMPT_PILOT_NARRATIVE, "Pilot handoff: the Drone's account", "What the Drone said it was stuck on. {trying_to}, {blocked_by} and {tried} are its words.", &["trying_to", "blocked_by", "tried"]),
    prompt(PromptSection::InjectedTurns, PROMPT_PILOT_RECORD, "Pilot handoff: the full record", "The closing line, pointing at get_handoff.", &[]),
];
