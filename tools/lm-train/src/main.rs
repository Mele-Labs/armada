// Trainer outside the workspace: LoRA on the pinned encoder's top layers plus heads, with the committed encoder code.
#![allow(dead_code)]
#[path = "../../../crates/adapters/src"]
mod adapters_src {
    pub mod language_model;
}
#[path = "../../../crates/fleet/src/session_host/asks.rs"]
mod asks;
#[path = "../../../crates/fleet/src/session_host/clarity.rs"]
mod clarity;
#[path = "../../../crates/fleet/src/session_host/extracting.rs"]
mod extracting;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use adapter_traits::TextReading;
use adapters_src::language_model::bert::{LayerAdapter, HIDDEN};
use adapters_src::language_model::encoding::{first_vector, logit, Encoder, Keep, Tuned};
use adapters_src::language_model::shape;
use candle_core::{DType, Device, Tensor, Var};
use candle_nn::optim::{AdamW, Optimizer, ParamsAdamW};

const RANK: usize = 8;
const LAYERS: usize = 12;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn uniform(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn normal(&mut self) -> f32 {
        let (u, v) = (self.uniform().max(1e-7), self.uniform());
        (-2.0 * u.ln()).sqrt() * (2.0 * std::f32::consts::PI * v).cos()
    }
    fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            xs.swap(i, j);
        }
    }
}

#[derive(Clone)]
struct Example {
    id: String,
    text: String,
    y: bool,
    weight: f32,
    new: bool,
    kind: String,
}

struct Cached {
    hidden: Tensor,
    length: usize,
}

fn machine(set: char, id: u32) -> bool {
    match set {
        'a' => [1, 9, 76, 81, 85, 90, 92, 148, 149, 162, 165, 214, 219, 255, 259, 260, 261, 286, 289, 299].contains(&id),
        'b' => [1056, 1060].contains(&id),
        _ => id == 2078,
    }
}

fn asks_examples(training: &Path, new_weight: f32) -> Vec<Example> {
    let mut out = Vec::new();
    for (set, labels, dir) in [('a', "labels_a.txt", "txt"), ('b', "labels_b.txt", "txt2"), ('c', "labels_c.txt", "txt3")] {
        for line in std::fs::read_to_string(training.join("reply-asks").join(labels)).unwrap().lines() {
            let f: Vec<&str> = line.split_whitespace().collect();
            let id: u32 = f[0].parse().unwrap();
            if machine(set, id) {
                continue;
            }
            let reply = std::fs::read_to_string(training.join("reply-asks").join(dir).join(format!("{id}.txt"))).unwrap();
            out.push(Example { id: format!("{set}{id}"), text: asks::closing_text(&reply), y: f[1] == "a", weight: 1.0, new: false, kind: String::new() });
        }
    }
    for line in std::fs::read_to_string(training.join("lm-train-data/new.labels")).unwrap().lines() {
        let f: Vec<&str> = line.split(' ').collect();
        let reply = std::fs::read_to_string(training.join("lm-train-data/new").join(format!("{}.txt", f[0]))).unwrap();
        out.push(Example { id: format!("n{}", f[0]), text: asks::closing_text(&reply), y: f[1] == "1", weight: new_weight, new: true, kind: String::new() });
    }
    out
}

fn tell_examples(training: &Path) -> Vec<Example> {
    std::fs::read_to_string(training.join("lm-train-data/tell.labels"))
        .unwrap()
        .lines()
        .map(|line| {
            let f: Vec<&str> = line.split(' ').collect();
            let text = std::fs::read_to_string(training.join("lm-train-data/tell").join(format!("{}.txt", f[0]))).unwrap();
            Example { id: f[0].to_string(), text, y: f[1] == "1", weight: 1.0, new: true, kind: f[2].to_string() }
        })
        .collect()
}

/// The frozen layers' output for each example, once.
fn cache(enc: &Encoder, examples: &[Example], reading: TextReading, from: usize) -> Vec<Cached> {
    let (_, longest, keep) = shape(reading);
    examples
        .iter()
        .map(|e| {
            let (ids, mask) = enc.batch(&[e.text.as_str()], longest, keep).unwrap();
            let (hidden, _) = enc.bert.lower(&ids, &mask, from).unwrap();
            let length = hidden.dim(1).unwrap();
            Cached { hidden, length }
        })
        .collect()
}

fn batch(cached: &[Cached], idx: &[usize]) -> (Tensor, Tensor) {
    let width = idx.iter().map(|&i| cached[i].length).max().unwrap();
    let hidden: Vec<Tensor> = idx.iter().map(|&i| cached[i].hidden.pad_with_zeros(1, 0, width - cached[i].length).unwrap()).collect();
    let mut additive = Vec::with_capacity(idx.len() * width);
    for &i in idx {
        additive.extend((0..width).map(|at| if at < cached[i].length { 0.0f32 } else { -10000.0 }));
    }
    (Tensor::cat(&hidden, 0).unwrap(), Tensor::from_vec(additive, (idx.len(), 1, 1, width), &Device::Cpu).unwrap())
}

struct Params {
    adapters: Vec<[Var; 4]>,
    weight: Var,
    bias: Var,
}

impl Params {
    fn new(layers: usize, rng: &mut Rng) -> Params {
        let mut randn = |shape: (usize, usize), sd: f32| {
            let v: Vec<f32> = (0..shape.0 * shape.1).map(|_| rng.normal() * sd).collect();
            Var::from_tensor(&Tensor::from_vec(v, shape, &Device::Cpu).unwrap()).unwrap()
        };
        let zeros = |shape: (usize, usize)| Var::zeros(shape, DType::F32, &Device::Cpu).unwrap();
        let adapters = (0..layers)
            .map(|_| [randn((RANK, HIDDEN), 0.02), zeros((HIDDEN, RANK)), randn((RANK, HIDDEN), 0.02), zeros((HIDDEN, RANK))])
            .collect();
        let weight = randn((1, HIDDEN), 0.02);
        let weight = Var::from_tensor(&weight.as_tensor().squeeze(0).unwrap()).unwrap();
        Params { adapters, weight, bias: Var::zeros(1, DType::F32, &Device::Cpu).unwrap() }
    }

    fn layer_adapters(&self) -> Vec<LayerAdapter> {
        self.adapters
            .iter()
            .map(|[qa, qb, va, vb]| LayerAdapter { query_a: qa.as_tensor().clone(), query_b: qb.as_tensor().clone(), value_a: va.as_tensor().clone(), value_b: vb.as_tensor().clone() })
            .collect()
    }

    fn vars(&self, adapter: bool) -> Vec<Var> {
        let mut v = vec![self.weight.clone(), self.bias.clone()];
        if adapter {
            for a in &self.adapters {
                v.extend(a.iter().cloned());
            }
        }
        v
    }
}

fn logits(enc: &Encoder, params: &Params, from: usize, hidden: Tensor, additive: &Tensor) -> Tensor {
    let out = enc.bert.upper(hidden, additive, from, &params.layer_adapters()).unwrap();
    logit(&first_vector(&out).unwrap(), params.weight.as_tensor(), params.bias.as_tensor()).unwrap()
}

fn predict(enc: &Encoder, params: &Params, from: usize, cached: &[Cached], idx: &[usize]) -> Vec<f32> {
    let mut out = vec![0.0; idx.len()];
    let mut order: Vec<usize> = (0..idx.len()).collect();
    order.sort_by_key(|&k| cached[idx[k]].length);
    for chunk in order.chunks(32) {
        let ids: Vec<usize> = chunk.iter().map(|&k| idx[k]).collect();
        let (h, a) = batch(cached, &ids);
        let p = candle_nn::ops::sigmoid(&logits(enc, params, from, h, &a)).unwrap().to_vec1::<f32>().unwrap();
        for (&k, p) in chunk.iter().zip(p) {
            out[k] = p;
        }
    }
    out
}

