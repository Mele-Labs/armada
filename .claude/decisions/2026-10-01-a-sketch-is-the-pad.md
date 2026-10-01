# Is a Studio's Sketch the same thing as the dispatch Sketch pad?

**Decided 2026-10-01.**

He asked why a Studio Sketch and the dispatch composer's "Sketch pad" were two different things. They were because the Studio Sketch was text only, `{ body: String }`, "the diagram in words", and it was text because of the *never pixels on a Studio* rule. He cut that rule on 28 Sep (`2026-09-28-a-studio-takes-pictures.md`), and nothing had followed it through to the Sketch.

**Chosen: "Make them one."** In his words, accepting the cost: *"A Studio Sketch opens as the pad (boxes, lines, pen, pasted pictures), saved on the Studio. Dispatching from it attaches that drawing. Cost: Fleet has to store a drawing instead of a line of text, and Studio Sketches already made as text need converting or showing as a single box."*

- **Fleet keeps the drawing**: boxes, joins, lines drawn by hand and pasted pictures, the pad's own four parts. A pasted picture's bytes are a file Fleet names and keeps beside the Studio's records, the way a Picture's are, under the same 4 MiB cap. Deleting the Sketch deletes them.
- **Sketches already made as text are converted**: each became one box holding its words (store V85), so none was lost and every Sketch opens on the pad.
- **On the board a Sketch draws its drawing**, read-only and at node size. Placing one from the rail, or opening one, opens the pad over the board. Closing the pad saves it.
- **Dispatching from a Sketch opens the composer with that drawing on its pad**, made from that node. What reaches Fleet with the request is unchanged until #1545 stages the PNG.
- **No title.** The pad's *About this sketch* line is about a request, so a Studio Sketch keeps none, and its card is named by its kind the way a Picture's is.

**Saved on close, not as it is drawn.** One write when the pad closes is the whole drawing, so a Studio is never left holding half a stroke, and a refusal is said once, on the pad, where the person still is. The cost is that closing the window with the pad open loses what was drawn since it opened.

**Protocol 20.0, a major.** The read shape lost `body`, and the version file's own table calls a removed field major. A derived `body` kept beside the drawing would have bought a minor for a Bridge that does not exist.

**Where it landed:** `studios/one-sketch`.
