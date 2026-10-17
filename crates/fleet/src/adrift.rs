//! Why Fleet could not carry out what it was asked.
//!
//! # The adapter failures are boxed and the domain failures are not
//!
//! `docs/contracts/error-contract.md` puts a typed leaf enum beside the code
//! that raised it and a real cause chain in the wrapper. The typed halves below
//! are the ones this crate can name: the store's, the Manifest file's. The
//! adapter halves arrive through a generic seam whose error type is the
//! implementation's — a `V::Error` cannot be a variant here without making this
//! enum generic over three parameters, which would put the adapter's vocabulary
//! into every signature in the crate. They are boxed, and reachable through
//! [`Error::source`].
//!
//! Every variant still says *what Fleet was doing*, which is the thing a caller
//! matches on, and the box carries only the leaf underneath it.

use std::io;

use store::{ReadError, WriteError};

/// Something Fleet was asked for that it could not do, and what stopped it.
#[derive(Debug)]
pub enum Adrift {
    /// A read did not land.
    Reading(ReadError),
    /// A write did not land.
    Writing(WriteError),
    /// Main's red could not be handed to a Session. `crate::main_fix`.
    MainNotFixable {
        root: String,
        why: crate::main_fix::MainNotFixable,
    },
    /// A proposal named a Manifest this Fleet does not hold.
    NoSuchManifest { named: String, held: String },
    /// A Manifest no repository this Fleet serves declares.
    NotServed { manifest: String },
    /// `armada.yml` would not open for the file view that edits it.
    ///
    /// **Not a refused parse.** That one is `config::LoadError` and reaches a
    /// client as `ManifestReading`, with the previous values still in force.
    /// This is the file itself unreadable, which is Fleet's own ground failing.
    ManifestUnreadable { path: String, cause: io::Error },
    /// A corrected `armada.yml` would not go to disk. **Nothing about the text
    /// can raise this** — a save is never refused for what it says.
    ManifestUnwritable { path: String, cause: io::Error },
    /// `armada.yml` changed after the edit that was about to be saved read it.
    ///
    /// **Not a failure, and that is why it is not the variant above.** Nothing
    /// broke; what a person does next is reconcile rather than retry. What is
    /// on disk now travels with it so a surface can show both, and is absent
    /// where the file is no longer there at all.
    ManifestMovedUnderTheEdit {
        path: String,
        on_disk: Option<String>,
    },
    /// A merge line Check's log was asked for that Fleet will not open.
    ///
    /// **One answer for every reason**: a root this Fleet does not serve, a
    /// branch with no outcome, a Check its turn has not started, or a name
    /// that would leave the turn's directory. The outcome file is the only
    /// thing that resolves the three names to a file.
    NoSuchLandLog {
        root: String,
        branch: String,
        check: String,
    },
}
