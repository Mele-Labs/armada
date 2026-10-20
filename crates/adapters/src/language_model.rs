//! Fleet's local language model: a small sentence-embedding model, fetched once into the machine
//! directory and run on the CPU. `docs/spikes/reply-asks-classifier.md` says which and why.
//!
//! **Pinned, never configured.** Every head a reader trains is valid for these exact weights, so the
//! model and its revision are constants beside the digests that prove the files, not a setting a
//! person could change under a head that would then read nonsense.

mod encoding;
pub(crate) mod wordpiece;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Embedder, NotEmbedded};
use sha2::{Digest, Sha256};

/// The model, as the Hugging Face hub names it, and the revision every head was trained on.
const REPOSITORY: &str = "BAAI/bge-small-en-v1.5";
const REVISION: &str = "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a";
/// What [`Embedder::model`] answers, and what a head names as the model it was trained on.
pub const MODEL: &str = "BAAI/bge-small-en-v1.5@5c38ec7c405ec4b44b94cc5a9bb96e735b38267a";
const LAYERS: usize = 12;

/// Each file the model needs, and the SHA-256 it must have.
const FILES: &[(&str, &str)] = &[
    ("vocab.txt", "07eced375cec144d27c900241f3e339478dec958f92fddbc551f295c992038a3"),
    ("model.safetensors", "3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad"),
];

/// Long enough for 133 MB on a slow line; a fetch that runs past it is tried again later.
const FETCH_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Where the pinned model's files are kept under `models`: one folder per revision, so a later pin
/// never reads an earlier one's files.
pub fn folder(models: &Path) -> PathBuf {
    models.join(format!("bge-small-en-v1.5@{REVISION}"))
}

/// The pinned model, fetched into `models` first if it is not there whole. Blocking: a fetch is a
/// download, and a load reads 133 MB.
pub fn load(models: &Path) -> Result<Arc<dyn Embedder>, String> {
    let at = folder(models);
    std::fs::create_dir_all(&at).map_err(|why| format!("{}: {why}", at.display()))?;
    for (name, digest) in FILES {
        let path = at.join(name);
        if !path.exists() {
            fetch(name, digest, &path)?;
        }
    }
    let vocab = std::fs::read_to_string(at.join("vocab.txt")).map_err(|why| why.to_string())?;
    let weights = std::fs::read(at.join("model.safetensors")).map_err(|why| why.to_string())?;
    let encoder = encoding::Encoder::load(&vocab, weights, LAYERS)?;
    Ok(Arc::new(Local { encoder }))
}

/// One file at the pinned revision, checked against its digest and only then put where `load` reads,
/// so a fetch cut short leaves nothing that looks whole.
fn fetch(name: &str, digest: &str, to: &Path) -> Result<(), String> {
    let address = format!("https://huggingface.co/{REPOSITORY}/resolve/{REVISION}/{name}");
    let client = reqwest::blocking::Client::builder().timeout(FETCH_TIMEOUT).build().map_err(|why| why.to_string())?;
    let mut response = client.get(&address).send().and_then(|response| response.error_for_status()).map_err(|why| why.to_string())?;
    let mut bytes = Vec::new();
    response.read_to_end(&mut bytes).map_err(|why| why.to_string())?;
    let got: String = Sha256::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect();
    if got != digest {
        return Err(format!("{name} came back with digest {got}, not the pinned one"));
    }
    let partial = to.with_extension("partial");
    std::fs::write(&partial, &bytes).and_then(|()| std::fs::rename(&partial, to)).map_err(|why| why.to_string())
}

struct Local {
    encoder: encoding::Encoder,
}

impl Embedder for Local {
    fn model(&self) -> &str {
        MODEL
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
        self.encoder.embed(texts).map_err(NotEmbedded)
    }
}
