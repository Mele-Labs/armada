# How does a person get Armada, and stay current on it?

**Decided 2026-10-25.** He asked for a quick install from GitHub, a release on every merge to `main`, a Stable and a Preview channel, contributing back with no setup, and updates that leave Jobs running. Asked the open parts one at a time:

| Asked | He chose | The cost he took |
|---|---|---|
| How a new person installs, against Ship's *signed, without a terminal* | One line in Terminal now, signed later | A Terminal step until there is a Developer ID. `armada-67` stays open for the signed download |
| What contributing back means | A Job can be pointed at Armada itself, with pull requests upstream, on by default and turned off in Settings | Every install clones the repository |
| Preview on a Mac without Rust and Node | Offer to install them in one press | A few minutes the first time Preview is chosen |
| Which Macs | Apple Silicon only | An Intel Mac cannot install it |
| A Bridge update that needs a relaunch | Ask first: *Relaunch to finish* | Fleet is on the new build while the window is on the old one until he presses it |

**Not decided by him:** the version is `v0.<minor>.<commits on main>`. He said he did not mind the scheme so long as releases are tagged.

**Where it landed:** the Ship issues `armada-p87.1` to `armada-p87.5`, `.github/workflows/release.yml`, `scripts/install.sh`, and `docs/practices/ci.md`, *Releases*.
