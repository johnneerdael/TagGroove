# Synthetic fingerprint compatibility corpus

These fourteen audio fixtures contain generated chirps, chords and amplitude envelopes, with no third-party recordings. `carrier-results.json` binds lossless carriers, decoded input hashes, the original generator and the separately captured Chromaprint 1.6.2 reference executable/library identities. `reference-results.json` retains the original capture record. Homebrew media tools generated/reference-tested these fixtures only; they are not linked or shipped as application dependencies.

Tests decompress carriers with an independent gzip implementation, verify their original hashes, then run OpenTag's actual embedded Rust API. The same expected fingerprints exercise Apple's vDSP and the bundled KissFFT backend on other CI targets. Normal tests require no `fpcalc` or `ffmpeg` program. The retained Python generators document regeneration and require developer media tools; regenerating fixtures does not establish compatibility without a fresh independent reference capture.

The project-authored synthetic audio inputs and generator code are distributed under OpenTag's MIT license.
