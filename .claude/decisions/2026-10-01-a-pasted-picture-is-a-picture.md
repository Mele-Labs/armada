# What is a screenshot pasted onto a Studio?

**Decided 2026-10-01.**

The paste was built to land a screenshot as a Note carrying the picture, through the door capture uses. It could not: the Note had no words, Fleet refuses a Note whose `said` is blank, and capture records an element the person pointed at, which a paste has none of. Asked whether Fleet should take a wordless Note, he answered: *"Why not have a new type of node instead of trying to reuse note?"*

**Chosen:** a fifth kind a person adds by hand, **Picture**. It is the "picture a person adds by hand, stored the way a Note's frame already is" that `2026-09-28-a-studio-takes-pictures.md` chose and nobody had built.

- It holds the image and nothing else. No words are asked for, so the card draws the picture with no title, and it is named by its kind.
- Its bytes are kept the way a Note's frame is: a file beside the Studio's records, with the same 4 MiB cap and the same read (`get_studio_frame`). The board draws it, Open frame shows it full size, and deleting the node deletes the file.
- A screenshot pasted onto the board lands at once as a Picture under the pointer. An image file copied in Finder is still a File (`2026-10-01-a-paste-lands-at-once.md`).
- One over 4 MiB is refused by Fleet, and the board says why, as it says every refusal.
- **Bytes in, never a path.** The renderer hands main the bytes. Main stages them, and only main ever names a staged file to Fleet.

**Cost he took:** a second new kind in the same change. It is folded into the File's migration, V84, and its protocol bump, 18.5, since neither had merged.

**Where it landed:** `studios/paste`, PR 1729.
