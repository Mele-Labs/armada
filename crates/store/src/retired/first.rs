/// Version 44 — how a Job meets a blocked command, and what a person allowed.
///
/// Beside the change it makes, like `V43`:
/// `schema.rs` is at the 900 lines the gate refuses at.
///
/// **The `DEFAULT` is the backfill only**: a Job written before this reads as
/// `refuse_and_hold`, which is what it already was. `Store::insert_job`
/// binds `ask_me` for a Job created after, the way it binds `title`, so this
/// `DEFAULT` is never the value a new Job gets. The table starts empty, which
/// is what a Job nobody allowed anything reads as.
///
/// `by` is quoted because it is an SQL keyword, and the key is the pair so a
/// second allow of one command is a conflict rather than a second row.
pub(crate) const V44: &str = r#"
ALTER TABLE jobs ADD COLUMN when_blocked TEXT NOT NULL DEFAULT 'refuse_and_hold';

CREATE TABLE job_allowed_commands (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    run        TEXT NOT NULL,
    reach      TEXT NOT NULL,
    allowed_at TEXT NOT NULL,
    "by"       TEXT NOT NULL,
    PRIMARY KEY (job_id, run)
) STRICT;
"#;

/// Version 98 — a Job's landing, its Drone cap, its policy overrides, and the
/// issue its request linked.
///
/// One row per Job in each, keyed by the Job. Nothing is backfilled: a Job
/// with no row is a Job approved before this, which lands as every Job did
/// then, runs under the machine's cap, defers to the repository at every gate
/// and came from no issue anybody recorded.
pub(crate) const V98: &str = r#"
CREATE TABLE job_landing (
    job_id   TEXT PRIMARY KEY REFERENCES jobs(job_id),
    target   TEXT CHECK (target IS NULL OR trim(target) <> ''),
    from_ref TEXT CHECK (from_ref IS NULL OR trim(from_ref) <> ''),
    pr_mode  TEXT NOT NULL CHECK (pr_mode IN ('ready', 'draft'))
) STRICT;

CREATE TABLE job_drone_caps (
    job_id TEXT PRIMARY KEY REFERENCES jobs(job_id),
    cap    INTEGER NOT NULL CHECK (cap > 0)
) STRICT;

CREATE TABLE job_policy_overrides (
    job_id TEXT NOT NULL REFERENCES jobs(job_id),
    policy TEXT NOT NULL CHECK (policy IN ('auto_merge', 'review_gate')),
    said   TEXT NOT NULL CHECK (trim(said) <> ''),
    PRIMARY KEY (job_id, policy)
) STRICT;

CREATE TABLE job_issue_sources (
    job_id    TEXT PRIMARY KEY REFERENCES jobs(job_id),
    reference TEXT NOT NULL CHECK (trim(reference) <> ''),
    url       TEXT NOT NULL CHECK (trim(url) <> ''),
    read_at   TEXT NOT NULL,
    moved_at  TEXT
) STRICT;
"#;

/// Version 104 — whether a Job stops at its branch, and whether its pull
/// request is set to merge itself. **Nothing is backfilled**: a row kept
/// before this opened a pull request and left the merge to somebody.
pub(crate) const V104: &str = r#"
ALTER TABLE job_landing ADD COLUMN local INTEGER NOT NULL DEFAULT 0
    CHECK (local IN (0, 1));
ALTER TABLE job_landing ADD COLUMN auto_merge INTEGER NOT NULL DEFAULT 0
    CHECK (auto_merge IN (0, 1));
"#;

/// Version 113 — one row per asked run. Nothing is backfilled: no ask was
/// kept before it.
pub(crate) const V113: &str = r#"
CREATE TABLE asked_runs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    drone       TEXT NOT NULL,
    task        INTEGER CHECK (task IS NULL OR task > 0),
    step_id     TEXT NOT NULL,
    attempt     INTEGER NOT NULL CHECK (attempt > 0),
    fleet       TEXT NOT NULL,
    started_at  TEXT NOT NULL,
    finished_at TEXT,
    state       TEXT NOT NULL CHECK (state IN ('running', 'passed', 'failed', 'stopped', 'lost')),
    checks      TEXT NOT NULL,
    narrowed    INTEGER NOT NULL CHECK (narrowed IN (0, 1)),
    only_check  TEXT,
    logs        TEXT NOT NULL DEFAULT '[]'
) STRICT;
CREATE INDEX asked_runs_by_job ON asked_runs (job_id, id);
"#;

