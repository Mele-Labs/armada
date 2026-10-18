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