struct Train {
    epochs: usize,
    lr: f64,
    adapter: bool,
    pos_weight: f32,
}

/// Trains on `train`, calling `each` after every epoch.
fn train(enc: &Encoder, from: usize, cached: &[Cached], ex: &[Example], train_idx: &[usize], cfg: &Train, seed: u64, mut each: impl FnMut(usize, &Params)) -> Params {
    let mut rng = Rng(seed);
    let params = Params::new(LAYERS - from, &mut rng);
    let mut opt = AdamW::new(params.vars(cfg.adapter), ParamsAdamW { lr: cfg.lr, weight_decay: 0.01, ..Default::default() }).unwrap();
    let mut idx = train_idx.to_vec();
    for epoch in 0..cfg.epochs {
        rng.shuffle(&mut idx);
        // Batches of similar length cut padding: sort within windows of 64.
        for window in idx.chunks_mut(64) {
            window.sort_by_key(|&i| cached[i].length);
        }
        for chunk in idx.chunks(16) {
            let (h, a) = batch(cached, chunk);
            let z = logits(enc, &params, from, h, &a);
            let y = Tensor::from_vec(chunk.iter().map(|&i| if ex[i].y { 1.0f32 } else { 0.0 }).collect::<Vec<_>>(), chunk.len(), &Device::Cpu).unwrap();
            let w: Vec<f32> = chunk.iter().map(|&i| ex[i].weight * if ex[i].y { cfg.pos_weight } else { 1.0 }).collect();
            let total: f32 = w.iter().sum();
            let w = Tensor::from_vec(w, chunk.len(), &Device::Cpu).unwrap();
            // Weighted binary cross-entropy on logits: max(z,0) - z*y + log(1 + exp(-|z|)).
            let loss = ((z.relu().unwrap() - (&z * &y).unwrap()).unwrap() + (z.abs().unwrap().neg().unwrap().exp().unwrap() + 1.0).unwrap().log().unwrap()).unwrap();
            let loss = ((loss * w).unwrap().sum_all().unwrap() / total as f64).unwrap();
            opt.backward_step(&loss).unwrap();
        }
        each(epoch, &params);
    }
    params
}

fn auc(scores: &[f32], ys: &[bool]) -> f64 {
    let pos: Vec<f32> = scores.iter().zip(ys).filter(|(_, y)| **y).map(|(s, _)| *s).collect();
    let neg: Vec<f32> = scores.iter().zip(ys).filter(|(_, y)| !**y).map(|(s, _)| *s).collect();
    let mut sum = 0.0;
    for p in &pos {
        for n in &neg {
            sum += if p > n { 1.0 } else if p == n { 0.5 } else { 0.0 };
        }
    }
    sum / (pos.len() * neg.len()) as f64
}

fn pr(scores: &[f32], ys: &[bool], t: f32) -> (f64, f64, usize, usize, usize) {
    let tp = scores.iter().zip(ys).filter(|(s, y)| **y && **s >= t).count();
    let fp = scores.iter().zip(ys).filter(|(s, y)| !**y && **s >= t).count();
    let fn_ = scores.iter().zip(ys).filter(|(s, y)| **y && **s < t).count();
    let p = if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 };
    let r = if tp + fn_ > 0 { tp as f64 / (tp + fn_) as f64 } else { 0.0 };
    (p, r, tp, fp, fn_)
}

/// The most recall at precision `floor` or above.
fn threshold(scores: &[f32], ys: &[bool], floor: f64) -> (f32, f64, f64) {
    let mut best = (0.99, 0.0, 0.0);
    for step in 1..100 {
        let t = step as f32 / 100.0;
        let (p, r, ..) = pr(scores, ys, t);
        if p >= floor && r > best.2 {
            best = (t, p, r);
        }
    }
    best
}

/// Stratified folds.
fn folds(ex: &[Example], k: usize, seed: u64) -> Vec<usize> {
    let mut rng = Rng(seed);
    let mut fold = vec![0; ex.len()];
    for (class, new) in [(true, true), (true, false), (false, true), (false, false)] {
        let mut idx: Vec<usize> = (0..ex.len()).filter(|&i| ex[i].y == class && ex[i].new == new).collect();
        rng.shuffle(&mut idx);
        for (n, i) in idx.into_iter().enumerate() {
            fold[i] = n % k;
        }
    }
    fold
}

/// Stratified folds by reply: every sentence of one reply lands in one fold, and replies with a
/// positive are spread evenly. `kind` holds the reply's id.
fn reply_folds(ex: &[Example], k: usize, seed: u64) -> Vec<usize> {
    let mut rng = Rng(seed);
    let mut replies: Vec<&str> = Vec::new();
    for e in ex {
        if !replies.contains(&e.kind.as_str()) {
            replies.push(&e.kind);
        }
    }
    let mut of: HashMap<&str, usize> = HashMap::new();
    for class in [true, false] {
        let mut idx: Vec<&str> = replies.iter().copied().filter(|r| ex.iter().any(|e| e.kind == *r && e.y) == class).collect();
        rng.shuffle(&mut idx);
        for (n, r) in idx.into_iter().enumerate() {
            of.insert(r, n % k);
        }
    }
    ex.iter().map(|e| of[e.kind.as_str()]).collect()
}

/// k-fold: out-of-fold scores after each epoch.
fn cross(enc: &Encoder, from: usize, cached: &[Cached], ex: &[Example], cfg: &Train, k: usize) -> Vec<Vec<f32>> {
    cross_folded(enc, from, cached, ex, cfg, k, &folds(ex, k, 7))
}

fn cross_folded(enc: &Encoder, from: usize, cached: &[Cached], ex: &[Example], cfg: &Train, k: usize, fold: &[usize]) -> Vec<Vec<f32>> {
    let mut oof = vec![vec![0.0; ex.len()]; cfg.epochs];
    for f in 0..k {
        let t = Instant::now();
        let train_idx: Vec<usize> = (0..ex.len()).filter(|&i| fold[i] != f).collect();
        let val: Vec<usize> = (0..ex.len()).filter(|&i| fold[i] == f).collect();
        train(enc, from, cached, ex, &train_idx, cfg, 100 + f as u64, |epoch, params| {
            for (i, p) in val.iter().zip(predict(enc, params, from, cached, &val)) {
                oof[epoch][*i] = p;
            }
        });
        eprintln!("  fold {f} in {:.0?}", t.elapsed());
    }
    oof
}

fn report(name: &str, oof: &[Vec<f32>], ex: &[Example], only_new: bool) {
    for (epoch, scores) in oof.iter().enumerate() {
        let keep: Vec<usize> = (0..ex.len()).filter(|&i| !only_new || ex[i].new).collect();
        let s: Vec<f32> = keep.iter().map(|&i| scores[i]).collect();
        let y: Vec<bool> = keep.iter().map(|&i| ex[i].y).collect();
        let (t, p, r) = threshold(&s, &y, 0.90);
        let half = pr(&s, &y, 0.5);
        println!(
            "{name} epoch {} auc {:.3} | at 0.5 P {:.3} R {:.3} (tp{} fp{} fn{}) | best@P>=.9 t {t:.2} P {p:.3} R {r:.3}",
            epoch + 1,
            auc(&s, &y),
            half.0,
            half.1,
            half.2,
            half.3,
            half.4
        );
    }
}

