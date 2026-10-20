//! Text to one unit vector each, on the CPU: WordPiece, the BERT encoder, the start mark's vector.
//!
//! Nothing here fetches or knows where the files came from, so a measurement outside the
//! repository can include this file by path and embed exactly what Fleet embeds.

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};

use super::wordpiece::WordPiece;

/// The longest input in pieces, the start and end marks included: the pinned model's own limit.
const LONGEST: usize = 512;

pub(crate) struct Encoder {
    model: BertModel,
    pieces: WordPiece,
}

impl Encoder {
    /// From the bytes of `vocab.txt` and `model.safetensors`. Read into memory rather than mapped,
    /// since mapping a file is `unsafe` and this workspace forbids it; the buffer goes once the
    /// weights are copied out of it.
    pub(crate) fn load(vocab: &str, weights: Vec<u8>, layers: usize) -> Result<Encoder, String> {
        let pieces = WordPiece::from_vocab(vocab).ok_or("the vocabulary has no [UNK], [CLS] or [SEP]")?;
        let config = Config { hidden_size: 384, num_hidden_layers: layers, intermediate_size: 1536, ..Config::default() };
        let weights = VarBuilder::from_buffered_safetensors(weights, DType::F32, &Device::Cpu).map_err(|why| why.to_string())?;
        let model = BertModel::load(weights, &config).map_err(|why| why.to_string())?;
        Ok(Encoder { model, pieces })
    }

    /// One unit-length vector per text, in order. A text longer than [`LONGEST`] is read from its start.
    pub(crate) fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<Vec<u32>> = texts
            .iter()
            .map(|text| {
                let mut pieces = self.pieces.pieces(text);
                pieces.truncate(LONGEST - 2);
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
        let failed = |why: candle_core::Error| why.to_string();
        let input = Tensor::from_vec(flat, shape, &Device::Cpu).map_err(failed)?;
        let mask = Tensor::from_vec(mask, shape, &Device::Cpu).map_err(failed)?;
        let types = input.zeros_like().map_err(failed)?;
        let hidden = self.model.forward(&input, &types, Some(&mask)).map_err(failed)?;
        // The start mark's vector stands for the text, as the pinned model was trained, scaled to unit length.
        let first = hidden.narrow(1, 0, 1).and_then(|first| first.squeeze(1)).map_err(failed)?;
        let norm = first.sqr().and_then(|squared| squared.sum_keepdim(1)).and_then(|sum| sum.sqrt()).map_err(failed)?;
        first.broadcast_div(&norm).and_then(|unit| unit.to_vec2::<f32>()).map_err(failed)
    }
}
