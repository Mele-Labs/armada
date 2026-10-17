//! Every log file a Job has, what each weighs, and whether a writer holds it.
//!
//! **"Being written" is an open file, never a recent mtime**, which cannot tell
//! a Drone paused for ten minutes from one that died. It is `lsof`'s answer: a
//! process holding the file open with write access.
//!
//! **Fleet is asked as well as the Job's tree, because Fleet is the writer.** A
//! Drone never opens its transcript: `crate::transcript::Recording` appends its
//! rows from Fleet's process, holding the transcript and the Job's log open
//! until the Drone's stdout closes. Asking only the tree would read `false` on
//! every live Drone. Fleet's own reads are read-only and are not writers, and a
//! kept brief is written whole and closed (`crate::asked`).
//!
//! Bounded and shelled out, [`crate::headroom`]'s precedent.

