//! The SQL of the migrations whose tables belong to Jobs, which are gone.
//!
//! **Nothing reads or writes these tables.** The list in `legacy_migrations.rs`
//! is frozen and a database's version row says it ran these, so the statements
//! stay and the tables are made empty and left. No migration drops them.

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

/// Version 35 — what each member of a panel read, and what each was handed.
///
/// Beside the change it makes, like [`V22`] and [`V30`] above and for the same
/// reason.
///
/// # Two columns, because they answer opposite questions
///
/// `cited` is where in the brief this member's own words are quoted from —
/// what it *read*, as coordinates into the file [`V22`]'s column already
/// points at. `given` is a digest of the exact text that member's call was
/// sent, which is what makes rule 5's guarantee — a panel is independent
/// judges against **identical** inputs — a reading rather than an assertion
/// about the shape of a loop.
///
/// Both are JSON, for `jobs.acceptance_criteria`'s reason: a list and a
/// three-field object are each one value about one row, and a table per row
/// would be two more joins on a read taken on every event naming the open Job.
/// `crate::columns` holds both directions of each, next to each other.
///
/// Two nullable columns, and **nothing is backfilled**. V5's rule. Neither is
/// recoverable: the placement is a function of a brief a stored row may no
/// longer have a path to, and the digest is of a string that was dropped when
/// the call returned. **Null on `given` is not "one judge"**, unlike
/// [`V30`]'s column — it is "nobody wrote this down", which is every row a
/// build before this one produced.
pub(crate) const V35: &str = r#"
ALTER TABLE job_step_judgments ADD COLUMN cited TEXT;
ALTER TABLE job_step_judgments ADD COLUMN given TEXT;
"#;

/// Version 82 — find a Job by the Job it replaced, on every open of a Job.
///
/// Beside the read that needs it, like [`V36`](crate::numbering::V36): the
/// order of the migration list is `migrations.rs`'s alone.
///
/// Not unique. A second redispatch of one original is legal today, and an
/// index refusing it would fail the write rather than the act.
pub(crate) const V82: &str = r#"
CREATE INDEX jobs_by_redispatched_from ON jobs (redispatched_from);
"#;

/// Version 46 — the Manifest snapshot column, beside the frozen workflow it
/// sits next to in what a Job carries out of its creation.
///
/// **Nullable, and no `DEFAULT`.** Unlike [`crate::allowing::V44`]'s
/// `when_blocked`, there is no value here a Job that named none could be said
/// to have chosen — the column is either the file as it stood the instant the
/// row was written, or nothing was ever written, and a placeholder string
/// would read as the first when it is the second.
pub(crate) const V46: &str = r#"
ALTER TABLE jobs ADD COLUMN manifest_snapshot TEXT;
"#;

/// Version 97 — a task's tier and a person's edit on the plan, a Job's tier
/// map, and the model each Drone ran.
///
/// The plan's changes are rebuilt the way [`V95`](crate::groups::V95) rebuilt
/// them, to admit `edited` and carry its `model`; an edit's other fields reuse
/// the columns an add writes, `NULL` being a field the person left alone.
/// Nothing is backfilled: a task recorded before this has no tier, which is
/// Armada picking, and a Drone spawned before it names no model.
pub(crate) const V97: &str = r#"
CREATE TABLE job_work_plan_changes_wide (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    seq        INTEGER NOT NULL CHECK (seq > 0),
    change     TEXT NOT NULL CHECK (change IN
                   ('recorded', 'added', 'updated', 'moved_task', 'moved_group', 'edited')),
    by_step    TEXT,
    by_attempt INTEGER CHECK (by_attempt IS NULL OR by_attempt > 0),
    at         TEXT NOT NULL,
    approach   TEXT,
    task_id    INTEGER CHECK (task_id IS NULL OR task_id > 0),
    title      TEXT,
    detail     TEXT,
    after_task INTEGER CHECK (after_task IS NULL OR after_task > 0),
    state      TEXT CHECK (state IS NULL OR state IN
                   ('open', 'working', 'handed_in', 'done', 'failed', 'dropped')),
    reason     TEXT,
    scope      TEXT,
    expects    TEXT,
    shown      TEXT,
    grp        INTEGER CHECK (grp IS NULL OR grp > 0),
    after_grp  INTEGER CHECK (after_grp IS NULL OR after_grp > 0),
    model      TEXT CHECK (model IS NULL OR trim(model) <> ''),
    PRIMARY KEY (job_id, seq),
    CHECK ((by_step IS NULL) = (by_attempt IS NULL)),
    CHECK (state IS NULL OR
           ((state IN ('dropped', 'failed')) = (reason IS NOT NULL AND trim(reason) <> ''))),
    CHECK (change = 'edited' OR model IS NULL)
) STRICT;

INSERT INTO job_work_plan_changes_wide (
    job_id, seq, change, by_step, by_attempt, at, approach, task_id, title, detail,
    after_task, state, reason, scope, expects, shown, grp, after_grp
) SELECT job_id, seq, change, by_step, by_attempt, at, approach, task_id, title, detail,
         after_task, state, reason, scope, expects, shown, grp, after_grp
  FROM job_work_plan_changes;

DROP TABLE job_work_plan_changes;
ALTER TABLE job_work_plan_changes_wide RENAME TO job_work_plan_changes;

CREATE TRIGGER job_work_plan_changes_are_never_edited
BEFORE UPDATE ON job_work_plan_changes
BEGIN
    SELECT RAISE(ABORT, 'a plan change is never edited');
END;

CREATE TRIGGER job_work_plan_changes_are_never_removed_from_a_job_that_exists
BEFORE DELETE ON job_work_plan_changes
WHEN EXISTS (SELECT 1 FROM jobs WHERE jobs.job_id = OLD.job_id)
BEGIN
    SELECT RAISE(ABORT, 'a plan change is never removed from a Job that exists');
END;

ALTER TABLE job_work_plan_tasks ADD COLUMN tier TEXT
    CHECK (tier IS NULL OR tier IN ('difficult', 'medium', 'easy'));

CREATE TABLE job_tier_models (
    job_id TEXT NOT NULL REFERENCES jobs(job_id),
    tier   TEXT NOT NULL CHECK (tier IN ('difficult', 'medium', 'easy')),
    model  TEXT NOT NULL CHECK (trim(model) <> ''),
    PRIMARY KEY (job_id, tier)
) STRICT;

CREATE TABLE job_drone_models (
    job_id   TEXT NOT NULL REFERENCES jobs(job_id),
    drone_id TEXT NOT NULL,
    model    TEXT NOT NULL CHECK (trim(model) <> ''),
    PRIMARY KEY (job_id, drone_id)
) STRICT;
"#;

/// Version 20 — a redirect that arrived with no Drone to take it.
///
/// Beside the change it makes, like [`V19`](crate::drone::V19): `schema.rs` is
/// at the 900 the gate refuses at.
///
/// One nullable column, and nothing is backfilled: a Job written before this
/// has no note waiting, which is what null already means.
pub(crate) const V20: &str = r#"
ALTER TABLE jobs ADD COLUMN redirect_waiting TEXT;
"#;

/// Version 36 — the number a person calls a Job by.
///
/// Beside the change it makes, like [`V20`](crate::note::V20): `schema.rs` is
/// at the 900 lines the gate refuses at.
///
/// **Backfilled, not left null.** Every Job gets a number, including the ones
/// written before this, so the record is total and nothing downstream has to
/// draw a Job that has no name. The order is creation order within each
/// Manifest, with the id breaking a tie — two Jobs of one proposal are minted
/// in the same millisecond, so ties are the ordinary case here rather than the
/// rare one.
///
/// `DEFAULT 0` is what `ALTER TABLE` requires of a `NOT NULL` column and is
/// never a value any row keeps: the `UPDATE` below covers every row, and the
/// index after it would refuse a second 0 in one Manifest if it did not.
pub(crate) const V36: &str = r#"
ALTER TABLE jobs ADD COLUMN number INTEGER NOT NULL DEFAULT 0;

