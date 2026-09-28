# What is the thing you ring off on a Studio board called?

**Decided 2026-09-28.**

A handoff named a new Studio container **Region**, having rejected Frame (a
captured Note's picture is its frame), Group, Cluster and Canvas as taken. Region
is taken the same way: a `Region` struct in `crates/verification/src/shown.rs:48`,
`citation.region` read in Bridge, and the word in `design-system.md` twice and
`studio.md` once. `studio.md:137` is a rule against exactly this — *"a second
name for something Armada already has is a vocabulary splitting, and the split is
invisible until two things that mean the same render differently"* — and
`studio.md:277` already uses the other sense in a sentence about Studios.

Offered Zone, or keeping Region and renaming the screen sense across built Rust
and TypeScript, or keeping Region and writing the disambiguation down:

**Chosen:** **Zone.** `nodes.zone`, and the menu reads *Add a zone*.

**Cost he took:** A third spatial word in a product that already has boards,
canvases and frames, bought for nothing except that it is unclaimed.

**What it says about him:** he would not pay a rename on shipped code for a
feature that is not built, and he would not accept a collision that a rule in
the same file forbids. The cheap unclaimed word won over both.

**Where it landed:** the workbook/region/attachments handoff, not yet filed.
Every mention of "Region" in that handoff means a Zone.
