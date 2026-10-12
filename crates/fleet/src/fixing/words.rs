//! The words of where another Job's fix stands, as Armada ships them: each the
//! default of a `prompts.fix…` key in settings.json.

/// Where a fix stands, as it ships. `{test}` and `{files}` are filled by Fleet.
pub(crate) const FIX_FIXING: &str = "is fixing `{test}`, which your checks failed on";

/// Where a fix stands, as it ships. `{test}` and `{files}` are filled by Fleet.
pub(crate) const FIX_FIXING_HOLDS: &str =
    "is fixing `{test}`, which your checks failed on. Until that fix lands and \
                 reaches your copy, {files} are outside what this Job may change";

/// Where a fix stands, as it ships. `{test}` and `{files}` are filled by Fleet.
pub(crate) const FIX_LANDED: &str = "landed its fix for `{test}`";

/// Where a fix stands, as it ships. `{test}` and `{files}` are filled by Fleet.
pub(crate) const FIX_LANDED_HOLDS: &str =
    "landed its fix for `{test}`. It reaches your copy when your next part starts, \
                 and until then {files} stay outside what this Job may change";

/// Where a fix stands, as it ships. `{test}` and `{files}` are filled by Fleet.
pub(crate) const FIX_IN_YOUR_COPY: &str =
    "landed its fix for `{test}`, and that fix is already in your copy. {files} \
                 are yours to change again";

/// Where a fix stands, as it ships. `{test}` and `{files}` are filled by Fleet.
pub(crate) const FIX_GONE: &str =
    "ended without landing its fix for `{test}`, so nobody is fixing it now";

/// Where a fix stands, as it ships. `{test}` and `{files}` are filled by Fleet.
pub(crate) const FIX_GONE_HOLDS: &str =
    "ended without landing its fix for `{test}`, so nobody is fixing it now. \
                 {files} are yours to change again";

/// The block the files another Job's fix holds follow, as it ships.
pub(crate) const HELD_OFF: &str = "FILES ANOTHER JOB IS FIXING\n\n\
             A test this Job's checks failed on is another Job's to fix, and these files are \
             that fix's. Until it lands and reaches your copy they are outside what this Job may \
             change: a declaration naming one is refused, and so is an edit to one. Do the rest \
             of your part around them, and say in your evidence that the test still fails until \
             the fix lands.";

/// A held file's fix, still being made, as it ships.
pub(crate) const HELD_OFF_FIXING: &str = "is fixing `{test}`";

/// A held file's fix, landed, as it ships.
pub(crate) const HELD_OFF_LANDED: &str =
    "landed its fix for `{test}`, and your branch could not take it yet";
