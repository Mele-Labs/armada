//! A question's option drawn as a still, checked and kept when `waiting_for` carries it.
//! `docs/practices/protocol/how-he-works-with-jobs-and-a-question-offered-as-pictures.md`.
//!
//! **Fleet draws nothing: the asking Job rendered the mock in its own browser and saved the still.** Here
//! the file is checked (it exists, is a PNG or a JPEG of at most 8 MiB, is the size the picture says, and
//! every part's box lies inside it with a name of its own), copied into the Job's own kept files, and
//! `image` becomes the address `GET /sessions/shown` serves it at. A file the Job already showed is
//! served where it is. An `image` that is such an address is checked again and kept as it is, so a list
//! stated twice, or the receipt's options stated back, keeps its pictures.

use std::path::{Component, Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{OptionPicture, WaitingInput};
use sha2::{Digest, Sha256};
use store::{Holder, KeptAttachment};

use super::showing_file::{showed_as_file, MOST_BYTES};
use crate::daemon::Fleet;

const PICTURE_MISSING: &str = "fleet.picture_missing";
const PICTURE_TOO_LARGE: &str = "fleet.picture_too_large";
const PICTURE_NOT_AN_IMAGE: &str = "fleet.picture_not_an_image";
const PICTURE_SIZE: &str = "fleet.picture_size";
const PICTURE_PART_OUTSIDE: &str = "fleet.picture_part_outside";
const PICTURE_PART_REPEATED: &str = "fleet.picture_part_repeated";
const PICTURE_NOT_KEPT: &str = "fleet.picture_not_kept";

/// Where a picture is served, before its query: the route `get_shown_file` answers.
const SERVED_AT: &str = "/sessions/shown";

/// A kind of still Fleet keeps, which names the kept file's extension.
#[derive(Clone, Copy)]
enum Still {
    Png,
    Jpeg,
}

impl Still {
    fn extension(self) -> &'static str {
        match self {
            Still::Png => "png",
            Still::Jpeg => "jpg",
        }
    }
}

/// The kind and the pixel size of a PNG or a JPEG, read from its header. `None` for anything else.
fn still_size(bytes: &[u8]) -> Option<(Still, u32, u32)> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        // The signature, then the first chunk, which is IHDR: length, name, width, height.
        let header = bytes.get(8..24)?;
        let (name, size) = header[4..].split_at(4);
        if name != b"IHDR" {
            return None;
        }
        let word = |from: usize| u32::from_be_bytes([size[from], size[from + 1], size[from + 2], size[from + 3]]);
        return Some((Still::Png, word(0), word(4)));
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        return jpeg_size(bytes).map(|(width, height)| (Still::Jpeg, width, height));
    }
    None
}

/// A JPEG's size, from its first frame header: the segments are walked by their lengths.
fn jpeg_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    loop {
        if *bytes.get(at)? != 0xFF {
            return None;
        }
        while *bytes.get(at)? == 0xFF {
            at += 1;
        }
        let marker = *bytes.get(at)?;
        at += 1;
        match marker {
            0x01 | 0xD0..=0xD7 => continue,
            // The end, or the scan before any frame header.
            0xD9 | 0xDA => return None,
            _ => {}
        }
        let length = u16::from_be_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]) as usize;
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let height = u16::from_be_bytes([*bytes.get(at + 3)?, *bytes.get(at + 4)?]);
            let width = u16::from_be_bytes([*bytes.get(at + 5)?, *bytes.get(at + 6)?]);
            return Some((u32::from(width), u32::from(height)));
        }
        if length < 2 {
            return None;
        }
        at += length;
    }
}