UPDATE jobs SET number = (
    SELECT count(*) FROM jobs AS earlier
    WHERE earlier.owner_manifest_id = jobs.owner_manifest_id
      AND (earlier.created_at < jobs.created_at
           OR (earlier.created_at = jobs.created_at AND earlier.job_id <= jobs.job_id))
);

CREATE UNIQUE INDEX jobs_number_within_a_manifest ON jobs (owner_manifest_id, number);
"#;

/// Version 111 — the marker on a paused Job.
///
/// **Null, and no backfill.** Every row before this column is a Job nobody has
/// paused. `pause_by` is the marker's presence, and the other two are only read
/// where it is set. `Store::record_pause` is the one writer.
pub(crate) const V111: &str = r#"
ALTER TABLE jobs ADD COLUMN pause_by TEXT CHECK (pause_by IN ('person', 'fleet'));
ALTER TABLE jobs ADD COLUMN pause_at TEXT;
ALTER TABLE jobs ADD COLUMN pause_resuming INTEGER NOT NULL DEFAULT 0 CHECK (pause_resuming IN (0, 1));
"#;

/// Version 76 — evidence waiting for the gate, one row per Job.
///
/// Beside the table it creates, like [`V27`](crate::process::V27): `schema.rs`
/// is at the 900 lines the gate refuses at. Nothing is backfilled — a
/// submission that landed before this migration reads the same as a Job
/// whose Drone never submitted.
pub(crate) const V76: &str = r#"
CREATE TABLE job_pending_evidence (
    job_id        TEXT NOT NULL PRIMARY KEY REFERENCES jobs(job_id),
    step_id       TEXT NOT NULL,
    evidence_type TEXT NOT NULL,
    claimed       TEXT NOT NULL,
    shown_by      TEXT NOT NULL,
    not_claimed   TEXT NOT NULL,
    landed_at     TEXT NOT NULL
) STRICT;
"#;

/// Version 18 — what a step declared its work would be.
///
/// Beside the tables it creates rather than in `schema.rs`, for the reason
/// `V17` is: that file sits on the 900 lines the gate refuses at. The order
/// still lives there, which is the part that may not be anywhere else.
///
/// **Nothing to backfill.** No declaration was written down before this table
/// existed, which is what zero rows says — and a Job that finished before it
/// reads as a Job whose steps declared nothing, which is the same sentence a
/// Job whose steps genuinely declared nothing gets. Both are silent rather than
/// counted, which is the answer this record exists to make possible.
pub(crate) const V18: &str = r#"
CREATE TABLE job_step_plans (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    attempt     INTEGER NOT NULL,
    declared_at TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt)
) STRICT;

-- One row per declared path, in the order the drone named them. A plan with no
-- rows is a step that declared it would touch nothing; a step with no header
-- row above declared nothing at all.
CREATE TABLE job_step_plan_paths (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    path    TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;
"#;

/// Version 27 — the process working a Job, by pid and by when it started.
///
/// Beside the table it creates, like [`V23`](crate::spend::V23) and the
/// migrations either side of it: `schema.rs` is at the 900 lines the gate
/// refuses at.
///
/// **One row per Job and not per step**, because a Job holds one Drone at a
/// time — `fleet::steps_holding_a_drone` iterating is what makes the record's
/// answer the answer, and a second row here would be a second Drone this
/// workspace has no way to produce. The step is carried on the row rather than
/// in the key, so a reader knows which step's pointer the process belongs to
/// without joining.
///
/// **The foreign key is the point of `job_id` being a real reference.**
/// `forget_job` asks the file which tables point at `jobs` rather than reading
/// a list somebody maintains, so this table joins that walk by existing.
///
/// Nothing is backfilled. A Drone spawned before this has no row, which is the
/// same answer as a Job that never spawned one: nothing recorded, so nothing
/// can be adopted and the reconciliation falls back to what it always did.
pub(crate) const V27: &str = r#"
CREATE TABLE job_drone_process (
    job_id     TEXT    PRIMARY KEY NOT NULL REFERENCES jobs(job_id),
    step_id    TEXT    NOT NULL,
    drone_id   TEXT    NOT NULL,
    pid        INTEGER NOT NULL,
    started_at TEXT    NOT NULL,
    spawned_at TEXT    NOT NULL,
    CHECK (pid > 0)
) STRICT;
"#;

/// Version 34 — the reading a Job was minted by.
///
/// Beside the change it makes, like [`V20`](crate::note::V20): `schema.rs` is
/// at the 900 lines the gate refuses at.
///
/// One nullable column and no backfill. Null is a Job nobody proposed, which is
/// exactly what every Job written before this reads as — the proposals that
/// minted them are over and their ids were never kept.
pub(crate) const V34: &str = r#"
ALTER TABLE jobs ADD COLUMN proposal_id TEXT;
"#;

/// Version 74 — a stopped step re-entered for a re-run of its Checks carries
/// the trigger it stopped on.
///
/// `core_model::StepTarget::Rechecking` crosses `stopped -> running` as a
/// restart does, and no Drone works it. The trigger is what tells the two rows
/// apart, so `attempt_now` and `spent_now` leave this one out. **A trigger is
/// now optional on that edge and still refused on every other arrival at
/// `running`.** Nothing to backfill.
pub(crate) const V74: &str = r#"
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
                  WHEN NEW.state_to = 'running' AND NEW.state_from = 'stopped'
                  THEN (NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL)
                    OR (NEW.reason_kind = 'escalation' AND NEW.reason_value IS NOT NULL)
                  ELSE NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL
             END)
        AND (NEW.returned_by IS NULL
             OR (NEW.state_from = 'advanced' AND NEW.state_to = 'running')))
 OR (NEW.kind IN ('drone_spawned', 'drone_exited')
        AND NEW.step_id IS NOT NULL AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL AND NEW.drone_id IS NOT NULL
        AND NEW.returned_by IS NULL
        AND NEW.status_from = NEW.status_to
        AND NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'a job_events row is one whole shape: a job transition with no step or drone columns, a step move beneath an unchanged status carrying a trigger only where it stops the step, hands it back, advances one that stopped, or re-enters one that stopped to run its Checks again, and the step that routed it only where it is a loop return, or a drone arriving on a step or leaving it beneath one');
END;
"#;

/// Version 31 — the comments on a Job's pull request that have reached a Drone.
///
/// Beside the change it makes, like [`V29`](crate::proving::V29): `schema.rs`
/// is at the 900 lines the gate refuses at.
///
/// **The pair is the key**, so writing the same handle twice for one Job is a
/// conflict rather than a second row — the whole content of this table is
/// "already", and a duplicate would be a fact stated twice.
///
/// `remark_id` is the forge's own text and is never parsed. It is compared and
/// nothing else.
pub(crate) const V31: &str = r#"
CREATE TABLE job_remarks_taken_up (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    remark_id   TEXT NOT NULL,
    at          TEXT NOT NULL,
    PRIMARY KEY (job_id, remark_id)
) STRICT;
"#;

/// Version 87 — the policies each gate resolved to, per run.
///
/// Its own table rather than columns on another: no table holds one row per
/// run. `job_step_evidence` is the nearest, and a gate can pass on a run whose
/// evidence row a later submission in that same run replaced.
pub(crate) const V87: &str = r#"
CREATE TABLE job_step_policies (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    attempt     INTEGER NOT NULL CHECK (attempt >= 1),
    auto_merge  TEXT NOT NULL,
    review_gate TEXT NOT NULL,
    resolved_at TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt)
) STRICT;
"#;

/// Version 89 — whether the run reached the advance gate, where the rule
/// decides. Rows are now written on a run an earlier gate stopped too, and
/// they say `0`.
///
/// **Every row written before this reads `1`, and that is a fact rather than a
/// default.** V87's table was written only by a ruling that read the advance gate,
/// held or advanced, so every row already in the table is one the rule
/// decided. The column default is what says so for them, and every write since
/// names the value.
pub(crate) const V89: &str = r#"
ALTER TABLE job_step_policies
    ADD COLUMN decided INTEGER NOT NULL DEFAULT 1 CHECK (decided IN (0, 1));