/// Version 52 — how a Job meets a Judge criterion that refuses, the question
/// held open while a person answers, and the criteria this repository has
/// stood down. Beside the change it makes, like every migration since
/// [`V17`](crate::report::V17) — `schema.rs` is at the 900 lines the gate
/// refuses at.
///
/// **`when_refused` defaults to `per_criterion`**, which is what a Job written
/// before this setting existed reads as: each criterion decides for itself,
/// off its own declaration.
///
/// **`job_judge_questions` has `job_id` alone as its key.** Only one criterion
/// is ever asked about at a time — `fleet::judging::looks::JudgeFold` asks
/// about the first ask-eligible refusal a pass produces — so a second row for
/// the same Job would mean this crate's caller lost track of the first
/// question rather than a real second one existing.
///
/// **`repository_judge_tolerances` has no `job_id` at all.** A store is opened
/// once per repository, so `criterion_id` alone is the key, and the fact
/// outlives whichever Job recorded it — read by every Job's gate from here on,
/// per `docs/concepts/judge.md`'s asking design.
pub(crate) const V52: &str = r#"
ALTER TABLE jobs ADD COLUMN when_refused TEXT NOT NULL DEFAULT 'per_criterion';

CREATE TABLE job_judge_questions (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    criterion_id TEXT NOT NULL,
    question    TEXT NOT NULL,
    expected    TEXT NOT NULL,
    produced    TEXT NOT NULL,
    consequence TEXT NOT NULL,
    asked_at    TEXT NOT NULL,
    PRIMARY KEY (job_id)
) STRICT;

CREATE TABLE repository_judge_tolerances (
    criterion_id TEXT NOT NULL PRIMARY KEY,
    recorded_at  TEXT NOT NULL,
    "by"         TEXT NOT NULL
) STRICT;
"#;

/// Version 28 — a loop return says which step caused it.
///
/// Beside the reading it serves rather than in `schema.rs`, which is at the 900
/// the gate refuses at — [`crate::report::V17`]'s precedent. The order still
/// lives in `MIGRATIONS`, which is the part that may not be anywhere else.
///
/// **The column is on the routed-*to* step's row, and it names the *emitting*
/// step.** `iteration_count` is the emitter's — `docs/journeys/triage-queue.md`
/// settles it, because a cap split from the count it bounds never fires and
/// because two loops sharing a target step would otherwise sum into one number
/// — and the emitter makes no move of its own on a return, so there was nothing
/// in the log to count against it. The alternatives were a seventh `StepState`,
/// which breaks the wire, and reusing `stopped`, whose registry row refuses
/// exactly this. Both were rejected; this is what is left.
///
/// **Nothing to backfill.** The `advanced -> running` edge landed one commit
/// before this and nothing walked it, so no stored row is a return. One written
/// by a build between the two would fold as `StepStateNotReachable` rather than
/// as an unattributed pass, which is the honest direction to be wrong in.
///
/// The shape trigger is rewritten whole because `SQLite` cannot alter one. The
/// step arm gains a second `CASE` beside the reason's, in the same shape and
/// for the same reason: the column is required on exactly the edge that stores
/// it and refused everywhere else, so a row cannot say a step looped without
/// saying whose loop it was.
pub(crate) const V28: &str = r#"
ALTER TABLE job_events ADD COLUMN returned_by TEXT;

DROP TRIGGER job_events_hold_one_whole_shape;

