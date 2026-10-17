import array
import math
import sys

def pcm(rate, channels, seconds):
    """Deterministic chirps/chords with a changing amplitude envelope."""
    result = array.array('h')
    for frame in range(round(rate * seconds)):
        t = frame / rate
        for channel in range(channels):
            phase = channel * 0.37
            envelope = 0.4 + 0.3 * math.sin(t * 1.31 + phase)
            value = envelope * (
                10000 * math.sin(2 * math.pi * (120 * t + 6 * t * t) + phase)
                + 7000 * math.sin(2 * math.pi * (330 + 17 * channel) * t)
                + 4000 * math.sin(2 * math.pi * (871 * t + 2 * t * t))
            )
            result.append(round(max(-32768, min(32767, value))))
    if sys.byteorder != 'little':
        result.byteswap()
    return result
