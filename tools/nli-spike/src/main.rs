// Zero-shot NLI scoring of the owner's replies, outside the workspace: does a free local model say whether a reply approves what a Job put to him?
// Usage: nli-spike <bert|deberta> <model folder> [<out folder>]
#![allow(dead_code)]
#[path = "../../../crates/adapters/src/language_model/wordpiece.rs"]
mod wordpiece;

use std::path::{Path, PathBuf};
use std::time::Instant;

use candle_core::{DType, Device, Tensor};
use candle_nn::{Module, VarBuilder};
use sha2::{Digest, Sha256};

/// Chosen before any score was looked at. The reply is the premise.
const HYPOTHESES: [&str; 3] = [
    "The person approves the plan and tells the agent to go ahead.",
    "The person says to proceed with what was proposed.",
    "The person is satisfied with this and wants it to continue as it is.",
];

/// Longest reply read, in word pieces, as the earlier reader had it.
const LONGEST_PREMISE: usize = 128;

/// Each model's pinned revision and the SHA-256 of every file it reads.
struct Pin {
    repository: &'static str,
    revision: &'static str,
    files: &'static [(&'static str, &'static str)],
}

const XTREMEDISTIL: Pin = Pin {
    repository: "MoritzLaurer/xtremedistil-l6-h256-zeroshot-v1.1-all-33",
    revision: "c07f66d9cbf781191bee66edfe8ad7856f045781",
    files: &[
        ("config.json", "7056598267ea0cf462d3b35c5f42c1f0be2bcb06f5e301a1861d7d6f957ca228"),
        ("vocab.txt", "07eced375cec144d27c900241f3e339478dec958f92fddbc551f295c992038a3"),
        ("model.safetensors", "bf2d83426aa6487f5cc3dc5c71d1566f33a65d883ebdb80fcb4e477c09ff0150"),
    ],
};

const DEBERTA_ZEROSHOT: Pin = Pin {
    repository: "MoritzLaurer/deberta-v3-base-zeroshot-v2.0",
    revision: "8e7e5af5983a0ddb1a5b45a38b129ab69e2258e8",
    files: &[
        ("config.json", "a1c8011b852ae46d6ac26160d358fa595861d261352885f8f7bfcd2811030d9f"),
        ("tokenizer.json", "05402ffae6dd382a8491b1d29bfc139bec5d332662e86a026f433ce54c25c202"),
        ("model.safetensors", "6e8f2af78c828dcbd5243aac40fb87430376f0b8a9c288f4993df3ea3558d557"),
    ],
};

const DEBERTA_NLI_SMALL: Pin = Pin {
    repository: "cross-encoder/nli-deberta-v3-small",
    revision: "fa2804872c3b4bd748f38c0185cc85775361e735",
    files: &[
        ("config.json", "885d0dceae8fa5c136da9209121ec9eb11160488e840de3bc1f29353674e5712"),
        ("tokenizer.json", "5124ef2ead1a10a717703bc436de7f353da76d6340e4587719b42b1693707964"),
        ("model.safetensors", "ebc79588dd73ccfb6a3f6078519cfbf512c5305384c5ea1845bc71cd32216e86"),
    ],
};

fn verify(pin: &Pin, folder: &Path) {
    for (name, digest) in pin.files {
        let bytes = std::fs::read(folder.join(name)).unwrap_or_else(|why| panic!("{name}: {why}"));
        let got: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(&got, digest, "{name} is not the pinned file of {}@{}", pin.repository, pin.revision);
    }
}

/// One probability per label, in the model's own label order, and where the two that matter sit.
struct Read {
    probs: Vec<f32>,
    entailment: usize,
    contradiction: Option<usize>,
}

trait Nli {
    fn read(&self, premise: &str, hypothesis: &str) -> Read;
}

fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().cloned().fold(f32::MIN, f32::max);
    let exp: Vec<f32> = logits.iter().map(|l| (l - max).exp()).collect();
    let sum: f32 = exp.iter().sum();
    exp.iter().map(|e| e / sum).collect()
}