/// `text` with every byte but a letter, a digit and `-_.~` as `%` and its two hex digits.
fn percent_encoded(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (byte as char).to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// What `percent_encoded` made, back. `None` for a `%` not followed by two hex digits, or bytes that are not text.
fn percent_decoded(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.bytes();
    while let Some(byte) = rest.next() {
        match byte {
            b'%' => {
                let high = (rest.next()? as char).to_digit(16)?;
                let low = (rest.next()? as char).to_digit(16)?;
                bytes.push((high * 16 + low) as u8);
            }
            other => bytes.push(other),
        }
    }
    String::from_utf8(bytes).ok()
}

/// The address a session's file is served at: `GET /sessions/shown` with the session and the path.
pub(crate) fn served_address(session: &str, path: &str) -> String {
    format!("{SERVED_AT}?session_id={}&path={}", percent_encoded(session), percent_encoded(path))
}

/// The path an address of this session's names, where `given` is one.
pub(crate) fn address_path(session: &str, given: &str) -> Option<String> {
    let query = given.strip_prefix(SERVED_AT)?.strip_prefix('?')?;
    let field = |name: &str| query.split('&').find_map(|pair| pair.strip_prefix(name)?.strip_prefix('=')).and_then(percent_decoded);
    (field("session_id")? == session).then(|| field("path")).flatten()
}

/// Whether `path` is under `folder` by its own components, with no way up out of it.
pub(crate) fn lies_under(folder: &Path, path: &Path) -> bool {
    path.starts_with(folder) && !path.components().any(|part| part == Component::ParentDir)
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Where a Job's pictures are kept, one file to each still, named for its bytes.
    pub(crate) fn pictures_of(&self, id: &str) -> PathBuf {
        Path::new(&self.host().attachments_dir).join("pictures").join(id)
    }

    /// Every picture an option of `items` carries, checked and kept, its `image` now the address it is served at.
    /// The first that fails is refused by name and nothing is kept for the rest.
    pub(super) async fn pictures_kept(&self, id: &str, items: &mut [WaitingInput]) -> Result<(), Refusal> {
        let drawn = |item: &WaitingInput| !item.approves && item.options.iter().any(|one| one.picture.is_some());
        if !items.iter().any(drawn) {
            return Ok(());
        }
        let (cwd, held) = {
            let store = self.store().lock().await;
            let session = store.session(id).map_err(|why| self.ledger_fault(why))?;
            let held = store.attachments_of(&Holder::session(id)).map_err(|why| self.ledger_fault(why))?;
            (session.map(|one| one.cwd).unwrap_or_default(), held)
        };
        for item in items.iter_mut().filter(|item| drawn(item)) {
            let item_id = item.id.trim().to_string();
            for option in &mut item.options {
                let whose = format!("option \"{}\" of \"{item_id}\"", option.label);
                if let Some(picture) = option.picture.as_mut() {
                    picture.image = self.picture_kept(id, &cwd, &held, &whose, picture).await?;
                }
            }
        }
        Ok(())
    }

    /// One picture checked and kept: the address its file is served at.
    async fn picture_kept(&self, id: &str, cwd: &str, held: &[KeptAttachment], whose: &str, picture: &OptionPicture) -> Result<String, Refusal> {
        let given = picture.image.trim();
        let named = address_path(id, given).unwrap_or_else(|| given.to_string());
        let joined = match Path::new(&named).is_absolute() {
            true => PathBuf::from(&named),
            false => Path::new(cwd).join(&named),
        };
        // A file this Fleet kept is named as it kept it, which a canonical path may spell differently.
        let kept_already = lies_under(&self.pictures_of(id), &joined);
        let resolved = match kept_already {
            true => joined,
            false => tokio::fs::canonicalize(&joined).await.map_err(|_| {
                self.hosted_refusal(
                    PICTURE_MISSING,
                    &format!("{whose} has a picture at {}, and there is no file there. Save the still with browser_screenshot and pass its path as picture.image.", joined.display()),
                )
            })?,
        };
        let at = resolved.to_string_lossy().into_owned();
        let missing = |why: &str| self.hosted_refusal(PICTURE_MISSING, &format!("{whose} has a picture at {at}, which {why}. Save the still with browser_screenshot and pass its path as picture.image."));
        let meta = tokio::fs::metadata(&resolved).await.map_err(|_| missing("cannot be read"))?;
        if !meta.is_file() {
            return Err(missing("is not a file"));
        }
        if meta.len() > MOST_BYTES {
            return Err(self.hosted_refusal(
                PICTURE_TOO_LARGE,
                &format!("{whose} has a picture of {} bytes, over the {MOST_BYTES} a picture may be. Render the mock at a narrower width, or save it as a JPEG.", meta.len()),
            ));
        }
        let bytes = tokio::fs::read(&resolved).await.map_err(|_| missing("cannot be read"))?;
        let Some((still, width, height)) = still_size(&bytes) else {
            return Err(self.hosted_refusal(
                PICTURE_NOT_AN_IMAGE,
                &format!("{whose} has a picture at {at} that is not a PNG or a JPEG. Save the still with browser_screenshot as a PNG."),
            ));
        };
        if (width, height) != (picture.width, picture.height) {
            return Err(self.hosted_refusal(
                PICTURE_SIZE,
                &format!("{whose} says its picture is {} by {} pixels, and the file is {width} by {height}. Pass the file's own width and height.", picture.width, picture.height),
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for part in &picture.parts {
            if !seen.insert(part.name.as_str()) {
                return Err(self.hosted_refusal(
                    PICTURE_PART_REPEATED,
                    &format!("{whose} has two parts named \"{}\". Give each part a name of its own.", part.name),
                ));
            }
            let reaches_x = u64::from(part.x) + u64::from(part.w) > u64::from(width);
            let reaches_y = u64::from(part.y) + u64::from(part.h) > u64::from(height);
            if reaches_x || reaches_y {
                return Err(self.hosted_refusal(
                    PICTURE_PART_OUTSIDE,
                    &format!(
                        "part \"{}\" of {whose} is the box x {}, y {}, {} wide and {} high, which runs past the {width} by {height} picture. Measure each box in the picture's pixels: scale getBoundingClientRect by the picture's width over the page's width.",
                        part.name, part.x, part.y, part.w, part.h,
                    ),
                ));
            }
        }
        if kept_already || held.iter().any(|one| showed_as_file(one, &at)) {
            return Ok(served_address(id, &at));
        }
        let folder = self.pictures_of(id);
        let name = Sha256::digest(&bytes).iter().take(8).map(|byte| format!("{byte:02x}")).collect::<String>();
        let target = folder.join(format!("{name}.{}", still.extension()));
        let written = target.clone();
        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            if written.exists() {
                return Ok(());
            }
            std::fs::create_dir_all(written.parent().unwrap_or(Path::new(".")))?;
            let staged = written.with_extension("writing");
            std::fs::write(&staged, &bytes)?;
            std::fs::rename(&staged, &written)
        })
        .await
        .map_err(std::io::Error::other)
        .and_then(|done| done)
        .map_err(|why| self.hosted_fault(PICTURE_NOT_KEPT, &format!("Fleet could not keep {whose}'s picture: {why}. Send the list again.")))?;
        Ok(served_address(id, &target.to_string_lossy()))
    }
}
