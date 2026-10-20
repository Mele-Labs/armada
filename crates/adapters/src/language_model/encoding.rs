//! Text to vectors and to readings, on the CPU: WordPiece, the BERT encoder, the start mark's vector.
//!
//! Nothing here fetches or knows where the files came from, so the trainer outside the repository
//! can include this file by path and read exactly what Fleet reads.

use std::collections::HashMap;

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;

use super::bert::{Bert, LayerAdapter};
use super::wordpiece::WordPiece;

/// The longest input in pieces, the start and end marks included: the pinned model's own limit.
const LONGEST: usize = 512;

/// Which end of a text is kept when it is longer than a reading reads.
#[derive(Clone, Copy)]
pub(crate) enum Keep {
    Start,
    End,
}

/// The fine-tuned part, loaded on top of the pinned weights: an adapter on each of the top layers,
/// and a head per reading over the start mark's vector.
pub(crate) struct Tuned {
    pub adapters: Vec<LayerAdapter>,
    pub heads: HashMap<String, (Tensor, Tensor)>,
}

impl Tuned {
    /// From a safetensors file holding `layer.{n}.{query,value}_{a,b}` and `head.{name}.{weight,bias}`.
    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Tuned, String> {
        let mut tensors = candle_core::safetensors::load_buffer(bytes, &Device::Cpu).map_err(|why| why.to_string())?;
        let mut adapters = Vec::new();
        loop {
            let at = adapters.len();
            let Some(query_a) = tensors.remove(&format!("layer.{at}.query_a")) else {
                break;
            };
            let mut take = |name: String| tensors.remove(&name).ok_or(format!("the adapter has no {name}"));
            adapters.push(LayerAdapter {
                query_a,
                query_b: take(format!("layer.{at}.query_b"))?,
                value_a: take(format!("layer.{at}.value_a"))?,
                value_b: take(format!("layer.{at}.value_b"))?,
            });
        }
        let names: Vec<String> = tensors.keys().filter_map(|key| key.strip_prefix("head.")?.strip_suffix(".weight").map(str::to_string)).collect();
        let mut heads = HashMap::new();
        for name in names {
            let weight = tensors.remove(&format!("head.{name}.weight")).ok_or("a head without its weight")?;
            let bias = tensors.remove(&format!("head.{name}.bias")).ok_or(format!("the head {name} has no bias"))?;
            heads.insert(name, (weight, bias));
        }
        Ok(Tuned { adapters, heads })
    }
}

pub(crate) struct Encoder {
    pub(crate) bert: Bert,
    pieces: WordPiece,
}

impl Encoder {
    /// From the bytes of `vocab.txt` and `model.safetensors`. Read into memory rather than mapped,
    /// since mapping a file is `unsafe` and this workspace forbids it; the buffer goes once the
    /// weights are copied out of it.
    pub(crate) fn load(vocab: &str, weights: Vec<u8>, layers: usize) -> Result<Encoder, String> {
        let pieces = WordPiece::from_vocab(vocab).ok_or("the vocabulary has no [UNK], [CLS] or [SEP]")?;
        let weights = VarBuilder::from_buffered_safetensors(weights, DType::F32, &Device::Cpu).map_err(|why| why.to_string())?;
        let bert = Bert::load(weights, layers).map_err(|why| why.to_string())?;
        Ok(Encoder { bert, pieces })
    }

    /// The pieces of each text between the start and end marks, cut to `longest` from the end `keep`
    /// names, padded to the longest of them: `(ids, mask)`, each `texts × length`.
    pub(crate) fn batch(&self, texts: &[&str], longest: usize, keep: Keep) -> candle_core::Result<(Tensor, Tensor)> {
        let ids: Vec<Vec<u32>> = texts
            .iter()
            .map(|text| {
                let mut pieces = self.pieces.pieces(text);
                let room = longest - 2;
                if pieces.len() > room {
                    pieces = match keep {
                        Keep::Start => pieces[..room].to_vec(),
                        Keep::End => pieces[pieces.len() - room..].to_vec(),
                    };
                }
                let mut ids = vec![self.pieces.start];
                ids.extend(pieces);
                ids.push(self.pieces.end);
                ids
            })
            .collect();
        let width = ids.iter().map(Vec::len).max().unwrap_or(2);
        let (mut flat, mut mask) = (Vec::with_capacity(width * ids.len()), Vec::with_capacity(width * ids.len()));
        for one in &ids {
            flat.extend(one.iter().copied().chain(std::iter::repeat(0).take(width - one.len())));
            mask.extend((0..width).map(|at| u32::from(at < one.len())));
        }
        let shape = (ids.len(), width);
        Ok((Tensor::from_vec(flat, shape, &Device::Cpu)?, Tensor::from_vec(mask, shape, &Device::Cpu)?))
    }

    /// One unit-length vector per text, in order, from the pinned weights alone. A text longer than
    /// [`LONGEST`] is read from its start.
    pub(crate) fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let failed = |why: candle_core::Error| why.to_string();
        let (ids, mask) = self.batch(texts, LONGEST, Keep::Start).map_err(failed)?;
        let (hidden, _) = self.bert.lower(&ids, &mask, self.bert.layers()).map_err(failed)?;
        let first = first_vector(&hidden).map_err(failed)?;
        let norm = first.sqr().and_then(|squared| squared.sum_keepdim(1)).and_then(|sum| sum.sqrt()).map_err(failed)?;
        first.broadcast_div(&norm).and_then(|unit| unit.to_vec2::<f32>()).map_err(failed)
    }

    /// The probability `head` gives each text, through the tuned layers.
    pub(crate) fn read(&self, tuned: &Tuned, head: &str, texts: &[&str], longest: usize, keep: Keep) -> Result<Vec<f32>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let (weight, bias) = tuned.heads.get(head).ok_or(format!("the adapter has no head {head}"))?;
        let failed = |why: candle_core::Error| why.to_string();
        let (ids, mask) = self.batch(texts, longest, keep).map_err(failed)?;
        let from = self.bert.layers() - tuned.adapters.len();
        let (hidden, additive) = self.bert.lower(&ids, &mask, from).map_err(failed)?;
        let hidden = self.bert.upper(hidden, &additive, from, &tuned.adapters).map_err(failed)?;
        let logits = logit(&first_vector(&hidden).map_err(failed)?, weight, bias).map_err(failed)?;
        candle_nn::ops::sigmoid(&logits).and_then(|p| p.to_vec1::<f32>()).map_err(failed)
    }
}

/// The start mark's vector of each text: `texts × HIDDEN`.
pub(crate) fn first_vector(hidden: &Tensor) -> candle_core::Result<Tensor> {
    hidden.narrow(1, 0, 1)?.squeeze(1)
}

/// A head's score before the sigmoid: `vectors · weight + bias`, one per text.
pub(crate) fn logit(vectors: &Tensor, weight: &Tensor, bias: &Tensor) -> candle_core::Result<Tensor> {
    vectors.matmul(&weight.unsqueeze(1)?)?.squeeze(1)?.broadcast_add(bias)
}