"#;

/// Version 38 — whether a Job's disk has been given back.
///
/// Beside the change it makes, like [`crate::numbering::V36`]: `schema.rs` is
/// at the 900 lines the gate refuses at.
///
/// **Null, and no backfill.** Nothing recorded a reclaim before this column
/// existed, so every row predating it is a Job whose disk — as far as this
/// column can say — still stands, which is the honest reading rather than a
/// guess. `Store::retain_job` is the one writer.
pub(crate) const V38: &str = r#"
ALTER TABLE jobs ADD COLUMN reclaimed_at TEXT;
"#;

/// Version 58 — a step the loop's return went back past is entered again with
/// no emitter on its row.
///
/// `advanced -> running` had one walker, and [`V28`](crate::attempt::V28)
/// required `returned_by` on every row across it.
/// `core_model::StepTarget::Retraced` is the second: the walk forward over a
/// step that advanced on the pass before. The pass was charged to the emitter
/// at the return, so this row names nobody and `store::step_iteration` stays
/// one count per loop. **The column is now optional on that edge and still
/// refused on every other.** Nothing to backfill.
pub(crate) const V58: &str = r#"
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
        AND (NEW.returned_by IS NULL
             OR (NEW.state_from = 'advanced' AND NEW.state_to = 'running')))
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

/// Version 69 — a Check's row says when it was answered from a asked run
/// instead of the gate's own run.
///
/// **Nullable, and no `DEFAULT`**: `NULL` is a Check the gate ran itself,
/// which is every row this database already holds.
pub(crate) const V70: &str = r#"
ALTER TABLE job_step_checks ADD COLUMN reused_from_dry_run TEXT;
"#;

/// Version 66 — what a person made of a review's findings.
///
/// **Nothing to backfill.** Nothing could follow a finding up before this.
pub(crate) const V66: &str = r#"
CREATE TABLE job_review_followups (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    finding TEXT NOT NULL,
    kind    TEXT NOT NULL CHECK (kind IN ('queued', 'issue')),
    target  TEXT NOT NULL,
    at      TEXT NOT NULL,
    PRIMARY KEY (job_id, finding, kind)
) STRICT;
"#;

/// Version 60 — Armada's review of a step, in the parts it is made of.
///
/// **Nothing to backfill.** No step asked for review evidence before this, and a Job
/// with no rows reads as a Job with no review.
pub(crate) const V60: &str = r#"
CREATE TABLE job_step_reviews (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    attempt     INTEGER NOT NULL CHECK (attempt >= 1),
    confidence  TEXT NOT NULL CHECK (confidence IN ('confident', 'not_confident')),
    recorded_at TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt)
) STRICT;

