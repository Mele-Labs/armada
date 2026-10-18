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
