//! The words of the blocks a step puts to its Drone, as Armada ships them:
//! each the default of a `prompts.drone…` key in settings.json. Moved here
//! byte for byte from where `super` assembles them.

/// The block naming the file a part writes, as it ships. `{target}` is its path.
pub(crate) const DELIVERS: &str =
    "WHAT THIS PART DELIVERS\n\nWrite this part's finding to a file, \
             at this exact path in your worktree:\n\n  {target}\n\nThis is the \
             work product, not a note to yourself, so it does not go in the \
             directory named above. This exact path is the one that is read: an \
             empty file or no file stops this part, and a file somewhere else \
             is not this part's work however good it is. What you submit \
             summarises it and does not replace it.";

/// The block for a part that records the plan beside its file, as it ships.
pub(crate) const RECORDS_PLAN_TOO: &str =
    "RECORDING THE PLAN\n\nBeside what this part delivers, it \
                 also records the Job's plan. Record it with record_plan: \
                 an approach in a paragraph, then the tasks it breaks into, \
                 in the order they will be done. Recording again replaces \
                 the whole plan, so correct one by recording it again. The \
                 checks each part must pass run on their own when that \
                 part is submitted, so the plan carries no task for \
                 running them. Recording the plan does not finish this \
                 part — submit_evidence still does.";

/// The block for a part whose product is the plan, as it ships.
pub(crate) const RECORDS_PLAN: &str =
    "WHAT THIS PART DELIVERS\n\nThis part's product is the Job's \
                 plan. Record it with record_plan: an approach in a \
                 paragraph, then the tasks it breaks into, in the order \
                 they will be done. Recording again replaces the whole \
                 plan, so correct one by recording it again. The checks \
                 each part must pass run on their own when that part is \
                 submitted, so the plan carries no task for running them. \
                 Recording does not finish this part — submit_evidence \
                 still does.";

/// The block for a captured part, as it ships.
pub(crate) const CAPTURED: &str =
    "THIS PART IS CAPTURED\n\nWhatever you name in `shown_by` is not \
             only read. This repository's own `evidence.run` runs it once \
             you submit — so it has to be a spec: one file of runnable code \
             that reaches the state your claim is about, landing in the \
             diff like anything else this part changes, not a description \
             of what you saw. What that run leaves behind is what a \
             reviewer sees, in place of reading the diff for themselves. \
             What the spec has to look like beyond that is this \
             repository's own decision, not something this instruction can \
             tell you.\n\nNaming none does not fail this part. It means \
             nothing is captured, and a reviewer reads the diff instead.";

/// The heading the checks that gate a part follow, as it ships.
pub(crate) const CHECKS: &str = "FINDING OUT WHERE YOU STAND\n\nThese are the checks that gate \
             this part:";

/// What an ask naming no check runs, where it runs every one, as it ships.
pub(crate) const CHECKS_WHOLE_ALL: &str = "An ask that names no check runs the whole list";

/// What an ask naming no check runs, where some run later, as it ships.
pub(crate) const CHECKS_WHOLE_EXCEPT: &str =
    "An ask that names no check runs every one of them but those named below";

/// How a Drone asks for checks, as it ships. `{whole}` is one of the two sentences.
pub(crate) const CHECKS_ASK: &str =
    "You can ask for them to be run against your worktree, and you \
             will be told what each one did. A check that fails carries its \
             own output back with the answer — you do not need to run the \
             command yourself or go looking for a log to read it. {whole}. \
             A build or test command run directly is not granted; asking is \
             how you get the same answer.\n\nYou can also name one check off \
             the list above, and the files you changed, and get an answer in \
             seconds rather than minutes. **Asking that way costs you \
             nothing** — only a run of every check counts against the limit \
             below, so ask about one check as often as it is useful. Naming \
             one does not lower the bar you are measured against: every check \
             is run whole when you submit, whatever you asked for before \
             then.";

/// Where a check covers only some files, as it ships.
pub(crate) const CHECKS_SKIPPED: &str =
    "A check on this list that covers only certain files comes \
                 back skipped rather than run, where your changes have not \
                 touched them.";

/// How a Drone is told results arrive, as it ships.
pub(crate) const CHECKS_WAIT: &str = "Use it \
             when you want to know whether the work holds up rather than \
             guessing. The call comes back at once, and each check's result \
             arrives as a later turn as it finishes, however long they take. \
             The first one that fails stops the rest, and the last turn says \
             the run is over. Wait for them rather than running them yourself.";

/// What a Drone is told an ask decides, as it ships.
pub(crate) const CHECKS_NOT_A_VERDICT: &str =
    "It is not a verdict and it advances nothing. A run in which \
             everything passes does not finish this part; the checks are run \
             again when you submit, and that run is the one that decides. \
             Submitting is still the only way to report.";

/// The limit on asking, where its number is not known, as it ships.
pub(crate) const CHECKS_LIMIT: &str =
    "There is a limit on how many times one part may ask for every check \
                 at once, and none on asking about one.";

/// The checks an ask does not run, as it ships.
pub(crate) const CHECKS_AT_GATE: &str =
    "Asking does not run {checks}: {runs} only when you submit.";

/// The checks that run last, as it ships.
pub(crate) const CHECKS_AT_HANDOFF: &str =
    "Asking does not run {checks} either: when you submit, {runs} only once \
             every other check here has passed, before the work is handed off.";

/// The checks held for later parts, as it ships.
pub(crate) const CHECKS_LATER: &str =
    "{checks} {is} not checked on this part at all, when you ask or when you \
             submit. {runs} once, before the work is handed off.";

/// The block asking a Drone to declare where its work is, as it ships.
pub(crate) const DECLARE: &str =
    "BEFORE YOU START\n\nCall the scope tool with the repository-relative \
             paths this part's work will be in. Include what you will change and \
             what has to be read to judge the change. Each part is checked \
             against what was declared for it, and what you declared for an \
             earlier part does not carry over.";

/// Where changed files are compared against the scope, as it ships.
pub(crate) const DECLARE_COMPARED: &str =
    "Files you change outside them are compared against what you \
                 declared. If the work turns out to be somewhere else, call the \
                 tool again — a scope that changed is fine, and a file changed \
                 for the next part is not.";

/// The line the paths a part stays out of follow, as it ships.
pub(crate) const DECLARE_EXCLUDED: &str =
    "This part of the work is meant to stay out of these. If \
                 the fix you find genuinely needs one of them, say so with the \
                 scope request tool and say why — somebody will look at whether \
                 it belongs here, and you keep working while they do:";

/// The line the files outside a declared scope follow, as it ships.
pub(crate) const REDECLARE: &str =
    "FILES OUTSIDE WHAT YOU DECLARED\n\nThe scope you declared for this \
             part does not cover everything that has changed:";

/// What a Drone is told to do about files outside its scope, as it ships.
pub(crate) const REDECLARE_WHAT: &str =
    "Nothing has failed and you are not being asked to stop. If this \
             part's work is there, call the scope tool again with every path the \
             work is in. The new call replaces the scope, and that is how a \
             scope that turned out wrong is corrected. If that work belongs to \
             a later part, leave it to that part.";