CREATE TRIGGER job_events_hold_one_whole_shape
BEFORE INSERT ON job_events
WHEN NOT (
    (NEW.kind = 'job_transition'
        AND NEW.step_id IS NULL AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL AND NEW.drone_id IS NULL
        AND NEW.returned_by IS NULL
        AND NEW.status_from <> NEW.status_to)
 OR (NEW.kind = 'step_transition'
        AND NEW.step_id IS NOT NULL AND NEW.state_from IS NOT NULL
        AND NEW.state_to IS NOT NULL AND NEW.drone_id IS NULL
        AND NEW.status_from = NEW.status_to
        AND (CASE WHEN NEW.state_to IN ('stopped', 'retrying')
                    OR (NEW.state_to = 'advanced' AND NEW.state_from = 'stopped')
                  THEN NEW.reason_kind = 'escalation' AND NEW.reason_value IS NOT NULL
                  ELSE NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL
             END)
        AND (CASE WHEN NEW.state_from = 'advanced' AND NEW.state_to = 'running'
                  THEN NEW.returned_by IS NOT NULL
                  ELSE NEW.returned_by IS NULL
             END))
 OR (NEW.kind IN ('drone_spawned', 'drone_exited')
        AND NEW.step_id IS NOT NULL AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL AND NEW.drone_id IS NOT NULL
        AND NEW.returned_by IS NULL
        AND NEW.status_from = NEW.status_to
        AND NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'a job_events row is one whole shape: a job transition with no step or drone columns, a step move beneath an unchanged status carrying a trigger only where it stops the step, hands it back, or advances one that stopped, and the step that routed it only where it is a loop return, or a drone arriving on a step or leaving it beneath one');
END;
"#;

/// Version 88 — the tree a Job's Checks last passed on.
///
/// Null is every Job whose Checks have passed on nothing yet, and every Job
/// before this existed; both are read as unchecked, so their next push runs
/// the Checks first.
pub(crate) const V88: &str = r#"
ALTER TABLE jobs ADD COLUMN checked_tree TEXT;
"#;

/// Version 99 — Drones beside the kept one, each task's edit calls, the pairs
/// run apart, and the tasks a planner said may run together.
///
/// One version, because each is half of running tasks at once. Nothing is
/// backfilled: no Job ran a Drone beside another before it.
pub(crate) const V99: &str = r#"
ALTER TABLE job_work_plan_tasks ADD COLUMN beside TEXT NOT NULL DEFAULT '';

ALTER TABLE job_task_drones ADD COLUMN extra INTEGER NOT NULL DEFAULT 0
    CHECK (extra IN (0, 1));
ALTER TABLE job_task_drones ADD COLUMN pid INTEGER CHECK (pid IS NULL OR pid > 0);
ALTER TABLE job_task_drones ADD COLUMN process_started_at TEXT;
ALTER TABLE job_task_drones ADD COLUMN edits_at TEXT;
ALTER TABLE job_task_drones ADD COLUMN left_at TEXT;
ALTER TABLE job_task_drones ADD COLUMN ended TEXT
    CHECK (ended IS NULL OR ended IN ('done', 'killed', 'failed'));

CREATE TABLE job_task_edits (
    job_id   TEXT NOT NULL REFERENCES jobs(job_id),
    drone_id TEXT NOT NULL,
    path     TEXT NOT NULL CHECK (trim(path) <> ''),
    PRIMARY KEY (job_id, drone_id, path)
) STRICT;

CREATE TABLE job_group_apart (
    job_id TEXT NOT NULL REFERENCES jobs(job_id),
    grp    INTEGER NOT NULL CHECK (grp > 0),
    first  INTEGER NOT NULL CHECK (first > 0),
    second INTEGER NOT NULL CHECK (second > first),
    paths  TEXT NOT NULL,
    at     TEXT NOT NULL,
    PRIMARY KEY (job_id, grp, first, second)
) STRICT;
"#;

/// Version 21 — what a Job's branch came to when it was sent out.
///
/// Beside the change it makes, like [`V20`](crate::note::V20): `schema.rs` is
/// at the 900 the gate refuses at.
///
/// Three nullable columns and no backfill. A Job written before this has none,
/// and null already means "nothing was recorded" — which is exactly true of
/// every Job that finished while the result was being dropped.
pub(crate) const V21: &str = r#"
ALTER TABLE jobs ADD COLUMN delivery_commit TEXT;
ALTER TABLE jobs ADD COLUMN delivery_pushed TEXT;
ALTER TABLE jobs ADD COLUMN delivery_pull_request TEXT;
"#;

/// Version 26 — what became of the pull request after somebody read it.
///
/// **A second write to the same three-column story, and the reason it is two
/// migrations rather than one.** [`V21`]'s columns are written once, by the
/// turn that finishes a Job, and are complete the moment they land. This one
/// is written by a later turn that asked the forge a question the finishing
/// turn could not have answered — Armada opens a pull request and a person
/// merges it — so it is a different fact with a different writer.
///
/// One nullable column and no backfill: a Job whose pull request merged before
/// this shipped reads as unasked, which is exactly what it is.
pub(crate) const V26: &str = r#"
ALTER TABLE jobs ADD COLUMN delivery_landed TEXT;
"#;

/// Version 51 — whether the branch a delivery describes ever reached its
/// remote. `#691`.
///
/// **One nullable column, and null is the ordinary answer.** A clean push, a
/// repository with no base to catch up to, and a worktree with nothing new to
/// commit are the same fact here: nothing is owed to a remote that does not
/// already have it. Non-null is the one case this exists for — catching the
/// branch up to its base conflicted, or its own commits would not replay, so
/// `fleet::delivery::deliver` left the commit standing in the worktree and
/// this names why, in the same words the Job's log carries.
pub(crate) const V51: &str = r#"
ALTER TABLE jobs ADD COLUMN delivery_unpushed_reason TEXT;
"#;

/// Version 75 — how many times in a row Fleet has sent a Job back from its
/// review gate to clear its pull request's conflicts. `#1131`.
///
/// **A count, not a history.** Each send raises it and a delivery that pushed
/// zeroes it, so it reads as passes in a row with nothing to fold. Zero is the
/// default and every Job written before it, which has never been sent.
pub(crate) const V75: &str = r#"
ALTER TABLE jobs ADD COLUMN delivery_clearing_sends INTEGER NOT NULL DEFAULT 0;
"#;

/// Version 48 — the base a pull request's branch was last brought up to,
/// where main moved under it. `#663`, in place of the close-and-reopen it
/// replaced.
///
/// **Durable, unlike the in-memory guard it replaces.** `fleet::noticing`
/// used to remember what it had closed and reopened in a map that did not
/// survive a restart, and #660 was closed and reopened twice for the same
/// base because of it. This is read back before a rebase is attempted, so
/// the memory outlives the process that wrote it.
///
/// Three nullable columns and no backfill, [`V21`]'s shape: a Job whose
/// branch has never needed to move has nothing to say here, and a Job written
/// before this column existed reads exactly the same as one that has not.
pub(crate) const V48: &str = r#"
ALTER TABLE jobs ADD COLUMN delivery_rebased_onto TEXT;
ALTER TABLE jobs ADD COLUMN delivery_rebased_at TEXT;
ALTER TABLE jobs ADD COLUMN delivery_rebase_conflict_files TEXT;
"#;

/// Version 53 — a step edited outside its declared plan, kept for as long as
/// the Job is. Beside the table it creates, per `crate::report::V17`.
pub(crate) const V53: &str = r#"
CREATE TABLE job_scope_drift (
    job_id        TEXT NOT NULL REFERENCES jobs(job_id),
    step_id       TEXT NOT NULL,
    path          TEXT NOT NULL,
    first_seen_at TEXT NOT NULL,
    PRIMARY KEY (job_id, path)
) STRICT;
"#;

/// Version 19 — a Drone belongs to a step.
///
/// Beside the change it makes, like [`V17`](crate::report::V17) and
/// [`V18`](crate::plan::V18): `schema.rs` is at the 900 the gate refuses at.
///
/// | What moves | Why |
/// |---|---|
/// | `job_steps.assigned_drone` | The pointer, one per step instead of one per Job |
/// | `job_events.step_id` on drone rows | The fold has to know which row's pointer a move is about; without it a Job that ran four Drones collapses them into one |
/// | `jobs.assigned_drone` | Dropped. A second statement of one fact is a pair that can disagree, and the Job-level reading is derived from these rows |
///
/// The shape trigger is rewritten whole because `SQLite` cannot alter one, and
/// the drone arm is what changes: `step_id` was required to be null and is now
/// required present. `job_events_are_never_edited` comes off for one statement
/// and goes straight back on — a column added to rows already written cannot be
/// filled without lifting it, and lifting it inside the migration list is that
/// edit being versioned rather than hidden in a query.
///
/// **The step is derived, not invented.** `fleet::spawning` moves a step to
/// `running` before a process exists, so the step in force at a `drone_spawned`
/// is the last `step_transition` to `running` before it, and an exit follows
/// its own spawn — `V5`'s rule, backfill only what is observed. A row with no
/// earlier one keeps a null `step_id`, which the fold refuses as it refuses a
/// null `workflow`. Nothing backfills `job_steps.assigned_drone`: it is a cache
/// the fold fills.
pub(crate) const V19: &str = r#"
ALTER TABLE job_steps ADD COLUMN assigned_drone TEXT;

DROP TRIGGER job_events_are_never_edited;

UPDATE job_events SET step_id = (
    SELECT earlier.step_id FROM job_events AS earlier
    WHERE earlier.job_id = job_events.job_id
      AND earlier.kind = 'step_transition'
      AND earlier.state_to = 'running'
      AND earlier.seq < job_events.seq
    ORDER BY earlier.seq DESC
    LIMIT 1
)
WHERE kind IN ('drone_spawned', 'drone_exited');

CREATE TRIGGER job_events_are_never_edited
BEFORE UPDATE ON job_events
BEGIN
    SELECT RAISE(ABORT, 'job_events is append-only: a recorded transition is never edited');
END;

DROP TRIGGER job_events_hold_one_whole_shape;

CREATE TRIGGER job_events_hold_one_whole_shape
BEFORE INSERT ON job_events
WHEN NOT (
    (NEW.kind = 'job_transition'
        AND NEW.step_id IS NULL AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL AND NEW.drone_id IS NULL
        AND NEW.status_from <> NEW.status_to)
 OR (NEW.kind = 'step_transition'
        AND NEW.step_id IS NOT NULL AND NEW.state_from IS NOT NULL
        AND NEW.state_to IS NOT NULL AND NEW.drone_id IS NULL
        AND NEW.status_from = NEW.status_to
        AND (CASE WHEN NEW.state_to IN ('stopped', 'retrying')
                    OR (NEW.state_to = 'advanced' AND NEW.state_from = 'stopped')
                  THEN NEW.reason_kind = 'escalation' AND NEW.reason_value IS NOT NULL
                  ELSE NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL
             END))
 OR (NEW.kind IN ('drone_spawned', 'drone_exited')
        AND NEW.step_id IS NOT NULL AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL AND NEW.drone_id IS NOT NULL
        AND NEW.status_from = NEW.status_to
        AND NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'a job_events row is one whole shape: a job transition with no step or drone columns, a step move beneath an unchanged status carrying a trigger only where it stops the step, hands it back, or advances one that stopped, or a drone arriving on a step or leaving it beneath one');
END;

ALTER TABLE jobs DROP COLUMN assigned_drone;
"#;

/// Version 25 — what each file in a footprint gained and lost.
///
/// Beside the table it changes, like [`V17`](crate::report::V17) and the
/// migrations after it: `schema.rs` is at the 900 lines the gate refuses at.
///
/// **Nullable, and null is "not counted" rather than zero.** A binary file has
/// no patch to count and a rename that edited nothing is a real `0` — a column
/// that could not tell those apart would turn an unmeasured file into a file
/// that changed nothing.
///
/// **The pair may not half-arrive**, which is why the table is rebuilt rather
/// than altered: `SQLite` cannot add a `CHECK` to a table that exists.
/// **Nothing to backfill**: a footprint written before this has no counts.
pub(crate) const V25: &str = r#"
CREATE TABLE job_footprint_files_counted (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    ordinal INTEGER NOT NULL,
    path    TEXT NOT NULL,
    change  TEXT NOT NULL,
    added   INTEGER,
    deleted INTEGER,
    PRIMARY KEY (job_id, ordinal),
    CHECK ((added IS NULL) = (deleted IS NULL))
) STRICT;

INSERT INTO job_footprint_files_counted (job_id, ordinal, path, change)
SELECT job_id, ordinal, path, change FROM job_footprint_files;

DROP TABLE job_footprint_files;

ALTER TABLE job_footprint_files_counted RENAME TO job_footprint_files;
"#;

/// Version 24 — where a flag points, beside what it cites.
///
/// Beside the table it changes rather than in `schema.rs`, like
/// [`V22`](crate::judged::V22): that file is at the 900 lines the gate refuses
/// at.
///
/// **Two nullable columns and no `CHECK`.** `SQLite` cannot add a constraint by
/// `ALTER`, and the invariant one would state is already unspeakable one layer
/// up: `core_model::CitedAt` has no constructor taking a line alone.
///
/// **Nothing is backfilled.** A flag written before this existed kept only the
/// sentence the check wrote, and where in the patch that sentence came from was
/// never recorded — so the honest answer for every existing row is null, V5's
/// rule.
pub(crate) const V24: &str = r#"
ALTER TABLE job_step_gaming_flags ADD COLUMN cited_file TEXT;
ALTER TABLE job_step_gaming_flags ADD COLUMN cited_line INTEGER;
"#;

/// Version 39 — what the flag was asked, and where the whole call was kept.
///
/// Beside the table it changes, like [`V24`] above and for its reason.
///
/// **Two columns and not one.** `asked` is the narrow question, which is what
/// reaches a person on a row; `brief_path` names the file holding the diff and
/// the baseline that question was put with, which is what an argument weeks
/// later is settled against. A row carrying only the path would put the reason
/// a flag fired behind a filesystem read the wire cannot make.
///
/// **Both nullable, and null is a real answer.** The three `DecidedBy::Diff`
/// patterns are answered by the patch and asked nothing, and `fleet::asked`
/// keeps no brief where there is no repository under the Fleet. **Nothing is
/// backfilled**, which is V5's rule: the answer to every judged call made
/// before this was thrown away, and no column can invent it back.
pub(crate) const V39: &str = r#"
ALTER TABLE job_step_gaming_flags ADD COLUMN asked TEXT;
ALTER TABLE job_step_gaming_flags ADD COLUMN brief_path TEXT;
"#;

/// Version 73 — what a second reading said about a flag it cleared.
///
/// Beside the table it changes, like [`V24`] and [`V39`]. **A null
/// `cleared_why` is a flag that stands**: every row before this, every flag a
/// second reading agreed with, and every flag the patch decided. A brief path
/// with no reason beside it reads as standing, because a clearance is its
/// reason. Nothing is backfilled, V5's rule.
pub(crate) const V73: &str = r#"
ALTER TABLE job_step_gaming_flags ADD COLUMN cleared_why TEXT;
ALTER TABLE job_step_gaming_flags ADD COLUMN cleared_brief_path TEXT;
"#;

/// Version 22 — where the brief a verdict answered was written.
///
/// Beside the change it makes rather than in `schema.rs`, like
/// [`V20`](crate::note::V20) and for the same reason: that file is at the 900
/// lines the gate refuses at.
///
/// One nullable column, and **nothing is backfilled**. No brief written before
/// this existed was kept anywhere, so every existing row's honest answer is
/// null — V5's rule, which is to backfill only what is observed.
pub(crate) const V22: &str = r#"
ALTER TABLE job_step_judgments ADD COLUMN brief_path TEXT;
"#;

/// Version 30 — which member of a panel an answer came back from.
///
/// Beside the change it makes, like [`V22`] above and for the same reason.
///
/// # The rows were already there
///
/// `fleet::judging::looks` has always run `panel_size` calls per criterion and
/// pushed a `Judgment` for each, so a step at `panel_size: 3` has always
/// written three rows per criterion. What it did not write is which call each
/// row came back from: `ordinal` is the position in the whole step's sequence
/// and moves when a criterion is appended, and the primary key holds nothing
/// else that varies between members. Three rows, one meaning, no way to tell
/// them apart — so the detail view drew one and dropped two while the step's
/// declaration said a panel of three had answered.
///
/// One nullable column, and **nothing is backfilled**. V5's rule: an existing
/// row's honest answer is null, because the number was discarded at the loop
/// and is not recoverable from anything stored. `ordinal` looks like it would
/// recover it and does not — it counts across criteria, so which member wrote
/// row 4 depends on how many criteria the step was carrying at the time.
///
/// **Null also means one judge**, which is the same value a panel of one
/// writes today. Both are read as "no panel to distinguish", which is what a
/// reader does with either.
pub(crate) const V30: &str = r#"
ALTER TABLE job_step_judgments ADD COLUMN member INTEGER;
"#;
