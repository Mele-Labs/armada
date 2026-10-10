//! The words of what crosses a step boundary, as Armada ships them: each the
//! default of a `prompts.crossing…` key in settings.json. Moved here byte for
//! byte from where `super` renders them.

/// The block carrying the Job's plan, as it ships. `{plan}` is the plan.
pub(crate) const PLAN: &str = "THE PLAN\n\n{plan}";

/// What a Drone that follows the plan is told under it, as it ships.
pub(crate) const PLAN_FOLLOW: &str = "Keep it current as you work: call update_task when you \
                 start a task and when it is done, and add_task for work the \
                 plan missed. The checks each part must pass run on their \
                 own when that part is submitted, so do not add a task for \
                 running them. Drop a task with a reason rather than leaving \
                 it open. A done task may move back; a dropped one stays \
                 dropped. A task's state decides nothing on its own — it is \
                 your account of the work, and the diff is what is checked.";

/// What a Drone that only reads the plan is told under it, as it ships.
pub(crate) const PLAN_READ: &str = "Read it before you start. It is not yours to change.";

/// The block naming a task's Drone's one task, as it ships. `{task}` is its id and `{title}` its title.
pub(crate) const PLAN_TASK: &str =
    "YOUR TASK\n\nThis part's tasks are worked one at a time, each by an \
             agent of its own, and yours is {task}: {title}. Its line in the plan above \
             says what it touches and what should show it is done. Do {task} and \
             nothing past it: the tasks handed in or done before it are already \
             on the branch you are in, and the open ones after it are each \
             another agent's.\n\nWhen {task} is done, submit it with the evidence \
             submission tool. What you claim, and what shows it, are about {task} \
             alone. That submission is your hand-in, and it ends your work on \
             this Job. The plan's states are kept from it, so you mark nothing \
             yourself.";

/// The block the Jobs a part dispatched follow, as it ships.
pub(crate) const DISPATCHED: &str =
    "THE JOBS YOU DISPATCHED\n\nEvery one of these has finished. This is \
             the whole of what you have to report on, and it is not visible to \
             you anywhere else.";

/// What the part before claimed, as it ships. `{part}` and `{claimed}` are filled by Fleet.
pub(crate) const PRODUCED: &str = "What part {part} produced:\n  \"{claimed}\"";

/// The same, where it left no claim, as it ships.
pub(crate) const PRODUCED_UNRECORDED: &str =
    "What part {part} produced:\n  There is no record of what it claimed.";

/// Where the part before wrote its finding, as it ships. `{file}` is the path.
pub(crate) const PRODUCED_FILE: &str =
    "It wrote that part's finding to {file}, in the worktree you \
                 are in. Read it before you start. What is quoted above \
                 summarises it and does not replace it.";

/// Where the part before wrote no file, as it ships.
pub(crate) const PRODUCED_ON_BRANCH: &str = "Its work is on the branch you are in.";

/// What the part before did not claim, as it ships.
pub(crate) const NOT_CLAIMED: &str =
    "What part {part} did not claim:\n  \"{not_claimed}\"\n\nThat is \
                 everything its claim does not cover — a gap it left on purpose, \
                 or something it changed that nobody asked for. It is context \
                 for this part and not a list of work this part owes.";

/// The part before passed its checks, as it ships. `{step}` is its label.
pub(crate) const SETTLED_BY_CHECKS: &str = "{step} passed the checks that gate it";

/// The part before was accepted by a person, as it ships.
pub(crate) const SETTLED_BY_PERSON: &str = "{step} was read by a person and accepted";

/// The block saying the part before is settled, as it ships. `{how}` is one of the two above.
pub(crate) const SETTLED: &str =
    "THE PART BEFORE THIS ONE\n\n{how}, and its work is on the branch \
             you are in. It is settled: it is not yours to do again, to review \
             or to improve on. Start this part from it.";

/// Brought up to the base, as it ships. `{base}` and `{commits}` are filled by Fleet.
pub(crate) const BRANCH_UP_TO_DATE: &str =
    "`{base}` moved on by {commits} commit(s) since this branch was cut, and the \
                 branch has been brought up to it before you started. The worktree is current. \
                 Work already on the branch may now sit on top of code that changed underneath \
                 it — read a file before you edit it.";

/// Left with conflicts, as it ships. `{base}` and `{files}` are filled by Fleet.
pub(crate) const BRANCH_CONFLICTED: &str =
    "`{base}` moved on since this branch was cut, and the branch has been brought \
                 up to it before you started. These files were left with conflict markers in \
                 them, and resolving them is the first piece of your work:\n\n{files}\n\nOpen each \
                 one, keep what belongs, and remove every marker before you submit.";

/// Could not follow the base, as it ships. `{base}` is filled by Fleet.
pub(crate) const BRANCH_NOT_MOVED: &str =
    "`{base}` moved on since this branch was cut, and the branch could not be put \
                 on top of it. It is exactly where it was. Nothing here is yours to fix — do \
                 the work described above, and somebody will reconcile the two.";

/// The block about the catch-up, as it ships. `{what_happened}` is one of the three above.
pub(crate) const BRANCH: &str = "THE BRANCH YOU ARE ON\n\n{what_happened}";

/// What a later part found when it sent the work back, as it ships.
pub(crate) const SENT_BACK: &str =
    "WHAT SENT THIS PART BACK\n\nPart {part} read the work after this part passed, \
             and sent it back to be worked again. It found:\n\n  \"{found}\"";

/// The same, where the later part left no record, as it ships.
pub(crate) const SENT_BACK_UNRECORDED: &str = "WHAT SENT THIS PART BACK\n\nPart {part} read the work after this part passed, \
             and sent it back to be worked again. It found:\n\n  There is no record of what it found.";

/// Where the whole finding is, as it ships. `{file}` is the path.
pub(crate) const SENT_BACK_FILE: &str =
    "The whole of it is in {file}, in the worktree you are in. Read it \
                 before you start and address what it found: it is why this part is \
                 being worked again. What is quoted above summarises it and does not \
                 replace it.";

/// Where the later part wrote no file, as it ships.
pub(crate) const SENT_BACK_ADDRESS: &str =
    "Address what it found: it is why this part is being worked again.";

/// The block the findings a person dismissed follow, as it ships.
pub(crate) const DISMISSED: &str =
    "WHAT A PERSON RULED OUT\n\nA person read an earlier review of this change and \
             dismissed these findings. Do not raise them again. If the change has moved so \
             that one is true again, raise it and say what moved.";

/// One finding a person dismissed, as it ships.
pub(crate) const DISMISSED_ROW: &str = "  \"{finding}\"\n  Why it was dismissed: {reason}";
