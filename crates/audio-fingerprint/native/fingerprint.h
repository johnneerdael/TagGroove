#pragma once
#include <stddef.h>
#include <stdint.h>

// Callbacks and user data remain owned by the Rust Job for this synchronous
// call. No callback is retained or invoked from another native thread.
struct OpentagAudioSource {
    void *user;
    int (*read)(void *, uint8_t *, int);
    int64_t (*seek)(void *, int64_t, int);
    int (*interrupted)(void *);
    const char *format_hint;
};
struct OpentagAudioResult {
    int status;
    int64_t duration_ms;
    uint64_t processed_frames;
    char *fingerprint;
    char error[512];
};
extern "C" int opentag_audio_fingerprint(const OpentagAudioSource *, OpentagAudioResult *) noexcept;
extern "C" void opentag_audio_result_free(OpentagAudioResult *) noexcept;
struct OpentagAudioInfo {
    const char *chromaprint;
    const char *ffmpeg;
    const char *license;
    uint32_t audio_decoders;
    uint32_t non_audio_decoders;
    uint32_t protocols;
};
extern "C" void opentag_audio_info(OpentagAudioInfo *) noexcept;
