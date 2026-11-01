#include "voice-native.h"

#include <dlfcn.h>
#include <mach-o/dyld.h>
#include <spawn.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "whisper-abi.h"

// Private but stable since macOS 10.14; terminals and editors spawn their shells with it for the same reason.
extern int responsibility_spawnattrs_setdisclaim(posix_spawnattr_t *attrs, int disclaim);
extern char **environ;

static const char *const OWN = "ARMADA_VOICE_OWN";

int voice_run_disclaimed(char *const argv[]) {
  if (getenv(OWN) != NULL) return -1;
  setenv(OWN, "1", 1);
  posix_spawnattr_t attrs;
  posix_spawnattr_init(&attrs);
  // SETEXEC replaces this process rather than starting a second, so Bridge's pid and its signals reach the helper.
  posix_spawnattr_setflags(&attrs, POSIX_SPAWN_SETEXEC);
  responsibility_spawnattrs_setdisclaim(&attrs, 1);
  char path[4096];
  uint32_t size = sizeof path;
  if (_NSGetExecutablePath(path, &size) != 0) return 1;
  pid_t pid;
  int failed = posix_spawn(&pid, path, NULL, &attrs, argv, environ);
  // Only reached when the exec failed.
  posix_spawnattr_destroy(&attrs);
  return failed;
}

typedef struct whisper_context_params *(*context_defaults_fn)(void);
typedef struct whisper_context *(*init_fn)(const char *, struct whisper_context_params);
typedef struct whisper_full_params *(*full_defaults_fn)(int);
typedef int (*full_fn)(struct whisper_context *, struct whisper_full_params, const float *, int);
typedef int (*segments_fn)(struct whisper_context *);
typedef const char *(*segment_text_fn)(struct whisper_context *, int);
typedef void (*free_fn)(struct whisper_context *);
typedef void (*log_set_fn)(void (*)(int, const char *, void *), void *);

static void quiet(int level, const char *text, void *data) { (void)level; (void)text; (void)data; }

char *voice_whisper(const char *library, const char *model, const float *samples, size_t count, const char **why) {
  void *lib = dlopen(library, RTLD_NOW | RTLD_LOCAL);
  if (lib == NULL) return (*why = "whisper_library_unloadable", NULL);
  context_defaults_fn context_defaults = (context_defaults_fn)dlsym(lib, "whisper_context_default_params_by_ref");
  init_fn init = (init_fn)dlsym(lib, "whisper_init_from_file_with_params");
  full_defaults_fn full_defaults = (full_defaults_fn)dlsym(lib, "whisper_full_default_params_by_ref");
  full_fn full = (full_fn)dlsym(lib, "whisper_full");
  segments_fn segments = (segments_fn)dlsym(lib, "whisper_full_n_segments");
  segment_text_fn segment_text = (segment_text_fn)dlsym(lib, "whisper_full_get_segment_text");
  free_fn release = (free_fn)dlsym(lib, "whisper_free");
  log_set_fn log_set = (log_set_fn)dlsym(lib, "whisper_log_set");
  if (!context_defaults || !init || !full_defaults || !full || !segments || !segment_text || !release) {
    return (*why = "whisper_library_unexpected", NULL);
  }
  if (log_set) log_set(quiet, NULL);

  struct whisper_context *ctx = init(model, *context_defaults());
  if (ctx == NULL) return (*why = "whisper_model_unloadable", NULL);
  struct whisper_full_params params = *full_defaults(0);
  long cores = sysconf(_SC_NPROCESSORS_ONLN);
  params.n_threads = cores > 8 ? 8 : (int)(cores < 1 ? 1 : cores);
  params.language = "en";
  params.no_timestamps = true;
  params.print_progress = false;
  params.print_realtime = false;
  params.print_timestamps = false;
  params.print_special = false;
  if (full(ctx, params, samples, (int)count) != 0) {
    release(ctx);
    return (*why = "whisper_failed", NULL);
  }
  size_t length = 0;
  int n = segments(ctx);
  for (int at = 0; at < n; at++) length += strlen(segment_text(ctx, at));
  char *text = calloc(length + 1, 1);
  for (int at = 0; at < n; at++) strcat(text, segment_text(ctx, at));
  release(ctx);
  return text;
}
