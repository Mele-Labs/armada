//! The BERT encoder of the pinned model, with low-rank adapters on its top layers' attention.
//!
//! **Written here rather than taken from `candle-transformers`**, whose encoder has no place to add an
//! adapter. Every operation is one candle can differentiate, so the trainer outside the repository
//! runs this same file and Fleet reproduces what it measured.

use candle_core::{Result, Tensor, D};
use candle_nn::{embedding, linear, Embedding, Linear, Module, VarBuilder};

pub(crate) const HIDDEN: usize = 384;
const HEADS: usize = 12;
const HEAD_WIDTH: usize = HIDDEN / HEADS;
const INTERMEDIATE: usize = 1536;
const EPS: f32 = 1e-12;
/// How much an adapter's update is scaled by: its alpha over its rank, 16 over 8.
const SCALE: f64 = 2.0;

struct Norm {
    weight: Tensor,
    bias: Tensor,
}

impl Norm {
    fn load(vb: VarBuilder) -> Result<Norm> {
        Ok(Norm { weight: vb.get(HIDDEN, "weight")?, bias: vb.get(HIDDEN, "bias")? })
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        candle_nn::ops::layer_norm_slow(x, &self.weight, &self.bias, EPS)
    }
}

/// The low-rank update to one layer's query and value: `W x + B (A x) · SCALE`. `A` is
/// `RANK × HIDDEN` and `B` is `HIDDEN × RANK`.
pub(crate) struct LayerAdapter {
    pub query_a: Tensor,
    pub query_b: Tensor,
    pub value_a: Tensor,
    pub value_b: Tensor,
}

fn adapted(x: &Tensor, base: &Tensor, a: &Tensor, b: &Tensor) -> Result<Tensor> {
    base + (x.broadcast_matmul(&a.t()?)?.broadcast_matmul(&b.t()?)? * SCALE)?
}

struct Layer {
    query: Linear,
    key: Linear,
    value: Linear,
    output: Linear,
    attention_norm: Norm,
    intermediate: Linear,
    out: Linear,
    out_norm: Norm,
}

impl Layer {
    fn load(vb: VarBuilder) -> Result<Layer> {
        let attention = vb.pp("attention");
        Ok(Layer {
            query: linear(HIDDEN, HIDDEN, attention.pp("self.query"))?,
            key: linear(HIDDEN, HIDDEN, attention.pp("self.key"))?,
            value: linear(HIDDEN, HIDDEN, attention.pp("self.value"))?,
            output: linear(HIDDEN, HIDDEN, attention.pp("output.dense"))?,
            attention_norm: Norm::load(attention.pp("output.LayerNorm"))?,
            intermediate: linear(HIDDEN, INTERMEDIATE, vb.pp("intermediate.dense"))?,
            out: linear(INTERMEDIATE, HIDDEN, vb.pp("output.dense"))?,
            out_norm: Norm::load(vb.pp("output.LayerNorm"))?,
        })
    }

    /// `mask` is additive, `batch × 1 × 1 × length`.
    fn forward(&self, x: &Tensor, mask: &Tensor, adapter: Option<&LayerAdapter>) -> Result<Tensor> {
        let (batch, length, _) = x.dims3()?;
        let (mut query, mut value) = (self.query.forward(x)?, self.value.forward(x)?);
        if let Some(adapter) = adapter {
            query = adapted(x, &query, &adapter.query_a, &adapter.query_b)?;
            value = adapted(x, &value, &adapter.value_a, &adapter.value_b)?;
        }
        let heads = |t: Tensor| t.reshape((batch, length, HEADS, HEAD_WIDTH))?.transpose(1, 2)?.contiguous();
        let (query, key, value) = (heads(query)?, heads(self.key.forward(x)?)?, heads(value)?);
        let scores = (query.matmul(&key.t()?)? / (HEAD_WIDTH as f64).sqrt())?.broadcast_add(mask)?;
        let weights = candle_nn::ops::softmax(&scores, D::Minus1)?;
        let context = weights.matmul(&value)?.transpose(1, 2)?.reshape((batch, length, HIDDEN))?;
        let attended = self.attention_norm.forward(&(self.output.forward(&context)? + x)?)?;
        let fed = self.out.forward(&self.intermediate.forward(&attended)?.gelu_erf()?)?;
        self.out_norm.forward(&(fed + attended)?)
    }
}

pub(crate) struct Bert {
    words: Embedding,
    positions: Embedding,
    types: Embedding,
    embedding_norm: Norm,
    layers: Vec<Layer>,
}

impl Bert {
    pub(crate) fn load(vb: VarBuilder, layers: usize) -> Result<Bert> {
        let embeddings = vb.pp("embeddings");
        Ok(Bert {
            words: embedding(30522, HIDDEN, embeddings.pp("word_embeddings"))?,
            positions: embedding(512, HIDDEN, embeddings.pp("position_embeddings"))?,
            types: embedding(2, HIDDEN, embeddings.pp("token_type_embeddings"))?,
            embedding_norm: Norm::load(embeddings.pp("LayerNorm"))?,
            layers: (0..layers).map(|at| Layer::load(vb.pp(format!("encoder.layer.{at}")))).collect::<Result<_>>()?,
        })
    }

    pub(crate) fn layers(&self) -> usize {
        self.layers.len()
    }

    /// `ids` and `mask` are `batch × length`; the mask is 1 for a piece and 0 for padding.
    /// Returns the hidden states after layer `until`, and the additive mask the layers read.
    pub(crate) fn lower(&self, ids: &Tensor, mask: &Tensor, until: usize) -> Result<(Tensor, Tensor)> {
        let length = ids.dim(1)?;
        let positions = Tensor::arange(0u32, length as u32, ids.device())?.unsqueeze(0)?;
        let x = self.words.forward(ids)?.broadcast_add(&self.positions.forward(&positions)?)?;
        let x = x.broadcast_add(&self.types.forward(&ids.zeros_like()?)?)?;
        let mut x = self.embedding_norm.forward(&x)?;
        let additive = ((mask.to_dtype(candle_core::DType::F32)?.ones_like()? - mask.to_dtype(candle_core::DType::F32)?)? * -10000.0)?
            .unsqueeze(1)?
            .unsqueeze(1)?;
        for layer in &self.layers[..until] {
            x = layer.forward(&x, &additive, None)?;
        }
        Ok((x, additive))
    }

    /// The layers from `from` on, each of the last `adapters.len()` with its adapter.
    pub(crate) fn upper(&self, x: Tensor, additive: &Tensor, from: usize, adapters: &[LayerAdapter]) -> Result<Tensor> {
        let first_adapted = self.layers.len() - adapters.len();
        let mut x = x;
        for (at, layer) in self.layers.iter().enumerate().skip(from) {
            let adapter = at.checked_sub(first_adapted).and_then(|index| adapters.get(index));
            x = layer.forward(&x, additive, adapter)?;
        }
        Ok(x)
    }
}
