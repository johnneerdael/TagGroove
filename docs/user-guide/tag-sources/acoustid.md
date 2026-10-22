# AcoustID fingerprints

AcoustID identifies recordings from the sound of your files. TagGroove computes
a fingerprint locally, asks AcoustID for matches, then retrieves the chosen
release's metadata and artwork from MusicBrainz.

<!-- figure:acoustid-form -->
<figure markdown>
![The AcoustID dialog that fingerprints the selected file and looks it up](../assets/screens/acoustid-form-light.png#only-light)
![The AcoustID dialog that fingerprints the selected file and looks it up](../assets/screens/acoustid-form-dark.png#only-dark)
</figure>
<!-- /figure -->

Use [Compare metadata…](index.md#compare-fields-from-several-sources) to mix this
source's fields with other providers for one track or an album. In Album mode,
one provider choice per field uses each track's own matched values. Inherit keeps
existing values; None explicitly removes them.

## Set up fingerprinting

TagGroove includes Chromaprint and its audio decoder libraries. Install TagGroove
normally: no separate `fpcalc` or `ffmpeg` program is required. Fingerprinting
runs on your device and reads the selected audio without changing its tags.

1. Add the album's audio files to TagGroove. On Android, choose documents with
   the system picker so TagGroove has permission to read them.
2. Open **Tag Sources → API keys…**. If this build has no AcoustID key, add
   your application's key using the registration address shown there.
3. Select the files to identify. Files need readable, supported audio; an
   encrypted or damaged file cannot be identified just because its tags load.

Advanced desktop users can optionally choose a different producer in
**Options → Tag Sources → Online services → Optional fpcalc override
(desktop)**. Enter a full executable path, without a shell command or extra
arguments. Leave it empty to use the built-in libraries. Old automatic
`fpcalc` settings also use the built-in default. Mobile builds always use the
built-in producer.

## What happens during fingerprinting

TagGroove decodes up to the first two minutes of each file to calculate its
fingerprint and sends the fingerprint with the full file duration. The
reported container duration can differ slightly from the audible duration.
It does not upload your audio. Processing a long file therefore does not
require fingerprinting the entire recording.

Cancel stops the search and asks local decoding to stop. The search also has
a two-minute time budget for each file. A blocked file read or decoder can
take longer to return; TagGroove allows only one built-in fingerprint worker
at a time and asks you to retry if the previous worker is still stopping.
The optional external producer has its own process timeout.

## Identify and review

1. Select the album's files and choose **Tag Sources → AcoustID (fingerprints)…**.
2. Select **Search**. TagGroove fingerprints each file and ranks releases by
   the number of selected files matched, then match score.
3. Select the correct release and choose **Next**. Compare the edition,
   track list, matches and artwork, then [save](index.md#search-review-and-save).

Identified files are paired with their matched MusicBrainz recordings,
including across discs. If files have overlapping candidate recordings,
TagGroove reassigns compatible pairs when necessary to keep the greatest number
of compatible fingerprint assignments. A direct free track keeps earlier
assignments stable; this does not globally optimize match scores.
**Auto-match**, **Selection order** and **Suggest sorting** arrange remaining files without replacing those fingerprint pairs.
You can still move files manually; inspect the resulting assignments.

`ACOUSTID_ID` is offered per file only when that file was identified as the
recording of its assigned track. Moving it onto another recording does not
copy another file's AcoustID. If only `ACOUSTID_ID` is checked and no ID
applies or cover is selected, the file is skipped.

## Limits and privacy

A match identifies a recording, not necessarily your exact edition. Several
releases can contain it, so the highest-ranked result still needs review.
Files without a usable match do not gain an inferred AcoustID.

TagGroove sends fingerprints and durations, not audio files. It performs
lookups only and does not submit new fingerprints. AcoustID requests are
spaced about 350 milliseconds apart; MusicBrainz lookups use a separate
spacing of about 1.1 seconds. Local fingerprinting adds to the time needed.

## Related

- [API keys](api-keys.md)
- [Compare metadata from several sources](compare.md)
- [Search, review and save](index.md#search-review-and-save)
- [Tag Sources options](../reference/options/tag-sources.md)
