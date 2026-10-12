//! The words a Drone's brief is written in, as Armada ships them: each the
//! default of a `prompts.drone…` key in settings.json. Moved here byte for byte
//! from where `super` assembles them.

/// Where a Drone keeps notes for itself, as it ships. `{job}` is the Job.
pub(crate) const NOTEKEEPING: &str = "FILES YOU WRITE FOR YOURSELF\n\nNotes you want to keep \
         between turns — anything you write for yourself rather than for the \
         work goes under .armada/{job}/, which is this Job's alone. None of it \
         belongs at the repository root, where a file outlives the Job that \
         wrote it with nothing to say that Job is over. Nothing here is \
         asking you to write any of it, and a file this part is asked to \
         deliver is not one of them — that one is named where it is asked \
         for, at the path that is read, and it does not go here.";

/// The block naming the Job, as it ships. `{title}` is its title.
pub(crate) const JOB_BRIEF: &str = "JOB BRIEF\n\n{title}";

/// The line the attached files follow, as it ships.
pub(crate) const JOB_ATTACHMENTS: &str = "Files attached to this brief, copied into your worktree:";

/// The line the acceptance criteria follow, as it ships.
pub(crate) const JOB_DONE_WHEN: &str = "This is done when:";

/// The block above the list of parts, as it ships. `{parts}` is how many.
pub(crate) const WHERE_YOU_ARE: &str = "WHERE YOU ARE\n\nThis work runs in {parts} parts.";

/// Which part a Drone is on, as it ships.
pub(crate) const YOU_ARE_ON: &str = "You are on part {part}.";

/// How a part before this one is marked, as it ships.
pub(crate) const PART_DONE: &str = "done";

/// How this part is marked, as it ships.
pub(crate) const PART_HERE: &str = "you are here";

/// How a part after this one is marked, as it ships.
pub(crate) const PART_NOT_YOURS: &str = "not yours — do not do it";

/// The line under this part, as it ships.
pub(crate) const PART_STOP: &str = "STOP. Submit when this part is done, then wait.";

/// What a Drone is told about the parts after its own, as it ships.
pub(crate) const PARTS_AFTER: &str =
    "The parts after this one happen after you submit, and doing them \
         yourself does not move the work forward. Leave the branch in a state \
         they can start from.";

/// The block naming the step, as it ships. `{step}` is its label.
pub(crate) const STEP: &str =
    "STEP: {step}\n\nWhat you claim should be what the work now does, not that \
         you finished. An adjacent problem you notice and leave alone goes \
         under Not claimed.";

/// What the approver left for this part, as it ships. `{note}` is their words.
pub(crate) const FOR_THIS_PART: &str =
    "FOR THIS PART\n\nThe person who approved this work left this for this part:\n\n  \"{note}\"";

/// The block a restarted Drone reads first, as it ships. `{why}` is a reason.
pub(crate) const AGAIN: &str =
    "WHY THIS PART IS BEING DONE AGAIN\n\n{why} Its work is on the branch you are in.";

/// Where something was cited against the last attempt, as it ships.
pub(crate) const AGAIN_ANSWER: &str =
    "Address this and submit again. Say what changed since the last \
                 submission.";

/// Where nothing was cited, as it ships.
pub(crate) const AGAIN_NOTHING_CITED: &str =
    "Nothing was cited for you to answer. Finish this part and submit.";

/// Why again, where the record holds no verdict, as it ships.
pub(crate) const AGAIN_NO_VERDICT: &str =
    "An earlier attempt at this part stopped. The record holds no verdict \
                    against its work.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_GATE_FAILURE: &str =
    "An earlier attempt at this part was checked and did not pass.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_SUSPECT: &str =
    "An earlier attempt at this part passed its checks, and what it submitted \
                 was not accepted as evidence that the work was done.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_UNDECIDED: &str =
    "An earlier attempt at this part was never checked. Something the check \
                 needed could not be read, so nothing was decided about the work itself.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_THRASHING: &str =
    "An earlier attempt at this part was stopped while it was still running. \
                 Nothing it did was checked.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_KILLED: &str =
    "An earlier attempt at this part was ended by a person while it was still \
                 running. Nothing it did was checked, and nothing about it was judged.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_RUN_ENDED: &str =
    "An earlier attempt at this part said its run was over without having \
                 submitted anything, and was stopped there. Nothing it did was checked, \
                 and anything it did after saying so was not kept.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_GONE: &str =
    "An earlier attempt at this part was still running when its Drone was found \
                 gone, and this restart is what stopped it there. Nothing it did was checked.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_SCOPE_REFUSED: &str =
    "An earlier attempt at this part asked to write files outside what this \
                 Job says it changes, and was told they are not part of it. Nothing it \
                 did was checked. Do this part inside the files the Job already names.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_NO_REPORT: &str =
    "An earlier attempt at this part was told to stop and report where it had \
                 got to, and did not answer. It was stopped there, and nothing it did was \
                 checked.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_CHECK_TIMEOUT: &str =
    "An earlier attempt at this part was stopped because a check did not \
                 finish. Nothing was decided about the work itself.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_TOO_LARGE: &str =
    "An earlier attempt at this part submitted more than could be read, so it \
                 was never checked.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_BLOCKED: &str =
    "An earlier attempt at this part was refused a tool or a command it needed, \
                 and stopped without submitting anything.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_ASK_UNANSWERED: &str =
    "An earlier attempt at this part asked a person whether it could run a \
                 command, and nobody answered before the limit on waiting ran out. It was \
                 stopped there, and nothing it did was checked.";

/// Why again, one reason, as it ships.
pub(crate) const AGAIN_LOOP_CAP: &str =
    "An earlier attempt at this part used every round it is allowed. Nothing it \
                 did was refused.";

/// Whole runs used up, as it ships.
pub(crate) const ASKS_USED: &str =
    "You may ask for every check {allowed} times in this part, and you have used all \
                 of them. Naming one check is still free.";

/// Whole runs a part may ask, as it ships.
pub(crate) const ASKS_ALL: &str = "You may ask for every check {allowed} times in this part.";

/// Whole runs left, as it ships.
pub(crate) const ASKS_LEFT: &str =
    "You may ask for every check {allowed} times in this part, and {left} of those are \
                 left.";
