/* Focused source-policy probe: compile with patched FFmpeg headers/objects.
 * This is not a complete decoder or release integration test. */
#include "config.h"
#if CONFIG_GPL || defined(NDEBUG)
#error This probe requires the no-GPL profile and active assertions
#endif
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include "libavutil/crc.h"
#include "libavcodec/ac3dsp.h"

/* Independent bit-at-a-time reference, adapted to av_crc's byte representation. */
static uint32_t bit_crc(const uint8_t *buf, size_t n, int bits, uint32_t poly, int le)
{
    uint32_t value = 0, mask = bits == 32 ? UINT32_MAX : ((1u << bits) - 1);
    for (size_t j = 0; j < n; j++) {
        value ^= le ? buf[j] : ((uint32_t)buf[j] << (bits - 8));
        for (int k = 0; k < 8; k++) {
            if (le)
                value = (value >> 1) ^ ((value & 1) ? poly : 0);
            else
                value = ((value << 1) ^ ((value & (1u << (bits - 1))) ? poly : 0)) & mask;
        }
    }
    if (!le) {
        value <<= 32 - bits;
        value = ((value & 255) << 24) | ((value & 0xff00) << 8)
              | ((value >> 8) & 0xff00) | ((value >> 24) & 255);
    }
    return value;
}

int main(void)
{
    uint8_t buf[1999];
    for (int i = 0; i < 1999; i++)
        buf[i] = i + i * i;
    const unsigned cases[8][4] = {
        { AV_CRC_32_IEEE_LE, 32, 0xedb88320, 1 },
        { AV_CRC_32_IEEE,    32, 0x04c11db7, 0 },
        { AV_CRC_24_IEEE,    24, 0x864cfb,   0 },
        { AV_CRC_16_ANSI_LE, 16, 0xa001,     1 },
        { AV_CRC_16_ANSI,    16, 0x8005,     0 },
        { AV_CRC_8_ATM,       8, 7,          0 },
        { AV_CRC_8_EBU,       8, 0x1d,       0 },
        { AV_CRC_16_CCITT,   16, 0x1021,     0 },
    };
    const size_t lengths[4] = { 0, 1, 31, 1996 };
    for (int i = 0; i < 8; i++) {
        const AVCRC *table = av_crc_get_table(cases[i][0]);
        AVCRC custom[1024];
        assert(table && table[0] == 0);
        assert(av_crc_init(custom, cases[i][3], cases[i][1], cases[i][2], sizeof(custom)) == 0);
        for (int offset = 0; offset < 4; offset++) {
            for (int j = 0; j < 4; j++) {
                uint32_t expected = bit_crc(buf + offset, lengths[j], cases[i][1], cases[i][2], cases[i][3]);
                assert(av_crc(table, 0, buf + offset, lengths[j]) == expected);
                assert(av_crc(custom, 0, buf + offset, lengths[j]) == expected);
                size_t half = lengths[j] / 2;
                uint32_t part = av_crc(table, 0, buf + offset, half);
                assert(av_crc(table, part, buf + offset + half, lengths[j] - half) == expected);
            }
        }
    }

    AC3DSPContext context = { 0 };
    ff_ac3dsp_init(&context);
    float samples[32];
    int32_t fixed[32];
    for (int i = 0; i < 32; i++)
        samples[i] = (i - 16) / 32.0f;
    context.float_to_fixed24(fixed, samples, 32);
    for (int i = 0; i < 32; i++)
        assert(fixed[i] == (i - 16) * (1 << 19));

    uint8_t exponents[3 * 256];
    for (int i = 0; i < 256; i++) {
        exponents[i] = 20;
        exponents[256 + i] = i % 24;
        exponents[512 + i] = 15;
    }
    context.ac3_exponent_min(exponents, 2, 256);
    for (int i = 0; i < 256; i++)
        assert(exponents[i] == (i % 24 < 15 ? i % 24 : 15));

    const int32_t left[4] = { 1, 2, 3, 4 }, right[4] = { 4, 3, 2, 1 };
    int64_t isum[4];
    context.sum_square_butterfly_int32(isum, left, right, 4);
    assert(isum[0] == 30 && isum[1] == 30 && isum[2] == 100 && isum[3] == 20);
    const float leftf[4] = { 1, 2, 3, 4 }, rightf[4] = { 4, 3, 2, 1 };
    float fsum[4];
    context.sum_square_butterfly_float(fsum, leftf, rightf, 4);
    assert(fsum[0] == 30 && fsum[1] == 30 && fsum[2] == 100 && fsum[3] == 20);
    int32_t coefficients[4] = { 0, 1, 2, 4 };
    uint8_t extracted[4];
    context.extract_exponents(extracted, coefficients, 4);
    assert(extracted[0] == 24 && extracted[1] == 23 && extracted[2] == 22 && extracted[3] == 21);
    puts("128 independent CRC vectors, custom-table and streaming paths, and 5 generic AC3 DSP operations pass");
    return 0;
}