fn label_positions(config: &serde_json::Value) -> (Vec<String>, usize, Option<usize>) {
    let map = config["id2label"].as_object().expect("id2label");
    let mut labels = vec![String::new(); map.len()];
    for (k, v) in map {
        labels[k.parse::<usize>().unwrap()] = v.as_str().unwrap().to_lowercase();
    }
    let entailment = labels.iter().position(|l| l.starts_with("entail")).expect("an entailment label");
    let contradiction = labels.iter().position(|l| l.starts_with("contra"));
    eprintln!("labels, in output order: {labels:?}");
    (labels, entailment, contradiction)
}

struct Bert {
    model: candle_transformers::models::bert::BertModel,
    pooler: candle_nn::Linear,
    classifier: candle_nn::Linear,
    vocab: wordpiece::WordPiece,
    entailment: usize,
    contradiction: Option<usize>,
}

impl Bert {
    fn load(folder: &Path) -> Bert {
        verify(&XTREMEDISTIL, folder);
        let config_text = std::fs::read_to_string(folder.join("config.json")).unwrap();
        let config_json: serde_json::Value = serde_json::from_str(&config_text).unwrap();
        let config: candle_transformers::models::bert::Config = serde_json::from_str(&config_text).unwrap();
        let (labels, entailment, contradiction) = label_positions(&config_json);
        let device = Device::Cpu;
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[folder.join("model.safetensors")], DType::F32, &device).unwrap() };
        let model = candle_transformers::models::bert::BertModel::load(vb.pp("bert"), &config).unwrap();
        let pooler = candle_nn::linear(config.hidden_size, config.hidden_size, vb.pp("bert.pooler.dense")).unwrap();
        let classifier = candle_nn::linear(config.hidden_size, labels.len(), vb.pp("classifier")).unwrap();
        let vocab = wordpiece::WordPiece::from_vocab(&std::fs::read_to_string(folder.join("vocab.txt")).unwrap()).unwrap();
        Bert { model, pooler, classifier, vocab, entailment, contradiction }
    }
}

impl Nli for Bert {
    fn read(&self, premise: &str, hypothesis: &str) -> Read {
        let mut first = self.vocab.pieces(premise);
        first.truncate(LONGEST_PREMISE);
        let second = self.vocab.pieces(hypothesis);
        let mut ids = vec![self.vocab.start];
        ids.extend(&first);
        ids.push(self.vocab.end);
        let split = ids.len();
        ids.extend(&second);
        ids.push(self.vocab.end);
        let types: Vec<u32> = (0..ids.len()).map(|at| u32::from(at >= split)).collect();
        let device = Device::Cpu;
        let ids = Tensor::new(ids.as_slice(), &device).unwrap().unsqueeze(0).unwrap();
        let types = Tensor::new(types.as_slice(), &device).unwrap().unsqueeze(0).unwrap();
        let hidden = self.model.forward(&ids, &types, None).unwrap();
        let first_vector = hidden.narrow(1, 0, 1).unwrap().squeeze(1).unwrap();
        let pooled = self.pooler.forward(&first_vector).unwrap().tanh().unwrap();
        let logits = self.classifier.forward(&pooled).unwrap().squeeze(0).unwrap().to_vec1::<f32>().unwrap();
        Read { probs: softmax(&logits), entailment: self.entailment, contradiction: self.contradiction }
    }
}

struct Deberta {
    model: candle_transformers::models::debertav2::DebertaV2SeqClassificationModel,
    tokenizer: tokenizers::Tokenizer,
    entailment: usize,
    contradiction: Option<usize>,
}

impl Deberta {
    fn load(pin: &Pin, folder: &Path) -> Deberta {
        verify(pin, folder);
        let config_text = std::fs::read_to_string(folder.join("config.json")).unwrap();
        let config_json: serde_json::Value = serde_json::from_str(&config_text).unwrap();
        let config: candle_transformers::models::debertav2::Config = serde_json::from_str(&config_text).unwrap();
        let (_, entailment, contradiction) = label_positions(&config_json);
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[folder.join("model.safetensors")], DType::F32, &Device::Cpu).unwrap() };
        let model = candle_transformers::models::debertav2::DebertaV2SeqClassificationModel::load(vb.pp("deberta"), &config, None).unwrap();
        let mut tokenizer = tokenizers::Tokenizer::from_file(folder.join("tokenizer.json")).unwrap();
        tokenizer
            .with_truncation(Some(tokenizers::TruncationParams { max_length: LONGEST_PREMISE + 32, strategy: tokenizers::TruncationStrategy::OnlyFirst, ..Default::default() }))
            .unwrap();
        Deberta { model, tokenizer, entailment, contradiction }
    }
}

