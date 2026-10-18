# Protocol 16.0: one delete on a Studio's nodes, one node or eighteen

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1411`. `remove_studio_node` is gone — the route, the DTO, the `Studios` method, the store write
and the capability Bridge reached it by. `remove_studio_nodes` replaces it, `POST
/studios/:studio_id/remove_nodes` carrying `RemoveStudioNodes { node_ids }`, `Bridge only`.
**Removing an operation is a major bump by this document's own table**, as at 14.0, so the major
moves and the minor resets.

**This was written as 15.0 and is 16.0**, because 15.0 landed underneath it while the branch was
open. Both files read `major = 15, minor = 0`, so git merged them clean and the collision was
invisible — two changes claiming one version, which is the failure this file's own numbering
exists against. A version taken on a branch is a claim about the base it was taken from, and it is
re-read at every merge of `main`.

**Two routes for one act is two paths that drift, and these had.** The single-node write left a
captured Note's frame on disk; the selection write deletes it. Retiring the first closes that leak
rather than writing it down. A Bridge built before this presses a route that answers
`fleet.route_not_found`, which the major is what stops it reaching.

**All of them or none is the store's transaction, not the caller's care.** Every name is checked
before anything is deleted, so a selection carrying one name the Studio does not hold refuses with
every node still on it — including the Studio's own `touched_at`, which rolls back with the rest.
A name given twice removes that node once. A call naming no node at all is refused as
`fleet.studio_no_nodes_named` rather than taken as a write that does nothing.

**The frames go after the write, never before it.** A Note's picture is a file beside the records
and the record is what names it, so the file is deleted once the row that named it is gone. A
refused write leaves every picture where the Note that keeps it can still draw it.
