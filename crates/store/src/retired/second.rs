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