impl Nli for Deberta {
    fn read(&self, premise: &str, hypothesis: &str) -> Read {
        let encoding = self.tokenizer.encode((premise, hypothesis), true).unwrap();
        let ids = Tensor::new(encoding.get_ids(), &Device::Cpu).unwrap().unsqueeze(0).unwrap();
        let logits = self.model.forward(&ids, None, None).unwrap().squeeze(0).unwrap().to_vec1::<f32>().unwrap();
        Read { probs: softmax(&logits), entailment: self.entailment, contradiction: self.contradiction }
    }
}

struct Example {
    id: String,
    fresh: bool,
    y: bool,
    text: String,
}

fn examples() -> Vec<Example> {
    let dir = PathBuf::from(std::env::var_os("HOME").unwrap()).join("Library/Application Support/Armada/training/reply-approves");
    let mut out = Vec::new();
    for part in ["train", "fresh"] {
        let text = std::fs::read_to_string(dir.join(format!("{part}.tsv"))).unwrap();
        for line in text.lines().skip(1) {
            let f: Vec<&str> = line.splitn(7, '\t').collect();
            out.push(Example { id: f[0].to_string(), fresh: part == "fresh", y: f[4] == "approves", text: f[6].replace("\\n", "\n") });
        }
    }
    out
}

fn auc(scores: &[f32], ys: &[bool]) -> f64 {
    let (mut sum, mut pairs) = (0.0, 0usize);
    for (p, _) in scores.iter().zip(ys).filter(|(_, y)| **y) {
        for (n, _) in scores.iter().zip(ys).filter(|(_, y)| !**y) {
            sum += if p > n { 1.0 } else if p == n { 0.5 } else { 0.0 };
            pairs += 1;
        }
    }
    if pairs == 0 { f64::NAN } else { sum / pairs as f64 }
}

fn short(text: &str) -> String {
    let flat: String = text.chars().map(|c| if c == '\n' { ' ' } else { c }).collect();
    if flat.chars().count() > 100 { format!("{}...", flat.chars().take(100).collect::<String>()) } else { flat }
}

