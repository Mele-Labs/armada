// The part of whisper.cpp's `whisper.h` the helper calls, copied from release b5454 (MIT, Copyright (c) 2023-2026 The
// ggml authors) so the helper builds before anything is downloaded. Install fetches that same release, pinned by
// digest in `src/main/whisper-install.ts`. `whisper_full` takes its params by value, so these layouts must match the
// downloaded library byte for byte: moving the pin means copying these structs again from the new `whisper.h`.

#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

struct whisper_context;
struct whisper_state;
typedef int32_t whisper_token;

typedef struct whisper_ahead {
  int n_text_layer;
  int n_head;
} whisper_ahead;

typedef struct whisper_aheads {
  size_t n_heads;
  const whisper_ahead *heads;
} whisper_aheads;

struct whisper_context_params {
  bool use_gpu;
  bool flash_attn;
  int gpu_device;
  bool dtw_token_timestamps;
  int dtw_aheads_preset; // enum whisper_alignment_heads_preset
  int dtw_n_top;
  struct whisper_aheads dtw_aheads;
  size_t dtw_mem_size;
};

typedef struct whisper_vad_params {
  float threshold;
  int min_speech_duration_ms;
  int min_silence_duration_ms;
  float max_speech_duration_s;
  int speech_pad_ms;
  float samples_overlap;
} whisper_vad_params;

typedef void *whisper_callback; // every callback slot below is a function pointer, left as the defaults set it

struct whisper_full_params {
  int strategy; // enum whisper_sampling_strategy: 0 greedy
  int n_threads;
  int n_max_text_ctx;
  int offset_ms;
  int duration_ms;
  bool translate;
  bool no_context;
  bool no_timestamps;
  bool single_segment;
  bool print_special;
  bool print_progress;
  bool print_realtime;
  bool print_timestamps;
  bool token_timestamps;
  float thold_pt;
  float thold_ptsum;
  int max_len;
  bool split_on_word;
  int max_tokens;
  bool debug_mode;
  int audio_ctx;
  bool tdrz_enable;
  const char *suppress_regex;
  const char *initial_prompt;
  bool carry_initial_prompt;
  const whisper_token *prompt_tokens;
  int prompt_n_tokens;
  const char *language;
  bool detect_language;
  bool suppress_blank;
  bool suppress_nst;
  float temperature;
  float max_initial_ts;
  float length_penalty;
  float temperature_inc;
  float entropy_thold;
  float logprob_thold;
  float no_speech_thold;
  struct { int best_of; } greedy;
  struct { int beam_size; float patience; } beam_search;
  whisper_callback new_segment_callback;
  void *new_segment_callback_user_data;
  whisper_callback progress_callback;
  void *progress_callback_user_data;
  whisper_callback encoder_begin_callback;
  void *encoder_begin_callback_user_data;
  whisper_callback abort_callback;
  void *abort_callback_user_data;
  whisper_callback logits_filter_callback;
  void *logits_filter_callback_user_data;
  const void **grammar_rules;
  size_t n_grammar_rules;
  size_t i_start_rule;
  float grammar_penalty;
  bool vad;
  const char *vad_model_path;
  whisper_vad_params vad_params;
};
