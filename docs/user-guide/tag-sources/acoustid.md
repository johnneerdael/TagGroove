# AcoustID fingerprints

AcoustID identifies recordings from the sound of your files. OpenTag computes
a fingerprint locally, asks AcoustID for matches, then retrieves the chosen
release's metadata and artwork from MusicBrainz.

## Set up fingerprinting

1. Use a desktop build on macOS, Windows or Linux. This source is not offered
   on Android or iOS.
2. Install Chromaprint's `fpcalc` separately. OpenTag does not bundle it.
   Obtain it from the [Chromaprint project](https://acoustid.org/chromaprint)
   or your package manager.
3. If OpenTag cannot find it, enter the executable's full path in **Options →
   Tag Sources → Online services → fpcalc program**. Leave the setting empty
   for automatic discovery on the executable search path and common Unix
   installation locations.
4. Open **Tag Sources → API keys…**. If this build has no AcoustID key, add
   your application's key using the registration address shown there.

Enter an executable path in **fpcalc program**, not a shell command or extra
arguments. Files must be accessible and decodable by that `fpcalc` installation.
A fingerprint process taking more than 2 minutes is stopped and reported.

## Identify and review

1. Select the album's files and choose **Tag Sources → AcoustID (fingerprints)…**.
2. Select **Search**. OpenTag fingerprints each file and ranks releases by
   the number of selected files matched, then match score.
3. Select the correct release and choose **Next**. Compare the edition,
   track list, matches and artwork, then [save](index.md#search-review-and-save).

Identified files are paired with their matched MusicBrainz recordings,
including across discs. If files have overlapping candidate recordings,
OpenTag reassigns compatible pairs when necessary to keep the greatest number
of exact fingerprint matches. A direct free track keeps earlier assignments
stable; this does not globally optimize match scores. **Auto-match**, **Selection order** and **Suggest
sorting** arrange remaining files without replacing those fingerprint pairs.
You can still move files manually; inspect the resulting assignments.

`ACOUSTID_ID` is offered per file only when that file was identified as the
recording of its assigned track. Moving it onto another recording does not
copy another file's AcoustID. If only `ACOUSTID_ID` is checked and no ID
applies or cover is selected, the file is skipped.

## Limits and privacy

A match identifies a recording, not necessarily your exact edition. Several
releases can contain it, so the highest-ranked result still needs review.
Files without a usable match do not gain an inferred AcoustID.

OpenTag sends fingerprints and durations, not audio files. It performs
lookups only and does not submit new fingerprints. AcoustID requests are
spaced about 350 milliseconds apart; MusicBrainz lookups use a separate
spacing of about 1.1 seconds. Local fingerprinting adds to the time needed.

## Related

- [API key setup](index.md#api-keys)
- [Search, review and save](index.md#search-review-and-save)
- [Tag Sources options](../reference/options/tag-sources.md)