fn report(title: &str, scores: &[f32], ex: &[Example]) {
    let ys: Vec<bool> = ex.iter().map(|e| e.y).collect();
    let fresh: Vec<bool> = ex.iter().map(|e| e.fresh).collect();
    let part = |want: bool| -> (Vec<f32>, Vec<bool>) {
        scores.iter().zip(&ys).zip(&fresh).filter(|(_, f)| **f == want).map(|((s, y), _)| (*s, *y)).unzip()
    };
    let (ts, ty) = part(false);
    let (fs, fy) = part(true);
    println!("\n### {title}");
    println!("AUC all {:.3} ({} replies, {} approve) | train {:.3} ({} approve) | fresh {:.3} ({} approve)",
        auc(scores, &ys), ys.len(), ys.iter().filter(|y| **y).count(), auc(&ts, &ty), ty.iter().filter(|y| **y).count(), auc(&fs, &fy), fy.iter().filter(|y| **y).count());
    let mut order: Vec<usize> = (0..scores.len()).collect();
    order.sort_by(|a, b| scores[*b].partial_cmp(&scores[*a]).unwrap());
    println!("threshold | called | right | precision | recall (all 179) || fresh: called right");
    for t in [0.05f32, 0.1, 0.2, 0.3, 0.5, 0.7, 0.8, 0.9, 0.95, 0.99] {
        let called = ex.iter().enumerate().filter(|(i, _)| scores[*i] >= t).count();
        let right = ex.iter().enumerate().filter(|(i, e)| scores[*i] >= t && e.y).count();
        let fc = ex.iter().enumerate().filter(|(i, e)| e.fresh && scores[*i] >= t).count();
        let fr = ex.iter().enumerate().filter(|(i, e)| e.fresh && e.y && scores[*i] >= t).count();
        let precision = if called > 0 { format!("{:.2}", right as f64 / called as f64) } else { "-".into() };
        println!("{t:.2} | {called} | {right} | {precision} | {right}/{} || {fc} {fr}", ys.iter().filter(|y| **y).count());
    }
    let mut best = None;
    let mut right = 0;
    for (k, i) in order.iter().enumerate() {
        if ex[*i].y { right += 1; }
        if right >= 1 && right as f64 / (k + 1) as f64 >= 0.9 { best = Some((scores[*i], k + 1, right)); }
    }
    match best {
        Some((t, called, right)) => println!("precision >= 0.9 with an approval caught: threshold {t:.4}, {called} called, {right} right"),
        None => println!("precision >= 0.9 with an approval caught: no threshold"),
    }
    println!("approvals by rank (of {}):", scores.len());
    for (k, i) in order.iter().enumerate().filter(|(_, i)| ex[**i].y) {
        println!("  rank {:>3}  score {:.4}  \"{}\"", k + 1, scores[*i], short(&ex[*i].text));
    }
    println!("top 8:");
    for i in order.iter().take(8) {
        println!("  {:.4} {}  \"{}\"", scores[*i], if ex[*i].y { "APPROVES" } else { "not     " }, short(&ex[*i].text));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (kind, folder) = (args[1].as_str(), PathBuf::from(&args[2]));
    let out = args.get(3).map(PathBuf::from);
    let scorer: Box<dyn Nli> = match kind {
        "bert" => Box::new(Bert::load(&folder)),
        "deberta-nli-small" => Box::new(Deberta::load(&DEBERTA_NLI_SMALL, &folder)),
        "deberta-zeroshot" => Box::new(Deberta::load(&DEBERTA_ZEROSHOT, &folder)),
        other => panic!("unknown model kind {other}"),
    };
    // Checks the plumbing, not the question: pairs whose answer is plain.
    for (premise, hypothesis) in [
        ("A man is playing a guitar on stage.", "A person is playing an instrument."),
        ("A man is playing a guitar on stage.", "Nobody is playing music."),
        ("Yes, go ahead and merge it.", "The person tells the agent to go ahead."),
        ("No, stop, do not merge it.", "The person tells the agent to go ahead."),
    ] {
        let r = scorer.read(premise, hypothesis);
        println!("sanity: P(entailment) {:.3} for \"{premise}\" / \"{hypothesis}\"", r.probs[r.entailment]);
    }
    let ex = examples();
    println!("model {} | {} replies scored (no training, so all of them)", folder.display(), ex.len());
    let mut elapsed = Vec::new();
    for (h, hypothesis) in HYPOTHESES.iter().enumerate() {
        let (mut entail, mut versus) = (Vec::new(), Vec::new());
        for e in &ex {
            let started = Instant::now();
            let r = scorer.read(&e.text, hypothesis);
            elapsed.push(started.elapsed().as_secs_f64() * 1000.0);
            entail.push(r.probs[r.entailment]);
            versus.push(match r.contradiction {
                Some(c) => r.probs[r.entailment] / (r.probs[r.entailment] + r.probs[c]),
                None => r.probs[r.entailment],
            });
        }
        report(&format!("H{} \"{hypothesis}\" score = P(entailment)", h + 1), &entail, &ex);
        if scorer_has_contradiction(&*scorer) {
            report(&format!("H{} score = entailment against contradiction", h + 1), &versus, &ex);
        }
        if let Some(dir) = &out {
            let rows: Vec<String> = ex.iter().enumerate().map(|(i, e)| format!("{}\t{}\t{}\t{:.5}\t{:.5}\t{}", e.id, e.fresh, e.y, entail[i], versus[i], short(&e.text))).collect();
            std::fs::write(dir.join(format!("scores-h{}.tsv", h + 1)), rows.join("\n")).unwrap();
        }
    }
    elapsed.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("\ntime per reply (one pair, one at a time, CPU): median {:.1} ms, p95 {:.1} ms, mean {:.1} ms",
        elapsed[elapsed.len() / 2], elapsed[elapsed.len() * 95 / 100], elapsed.iter().sum::<f64>() / elapsed.len() as f64);
}

fn scorer_has_contradiction(scorer: &dyn Nli) -> bool {
    scorer.read("ok", "ok").contradiction.is_some()
}