/// The labelled sentences of `sentences/`, each read alone: `kind` is the reply's id, `new` whether
/// the reply is in the dev half, and `y` whether its label is one of `positive`.
fn sentence_examples(training: &Path, positive: &[&str]) -> Vec<Example> {
    let dir = training.join("sentences");
    let mut text: HashMap<(String, String, String), String> = HashMap::new();
    for set in ["new", "fresh"] {
        for line in std::fs::read_to_string(dir.join(format!("{set}.tsv"))).unwrap().lines() {
            let f: Vec<&str> = line.split('\t').collect();
            text.insert((set.to_string(), f[0].to_string(), f[1].to_string()), f[2].to_string());
        }
    }
    let dev: std::collections::HashSet<(String, String)> = std::fs::read_to_string(dir.join("split.tsv"))
        .unwrap()
        .lines()
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .filter(|f| f[2] == "dev")
        .map(|f| (f[0].to_string(), f[1].to_string()))
        .collect();
    let labelled = std::fs::read_to_string(dir.join("labels.tsv"))
        .unwrap()
        .lines()
        .map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            let key = (f[0].to_string(), f[1].to_string(), f[2].to_string());
            Example {
                id: format!("{}:{}:{}", f[0], f[1], f[2]),
                text: text[&key].clone(),
                y: positive.contains(&f[3]),
                weight: 1.0,
                new: dev.contains(&(f[0].to_string(), f[1].to_string())),
                kind: format!("{}:{}", f[0], f[1]),
            }
        })
        .collect::<Vec<_>>();
    // The sentences he marked Not a question in Bridge, each a reply of its own in the dev half, asking
    // nothing and weighted as a positive is, since he labelled them himself.
    let marked = misreads().into_iter().enumerate().map(|(n, sentence)| Example {
        id: format!("misread:{n}"),
        text: sentence,
        y: false,
        weight: 3.0,
        new: true,
        kind: format!("misread:{n}"),
    });
    // The sentences he confirmed with thumbs up, each a reply of its own in the dev half, asking. A
    // positive is already weighted by `pos_weight`, so these carry no weight of their own.
    let asked = confirmed().into_iter().enumerate().map(|(n, sentence)| Example {
        id: format!("confirmed:{n}"),
        text: sentence,
        y: true,
        weight: 1.0,
        new: true,
        kind: format!("confirmed:{n}"),
    });
    // The follow-ups he marked with thumbs, read by the follow-up head alone: the texts of the other
    // reading stay the same, so these weigh nothing there.
    let follow = positive.contains(&"follow_up");
    let not_deferred = later_misreads().into_iter().enumerate().map(|(n, sentence)| Example {
        id: format!("later_misread:{n}"),
        text: sentence,
        y: false,
        weight: if follow { 3.0 } else { 0.0 },
        new: true,
        kind: format!("later_misread:{n}"),
    });
    let deferred = later_confirmed().into_iter().enumerate().map(|(n, sentence)| Example {
        id: format!("later_confirmed:{n}"),
        text: sentence,
        y: true,
        weight: if follow { 1.0 } else { 0.0 },
        new: true,
        kind: format!("later_confirmed:{n}"),
    });
    labelled.into_iter().chain(marked).chain(asked).chain(not_deferred).chain(deferred).collect()
}

/// The sentences marked Not a question, read from Fleet's store with the `sqlite3` macOS ships. None
/// where the store or its table is not there yet.
fn misreads() -> Vec<String> {
    kept_sentences("SELECT sentence FROM reply_misreads ORDER BY marked_at, sentence")
}

/// The sentences he confirmed with thumbs up, read the same way.
fn confirmed() -> Vec<String> {
    kept_sentences("SELECT sentence FROM reply_confirmed ORDER BY marked_at, sentence")
}

/// The follow-ups he marked Not deferred, read the same way.
fn later_misreads() -> Vec<String> {
    kept_sentences("SELECT sentence FROM later_misreads ORDER BY marked_at, sentence")
}

/// The follow-ups he confirmed as deferred, read the same way.
fn later_confirmed() -> Vec<String> {
    kept_sentences("SELECT sentence FROM later_confirmed ORDER BY marked_at, sentence")
}

fn kept_sentences(query: &str) -> Vec<String> {
    let store = support_dir().join("armada.db");
    let Ok(out) = std::process::Command::new("sqlite3").arg("-readonly").arg(&store).arg(query).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout).lines().filter(|line| !line.trim().is_empty()).map(str::to_string).collect()
}

/// The whole encoder with `tuned`'s frozen adapters, cached: a head trained on it leaves the adapters as they are.
fn cache_adapted(enc: &Encoder, tuned: &Tuned, texts: &[&str], reading: TextReading) -> Vec<Cached> {
    let (_, longest, keep) = shape(reading);
    let from = LAYERS - tuned.adapters.len();
    texts
        .iter()
        .enumerate()
        .map(|(n, text)| {
            if n > 0 && n % 500 == 0 {
                eprintln!("  cached {n}");
            }
            let (ids, mask) = enc.batch(&[text], longest, keep).unwrap();
            let (h, a) = enc.bert.lower(&ids, &mask, from).unwrap();
            let hidden = enc.bert.upper(h, &a, from, &tuned.adapters).unwrap();
            let length = hidden.dim(1).unwrap();
            Cached { hidden, length }
        })
        .collect()
}

/// A line of figures for `scores` against `ys`: AUC, at 0.5, and the most recall at precision 0.90 and 0.80.
fn figures(scores: &[f32], ys: &[bool]) -> String {
    let (t9, p9, r9) = threshold(scores, ys, 0.90);
    let (t8, p8, r8) = threshold(scores, ys, 0.80);
    let half = pr(scores, ys, 0.5);
    format!(
        "auc {:.3} | at 0.5 P {:.3} R {:.3} (tp{} fp{} fn{}) | P>=.9 t {t9:.2} P {p9:.3} R {r9:.3} | P>=.8 t {t8:.2} P {p8:.3} R {r8:.3}",
        auc(scores, ys),
        half.0,
        half.1,
        half.2,
        half.3,
        half.4
    )
}

/// The agents' waiting items of `items/`, each read alone: `y` is whether the label is `clear`, an unclear
/// item weighs three so the rarer class is not drowned, `new` whether the item is in the dev half, and
/// `kind` its session, so a fold keeps a session whole. The second list says whether a command goes with it.
fn clarity_examples(training: &Path) -> (Vec<Example>, Vec<bool>) {
    let dir = training.join("items");
    let table = |name: &str| -> Vec<Vec<String>> {
        std::fs::read_to_string(dir.join(name)).unwrap().lines().skip(1).map(|line| line.split('\t').map(str::to_string).collect()).collect()
    };
    let label: HashMap<(String, String), bool> = table("clarity.tsv").into_iter().map(|f| ((f[0].clone(), f[1].clone()), f[2] == "clear")).collect();
    let dev: HashMap<(String, String), bool> = table("split.tsv").into_iter().map(|f| ((f[1].clone(), f[0].clone()), f[2] == "dev")).collect();
    let mut out = (Vec::new(), Vec::new());
    for f in table("items.tsv") {
        let key = (f[0].clone(), f[2].clone());
        let clear = label[&key];
        out.0.push(Example { id: format!("{}:{}", f[0], f[2]), text: f[3].clone(), y: clear, weight: if clear { 1.0 } else { 3.0 }, new: dev[&key], kind: f[0].clone() });
        out.1.push(f[4] == "run" && !f[5].trim().is_empty());
    }
    out
}

/// Folds that keep a session whole, the sessions with an unclear item spread evenly.
fn session_folds(ex: &[Example], k: usize, seed: u64) -> Vec<usize> {
    let mut rng = Rng(seed);
    let mut sessions: Vec<&str> = Vec::new();
    for e in ex {
        if !sessions.contains(&e.kind.as_str()) {
            sessions.push(&e.kind);
        }
    }
    let mut of: HashMap<&str, usize> = HashMap::new();
    for class in [true, false] {
        let mut idx: Vec<&str> = sessions.iter().copied().filter(|s| ex.iter().any(|e| e.kind == *s && !e.y) == class).collect();
        rng.shuffle(&mut idx);
        for (n, s) in idx.into_iter().enumerate() {
            of.insert(s, n % k);
        }
    }
    ex.iter().map(|e| of[e.kind.as_str()]).collect()
}

