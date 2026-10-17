//! What an [`Adrift`] tells whoever reports it: the sentence and the cause
//! underneath.
//!
//! Split from [`adrift`](mod@crate::adrift) along the seam that module's own
//! header already names — it declares what could stop Fleet and why each
//! variant is shaped as it is, and this answers the two questions anything asks
//! of one afterwards; `refusing` asks a third on the way to a wire code.
//!
//! **Nothing here decides anything.** No arm maps a failure onto a recovery, a
//! retry or a status — those live where the failure is raised.
//!
//! Why the adapter halves are boxed and the domain halves are not is on
//! `adrift`'s own header.

use std::error::Error;
use std::fmt;

use crate::adrift::Adrift;

impl fmt::Display for Adrift {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Adrift::Reading(cause) => write!(out, "a read did not land: {cause}"),
            Adrift::Writing(cause) => write!(out, "a write did not land: {cause}"),
            Adrift::MainNotFixable { why, .. } => write!(out, "{why}"),
            Adrift::NoSuchLandLog {
                root,
                branch,
                check,
            } => write!(
                out,
                "no Check named `{check}` has a log for `{branch}` on the merge line in {root}. \
                 The branch's outcome names its turn and the Checks that turn has started, and \
                 nothing else is opened"
            ),
            Adrift::NoSuchManifest { named, held } => write!(
                out,
                "no Manifest is named `{named}`. This Fleet holds `{held}`, and `list_manifests` \
                 says which repository declares each"
            ),
            Adrift::NotServed { manifest } => write!(
                out,
                "Manifest `{manifest}` is declared by no repository this Fleet serves. Add that \
                 repository and it is served again"
            ),
            Adrift::ManifestUnreadable { path, cause } => write!(
                out,
                "{path} could not be read: {cause}. What Fleet is running with is unchanged — \
                 this is the file on disk, not the configuration in force"
            ),
            Adrift::ManifestMovedUnderTheEdit {
                path,
                on_disk: Some(_),
            } => write!(
                out,
                "{path} changed after this edit read it, so the save was refused rather than \
                 taking the incoming change with it. What is on disk now is on `on_disk`"
            ),
            Adrift::ManifestMovedUnderTheEdit { path, on_disk: None } => write!(
                out,
                "{path} is no longer there, so the save was refused rather than putting a file \
                 back that nobody asked for. Whatever removed it is what to look at first"
            ),
            Adrift::ManifestUnwritable { path, cause } => write!(
                out,
                "the corrected Manifest could not be written to {path}: {cause}. Nothing was \
                 changed, so what is on disk is what was there before"
            ),
        }
    }
}

impl Error for Adrift {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Adrift::Reading(cause) => Some(cause),
            Adrift::Writing(cause) => Some(cause),
            Adrift::ManifestUnreadable { cause, .. } | Adrift::ManifestUnwritable { cause, .. } => {
                Some(cause)
            }
            // The rest say what the record or the request holds rather than wrapping anything that
            // failed underneath it.
            Adrift::MainNotFixable { .. }
            | Adrift::NoSuchManifest { .. }
            | Adrift::NotServed { .. }
            | Adrift::ManifestMovedUnderTheEdit { .. }
            | Adrift::NoSuchLandLog { .. } => None,
        }
    }
}
