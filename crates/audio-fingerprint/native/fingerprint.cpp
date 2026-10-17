#include "fingerprint.h"
#include <algorithm>
#include <cerrno>
#include <cstdio>
#include <exception>
#include <limits>
#include <memory>
#include <chromaprint.h>
#include <audio/ffmpeg_audio_reader.h>

namespace {
enum Status { Ok = 0, Decode = 1, TooShort = 2, InvalidDuration = 3, Interrupted = 4, Internal = 5 };

int interrupted(void *user) {
    const auto *source = static_cast<const OpentagAudioSource *>(user);
    return source->interrupted(source->user);
}
int read(void *user, uint8_t *data, int length) {
    const auto *source = static_cast<const OpentagAudioSource *>(user);
    if (source->interrupted(source->user)) return AVERROR_EXIT;
    const int count = source->read(source->user, data, length);
    if (count == 0) return AVERROR_EOF;
    if (count < 0 || count > length) return AVERROR(EIO);
    return count;
}
int64_t seek(void *user, int64_t offset, int whence) {
    const auto *source = static_cast<const OpentagAudioSource *>(user);
    if (source->interrupted(source->user)) return AVERROR_EXIT;
    const int64_t position = source->seek(source->user, offset, whence & ~AVSEEK_FORCE);
    return position < 0 ? AVERROR(EIO) : position;
}
struct Input {
    AVIOContext *io = nullptr;
    ~Input() {
        if (io) av_freep(&io->buffer);
        avio_context_free(&io);
    }
};
}

extern "C" int opentag_audio_fingerprint(const OpentagAudioSource *source, OpentagAudioResult *result) noexcept {
    if (!result) return Internal;
    *result = {};
    auto fail = [result](Status status, const char *message) {
        result->status = status;
        std::snprintf(result->error, sizeof(result->error), "%s", message);
        return static_cast<int>(status);
    };
    if (!source || !source->read || !source->seek || !source->interrupted || !source->format_hint) {
        return fail(Internal, "Invalid local audio source callbacks");
    }
    try {
        using Context = std::unique_ptr<ChromaprintContext, decltype(&chromaprint_free)>;
        Context ctx(chromaprint_new(CHROMAPRINT_ALGORITHM_TEST2), chromaprint_free);
        if (!ctx) return fail(Internal, "Could not create the fingerprinter");
        Input input;
        auto *buffer = static_cast<uint8_t *>(av_malloc(32 * 1024));
        if (!buffer) return fail(Internal, "Could not allocate the audio input buffer");
        // Only this read-only callback source is available. The bundled FFmpeg
        // build has no protocols or networking and cannot open external URLs.
        input.io = avio_alloc_context(buffer, 32 * 1024, 0,
                                     const_cast<OpentagAudioSource *>(source), read, nullptr, seek);
        if (!input.io) {
            av_free(buffer);
            return fail(Internal, "Could not create the audio input buffer");
        }
        chromaprint::FFmpegAudioReader reader;
        reader.SetOutputSampleRate(chromaprint_get_sample_rate(ctx.get()));
        reader.SetOutputChannels(chromaprint_get_num_channels(ctx.get()));
        AVIOInterruptCB interrupt = {interrupted, const_cast<OpentagAudioSource *>(source)};
        if (interrupted(const_cast<OpentagAudioSource *>(source))) return fail(Interrupted, "Fingerprinting stopped");
        if (!reader.Open(source->format_hint, input.io, interrupt)) return fail(Decode, reader.GetError().c_str());
        if (!chromaprint_start(ctx.get(), reader.GetSampleRate(), reader.GetChannels())) {
            return fail(Internal, "Could not start fingerprinting");
        }
        const auto limit = 120 * static_cast<size_t>(reader.GetSampleRate());
        size_t total = 0;
        while (!reader.IsFinished()) {
            if (interrupted(const_cast<OpentagAudioSource *>(source))) return fail(Interrupted, "Fingerprinting stopped");
            const int16_t *samples = nullptr;
            size_t frames = 0;
            if (!reader.Read(&samples, &frames)) return fail(Decode, reader.GetError().c_str());
            const bool done = frames > limit - total;
            frames = std::min(frames, limit - total);
            total += frames;
            const auto channels = static_cast<size_t>(reader.GetChannels());
            if (!channels || frames > static_cast<size_t>(std::numeric_limits<int>::max()) / channels) {
                return fail(Decode, "Invalid decoded audio frame size");
            }
            if (frames && !chromaprint_feed(ctx.get(), samples, static_cast<int>(frames * channels))) {
                return fail(Internal, "Could not fingerprint the audio frame");
            }
            if (done) break;
        }
        if (interrupted(const_cast<OpentagAudioSource *>(source))) return fail(Interrupted, "Fingerprinting stopped");
        if (!chromaprint_finish(ctx.get())) return fail(Internal, "Could not finish fingerprinting");
        result->processed_frames = total;
        result->duration_ms = reader.GetDuration();
        int raw_items = 0;
        if (!chromaprint_get_raw_fingerprint_size(ctx.get(), &raw_items)) return fail(Internal, "Could not get the fingerprint size");
        if (!raw_items) return fail(TooShort, "Not enough audio to fingerprint");
        if (result->duration_ms <= 0) return fail(InvalidDuration, "The full audio duration is unavailable");
        if (!chromaprint_get_fingerprint(ctx.get(), &result->fingerprint)) return fail(Internal, "Could not encode the fingerprint");
        result->status = Ok;
        return Ok;
    } catch (const std::exception &error) {
        return fail(Internal, error.what());
    } catch (...) {
        return fail(Internal, "Unexpected native fingerprint error");
    }
}

extern "C" void opentag_audio_result_free(OpentagAudioResult *result) noexcept {
    if (result) {
        chromaprint_dealloc(result->fingerprint);
        result->fingerprint = nullptr;
    }
}

extern "C" void opentag_audio_info(OpentagAudioInfo *info) noexcept {
    if (!info) return;
    *info = {};
    info->chromaprint = chromaprint_get_version();
    info->ffmpeg = av_version_info();
    info->license = avcodec_license();
    void *iterator = nullptr;
    while (const AVCodec *codec = av_codec_iterate(&iterator)) {
        if (av_codec_is_decoder(codec)) {
            if (codec->type == AVMEDIA_TYPE_AUDIO) ++info->audio_decoders;
            else ++info->non_audio_decoders;
        }
    }
    iterator = nullptr;
    while (avio_enum_protocols(&iterator, 0)) ++info->protocols;
}
