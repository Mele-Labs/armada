# Protocol 14.12: a Note's frame, read back

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1352`. One route, additive: `GET /studios/:studio_id/frames/:node_id` answers the picture a Note
kept as the file itself, the way `get_frame` answers a step's. 14.11 wrote the frame and gave a
client no way to read it.

**One path segment where a step's frame takes two.** A Studio keeps one frame per node, under the
node's own id, so the node names the file — and the name is read off the node's `capture.frame`
before anything is opened, which is what keeps a caller's text off a path. A node that is not on
the Studio is `fleet.no_such_studio_node`, a node that kept no frame is
`fleet.studio_frame_not_kept`, and a file that will not open is `fleet.studio_frame_unreadable`.

`agent_access` is `Bridge only`, where `get_frame` is `Yes`: a Note's frame is a photograph of the
window a person was working in, not of a harness's own page. **The bytes reach Bridge's renderer
over the preload and become a `blob:`** — the CSP's `img-src 'self' blob:` is unchanged, and no
scheme was added to it.