/// Precision and recall of the call `unclear` against the labels `ys`, with the counts.
fn calls(unclear: &[bool], ys: &[bool]) -> String {
    let scores: Vec<f32> = unclear.iter().map(|u| if *u { 1.0 } else { 0.0 }).collect();
    let (p, r, tp, fp, fn_) = pr(&scores, ys, 0.5);
    format!("P {p:.3} R {r:.3} (tp{tp} fp{fp} fn{fn_})")
}

fn save(params: &Params, heads: &[(&str, &Params)], path: &Path) {
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    for (n, [qa, qb, va, vb]) in params.adapters.iter().enumerate() {
        tensors.insert(format!("layer.{n}.query_a"), qa.as_tensor().clone());
        tensors.insert(format!("layer.{n}.query_b"), qb.as_tensor().clone());
        tensors.insert(format!("layer.{n}.value_a"), va.as_tensor().clone());
        tensors.insert(format!("layer.{n}.value_b"), vb.as_tensor().clone());
    }
    for (name, head) in heads {
        tensors.insert(format!("head.{name}.weight"), head.weight.as_tensor().clone());
        tensors.insert(format!("head.{name}.bias"), head.bias.as_tensor().clone());
    }
    candle_core::safetensors::save(&tensors, path).unwrap();
}

fn support_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap()).join("Library/Application Support/Armada")
}

