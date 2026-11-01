// What `main.swift` calls in C: the respawn that makes the helper answer for its own permissions, and whisper.cpp
// loaded from wherever Install put it.

#pragma once

#include <stddef.h>

/**
 * Replaces this process with this executable again, as a process macOS holds responsible for itself. Returns -1 where
 * this already is that process, and anything else only when the replacing failed. Without it the microphone and speech prompts are asked for whatever launched Bridge,
 * which in development is `node` and carries no usage strings.
 */
int voice_run_disclaimed(char *const argv[]);

/**
 * Transcribes 16 kHz mono samples with the whisper.cpp library at `library` and the model at `model`. Returns the
 * text, which the caller frees, or NULL with `why` set to a static reason.
 */
char *voice_whisper(const char *library, const char *model, const float *samples, size_t count, const char **why);