CREATE TABLE job_step_review_reasons (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    reason  TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;

CREATE TABLE job_step_review_areas (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    name    TEXT NOT NULL,
    what    TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;

CREATE TABLE job_step_review_area_files (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    area    INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    path    TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, area, ordinal)
) STRICT;

CREATE TABLE job_step_review_proves (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    area    TEXT NOT NULL,
    what    TEXT NOT NULL,
    tests   INTEGER NOT NULL CHECK (tests >= 0),
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;

CREATE TABLE job_step_review_changed (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    attempt     INTEGER NOT NULL,
    ordinal     INTEGER NOT NULL,
    name        TEXT NOT NULL,
    change      TEXT NOT NULL CHECK (change IN ('removed', 'loosened')),
    replaced_by TEXT,
    why         TEXT,
    unexplained INTEGER NOT NULL CHECK (unexplained IN (0, 1)),
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;

CREATE TABLE job_step_review_untested (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    code    TEXT NOT NULL,
    why     TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;

CREATE TABLE job_step_review_findings (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    bucket  TEXT NOT NULL CHECK (bucket IN ('needs_you', 'small_fix', 'for_context')),
    finding TEXT NOT NULL,
    why     TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;
"#;

/// Version 23 — what each Drone of a Job spent.
///
/// Beside the change it makes, like [`V22`](crate::judged::V22):
/// `schema.rs` is at the 900 the gate refuses at.
///
/// **A table and not columns on `jobs`**, which is the opposite call to
/// `delivery.v21`'s and for the reason that one gives: delivery is at most
/// one row per Job, written once. This is one row per Drone, and a Job has as
/// many Drones as its workflow has steps.
///
/// **No event, and nothing folded.** A spend is not a move — no status
/// changes, no step changes — so there is nothing for `job_events` to carry
/// and `crate::read` has nothing to rebuild. The table is the authority for its
/// own figures, as `note.v20`'s column is for the note.
///
/// Nothing is backfilled. A Job that ran before this spent what it spent and
/// the figure was thrown away; an empty set of rows and a Job that has not
/// started yet are the same answer, which is a total of nothing.
pub(crate) const V23: &str = r#"
CREATE TABLE job_drone_spend (
    job_id      TEXT    NOT NULL REFERENCES jobs(job_id),
    drone_id    TEXT    NOT NULL,
    cost_micros INTEGER NOT NULL,
    turns       INTEGER NOT NULL,
    ran_ms      INTEGER NOT NULL,
    PRIMARY KEY (job_id, drone_id)
) STRICT;
"#;

/// Version 32 — what this one Job may spend.
///
/// **A column on `jobs` and not a table**, which is [`V23`]'s call inverted for
/// `delivery.v21`'s reason: at most one ceiling per Job, written when
/// somebody sets one and read on every admission.
///
/// **Null on every existing row, and that is the answer rather than a gap.** A
/// Job that ran before this had no cap of its own and took the machine's, which
/// is exactly what null says. There is nothing to backfill and nothing to
/// guess — [`crate::schema::V5`]'s rule.
///
/// **Null and zero are different sentences and the schema keeps them apart.**
/// Null defers to `armada.yml` and then to the composition root; zero is a Job
/// that starts nothing. A column with a `NOT NULL DEFAULT 0` would have made
/// the two one value and the second unreachable.
///
/// The trigger refuses a negative on both writes. `INTEGER` is signed, the
/// reader widens to `u64`, and a `-1` that reached the row would come back as
/// eighteen quintillion dollars — a cap that cannot be exceeded, arriving
/// silently. Nothing in this crate can write one; the trigger is for whatever
/// opens the file that is not this crate.
pub(crate) const V32: &str = r#"
ALTER TABLE jobs ADD COLUMN cost_cap_micros INTEGER;

CREATE TRIGGER jobs_are_never_given_a_negative_cost_cap_on_insert
BEFORE INSERT ON jobs
WHEN NEW.cost_cap_micros IS NOT NULL AND NEW.cost_cap_micros < 0
BEGIN
    SELECT RAISE(ABORT, 'a cost cap is what a Job may spend, and below nothing is not a number it may spend');
END;

CREATE TRIGGER jobs_are_never_given_a_negative_cost_cap_on_update
BEFORE UPDATE ON jobs
WHEN NEW.cost_cap_micros IS NOT NULL AND NEW.cost_cap_micros < 0
BEGIN
    SELECT RAISE(ABORT, 'a cost cap is what a Job may spend, and below nothing is not a number it may spend');
END;
"#;

/// Version 33 — a price that was never named.
///
/// **`NULL`, and it is not a cost of zero.** `total_cost_usd` reaches Armada
/// on one line — the terminating line of a session — so a Drone signalled
/// before it emits one leaves no figure at all. Written as `0` those two are
/// the same row, and one Job is what that cost: two of its six Drones ran 277
/// and 299 seconds against a $5 cap and were billed nothing, because both were
/// stopped by defects that killed them mid-run. The Job read as $5.28 spent
/// and had spent more, and nothing on it said so.
///
/// **Nothing is backfilled**, for [`V23`]'s reason and one of its own: a `0`
/// already on disk may be either, and a migration that guessed would relabel a
/// Drone that genuinely cost nothing. Rows written from here on can be told
/// apart; rows written before cannot, and no surface pretends otherwise.
pub(crate) const V33: &str = r#"
ALTER TABLE job_drone_spend RENAME TO job_drone_spend_priced;
CREATE TABLE job_drone_spend (
    job_id      TEXT    NOT NULL REFERENCES jobs(job_id),
    drone_id    TEXT    NOT NULL,
    cost_micros INTEGER,
    turns       INTEGER NOT NULL,
    ran_ms      INTEGER NOT NULL,
    PRIMARY KEY (job_id, drone_id)
) STRICT;
INSERT INTO job_drone_spend (job_id, drone_id, cost_micros, turns, ran_ms)
    SELECT job_id, drone_id, cost_micros, turns, ran_ms FROM job_drone_spend_priced;
DROP TABLE job_drone_spend_priced;
"#;

/// Version 40 — how many turns this one Job may take.
///
/// **[`V32`] again, for the other ceiling.** The column, the null, the two
/// triggers and the reasoning behind each are that migration's; what is new is
/// only which number it holds. One Job is the Job that bought this one — 393
/// turns against a 300 constant with no tier under it, stopped on the cheap
/// final step of work every Check had passed, and its branch committed by
/// hand.
///
/// **Turns and not a second unit of money**, so nothing here converts: a turn
/// is a count off the harness's terminating line, which is why the column is
/// bare `INTEGER` where the dollars are micros.
pub(crate) const V40: &str = r#"
ALTER TABLE jobs ADD COLUMN turn_cap INTEGER;

CREATE TRIGGER jobs_are_never_given_a_negative_turn_cap_on_insert
BEFORE INSERT ON jobs
WHEN NEW.turn_cap IS NOT NULL AND NEW.turn_cap < 0
BEGIN
    SELECT RAISE(ABORT, 'a turn cap is how many turns a Job may take, and below nothing is not a number it may take');
END;

CREATE TRIGGER jobs_are_never_given_a_negative_turn_cap_on_update
BEFORE UPDATE ON jobs
WHEN NEW.turn_cap IS NOT NULL AND NEW.turn_cap < 0
BEGIN
    SELECT RAISE(ABORT, 'a turn cap is how many turns a Job may take, and below nothing is not a number it may take');
END;
"#;

/// Version 92 — a task's Drone and its hand-in.
///
/// Beside the table it creates, like [`V76`](crate::pending_evidence::V76).
/// Nothing is backfilled: no Drone was put on a task before it.
pub(crate) const V92: &str = r#"
CREATE TABLE job_task_drones (
    job_id       TEXT NOT NULL REFERENCES jobs(job_id),
    drone_id     TEXT NOT NULL,
    step_id      TEXT NOT NULL,
    task_id      INTEGER NOT NULL CHECK (task_id > 0),
    spawned_at   TEXT NOT NULL,
    handed_in_at TEXT,
    claimed      TEXT,
    shown_by     TEXT,
    not_claimed  TEXT,
    PRIMARY KEY (job_id, drone_id),
    CHECK ((handed_in_at IS NULL) = (claimed IS NULL)),
    CHECK ((handed_in_at IS NULL) = (shown_by IS NULL)),
    CHECK ((handed_in_at IS NULL) = (not_claimed IS NULL))
) STRICT;
"#;

/// Version 101 — the pass that made a member, when its pull request merged,
/// and what finishes a Job. Nothing is backfilled: a null is a row written
/// before this, which reads as it did then.
pub(crate) const V101: &str = r#"
ALTER TABLE jobs ADD COLUMN dispatched_by_pass INTEGER
    CHECK (dispatched_by_pass IS NULL OR dispatched_by_pass > 0);

ALTER TABLE jobs ADD COLUMN delivery_merged_at TEXT
    CHECK (delivery_merged_at IS NULL OR trim(delivery_merged_at) <> '');

ALTER TABLE job_landing ADD COLUMN complete_when TEXT
    CHECK (complete_when IS NULL OR complete_when IN ('delivered', 'all_members_landed'));
"#;

/// Version 80 — a task's own scope, the evidence expected of it, and the
/// evidence the work showed.
///
/// **Added as columns rather than a new table**, because each belongs to one
/// task of one recording and has no life apart from it. `shown` sits on the
/// change rather than the task: it is written by a later update, by whoever
/// worked the task, and the recording is not edited.
///
/// **Nothing to backfill.** A task recorded before this named its paths in
/// prose, in the note, where the next step could not act on them; there is no
/// reading of that prose that is more honest than an empty list.
pub(crate) const V80: &str = r#"
ALTER TABLE job_work_plan_tasks ADD COLUMN scope TEXT NOT NULL DEFAULT '';
ALTER TABLE job_work_plan_tasks ADD COLUMN expects TEXT NOT NULL DEFAULT '';
ALTER TABLE job_work_plan_changes ADD COLUMN scope TEXT;
ALTER TABLE job_work_plan_changes ADD COLUMN expects TEXT;
ALTER TABLE job_work_plan_changes ADD COLUMN shown TEXT;
"#;

/// Version 90 — a task may stand `handed_in` or `failed`, and a Judge or a
/// Check may sign a row (spike 022, slice 1a).
///
/// **The signers change no column**: `job_events.actor` is unconstrained text.
/// What they change is what a build can read, so the version moves and a build
/// that knows 89 refuses this file at open rather than failing to fold a Job
/// signed `judge`. The plan's `state` check is rebuilt to admit the two states
/// for the same reason, though nothing writes either yet.
///
/// Rebuilt under a `_wide` name and renamed over, [`V13`](crate::schema)'s way,
/// because SQLite cannot alter a check. Every column V59 and V80 gave it, in
/// order, and its two triggers again: a dropped table takes its triggers.
pub(crate) const V90: &str = r#"
CREATE TABLE job_work_plan_changes_wide (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    seq        INTEGER NOT NULL CHECK (seq > 0),
    change     TEXT NOT NULL CHECK (change IN ('recorded', 'added', 'updated')),
    by_step    TEXT,
    by_attempt INTEGER CHECK (by_attempt IS NULL OR by_attempt > 0),
    at         TEXT NOT NULL,
    approach   TEXT,
    task_id    INTEGER CHECK (task_id IS NULL OR task_id > 0),
    title      TEXT,
    detail     TEXT,
    after_task INTEGER CHECK (after_task IS NULL OR after_task > 0),
    state      TEXT CHECK (state IS NULL OR state IN
                   ('open', 'working', 'handed_in', 'done', 'failed', 'dropped')),
    reason     TEXT,
    scope      TEXT,
    expects    TEXT,
    shown      TEXT,
    PRIMARY KEY (job_id, seq),
    CHECK ((by_step IS NULL) = (by_attempt IS NULL)),
    CHECK (state IS NULL OR ((state = 'dropped') = (reason IS NOT NULL AND trim(reason) <> '')))
) STRICT;

INSERT INTO job_work_plan_changes_wide (
    job_id, seq, change, by_step, by_attempt, at, approach, task_id, title, detail,
    after_task, state, reason, scope, expects, shown
) SELECT job_id, seq, change, by_step, by_attempt, at, approach, task_id, title, detail,
         after_task, state, reason, scope, expects, shown
  FROM job_work_plan_changes;

DROP TABLE job_work_plan_changes;
ALTER TABLE job_work_plan_changes_wide RENAME TO job_work_plan_changes;

CREATE TRIGGER job_work_plan_changes_are_never_edited
BEFORE UPDATE ON job_work_plan_changes
BEGIN
    SELECT RAISE(ABORT, 'a plan change is never edited');
END;

CREATE TRIGGER job_work_plan_changes_are_never_removed_from_a_job_that_exists
BEFORE DELETE ON job_work_plan_changes
WHEN EXISTS (SELECT 1 FROM jobs WHERE jobs.job_id = OLD.job_id)
BEGIN
    SELECT RAISE(ABORT, 'a plan change is never removed from a Job that exists');
END;
"#;

/// Version 59 — a Job's plan: its changes, and the tasks each recording named.
///
/// **Nothing to backfill.** A plan written to a file before this is not one
/// Fleet held, and a Job with no rows reads as a Job with no plan.
pub(crate) const V59: &str = r#"
CREATE TABLE job_work_plan_changes (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    seq        INTEGER NOT NULL CHECK (seq > 0),
    change     TEXT NOT NULL CHECK (change IN ('recorded', 'added', 'updated')),
    by_step    TEXT,
    by_attempt INTEGER CHECK (by_attempt IS NULL OR by_attempt > 0),
    at         TEXT NOT NULL,
    approach   TEXT,
    task_id    INTEGER CHECK (task_id IS NULL OR task_id > 0),
    title      TEXT,
    detail     TEXT,
    after_task INTEGER CHECK (after_task IS NULL OR after_task > 0),
    state      TEXT CHECK (state IS NULL OR state IN ('open', 'working', 'done', 'dropped')),
    reason     TEXT,
    PRIMARY KEY (job_id, seq),
    CHECK ((by_step IS NULL) = (by_attempt IS NULL)),
    CHECK (state IS NULL OR ((state = 'dropped') = (reason IS NOT NULL AND trim(reason) <> '')))
) STRICT;

CREATE TABLE job_work_plan_tasks (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    seq     INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    title   TEXT NOT NULL CHECK (trim(title) <> ''),
    detail  TEXT NOT NULL,
    PRIMARY KEY (job_id, seq, ordinal)
) STRICT;

CREATE TRIGGER job_work_plan_changes_are_never_edited
BEFORE UPDATE ON job_work_plan_changes
BEGIN
    SELECT RAISE(ABORT, 'a plan change is never edited');
END;

CREATE TRIGGER job_work_plan_changes_are_never_removed_from_a_job_that_exists
BEFORE DELETE ON job_work_plan_changes
WHEN EXISTS (SELECT 1 FROM jobs WHERE jobs.job_id = OLD.job_id)
BEGIN
    SELECT RAISE(ABORT, 'a plan change is never removed from a Job that exists');
END;

CREATE TRIGGER job_work_plan_tasks_are_never_edited
BEFORE UPDATE ON job_work_plan_tasks
BEGIN
    SELECT RAISE(ABORT, 'a recorded task is never edited');
END;

CREATE TRIGGER job_work_plan_tasks_are_never_removed_from_a_job_that_exists
BEFORE DELETE ON job_work_plan_tasks
WHEN EXISTS (SELECT 1 FROM jobs WHERE jobs.job_id = OLD.job_id)
BEGIN
    SELECT RAISE(ABORT, 'a recorded task is never removed from a Job that exists');
END;
"#;

/// Version 103 — a Job's walk notes.
///
/// **`REFERENCES jobs(job_id)`**, so `forget_job`'s sweep takes them with the
/// Job and nothing registers them anywhere else. **Nothing to backfill**:
/// nothing could keep a walk note before this.
pub(crate) const V103: &str = r#"
CREATE TABLE job_walk_notes (
    note_id        TEXT PRIMARY KEY,
    job_id         TEXT NOT NULL REFERENCES jobs(job_id),
    said           TEXT NOT NULL CHECK (trim(said) <> ''),
    at             TEXT NOT NULL,
    element        TEXT NOT NULL,
    selector       TEXT NOT NULL,
    location       TEXT NOT NULL,
    served_run     TEXT,
    served_name    TEXT,
    served_address TEXT,
    frame          TEXT,
    sent_at        TEXT,
    CHECK ((served_run IS NULL) = (served_name IS NULL)
       AND (served_run IS NULL) = (served_address IS NULL))
) STRICT;

CREATE INDEX job_walk_notes_by_job ON job_walk_notes (job_id, at, note_id);

CREATE TRIGGER job_walk_notes_are_never_edited
BEFORE UPDATE ON job_walk_notes
WHEN OLD.sent_at IS NOT NULL
  OR NEW.sent_at IS NULL
  OR NEW.note_id IS NOT OLD.note_id
  OR NEW.job_id IS NOT OLD.job_id
  OR NEW.said IS NOT OLD.said
  OR NEW.at IS NOT OLD.at
  OR NEW.element IS NOT OLD.element
  OR NEW.selector IS NOT OLD.selector
  OR NEW.location IS NOT OLD.location
  OR NEW.served_run IS NOT OLD.served_run
  OR NEW.served_name IS NOT OLD.served_name
  OR NEW.served_address IS NOT OLD.served_address
  OR NEW.frame IS NOT OLD.frame
BEGIN
    SELECT RAISE(ABORT, 'a walk note is never edited, and is marked sent once');
END;
"#;

/// Version 95 — a plan's groups, a task that failed, and each group's runs.
///
/// One version for four tables, because each is half of one fact: the plan's
/// changes gain two moves and a failed task's reason, a recorded task the
/// planner's group number, `job_step_checks` the group and its run in its key
/// (so two groups gated on one run of a step keep both), and `job_group_runs`
/// is new. Rebuilt the way [`V90`](crate::work_plan::V90) is. Nothing is
/// backfilled: a plan recorded before this reads as one group.
pub(crate) const V95: &str = r#"
CREATE TABLE job_work_plan_changes_wide (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    seq        INTEGER NOT NULL CHECK (seq > 0),
    change     TEXT NOT NULL CHECK (change IN
                   ('recorded', 'added', 'updated', 'moved_task', 'moved_group')),
    by_step    TEXT,
    by_attempt INTEGER CHECK (by_attempt IS NULL OR by_attempt > 0),
    at         TEXT NOT NULL,
    approach   TEXT,
    task_id    INTEGER CHECK (task_id IS NULL OR task_id > 0),
    title      TEXT,
    detail     TEXT,
    after_task INTEGER CHECK (after_task IS NULL OR after_task > 0),
    state      TEXT CHECK (state IS NULL OR state IN
                   ('open', 'working', 'handed_in', 'done', 'failed', 'dropped')),
    reason     TEXT,
    scope      TEXT,
    expects    TEXT,
    shown      TEXT,
    grp        INTEGER CHECK (grp IS NULL OR grp > 0),
    after_grp  INTEGER CHECK (after_grp IS NULL OR after_grp > 0),
    PRIMARY KEY (job_id, seq),
    CHECK ((by_step IS NULL) = (by_attempt IS NULL)),
    CHECK (state IS NULL OR
           ((state IN ('dropped', 'failed')) = (reason IS NOT NULL AND trim(reason) <> '')))
) STRICT;

INSERT INTO job_work_plan_changes_wide (
    job_id, seq, change, by_step, by_attempt, at, approach, task_id, title, detail,
    after_task, state, reason, scope, expects, shown
) SELECT job_id, seq, change, by_step, by_attempt, at, approach, task_id, title, detail,
         after_task, state, reason, scope, expects, shown
  FROM job_work_plan_changes;

DROP TABLE job_work_plan_changes;
ALTER TABLE job_work_plan_changes_wide RENAME TO job_work_plan_changes;

CREATE TRIGGER job_work_plan_changes_are_never_edited
BEFORE UPDATE ON job_work_plan_changes
BEGIN
    SELECT RAISE(ABORT, 'a plan change is never edited');
END;

CREATE TRIGGER job_work_plan_changes_are_never_removed_from_a_job_that_exists
BEFORE DELETE ON job_work_plan_changes
WHEN EXISTS (SELECT 1 FROM jobs WHERE jobs.job_id = OLD.job_id)
BEGIN
    SELECT RAISE(ABORT, 'a plan change is never removed from a Job that exists');
END;

ALTER TABLE job_work_plan_tasks ADD COLUMN grp INTEGER CHECK (grp IS NULL OR grp > 0);

CREATE TABLE job_step_checks_wide (
    job_id              TEXT NOT NULL REFERENCES jobs(job_id),
    step_id             TEXT NOT NULL,
    attempt             INTEGER NOT NULL CHECK (attempt >= 1),
    grp                 INTEGER NOT NULL DEFAULT 0 CHECK (grp >= 0),
    group_run           INTEGER NOT NULL DEFAULT 0 CHECK (group_run >= 0),
    ordinal             INTEGER NOT NULL,
    name                TEXT NOT NULL,
    outcome             TEXT NOT NULL,
    expected            TEXT,
    produced            TEXT,
    ran_at              TEXT NOT NULL,
    output_path         TEXT,
    reused_from_dry_run TEXT,
    PRIMARY KEY (job_id, step_id, attempt, grp, ordinal),
    CHECK ((grp = 0) = (group_run = 0))
) STRICT;

INSERT INTO job_step_checks_wide (
    job_id, step_id, attempt, ordinal, name, outcome, expected, produced, ran_at,
    output_path, reused_from_dry_run
) SELECT job_id, step_id, attempt, ordinal, name, outcome, expected, produced, ran_at,
         output_path, reused_from_dry_run
  FROM job_step_checks;

DROP TABLE job_step_checks;
ALTER TABLE job_step_checks_wide RENAME TO job_step_checks;

CREATE TABLE job_group_runs (
    job_id       TEXT NOT NULL REFERENCES jobs(job_id),
    seq          INTEGER NOT NULL CHECK (seq > 0),
    grp          INTEGER NOT NULL CHECK (grp > 0),
    run          INTEGER NOT NULL CHECK (run > 0),
    kind         TEXT NOT NULL CHECK (kind IN ('started', 'ended')),
    step_id      TEXT,
    step_attempt INTEGER CHECK (step_attempt IS NULL OR step_attempt > 0),
    at           TEXT NOT NULL,
    verdict      TEXT CHECK (verdict IS NULL OR verdict IN ('passed', 'failed', 'not_reached')),
    trigger      TEXT,
    commit_sha   TEXT,
    event_seq    INTEGER CHECK (event_seq IS NULL OR event_seq > 0),
    PRIMARY KEY (job_id, seq),
    CHECK ((kind = 'started') = (step_id IS NOT NULL AND step_attempt IS NOT NULL)),
    CHECK ((kind = 'ended') = (verdict IS NOT NULL)),
    CHECK ((verdict = 'failed') = (trigger IS NOT NULL))
) STRICT;

CREATE TRIGGER job_group_runs_are_never_edited
BEFORE UPDATE ON job_group_runs
BEGIN
    SELECT RAISE(ABORT, 'a group run is never edited');
END;

CREATE TRIGGER job_group_runs_are_never_removed_from_a_job_that_exists
BEFORE DELETE ON job_group_runs
WHEN EXISTS (SELECT 1 FROM jobs WHERE jobs.job_id = OLD.job_id)
BEGIN
    SELECT RAISE(ABORT, 'a group run is never removed from a Job that exists');
END;
"#;

/// Version 37 — the frames a step's harness produced.
///
/// **`REFERENCES jobs(job_id)`, unlike `commit_checks`.**
/// A frame belongs to one step of one Job and to nothing else, so it belongs in
/// `forget_job`'s sweep — which reads the file's foreign keys rather than a
/// list, so this row needs no second registration anywhere.
///
/// `attempt` carries the same `CHECK (attempt >= 1)` `job_step_checks` does,
/// for that column's reason: zero is not an attempt and `Attempt` cannot hold
/// one, so the database refuses such a row rather than reading it as a first
/// run.
///
/// **No column for which side the frame came from.** That waited for the run
/// against `base`, which `#209` shipped and `#602` switched off again: one run
/// now, in the Job's own worktree, with the base run's code kept. [`V41`] added
/// the column backfilled rather than nullable, which is what V5's rule permits
/// here — every row this table held before it was taken on the branch — and it
/// stays, because every row written since is a branch frame too.
///
/// **`bytes` and no media type.** The file's own extension is in `name` and in
/// `path`, and a second field naming the same fact is a second place to keep it
/// true. What serves the bytes reads the extension off the path it opened.
pub(crate) const V37: &str = r#"
CREATE TABLE job_step_frames (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL CHECK (attempt >= 1),
    ordinal INTEGER NOT NULL,
    name    TEXT NOT NULL,
    path    TEXT NOT NULL,
    bytes   INTEGER NOT NULL,
    kept_at TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;
"#;

/// Version 41 — which checkout each frame is a photograph of.
///
/// **Backfilled to `branch` rather than left null**, which is the one shape V5's
/// rule allows and only because the fact is observed: until the migration above
/// this one, `fleet::showing` served and shot the Job's own worktree and
/// nothing else, so every existing row *is* a branch frame. A nullable column
/// would say Fleet cannot tell, which would be untrue of every row it applies
/// to.
///
/// **A trigger and not a `CHECK`.** SQLite's `ALTER TABLE ADD COLUMN` takes no
/// constraint, and the alternative — rebuilding the table to carry one — would
/// move every row to add a word. The trigger is what `jobs_are_never_given_a_
/// blank_branch` already does for the same reason, and it refuses the same
/// thing: a value nothing in `Side` spells, which would
/// reach a reader as a frame belonging to neither side of a pair.
pub(crate) const V41: &str = r#"
ALTER TABLE job_step_frames ADD COLUMN side TEXT NOT NULL DEFAULT 'branch';

CREATE TRIGGER frames_are_taken_on_one_of_two_sides
BEFORE INSERT ON job_step_frames
WHEN NEW.side NOT IN ('base', 'branch')
BEGIN
    SELECT RAISE(ABORT, 'a frame is taken on the base or on the branch');
END;
"#;

/// Version 42 — a digest of each frame's own bytes.
///
/// **What lets a surface fold a pair away without opening a file.** A spec that
/// photographs ten screens photographs ten screens of which the change touched
/// one, and the nine that did not are the noise `#209`'s panel exists to cut.
/// Comparing two digests on the record is how it is cut; comparing two files
/// would be Bridge reading a filesystem it has no access to.
///
/// **Empty rather than backfilled, the opposite of [`V41`]'s call and for the
/// same rule.** That migration could say what every existing row was, because
/// until it landed there was only one side to be on. This one cannot: nothing
/// recorded the bytes of a frame kept before it, and a digest invented here
/// would compare unequal to everything including itself re-read. Empty reads as
/// *this pair cannot be folded* — the safe direction.
pub(crate) const V42: &str = r#"
ALTER TABLE job_step_frames ADD COLUMN digest TEXT NOT NULL DEFAULT '';
"#;

/// Version 43 — the frames a person's press captured, as runs of their own.
///
/// **`press` is the key and `attempt` is not.** Pressing twice reruns the same
/// attempt's spec twice, and those are two sets. The number is the Job's own
/// count of presses, one-based; `pressed_at` is what a person tells them apart
/// by. `step_id` and `attempt` name the submission whose `shown_by` was rerun,
/// so a reader can put the set under the step it belongs to.
///
/// **No `side`.** A press runs in the worktree and nowhere else — there is no
/// base run to switch back on here — so every row is a branch frame by
/// construction, and a column would hold one value forever.
///
/// `REFERENCES jobs(job_id)` puts it in `forget_job`'s sweep, which reads the
/// file's foreign keys rather than a list.
pub(crate) const V43: &str = r#"
CREATE TABLE job_shown_again (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    press      INTEGER NOT NULL CHECK (press >= 1),
    step_id    TEXT NOT NULL,
    attempt    INTEGER NOT NULL CHECK (attempt >= 1),
    ordinal    INTEGER NOT NULL,
    name       TEXT NOT NULL,
    path       TEXT NOT NULL,
    bytes      INTEGER NOT NULL,
    digest     TEXT NOT NULL,
    pressed_at TEXT NOT NULL,
    PRIMARY KEY (job_id, press, ordinal)
) STRICT;
"#;

/// Version 54 — which spec a press ran.
///
/// **A person can pick one now**, so the step and the run no longer say it: a
/// Job whose Drones named three specs can be pressed three ways, and two sets
/// filed under one step would be told apart by nothing but the minute they ran.
///
/// **Empty rather than backfilled, for [`V42`](crate::showing::V42)'s rule.**
/// Every press written before this ran the last spec the record held, and that
/// is a spec today's record can no longer point at — a later run may have
/// renamed it. Empty reads as *this set does not say*.
pub(crate) const V54: &str = r#"
ALTER TABLE job_shown_again ADD COLUMN spec TEXT NOT NULL DEFAULT '';
"#;

/// Version 17 — what a person says went wrong, and the record they say it
/// about.
///
/// **No `REFERENCES jobs(job_id)`, and that is the whole design of the table.**
/// See this module's own note: the delete that forgets a Job is derived from
/// the file's catalog of tables pointing at `jobs`, so a report is out of its
/// reach by construction rather than by a rule somebody has to remember.
///
/// `record` is the Job's own evidence as it stood when the report was filed,
/// rendered once and stored. Not a join, because the rows it joins to are
/// exactly the rows `armada clean` takes away.
pub(crate) const V17: &str = r#"
CREATE TABLE reports (
    report_id  TEXT PRIMARY KEY NOT NULL,
    filed_at   TEXT NOT NULL,
    -- `human` today, and the only value written. A wire spelling, never read
    -- back into an enum here.
    origin     TEXT NOT NULL,
    -- What the person says the machine got wrong. A wire spelling, and what a
    -- count groups on.
    claim      TEXT NOT NULL,
    -- The id the Job had. Deliberately not a foreign key: the Job may be gone.
    job_id     TEXT NOT NULL,
    job_title  TEXT NOT NULL,
    -- How narrowly the claim is aimed: both null for the whole Job, `step_id`
    -- alone for a step no criterion was judged on, both set for one verdict.
    -- A criterion with no step is the one combination that is malformed.
    step_id    TEXT,
    criterion  TEXT,
    -- The person's own sentence. The finding.
    said       TEXT NOT NULL,
    -- The Job's record, rendered at filing time. The evidence.
    record     TEXT NOT NULL
) STRICT;
"#;

/// Version 68 — which Job is fixing which test broken on main.
///
/// **`reported_by` is not a reference.** The reporting Job can be forgotten
/// while the fix is still worked, and a second key to `jobs` would make that
/// forget fail at commit. `job_id` is the fix, which is what `forget_job` keys on.
pub(crate) const V68: &str = r#"
CREATE TABLE job_breakage_claims (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    repository  TEXT NOT NULL,
    check_name  TEXT NOT NULL,
    test        TEXT NOT NULL,
    failure     TEXT NOT NULL,
    reported_by TEXT NOT NULL,
    at          TEXT NOT NULL,
    PRIMARY KEY (repository, check_name, test)
) STRICT;
"#;

/// Version 69 — which Jobs failed on a claimed test, pointed at its fix. #1001.
///
/// **`fix` is not a reference**, for `reported_by`'s reason one table up.
/// `job_id` is the waiting Job, so forgetting it removes its pointers.
pub(crate) const V69: &str = r#"
CREATE TABLE job_fix_waiters (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    fix         TEXT NOT NULL,
    repository  TEXT NOT NULL,
    check_name  TEXT NOT NULL,
    test        TEXT NOT NULL,
    at          TEXT NOT NULL,
    PRIMARY KEY (job_id, repository, check_name, test)
) STRICT;
"#;

/// Version 94 — the test's files on a claim, and what a landed fix still
/// holds off a Job until its copy takes the fix. #1673.
///
/// **Two tables beside the claim rather than a column on it**, for
/// `crate::plan`'s reason: a list is rows. `job_id` is the fix on the first
/// and the held Job on the second, so each goes with the Job that owns it.
pub(crate) const V94: &str = r#"
CREATE TABLE job_breakage_claim_files (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    repository  TEXT NOT NULL,
    check_name  TEXT NOT NULL,
    test        TEXT NOT NULL,
    path        TEXT NOT NULL,
    PRIMARY KEY (repository, check_name, test, path)
) STRICT;

CREATE TABLE job_landed_holds (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    fix     TEXT NOT NULL,
    test    TEXT NOT NULL,
    path    TEXT NOT NULL,
    at      TEXT NOT NULL,
    PRIMARY KEY (job_id, fix, test, path)
) STRICT;
"#;

/// Version 109 — one row per run of a Check. No row points at `jobs`, so
/// forgetting a Job keeps its runs: the history outlives what it was about.
pub(crate) const V109: &str = r#"
CREATE TABLE check_runs (
    repository        TEXT NOT NULL,
    check_name        TEXT NOT NULL,
    job_id            TEXT,
    started_at        TEXT NOT NULL,
    outcome           TEXT NOT NULL CHECK (outcome IN ('passed', 'failed', 'timed_out')),
    narrowed          INTEGER NOT NULL CHECK (narrowed IN (0, 1)),
    narrowed_to       TEXT,
    queue_wait_ms     INTEGER NOT NULL CHECK (queue_wait_ms >= 0),
    took_ms           INTEGER NOT NULL CHECK (took_ms >= 0),
    tests_run         INTEGER,
    tests_failed      INTEGER,
    load_avg_at_start REAL,
    cores             INTEGER NOT NULL,
    width             INTEGER NOT NULL,
    places_held       INTEGER NOT NULL
) STRICT;
CREATE INDEX check_runs_by_check ON check_runs (repository, check_name, started_at);
"#;

/// Version 100 — the door a move came through, a Drone's note on what got in
/// its way, and a Job's retro.
///
/// **Every Job already ended is marked `skipped`.** A retro is owed by a Job
/// that ends from here on, and without the mark the first turn after this
/// migration would spend one model call on every Job the file has finished.
pub(crate) const V100: &str = r#"
CREATE TABLE job_event_via (
    seq    INTEGER PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(job_id),
    via    TEXT NOT NULL CHECK (via IN ('bridge', 'helm', 'door', 'http'))
) STRICT;

CREATE TABLE job_drone_notes (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    said    TEXT NOT NULL CHECK (trim(said) <> ''),
    at      TEXT NOT NULL
) STRICT;

CREATE TABLE job_retros (
    job_id TEXT PRIMARY KEY REFERENCES jobs(job_id),
    state  TEXT NOT NULL CHECK (state IN ('written', 'failed', 'skipped')),
    model  TEXT CHECK ((state = 'written') = (model IS NOT NULL)),
    why    TEXT CHECK ((state = 'written') = (why IS NULL)),
    at     TEXT NOT NULL
) STRICT;

CREATE TABLE job_retro_items (
    job_id   TEXT NOT NULL REFERENCES jobs(job_id),
    ordinal  INTEGER NOT NULL CHECK (ordinal >= 0),
    whose    TEXT NOT NULL CHECK (whose IN ('drone', 'owner', 'fleet')),
    said     TEXT NOT NULL CHECK (trim(said) <> ''),
    evidence TEXT NOT NULL,
    PRIMARY KEY (job_id, ordinal)
) STRICT;

INSERT INTO job_retros (job_id, state, why, at)
SELECT job_id, 'skipped', 'it ended before Fleet wrote retros',
       strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM jobs
WHERE status IN ('completed_success', 'completed_failed', 'killed', 'rejected', 'superseded');
"#;

/// Version 102 — where each retro item's fix lands. **Nothing is backfilled**:
/// a null is an item written before the model was asked, and guessing one
/// after the fact would put it under a place nobody chose.
pub(crate) const V102: &str = r#"
ALTER TABLE job_retro_items ADD COLUMN lands_in TEXT
    CHECK (lands_in IS NULL OR lands_in IN ('armada', 'kit', 'manifest'));
"#;

/// Version 104 — an item's headline, what happened and what to change, and
/// where it stands with the person.
///
/// **The three texts are null on every item kept before**, which has `said`
/// alone, and nothing is backfilled. **Every existing item starts `open`.**
pub(crate) const V105: &str = r#"
ALTER TABLE job_retro_items ADD COLUMN title TEXT CHECK (title IS NULL OR trim(title) <> '');
ALTER TABLE job_retro_items ADD COLUMN what TEXT CHECK (what IS NULL OR trim(what) <> '');
ALTER TABLE job_retro_items ADD COLUMN fix TEXT CHECK (fix IS NULL OR trim(fix) <> '');
ALTER TABLE job_retro_items ADD COLUMN state TEXT NOT NULL DEFAULT 'open'
    CHECK (state IN ('open', 'agreed', 'accepted', 'discarded'));
ALTER TABLE job_retro_items ADD COLUMN job_proposed TEXT;
"#;

/// Version 109 — the change to Kit an item carries, and whether pressing
/// Accept applied it.
///
/// **Null on every item kept before, and nothing is backfilled**: a change is
/// copied off a refusal the record showed, and an item written before the model
/// was asked has none to copy. `applied` is what makes a second press apply
/// nothing, and is only ever set by the same write that moves the item to
/// `accepted`.
pub(crate) const V110: &str = r#"
ALTER TABLE job_retro_items ADD COLUMN change_kind TEXT
    CHECK (change_kind IS NULL OR change_kind IN ('allow_command'));
ALTER TABLE job_retro_items ADD COLUMN change_command TEXT
    CHECK (change_command IS NULL OR trim(change_command) <> '');
ALTER TABLE job_retro_items ADD COLUMN applied INTEGER NOT NULL DEFAULT 0
    CHECK (applied IN (0, 1));
"#;

/// Version 49 — the model a person chose for a Job's later steps.
///
/// **Nullable, and no `DEFAULT`**: `NULL` is nobody having chosen, which is
/// every Job written before this and most written after it.
pub(crate) const V49: &str = r#"
ALTER TABLE jobs ADD COLUMN model_override TEXT;
"#;

/// Version 47 — the review composed at a Job's gate, in the four parts it is
/// made of.
///
/// Beside the change it makes, like [`V21`](crate::delivery::V21): `schema.rs`
/// is at the 900 lines the gate refuses at.
///
/// Four nullable columns and no backfill, written and cleared together: a Job
/// that has not yet reached a gate, and a Job that reached one before this
/// shipped, are the same absence.
pub(crate) const V47: &str = r#"
ALTER TABLE jobs ADD COLUMN review_why TEXT;
ALTER TABLE jobs ADD COLUMN review_outcome TEXT;
ALTER TABLE jobs ADD COLUMN review_evidence TEXT;
ALTER TABLE jobs ADD COLUMN review_risks TEXT;
"#;

/// Version 65 — the model a person chose for a Job's review step.
///
/// **Nullable, and no `DEFAULT`**: `NULL` is nobody having chosen, and the review step
/// runs on the Job's chosen model, or its own.
pub(crate) const V65: &str = r#"
ALTER TABLE jobs ADD COLUMN review_model_override TEXT;
"#;

/// Version 96 — the pull request's title and how many comments it carries.
///
/// **Null, and no backfill.** A null title is a pull request no read has named
/// since this column arrived; a null count is one the sweep never read open.
/// Neither is zero, and nothing here guesses one.
pub(crate) const V96: &str = r#"
ALTER TABLE jobs ADD COLUMN delivery_pr_title TEXT;
ALTER TABLE jobs ADD COLUMN delivery_pr_comments INTEGER;
"#;

/// Version 93 — the slot a Job leased.
///
/// **Null, and no backfill.** Every row before this column is a Job cut before
/// the pool, whose worktree is still at the path its handle derives, and null
/// is what says so. `Store::record_slot` is the one writer.
pub(crate) const V93: &str = r#"
ALTER TABLE jobs ADD COLUMN worktree_slot INTEGER;
"#;

/// Version 61 — the steps of each View a review carries.
///
/// **Nothing to backfill.** A review kept before this carried no View, and reads as one with none.
pub(crate) const V61: &str = r#"
CREATE TABLE job_step_review_view (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    attempt     INTEGER NOT NULL,
    owner       TEXT NOT NULL CHECK (owner IN ('area', 'finding')),
    owned_by    INTEGER NOT NULL,
    ordinal     INTEGER NOT NULL,
    file        TEXT NOT NULL,
    hunk        TEXT NOT NULL,
    summary     TEXT NOT NULL,
    tie_to_next TEXT,
    PRIMARY KEY (job_id, step_id, attempt, owner, owned_by, ordinal)
) STRICT;
"#;

/// Version 108 — the merge line, its turn, and the size a turn takes.
///
/// **One waiting entry per Job**, by a partial unique index: a press made again
/// after a restart finds the entry it left and keeps its place. **One turn per
/// repository**, by the primary key. Entries point at `jobs`, so
/// `forget_job` takes a forgotten Job's with it.
pub(crate) const V108: &str = r#"
CREATE TABLE merge_line_entries (
    entry_id     INTEGER PRIMARY KEY AUTOINCREMENT,
    repository   TEXT NOT NULL,
    job_id       TEXT NOT NULL REFERENCES jobs(job_id),
    nonce        TEXT NOT NULL,
    actor        TEXT NOT NULL,
    pull_request TEXT NOT NULL,
    queued_at    TEXT NOT NULL,
    state        TEXT NOT NULL DEFAULT 'waiting'
                 CHECK (state IN ('waiting', 'landed', 'refused', 'stopped')),
    kind         TEXT,
    said         TEXT,
    merge_commit TEXT,
    base         TEXT,
    blamed       TEXT CHECK (blamed IN ('branch', 'base')),
    ended_at     TEXT,
    finished     INTEGER NOT NULL DEFAULT 0 CHECK (finished IN (0, 1)),
    held_back    TEXT NOT NULL DEFAULT 'none'
                 CHECK (held_back IN ('clash_member', 'clash_main', 'kept_place_after_red',
                                      'joined_after_turn_began', 'none')),
    next_position INTEGER,
    CHECK ((state = 'waiting') = (ended_at IS NULL))
) STRICT;

CREATE UNIQUE INDEX merge_line_one_waiting_a_job
    ON merge_line_entries (job_id) WHERE state = 'waiting';
CREATE INDEX merge_line_by_repository
    ON merge_line_entries (repository, state, entry_id);

CREATE TABLE merge_line_turns (
    repository TEXT PRIMARY KEY,
    run        TEXT NOT NULL,
    pid        INTEGER NOT NULL,
    started    TEXT NOT NULL,
    taken_at   TEXT NOT NULL
) STRICT;

CREATE TABLE merge_line_sizes (
    repository TEXT PRIMARY KEY,
    size       INTEGER NOT NULL CHECK (size >= 1),
    reason     TEXT NOT NULL,
    at         TEXT NOT NULL
) STRICT;
"#;

/// Version 63 — a person's dismissals of a review's findings.
///
/// **Nothing to backfill.** Nothing could dismiss a finding before this.
pub(crate) const V63: &str = r#"
CREATE TABLE job_review_dismissals (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    finding TEXT NOT NULL,
    reason  TEXT NOT NULL,
    at      TEXT NOT NULL,
    PRIMARY KEY (job_id, finding)
) STRICT;
"#;

/// Version 29 — what the repository's Checks said about one commit.
///
/// Beside the change it makes, like [`V21`](crate::delivery::V21): `schema.rs`
/// is at the 900 lines the gate refuses at.
///
/// **No `REFERENCES jobs(job_id)` and no `job_id` column.** Both are the point:
/// this is the one per-Check table in the file that a Job does not own, and a
/// foreign key would put it in `tables_pointing_at_a_job` and so into
/// `forget_job`'s sweep.
///
/// `at_commit` and not `commit`, which is a reserved word in SQLite and would
/// have to be quoted at every use — a quoting rule is a thing somebody
/// eventually forgets.
pub(crate) const V29: &str = r#"
CREATE TABLE commit_checks (
    at_commit   TEXT NOT NULL,
    ordinal     INTEGER NOT NULL,
    base        TEXT NOT NULL,
    name        TEXT NOT NULL,
    outcome     TEXT NOT NULL,
    expected    TEXT,
    produced    TEXT,
    ran_at      TEXT NOT NULL,
    output_path TEXT,
    PRIMARY KEY (at_commit, ordinal)
) STRICT;
"#;