/// The labelled replies and run outputs, which stay out of the repository.
fn training_dir() -> PathBuf {
    std::env::var_os("ARMADA_TRAINING").map_or_else(|| support_dir().join("training"), PathBuf::from)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let training = training_dir();
    let runs = training.join("lm-train-runs");
    if args[1] == "sentences-rules" {
        // sentences-rules: over the labelled replies, how many closing sentences the heading rule
        // drops and what they were labelled, and how many question and act sentences the options
        // rule gives choices to. No model.
        let dir = training.join("sentences");
        let mut label: HashMap<(String, String, String), String> = HashMap::new();
        for line in std::fs::read_to_string(dir.join("labels.tsv")).unwrap().lines() {
            let f: Vec<&str> = line.split('\t').collect();
            label.insert((f[0].to_string(), f[1].to_string(), f[2].to_string()), f[3].to_string());
        }
        let mut dropped: HashMap<String, usize> = HashMap::new();
        let (mut sentences, mut kept, mut asks_n, mut with_options, mut by_shape) = (0, 0, 0, 0, [0, 0]);
        let mut shown: Vec<String> = Vec::new();
        for (set, folder) in [("new", "lm-train-data/new"), ("fresh", "lm-fresh/txt")] {
            let mut files: Vec<PathBuf> = std::fs::read_dir(training.join(folder)).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|x| x == "txt")).collect();
            files.sort();
            for file in &files {
                let stem = file.file_stem().unwrap().to_string_lossy().to_string();
                let reply = std::fs::read_to_string(file).unwrap();
                let all = asks::closing_sentences(&reply);
                let close = extracting::candidates(&reply);
                sentences += all.len();
                kept += close.len();
                for (n, sentence) in all.iter().enumerate() {
                    let Some(l) = label.get(&(set.to_string(), stem.clone(), n.to_string())) else { continue };
                    let at = close.iter().position(|one| one.sentence == *sentence);
                    if at.is_none() {
                        *dropped.entry(l.clone()).or_default() += 1;
                        if l != "none" {
                            shown.push(format!("  dropped {l}: {sentence}"));
                        }
                        continue;
                    }
                    if l == "question" || l == "act" {
                        asks_n += 1;
                        let options = extracting::options_of(sentence, &close, at);
                        if !options.is_empty() {
                            with_options += 1;
                            by_shape[usize::from(!sentence.trim_end_matches(['*', '"', ')']).ends_with('?'))] += 1;
                            shown.push(format!("  options {l}: {sentence} -> {options:?}"));
                        }
                    }
                }
            }
        }
        println!("{sentences} closing sentences, {kept} kept, {} dropped as headings: {dropped:?}", sentences - kept);
        println!("{asks_n} question or act sentences kept; {with_options} given options ({} from an or-question, {} from a list)", by_shape[0], by_shape[1]);
        for line in shown {
            println!("{line}");
        }
        return;
    }
    if args[1] == "sentences-time" {
        // sentences-time <adapter>: as Fleet reads a reply for its sentences, the close's candidates
        // through both heads, over the labelled replies.
        let base = support_dir().join("models/bge-small-en-v1.5@5c38ec7c405ec4b44b94cc5a9bb96e735b38267a");
        let vocab = std::fs::read_to_string(base.join("vocab.txt")).unwrap();
        let enc = Encoder::load(&vocab, std::fs::read(base.join("model.safetensors")).unwrap(), LAYERS).unwrap();
        let tuned = Tuned::from_bytes(&std::fs::read(&args[2]).unwrap()).unwrap();
        let mut times = Vec::new();
        let mut counts = Vec::new();
        for folder in ["lm-train-data/new", "lm-fresh/txt"] {
            for entry in std::fs::read_dir(training.join(folder)).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().is_none_or(|x| x != "txt") {
                    continue;
                }
                let reply = std::fs::read_to_string(&path).unwrap();
                let t = Instant::now();
                let close = extracting::candidates(&reply);
                let texts: Vec<&str> = close.iter().map(|one| one.sentence.as_str()).collect();
                for reading in [TextReading::SentenceAsks, TextReading::SentenceFollowsUp] {
                    let (head, longest, keep) = shape(reading);
                    let _ = enc.read(&tuned, head, &texts, longest, keep).unwrap();
                }
                times.push(t.elapsed().as_secs_f64() * 1000.0);
                counts.push(texts.len());
            }
        }
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let rss = std::process::Command::new("ps").args(["-o", "rss=", "-p", &std::process::id().to_string()]).output().unwrap();
        println!(
            "{} replies, mean {:.1} sentences; per reply ms: mean {:.1} median {:.1} p95 {:.1} max {:.1}; rss {} KB",
            times.len(),
            counts.iter().sum::<usize>() as f64 / counts.len() as f64,
            times.iter().sum::<f64>() / times.len() as f64,
            times[times.len() / 2],
            times[times.len() * 95 / 100],
            times[times.len() - 1],
            String::from_utf8_lossy(&rss.stdout).trim()
        );
        return;
    }
    if args[1] == "sentences" {
        // sentences <dir> <out.tsv>: one line per sentence of each reply's close, for labelling.
        // The last column is 0 throughout: whether a command follows is private to `asks`.
        let mut files: Vec<PathBuf> = std::fs::read_dir(&args[2]).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|x| x == "txt")).collect();
        files.sort();
        let mut lines = Vec::new();
        for file in &files {
            let stem = file.file_stem().unwrap().to_string_lossy();
            for (n, sentence) in asks::closing_sentences(&std::fs::read_to_string(file).unwrap()).iter().enumerate() {
                lines.push(format!("{stem}\t{n}\t{}\t0", sentence.replace(['\t', '\n'], " ")));
            }
        }
        std::fs::write(&args[3], lines.join("\n") + "\n").unwrap();
        eprintln!("{} sentences from {} replies", lines.len(), files.len());
        return;
    }
    let base =support_dir().join("models/bge-small-en-v1.5@5c38ec7c405ec4b44b94cc5a9bb96e735b38267a");
    let vocab = std::fs::read_to_string(base.join("vocab.txt")).unwrap();
    let enc = Encoder::load(&vocab, std::fs::read(base.join("model.safetensors")).unwrap(), LAYERS).unwrap();
    if args[1] == "score" {
        // score <reply.txt>: each sentence of the close with its `sentence_asks` probability under the
        // committed adapter, marked where it reaches `extracting::ASKS_AT`, and the rules' own line.
        let reply = std::fs::read_to_string(&args[2]).unwrap();
        let tuned = Tuned::from_bytes(adapters_src::language_model::TUNED).unwrap();
        let close = extracting::candidates(&reply);
        let texts: Vec<&str> = close.iter().map(|one| one.sentence.as_str()).collect();
        let (head, longest, keep) = shape(TextReading::SentenceAsks);
        let scores = enc.read(&tuned, head, &texts, longest, keep).unwrap();
        for (text, score) in texts.iter().zip(&scores) {
            println!("{}{score:.3}\t{text}", if *score >= extracting::ASKS_AT { "ASKS " } else { "     " });
        }
        println!("rules: {:?}", asks::asking_line(&reply));
        return;
    }
    if args[1] == "pairs" {
        // pairs <pairs.tsv> <out.tsv>: the cosine of each pair's two texts under the base encoder, the
        // vectors Fleet's `TextModel::embed` gives (unit length, so the dot product is the cosine).
        let file = std::fs::read_to_string(&args[2]).unwrap();
        let mut lines = Vec::new();
        let mut per_pair = Vec::new();
        for row in file.lines().skip(1) {
            let f: Vec<&str> = row.split('\t').collect();
            if f.len() < 7 {
                continue;
            }
            let t = Instant::now();
            let v = enc.embed(&[f[4], f[6]]).unwrap();
            per_pair.push(t.elapsed().as_secs_f64() * 1000.0);
            let cosine: f32 = v[0].iter().zip(&v[1]).map(|(a, b)| a * b).sum();
            lines.push(format!("{}\t{cosine:.4}", f[0]));
        }
        std::fs::write(&args[3], lines.join("\n") + "\n").unwrap();
        per_pair.sort_by(|a, b| a.partial_cmp(b).unwrap());
        eprintln!(
            "{} pairs; ms per pair (two texts, one batch): mean {:.1} median {:.1} p95 {:.1} max {:.1}",
            lines.len(),
            per_pair.iter().sum::<f64>() / per_pair.len().max(1) as f64,
            per_pair[per_pair.len() / 2],
            per_pair[per_pair.len() * 95 / 100],
            per_pair[per_pair.len() - 1]
        );
        return;
    }
    let adapted: usize = args.get(2).map_or(4, |a| a.parse().unwrap());
    let from = LAYERS - adapted;
    let epochs: usize = args.get(3).map_or(6, |a| a.parse().unwrap());
    let lr: f64 = args.get(4).map_or(5e-4, |a| a.parse().unwrap());
    let new_weight: f32 = args.get(5).map_or(2.5, |a| a.parse().unwrap());
    match args[1].as_str() {
        "check" => {
            // The rewritten encoder against the vectors the earlier one gave.
            let v = enc.embed(&["Want me to land it?", "The walk is open; approve it when you're ready.", "Tests pass and the slot is released."]).unwrap();
            for x in &v {
                println!("VEC {:?}", x.iter().step_by(48).map(|f| format!("{f:.5}")).collect::<Vec<_>>());
            }
        }
        "asks-cv" => {
            let ex = asks_examples(&training, new_weight);
            let t = Instant::now();
            let cached = cache(&enc, &ex, TextReading::ReplyAsks, from);
            eprintln!("cached {} in {:.0?}", ex.len(), t.elapsed());
            let cfg = Train { epochs, lr, adapter: true, pos_weight: 1.0 };
            let oof = cross(&enc, from, &cached, &ex, &cfg, 5);
            report("asks all", &oof, &ex, false);
            report("asks new", &oof, &ex, true);
            let lines: Vec<String> = ex.iter().enumerate().map(|(i, e)| format!("{}\t{}\t{}\t{}", e.id, e.y, e.new, oof.iter().map(|o| format!("{:.4}", o[i])).collect::<Vec<_>>().join(","))).collect();
            std::fs::write(runs.join(format!("asks-oof-{adapted}-{epochs}-{lr}-{new_weight}.tsv")), lines.join("\n")).unwrap();
        }
        "tell-cv" => {
            // mode: own (own adapter), head (head only on the base), shared (head only on the asks adapter in lm-train-runs/asks.safetensors)
            let mode = args[6].as_str();
            let ex = tell_examples(&training);
            let k = 5;
            let (cached, from_here, cfg) = match mode {
                "own" => (cache(&enc, &ex, TextReading::TellRefused, from), from, Train { epochs, lr, adapter: true, pos_weight: 3.0 }),
                "head" => (cache(&enc, &ex, TextReading::TellRefused, LAYERS), LAYERS, Train { epochs, lr, adapter: false, pos_weight: 3.0 }),
                _ => {
                    let tuned = Tuned::from_bytes(&std::fs::read(runs.join("asks.safetensors")).unwrap()).unwrap();
                    let (_, longest, keep) = shape(TextReading::TellRefused);
                    let from_tuned = LAYERS - tuned.adapters.len();
                    let cached = ex
                        .iter()
                        .map(|e| {
                            let (ids, mask) = enc.batch(&[e.text.as_str()], longest, keep).unwrap();
                            let (h, a) = enc.bert.lower(&ids, &mask, from_tuned).unwrap();
                            let hidden = enc.bert.upper(h, &a, from_tuned, &tuned.adapters).unwrap();
                            let length = hidden.dim(1).unwrap();
                            Cached { hidden, length }
                        })
                        .collect();
                    (cached, LAYERS, Train { epochs, lr, adapter: false, pos_weight: 3.0 })
                }
            };
            let oof = cross(&enc, from_here, &cached, &ex, &cfg, k);
            report(&format!("tell {mode}"), &oof, &ex, false);
        }
        "tell-oof" => {
            // Out-of-fold scores of the refusal head on the asks adapter: whole reply as trained, its
            // 1,500-char tail as Fleet would hand it, and the tail's closing sentences one by one.
            let ex = tell_examples(&training);
            let k = 5;
            let tuned = Tuned::from_bytes(&std::fs::read(runs.join("asks.safetensors")).unwrap()).unwrap();
            let (_, longest, keep) = shape(TextReading::TellRefused);
            let from_tuned = LAYERS - tuned.adapters.len();
            let hid = |text: &str| -> Cached {
                let (ids, mask) = enc.batch(&[text], longest, keep).unwrap();
                let (h, a) = enc.bert.lower(&ids, &mask, from_tuned).unwrap();
                let hidden = enc.bert.upper(h, &a, from_tuned, &tuned.adapters).unwrap();
                let length = hidden.dim(1).unwrap();
                Cached { hidden, length }
            };
            let tail = |said: &str| -> String {
                let said = said.trim();
                let count = said.chars().count();
                if count <= 1500 {
                    return said.to_string();
                }
                let cut = said.char_indices().nth(count - 1500).map_or(0, |(at, _)| at);
                let kept = &said[cut..];
                match kept.split_once('\n') {
                    Some((_, rest)) if !rest.trim().is_empty() => rest.trim_start().to_string(),
                    _ => kept.to_string(),
                }
            };
            let full: Vec<Cached> = ex.iter().map(|e| hid(&e.text)).collect();
            let tails: Vec<String> = ex.iter().map(|e| tail(&e.text)).collect();
            let tailed: Vec<Cached> = tails.iter().map(|t| hid(t)).collect();
            let cfg = Train { epochs, lr, adapter: false, pos_weight: 3.0 };
            let fold = folds(&ex, k, 7);
            let mut out = vec![String::new(); ex.len()];
            for f in 0..k {
                let train_idx: Vec<usize> = (0..ex.len()).filter(|&i| fold[i] != f).collect();
                let val: Vec<usize> = (0..ex.len()).filter(|&i| fold[i] == f).collect();
                let params = train(&enc, LAYERS, &full, &ex, &train_idx, &cfg, 100 + f as u64, |_, _| {});
                let a = predict(&enc, &params, LAYERS, &full, &val);
                let b = predict(&enc, &params, LAYERS, &tailed, &val);
                for (n, &i) in val.iter().enumerate() {
                    let sentences = asks::closing_sentences(&tails[i]);
                    let last: Vec<String> = sentences[sentences.len().saturating_sub(6)..].to_vec();
                    let sc: Vec<Cached> = last.iter().map(|t| hid(t)).collect();
                    let idx: Vec<usize> = (0..sc.len()).collect();
                    let ss = if sc.is_empty() { vec![] } else { predict(&enc, &params, LAYERS, &sc, &idx) };
                    let mut line = format!("{}\t{}\t{}\t{:.4}\t{:.4}", ex[i].id, ex[i].kind, ex[i].y as u8, a[n], b[n]);
                    for (t, s) in last.iter().zip(ss) {
                        line.push_str(&format!("\t{s:.4}|{}", t.replace(['\t', '\n'], " ")));
                    }
                    out[i] = line;
                }
                eprintln!("  fold {f} done");
            }
            std::fs::write(runs.join("tell-oof.tsv"), out.join("\n")).unwrap();
        }
        "asks-final" => {
            let ex = asks_examples(&training, new_weight);
            let cached = cache(&enc, &ex, TextReading::ReplyAsks, from);
            let cfg = Train { epochs, lr, adapter: true, pos_weight: 1.0 };
            let all: Vec<usize> = (0..ex.len()).collect();
            let params = train(&enc, from, &cached, &ex, &all, &cfg, 1, |e, _| eprintln!("  epoch {}", e + 1));
            save(&params, &[("asks", &params)], &runs.join("asks.safetensors"));
            // The trainer's own probabilities, to hold the committed reader to.
            let probe: Vec<usize> = (0..ex.len()).step_by(37).collect();
            let mine = predict(&enc, &params, from, &cached, &probe);
            let tuned = Tuned::from_bytes(&std::fs::read(runs.join("asks.safetensors")).unwrap()).unwrap();
            let (head, longest, keep) = shape(TextReading::ReplyAsks);
            for (&i, p) in probe.iter().zip(mine) {
                let read = enc.read(&tuned, head, &[ex[i].text.as_str()], longest, keep).unwrap()[0];
                println!("probe {} trainer {p:.6} reader {read:.6}", ex[i].id);
            }
        }
        "tell-final" => {
            // The refusal head on the asks adapter, trained on every labelled tell reply, written beside
            // the asks head into one file.
            let ex = tell_examples(&training);
            let bytes = std::fs::read(runs.join("asks.safetensors")).unwrap();
            let tuned = Tuned::from_bytes(&bytes).unwrap();
            let (head, longest, keep) = shape(TextReading::TellRefused);
            let from_tuned = LAYERS - tuned.adapters.len();
            let cached: Vec<Cached> = ex
                .iter()
                .map(|e| {
                    let (ids, mask) = enc.batch(&[e.text.as_str()], longest, keep).unwrap();
                    let (h, a) = enc.bert.lower(&ids, &mask, from_tuned).unwrap();
                    let hidden = enc.bert.upper(h, &a, from_tuned, &tuned.adapters).unwrap();
                    let length = hidden.dim(1).unwrap();
                    Cached { hidden, length }
                })
                .collect();
            let cfg = Train { epochs, lr, adapter: false, pos_weight: 3.0 };
            let all: Vec<usize> = (0..ex.len()).collect();
            let params = train(&enc, LAYERS, &cached, &ex, &all, &cfg, 2, |_, _| {});
            let mut tensors = candle_core::safetensors::load_buffer(&bytes, &Device::Cpu).unwrap();
            tensors.insert(format!("head.{head}.weight"), params.weight.as_tensor().clone());
            tensors.insert(format!("head.{head}.bias"), params.bias.as_tensor().clone());
            candle_core::safetensors::save(&tensors, runs.join("tuned.safetensors")).unwrap();
            let probe: Vec<usize> = (0..ex.len()).step_by(23).collect();
            let mine = predict(&enc, &params, LAYERS, &cached, &probe);
            let both = Tuned::from_bytes(&std::fs::read(runs.join("tuned.safetensors")).unwrap()).unwrap();
            for (&i, p) in probe.iter().zip(mine) {
                let read = enc.read(&both, head, &[ex[i].text.as_str()], longest, keep).unwrap()[0];
                println!("probe tell {} trainer {p:.6} reader {read:.6}", ex[i].id);
            }
            let (ah, al, ak) = shape(TextReading::ReplyAsks);
            let text = "Both fixes are pushed. Want me to land them?";
            println!("asks head unchanged: {:.6} vs {:.6}", enc.read(&tuned, ah, &[text], al, ak).unwrap()[0], enc.read(&both, ah, &[text], al, ak).unwrap()[0]);
        }
        "sentences-train" => {
            // sentences-train <adapted, unread> <epochs> <lr>: the two sentence heads on the frozen
            // committed adapter, so `asks` and `refused` keep their weights. Five folds by reply on the
            // dev half, then one head on all of dev scored once on the held half, then the file.
            let committed = adapters_src::language_model::TUNED;
            let tuned = Tuned::from_bytes(committed).unwrap();
            let cfg = Train { epochs, lr, adapter: false, pos_weight: 3.0 };
            let mut tensors = candle_core::safetensors::load_buffer(committed, &Device::Cpu).unwrap();
            let mut probes: Vec<(TextReading, String, String, f32)> = Vec::new();
            // Both readings read a sentence the same way, so the frozen layers run once.
            let (cached, all_texts) = {
                let ex = sentence_examples(&training, &[]);
                let t = Instant::now();
                let texts: Vec<&str> = ex.iter().map(|e| e.text.as_str()).collect();
                let cached = cache_adapted(&enc, &tuned, &texts, TextReading::SentenceAsks);
                eprintln!("cached {} sentences in {:.0?}", ex.len(), t.elapsed());
                (cached, ex)
            };
            for (reading, positive) in [(TextReading::SentenceAsks, &["question", "act"][..]), (TextReading::SentenceFollowsUp, &["follow_up"][..])] {
                let (head, ..) = shape(reading);
                assert_eq!(shape(reading).1, shape(TextReading::SentenceAsks).1);
                let ex = sentence_examples(&training, positive);
                assert!(ex.iter().zip(&all_texts).all(|(a, b)| a.text == b.text));
                let dev: Vec<usize> = (0..ex.len()).filter(|&i| ex[i].new).collect();
                let held: Vec<usize> = (0..ex.len()).filter(|&i| !ex[i].new).collect();
                let dev_ex: Vec<Example> = dev.iter().map(|&i| ex[i].clone()).collect();
                let dev_cached: Vec<Cached> = dev.iter().map(|&i| Cached { hidden: cached[i].hidden.clone(), length: cached[i].length }).collect();
                let positives = |idx: &[usize]| idx.iter().filter(|&&i| ex[i].y).count();
                eprintln!("{head}: dev {} sentences, {} positive; held {} sentences, {} positive", dev.len(), positives(&dev), held.len(), positives(&held));
                let fold = reply_folds(&dev_ex, 5, 7);
                let oof = cross_folded(&enc, LAYERS, &dev_cached, &dev_ex, &cfg, 5, &fold);
                let dev_y: Vec<bool> = dev_ex.iter().map(|e| e.y).collect();
                for (epoch, scores) in oof.iter().enumerate() {
                    println!("{head} dev folds epoch {} {}", epoch + 1, figures(scores, &dev_y));
                }
                let last = &oof[oof.len() - 1];
                let (t9, ..) = threshold(last, &dev_y, 0.90);
                let (t8, ..) = threshold(last, &dev_y, 0.80);
                let lines: Vec<String> = dev_ex.iter().zip(last).map(|(e, s)| format!("{}\t{}\t{s:.4}", e.id, e.y)).collect();
                std::fs::write(runs.join(format!("sentences-oof-{head}.tsv")), lines.join("\n")).unwrap();
                // Once, on all of dev; the held half is scored once, at the thresholds dev folds chose.
                let all: Vec<usize> = (0..dev_ex.len()).collect();
                let params = train(&enc, LAYERS, &dev_cached, &dev_ex, &all, &cfg, 3, |_, _| {});
                let scores = predict(&enc, &params, LAYERS, &cached, &held);
                let held_y: Vec<bool> = held.iter().map(|&i| ex[i].y).collect();
                println!("{head} held {}", figures(&scores, &held_y));
                for (name, t) in [("P>=.9", t9), ("P>=.8", t8)] {
                    let (p, r, tp, fp, fn_) = pr(&scores, &held_y, t);
                    println!("{head} held at dev's {name} t {t:.2}: P {p:.3} R {r:.3} (tp{tp} fp{fp} fn{fn_})");
                }
                tensors.insert(format!("head.{head}.weight"), params.weight.as_tensor().clone());
                tensors.insert(format!("head.{head}.bias"), params.bias.as_tensor().clone());
                let probe: Vec<usize> = (0..ex.len()).step_by(409).collect();
                let mine = predict(&enc, &params, LAYERS, &cached, &probe);
                probes.extend(probe.iter().zip(mine).map(|(&i, p)| (reading, ex[i].id.clone(), ex[i].text.clone(), p)));
            }
            let out = runs.join("tuned-sentences.safetensors");
            candle_core::safetensors::save(&tensors, &out).unwrap();
            let both = Tuned::from_bytes(&std::fs::read(&out).unwrap()).unwrap();
            // The trainer's own probabilities against the reader's, through the file.
            for (reading, id, text, mine) in probes {
                let (head, longest, keep) = shape(reading);
                let read = enc.read(&both, head, &[text.as_str()], longest, keep).unwrap()[0];
                println!("probe {head} {id} trainer {mine:.6} reader {read:.6}");
            }
            for reading in [TextReading::ReplyAsks, TextReading::TellRefused] {
                let (head, longest, keep) = shape(reading);
                let text = "Both fixes are pushed. Want me to land them?";
                println!("{head} head unchanged: {:.6} vs {:.6}", enc.read(&tuned, head, &[text], longest, keep).unwrap()[0], enc.read(&both, head, &[text], longest, keep).unwrap()[0]);
            }
        }
        "clarity" => {
            // clarity <adapted, unread> <epochs> <lr> <dev|held>: the `item_clear` head on the frozen
            // committed adapter, so the other four heads keep their weights. Five folds by session on the
            // dev half pick the threshold for "unclear" (score = 1 - clear); `dev` stops there and prints
            // the rules' errors on dev. `held` trains on all of dev, scores the held half once, and writes the file.
            let committed = adapters_src::language_model::TUNED;
            let tuned = Tuned::from_bytes(committed).unwrap();
            let cfg = Train { epochs, lr, adapter: false, pos_weight: 1.0 };
            let (ex, runs_with) = clarity_examples(&training);
            let t = Instant::now();
            let texts: Vec<&str> = ex.iter().map(|e| e.text.as_str()).collect();
            let cached = cache_adapted(&enc, &tuned, &texts, TextReading::ItemClear);
            eprintln!("cached {} items in {:.0?}", ex.len(), t.elapsed());
            let dev: Vec<usize> = (0..ex.len()).filter(|&i| ex[i].new).collect();
            let held: Vec<usize> = (0..ex.len()).filter(|&i| !ex[i].new).collect();
            let dev_ex: Vec<Example> = dev.iter().map(|&i| ex[i].clone()).collect();
            let dev_cached: Vec<Cached> = dev.iter().map(|&i| Cached { hidden: cached[i].hidden.clone(), length: cached[i].length }).collect();
            let unclear_of = |i: usize| clarity::unclear(&ex[i].text, runs_with[i]).is_some();
            let dev_unclear: Vec<bool> = dev.iter().map(|&i| !ex[i].y).collect();
            let rules_dev: Vec<bool> = dev.iter().map(|&i| unclear_of(i)).collect();
            println!("rules dev {}", calls(&rules_dev, &dev_unclear));
            let mode = args[6].as_str();
            if mode == "dev" {
                for (&i, ys) in dev.iter().zip(&dev_unclear) {
                    if unclear_of(i) != *ys {
                        println!("  rules {} on dev: {}", if *ys { "missed" } else { "wrongly refused" }, ex[i].text);
                    }
                }
            }
            let fold = session_folds(&dev_ex, 5, 7);
            let oof = cross_folded(&enc, LAYERS, &dev_cached, &dev_ex, &cfg, 5, &fold);
            for (epoch, scores) in oof.iter().enumerate() {
                let unclear: Vec<f32> = scores.iter().map(|p| 1.0 - p).collect();
                println!("item_clear dev folds epoch {} {}", epoch + 1, figures(&unclear, &dev_unclear));
            }
            let last: Vec<f32> = oof[oof.len() - 1].iter().map(|p| 1.0 - p).collect();
            let (t_head, ..) = threshold(&last, &dev_unclear, 0.90);
            // The most recall at precision 0.90 when the rules' call also counts.
            let both_dev = |t: f32| -> Vec<bool> { last.iter().zip(&rules_dev).map(|(s, r)| *r || *s >= t).collect() };
            let mut t_both = (0.99f32, 0.0f64);
            for step in 1..100 {
                let t = step as f32 / 100.0;
                let call: Vec<f32> = both_dev(t).iter().map(|c| if *c { 1.0 } else { 0.0 }).collect();
                let (p, r, ..) = pr(&call, &dev_unclear, 0.5);
                if p >= 0.90 && r > t_both.1 {
                    t_both = (t, r);
                }
            }
            println!("head alone on dev folds at t {t_head:.2}: {}", calls(&last.iter().map(|s| *s >= t_head).collect::<Vec<_>>(), &dev_unclear));
            println!("rules and head on dev folds at t {:.2}: {}", t_both.0, calls(&both_dev(t_both.0), &dev_unclear));
            let lines: Vec<String> = dev_ex.iter().zip(&last).map(|(e, s)| format!("{}\t{}\t{s:.4}", e.id, !e.y)).collect();
            std::fs::write(runs.join("clarity-oof.tsv"), lines.join("\n")).unwrap();
            if mode != "held" {
                return;
            }
            let all: Vec<usize> = (0..dev_ex.len()).collect();
            let params = train(&enc, LAYERS, &dev_cached, &dev_ex, &all, &cfg, 3, |_, _| {});
            let scores: Vec<f32> = predict(&enc, &params, LAYERS, &cached, &held).iter().map(|p| 1.0 - p).collect();
            let held_unclear: Vec<bool> = held.iter().map(|&i| !ex[i].y).collect();
            let rules_held: Vec<bool> = held.iter().map(|&i| unclear_of(i)).collect();
            println!("held rules {}", calls(&rules_held, &held_unclear));
            println!("held head auc {:.3} at dev's t {t_head:.2}: {}", auc(&scores, &held_unclear), calls(&scores.iter().map(|s| *s >= t_head).collect::<Vec<_>>(), &held_unclear));
            let both: Vec<bool> = scores.iter().zip(&rules_held).map(|(s, r)| *r || *s >= t_both.0).collect();
            println!("held rules and head at dev's t {:.2}: {}", t_both.0, calls(&both, &held_unclear));
            let mut tensors = candle_core::safetensors::load_buffer(committed, &Device::Cpu).unwrap();
            tensors.insert("head.item_clear.weight".to_string(), params.weight.as_tensor().clone());
            tensors.insert("head.item_clear.bias".to_string(), params.bias.as_tensor().clone());
            let out = runs.join("tuned-clarity.safetensors");
            candle_core::safetensors::save(&tensors, &out).unwrap();
            let both_file = Tuned::from_bytes(&std::fs::read(&out).unwrap()).unwrap();
            let (head, longest, keep) = shape(TextReading::ItemClear);
            let probe: Vec<usize> = (0..ex.len()).step_by(37).collect();
            let mine = predict(&enc, &params, LAYERS, &cached, &probe);
            for (&i, p) in probe.iter().zip(mine) {
                let read = enc.read(&both_file, head, &[ex[i].text.as_str()], longest, keep).unwrap()[0];
                println!("probe {head} {} trainer {p:.6} reader {read:.6}", ex[i].id);
            }
            for reading in [TextReading::ReplyAsks, TextReading::TellRefused, TextReading::SentenceAsks, TextReading::SentenceFollowsUp] {
                let (head, longest, keep) = shape(reading);
                let text = "Both fixes are pushed. Want me to land them?";
                println!("{head} head unchanged: {:.6} vs {:.6}", enc.read(&tuned, head, &[text], longest, keep).unwrap()[0], enc.read(&both_file, head, &[text], longest, keep).unwrap()[0]);
            }
        }
        "clarity-time" => {
            // clarity-time: what one list costs as `waiting_for` would pay it: the rules over every
            // labelled item, then the head over each item alone, as a one-item list.
            let tuned = Tuned::from_bytes(adapters_src::language_model::TUNED).unwrap();
            let (ex, runs_with) = clarity_examples(&training);
            let t = Instant::now();
            for _ in 0..1000 {
                for (e, runs) in ex.iter().zip(&runs_with) {
                    std::hint::black_box(clarity::unclear(&e.text, *runs));
                }
            }
            eprintln!("rules: {:.2} us per item over {} items", t.elapsed().as_secs_f64() * 1e6 / (1000 * ex.len()) as f64, ex.len());
            let (head, longest, keep) = shape(TextReading::ItemClear);
            let mut times = Vec::new();
            for e in &ex {
                let t = Instant::now();
                let _ = enc.read(&tuned, head, &[e.text.as_str()], longest, keep).unwrap();
                times.push(t.elapsed().as_secs_f64() * 1000.0);
            }
            times.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let rss = std::process::Command::new("ps").args(["-o", "rss=", "-p", &std::process::id().to_string()]).output().unwrap();
            eprintln!(
                "head per item ms: mean {:.1} median {:.1} p95 {:.1} max {:.1}; rss {} KB",
                times.iter().sum::<f64>() / times.len() as f64,
                times[times.len() / 2],
                times[times.len() * 95 / 100],
                times[times.len() - 1],
                String::from_utf8_lossy(&rss.stdout).trim()
            );
        }
        "thresholds" => {
            // From out-of-fold scores at one epoch, on the post-18-Oct replies: the most recall at
            // precision 0.90 for the model alone and for the model beside the rules.
            let epoch: usize = args[7].parse().unwrap();
            let mut s = Vec::new();
            let mut y = Vec::new();
            let mut rules = Vec::new();
            for line in std::fs::read_to_string(&args[6]).unwrap().lines() {
                let f: Vec<&str> = line.split('\t').collect();
                if f[2] != "true" {
                    continue;
                }
                let id = f[0].trim_start_matches('n');
                let reply = std::fs::read_to_string(training.join("lm-train-data/new").join(format!("{id}.txt"))).unwrap();
                rules.push(asks::asking_line(&reply).is_some());
                y.push(f[1] == "true");
                s.push(f[3].split(',').nth(epoch - 1).unwrap().parse::<f32>().unwrap());
            }
            let (t, p, r) = threshold(&s, &y, 0.90);
            println!("model alone: t {t:.2} P {p:.3} R {r:.3}");
            let rp = pr(&rules.iter().map(|r| if *r { 1.0 } else { 0.0 }).collect::<Vec<f32>>(), &y, 0.5);
            println!("rules: P {:.3} R {:.3}", rp.0, rp.1);
            let mut best = (0.99, 0.0, 0.0);
            for step in 1..100 {
                let t = step as f32 / 100.0;
                let either: Vec<f32> = s.iter().zip(&rules).map(|(s, r)| if *r || *s >= t { 1.0 } else { 0.0 }).collect();
                let (p, r, ..) = pr(&either, &y, 0.5);
                if p >= 0.90 && r > best.2 {
                    best = (t, p, r);
                }
            }
            println!("either: t {:.2} P {:.3} R {:.3}", best.0, best.1, best.2);
        }
        "fresh" => {
            // Once: the held-out replies, with the committed adapter and the committed thresholds.
            let tuned = Tuned::from_bytes(&std::fs::read(&args[6]).unwrap()).unwrap();
            let (alone, beside): (f32, f32) = (args[7].parse().unwrap(), args[8].parse().unwrap());
            let unlisted: std::collections::HashSet<String> = std::fs::read_to_string(training.join("lm-fresh2/unlisted.txt")).unwrap().lines().map(str::to_string).collect();
            let (head, longest, keep) = shape(TextReading::ReplyAsks);
            let mut rows = Vec::new();
            for line in std::fs::read_to_string(training.join("lm-fresh/labels.txt")).unwrap().lines() {
                let f: Vec<&str> = line.split(' ').collect();
                let reply = std::fs::read_to_string(training.join("lm-fresh/txt").join(format!("{}.txt", f[0]))).unwrap();
                let score = if asks::closing_sentences(&reply).is_empty() { 0.0 } else { enc.read(&tuned, head, &[asks::closing_text(&reply).as_str()], longest, keep).unwrap()[0] };
                let rules = asks::asking_line(&reply).is_some();
                println!("{}\t{}\t{}\t{:.4}", f[0], f[1], rules, score);
                rows.push((f[0].to_string(), f[1] == "asks", rules, score));
            }
            for (subset, keep_row) in [("all", Box::new(|_: &str| true) as Box<dyn Fn(&str) -> bool>), ("nothing listed before", Box::new(|i: &str| unlisted.contains(i)))] {
                let kept: Vec<_> = rows.iter().filter(|r| keep_row(&r.0)).collect();
                let asks_n = kept.iter().filter(|r| r.1).count();
                eprintln!("{subset}: {} replies, {} asks", kept.len(), asks_n);
                for (name, call) in [
                    ("rules", Box::new(|r: bool, _s: f32| r) as Box<dyn Fn(bool, f32) -> bool>),
                    ("model", Box::new(move |_r: bool, s: f32| s >= alone)),
                    ("either", Box::new(move |r: bool, s: f32| r || s >= beside)),
                ] {
                    let s: Vec<f32> = kept.iter().map(|r| if call(r.2, r.3) { 1.0 } else { 0.0 }).collect();
                    let y: Vec<bool> = kept.iter().map(|r| r.1).collect();
                    let (p, rc, tp, fp, fn_) = pr(&s, &y, 0.5);
                    eprintln!("  {name:6} P={p:.3} R={rc:.3} right={tp} wrong={fp} missed={fn_}");
                }
            }
        }
        "time" => {
            // As Fleet reads a reply: the close and its last six sentences in one batch.
            let tuned = Tuned::from_bytes(&std::fs::read(&args[6]).unwrap()).unwrap();
            let (head, longest, keep) = shape(TextReading::ReplyAsks);
            let ex = asks_examples(&training, 1.0);
            let mut times = Vec::new();
            for e in &ex {
                let sentences: Vec<&str> = e.text.split(". ").collect();
                let _ = sentences;
                let reply_sentences = asks::closing_sentences(&e.text);
                let last = &reply_sentences[reply_sentences.len().saturating_sub(6)..];
                let mut texts = vec![e.text.as_str()];
                texts.extend(last.iter().map(String::as_str));
                let t = Instant::now();
                let _ = enc.read(&tuned, head, &texts, longest, keep).unwrap();
                times.push(t.elapsed().as_secs_f64() * 1000.0);
            }
            times.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let rss = std::process::Command::new("ps").args(["-o", "rss=", "-p", &std::process::id().to_string()]).output().unwrap();
            eprintln!(
                "per reply ms: mean {:.1} median {:.1} p95 {:.1} max {:.1}; rss {} KB",
                times.iter().sum::<f64>() / times.len() as f64,
                times[times.len() / 2],
                times[times.len() * 95 / 100],
                times[times.len() - 1],
                String::from_utf8_lossy(&rss.stdout).trim()
            );
        }
        _ => panic!(),
    }
}
