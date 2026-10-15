//! Subsongs: chapters and cue sheet tracks listed as rows of their own
//! (Options → Tags → Advanced → List chapters as separate files,
//! https://docs.mp3tag.de/options/tags/advanced/).
//!
//! A row is addressed by its file path, a U+0001 separator and the 1-based
//! subsong number, which no real path contains on Windows and practically
//! none elsewhere. Rows show the file's fields overlaid with the chapter's
//! own (its TITLE and, where the format stores them, further fields) and
//! offer `%_subsong%` (0-based), `%_subsong_count%` and the chapter's length.
//! Files need at least two chapters to be listed as rows.
//!
//! Chapters come from MP4 (Nero and QuickTime, with Nero per-chapter tags)
//! and Matroska files (the CHAPTERS field, with chapter-targeted tags),
//! ID3v2 `CHAP` frames (with their sub-frames) and cue sheets embedded in a
//! CUESHEET field; a loaded cue sheet lists its tracks. Saving rows works
//! like Mp3tag's sub-tracks: a field with the same value on every row of a
//! file is a file field, and a field whose rows differ is stored with each
//! chapter instead.
use super::*;
use crate::cue::CueSheet;
use lofty::id3::v2::FrameList;

/// Separates the file path and the subsong number in a row path.
pub const SUBSONG_SEPARATOR: char = '\u{1}';
/// The multi-value field listing MP4 and Matroska chapters.
pub(crate) const CHAPTERS_FIELD: &str = "CHAPTERS";
const CUESHEET_FIELD: &str = "CUESHEET";
pub(crate) const ROW_OPERATION: &str =
    "Chapter rows only support editing tag fields. Turn off Options → Tags → Advanced → List chapters as separate files to work with the whole file";

/// Where a file's chapters are stored.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ChapterSource {
    /// The CHAPTERS field of an MP4 file, with Nero per-chapter tags.
    #[default]
    Mp4,
    /// The CHAPTERS field of a Matroska file, with chapter-targeted tags.
    Matroska,
    /// ID3v2 `CHAP` frames and their sub-frames.
    Id3,
    /// A cue sheet in the CUESHEET field.
    Cue,
}

/// One chapter of a file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChapterInfo {
    /// Start in milliseconds.
    pub start: u64,
    /// End in milliseconds when the file stores one.
    #[serde(default)]
    pub end: Option<u64>,
    pub title: String,
    /// Further fields stored with this chapter.
    #[serde(default, skip_serializing_if = "Fields::is_empty")]
    pub fields: Fields,
    #[serde(skip)]
    pub(crate) source: ChapterSource,
}

/// The row path of subsong `number` (1-based) of `file`.
pub fn subsong_path(file: &str, number: usize) -> String {
    format!("{file}{SUBSONG_SEPARATOR}{number}")
}

/// The file path and subsong number of a row path.
pub fn subsong_parts(path: &str) -> Option<(&str, usize)> {
    let (file, number) = path.rsplit_once(SUBSONG_SEPARATOR)?;
    let number: usize = number.parse().ok()?;
    (number > 0 && !file.is_empty()).then_some((file, number))
}

pub(crate) fn split_path(path: &Path) -> Option<(PathBuf, usize)> {
    let (file, number) = subsong_parts(path.to_str()?)?;
    Some((PathBuf::from(file), number))
}

fn is_cue(path: &Path) -> bool {
    crate::playlist::playlist_kind(path) == Some(crate::playlist::PlaylistKind::Cue)
}

/// "HH:MM:SS.mmm Title" for a chapter starting at `start` nanoseconds.
pub(crate) fn chapter_value(start: u64, title: &str) -> String {
    let ms = start / 1_000_000;
    let time = format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    );
    if title.is_empty() {
        time
    } else {
        format!("{time} {title}")
    }
}

/// Parse "[H:]MM:SS[.fraction] Title" into a start in nanoseconds and a title.
pub(crate) fn parse_chapter_value(value: &str) -> Result<(u64, String), String> {
    let error = || format!("Invalid chapter {value:?}: use HH:MM:SS.mmm followed by the title");
    let trimmed = value.trim_start();
    let (time, title) = trimmed
        .split_once(char::is_whitespace)
        .map_or((trimmed, ""), |(time, title)| (time, title.trim_start()));
    let (clock, fraction) = time.split_once('.').unwrap_or((time, ""));
    let parts: Vec<&str> = clock.split(':').collect();
    if !(2..=3).contains(&parts.len())
        || fraction.len() > 9
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(error());
    }
    let mut seconds = 0u64;
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(error());
        }
        let number: u64 = part.parse().map_err(|_| error())?;
        if index > 0 && number >= 60 {
            return Err(error());
        }
        seconds = seconds
            .checked_mul(60)
            .and_then(|s| s.checked_add(number))
            .ok_or_else(error)?;
    }
    let nanos = if fraction.is_empty() {
        0
    } else {
        format!("{fraction:0<9}")
            .parse::<u64>()
            .map_err(|_| error())?
    };
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|s| s.checked_add(nanos))
        .map(|start| (start, title.to_owned()))
        .ok_or_else(error)
}

/// Write CHAPTERS values in their canonical form, so they read back as written.
pub(crate) fn normalize_chapters(updates: &mut Fields) -> Result<(), String> {
    for (name, values) in updates.iter_mut() {
        if name.eq_ignore_ascii_case(CHAPTERS_FIELD) {
            *values = values
                .iter()
                .map(|value| {
                    parse_chapter_value(value).map(|(start, title)| chapter_value(start, &title))
                })
                .collect::<Result<_, _>>()?;
        }
    }
    Ok(())
}

/// Record `chapters` on `entry`, with `_SUBSONG_COUNT`. As in Mp3tag, a
/// file with fewer than two chapters is a single subsong.
pub(crate) fn attach(entry: &mut AudioEntry, chapters: Vec<ChapterInfo>) {
    if chapters.len() < 2 {
        return;
    }
    entry
        .technical
        .insert("_SUBSONG_COUNT".into(), vec![chapters.len().to_string()]);
    entry.chapters = chapters;
}

fn key<'a>(fields: &'a Fields, name: &str) -> Option<&'a String> {
    fields.keys().find(|k| k.eq_ignore_ascii_case(name))
}
fn put(fields: &mut Fields, name: &str, values: Vec<String>) {
    fields.retain(|k, _| !k.eq_ignore_ascii_case(name));
    if values.iter().any(|v| !v.is_empty()) {
        fields.insert(name.to_string(), values);
    }
}

/// The sub-frames of a `CHAP` frame as a tag of their own.
fn sub_frames(children: &FrameList<'static>) -> Id3v2Tag {
    let mut tag = Id3v2Tag::new();
    for frame in children.iter() {
        tag.insert(frame.clone());
    }
    tag
}

/// Chapters of an ID3v2 tag in start order, with each `CHAP` element ID.
/// Text, URL and comment sub-frames are the chapter's fields (TIT2 is its
/// title); pictures and other sub-frames are kept but not shown.
pub(crate) fn id3_chapters(tag: &Id3v2Tag) -> Vec<(String, ChapterInfo)> {
    let version = tag.original_version();
    let mut chapters: Vec<(String, ChapterInfo)> = tag
        .into_iter()
        .filter_map(|frame| match frame {
            Frame::Chapter(chapter) => {
                let mut fields =
                    mapping::id3_fields_with(&sub_frames(&chapter.children), version, false);
                fields.remove("TITLE");
                Some((
                    chapter.id.to_string(),
                    ChapterInfo {
                        start: u64::from(chapter.times.start),
                        end: (chapter.times.end > chapter.times.start)
                            .then_some(u64::from(chapter.times.end)),
                        title: chapter.title().unwrap_or_default().to_string(),
                        fields,
                        source: ChapterSource::Id3,
                    },
                ))
            }
            _ => None,
        })
        .collect();
    chapters.sort_by_key(|(_, c)| c.start);
    chapters
}

/// Chapters of a cue sheet embedded in a CUESHEET field.
pub(crate) fn embedded_cue_chapters(fields: &Fields) -> Vec<ChapterInfo> {
    let Some(text) = key(fields, CUESHEET_FIELD).and_then(|k| fields[k].first()) else {
        return Vec::new();
    };
    let sheet = CueSheet::parse(text);
    sheet
        .subsongs()
        .into_iter()
        .map(|index| {
            let (start, end) = sheet.track_time(index);
            let mut fields = sheet.own_track_fields(index);
            let title = fields
                .remove("TITLE")
                .map(|t| t.join("; "))
                .unwrap_or_default();
            ChapterInfo {
                start,
                end,
                title,
                fields,
                source: ChapterSource::Cue,
            }
        })
        .collect()
}

/// One row for chapter `index` (0-based) of `parent`.
fn row(parent: &AudioEntry, path: String, index: usize, count: usize) -> AudioEntry {
    let chapter = &parent.chapters[index];
    let duration_ms = (parent.duration * 1000.0).round() as u64;
    let end = chapter
        .end
        .or_else(|| parent.chapters.get(index + 1).map(|next| next.start))
        .unwrap_or(duration_ms)
        .max(chapter.start);
    let mut fields = parent.fields.clone();
    fields.retain(|k, _| {
        !k.eq_ignore_ascii_case(CHAPTERS_FIELD) && !k.eq_ignore_ascii_case(CUESHEET_FIELD)
    });
    for (name, values) in &chapter.fields {
        put(&mut fields, name, values.clone());
    }
    put(&mut fields, "TITLE", vec![chapter.title.clone()]);
    // Mp3tag numbers the sub-tracks of an MP4 file that has no numbers.
    if chapter.source == ChapterSource::Mp4 {
        for (name, value) in [("TRACK", index + 1), ("TOTALTRACKS", count)] {
            if key(&fields, name).is_none() {
                fields.insert(name.into(), vec![value.to_string()]);
            }
        }
    }
    let seconds = (end - chapter.start) as f64 / 1000.0;
    let mut technical = parent.technical.clone();
    let rounded = seconds.round() as u64;
    for (name, value) in [
        ("_SUBSONG", index.to_string()),
        ("_SUBSONG_COUNT", count.to_string()),
        ("_LENGTH_SECONDS", rounded.to_string()),
        ("_LENGTH", format_length(rounded)),
    ] {
        technical.insert(name.into(), vec![value]);
    }
    AudioEntry {
        path,
        filename: parent.filename.clone(),
        format: parent.format.clone(),
        duration: seconds,
        bitrate: parent.bitrate,
        sample_rate: parent.sample_rate,
        channels: parent.channels,
        size: parent.size,
        fields,
        covers: parent.covers.clone(),
        technical,
        chapters: Vec::new(),
    }
}

/// The rows of a file with chapters, in chapter order.
pub fn chapter_rows(parent: &AudioEntry) -> Vec<AudioEntry> {
    let count = parent.chapters.len();
    (0..count)
        .map(|index| row(parent, subsong_path(&parent.path, index + 1), index, count))
        .collect()
}

/// Replace every entry that has chapters by its chapter rows.
pub fn expand_chapter_rows(entries: Vec<AudioEntry>) -> Vec<AudioEntry> {
    entries
        .into_iter()
        .flat_map(|entry| {
            if entry.chapters.is_empty() {
                vec![entry]
            } else {
                chapter_rows(&entry)
            }
        })
        .collect()
}

/// A cue sheet on disk, decoded.
fn read_sheet(path: &Path) -> Result<(CueSheet, String, crate::cue::CueEncoding), String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (text, encoding) = crate::cue::decode(&bytes);
    Ok((CueSheet::parse(&text), text, encoding))
}

/// The audio file of a cue sheet track.
fn cue_audio(cue: &Path, location: Option<&str>) -> Result<PathBuf, String> {
    let location = location.ok_or("The cue sheet track has no FILE line")?;
    let directory = cue.parent().unwrap_or(Path::new("."));
    crate::playlist::entry_candidates(location, directory)?
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| format!("{location}: file not found"))
}

/// The sheet position of subsong `number` (1-based) of a cue sheet.
fn cue_track(sheet: &CueSheet, number: usize) -> Result<usize, String> {
    number
        .checked_sub(1)
        .and_then(|n| sheet.subsongs().get(n).copied())
        .ok_or_else(|| format!("Track {number} is no longer in the cue sheet"))
}

/// The track of a loaded cue sheet as a row.
fn cue_row(
    cue: &Path,
    number: usize,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    let (sheet, _, _) = read_sheet(cue)?;
    let index = cue_track(&sheet, number)?;
    let audio = cue_audio(cue, sheet.tracks[index].file.as_deref())?;
    let mut parent = read_description(&audio, include_cover_data, rules, options)?;
    let (start, end) = sheet.track_time(index);
    let mut fields = sheet.track_fields(index);
    let title = fields
        .remove("TITLE")
        .map(|t| t.join("; "))
        .unwrap_or_default();
    parent.fields = Fields::new();
    let count = sheet.subsongs().len();
    parent.chapters = vec![ChapterInfo {
        start,
        end,
        title,
        fields,
        source: ChapterSource::Cue,
    }];
    let mut entry = row(
        &parent,
        subsong_path(&cue.to_string_lossy(), number),
        0,
        count,
    );
    entry
        .technical
        .insert("_SUBSONG".into(), vec![(number - 1).to_string()]);
    Ok(entry)
}

/// The audio file a path plays from: a chapter row's file, a cue sheet
/// track's FILE, or the path itself.
pub fn subsong_file(path: &Path) -> Result<PathBuf, String> {
    let Some((file, number)) = split_path(path) else {
        return Ok(path.to_path_buf());
    };
    if !is_cue(&file) {
        return Ok(file);
    }
    let (sheet, _, _) = read_sheet(&file)?;
    let index = cue_track(&sheet, number)?;
    cue_audio(&file, sheet.tracks[index].file.as_deref())
}

/// Read one row.
pub(super) fn read_row(
    file: &Path,
    number: usize,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    if is_cue(file) {
        return cue_row(file, number, include_cover_data, rules, options);
    }
    let parent = read_description(file, include_cover_data, rules, options)?;
    if number > parent.chapters.len() {
        return Err(format!(
            "Chapter {number} is no longer in {}",
            parent.filename
        ));
    }
    Ok(row(
        &parent,
        subsong_path(&file.to_string_lossy(), number),
        number - 1,
        parent.chapters.len(),
    ))
}

fn conflict(name: &str, filename: &str) -> String {
    format!(
        "{name} is stored once for {filename}, not per chapter; give its chapter rows the same value"
    )
}

fn refuse_whole_file_fields(name: &str) -> Result<(), String> {
    if name.eq_ignore_ascii_case(CHAPTERS_FIELD) || name.eq_ignore_ascii_case(CUESHEET_FIELD) {
        return Err(format!(
            "{name} cannot be edited on a chapter row; turn off List chapters as separate files to edit it"
        ));
    }
    Ok(())
}

/// The values of `name`, or none when every value is empty.
fn values_of(fields: &Fields, name: &str) -> Vec<String> {
    key(fields, name)
        .map(|k| fields[k].clone())
        .filter(|values| values.iter().any(|v| !v.is_empty()))
        .unwrap_or_default()
}

/// How the edited rows of a file are stored: file field changes and, per
/// chapter, changes of the chapter's own fields.
#[derive(Debug, Default, PartialEq)]
struct Plan {
    file: Fields,
    chapters: Vec<Fields>,
}

/// Mp3tag's rule for sub-tracks: a field that has the same value on every
/// row is a file field (and leaves the chapters); otherwise the file loses it
/// and each chapter stores its own value. Only the `changed` fields are
/// looked at, so untouched fields keep how they are stored.
fn consolidate(parent: &AudioEntry, views: &[Fields], changed: &[String]) -> Plan {
    let mut plan = Plan {
        file: Fields::new(),
        chapters: vec![Fields::new(); views.len()],
    };
    for name in changed {
        let values: Vec<Vec<String>> = views.iter().map(|v| values_of(v, name)).collect();
        let file = values_of(&parent.fields, name);
        if values.iter().all(|v| *v == values[0]) {
            if file != values[0] {
                plan.file.insert(name.clone(), values[0].clone());
            }
            for (index, chapter) in parent.chapters.iter().enumerate() {
                if key(&chapter.fields, name).is_some() {
                    plan.chapters[index].insert(name.clone(), Vec::new());
                }
            }
        } else {
            if !file.is_empty() {
                plan.file.insert(name.clone(), Vec::new());
            }
            for (index, chapter) in parent.chapters.iter().enumerate() {
                if values_of(&chapter.fields, name) != values[index] {
                    plan.chapters[index].insert(name.clone(), values[index].clone());
                }
            }
        }
    }
    plan
}

/// Write the edits of several rows (`number`, updates) of one file and
/// return those rows as read back.
pub(super) fn write_rows(
    file: &Path,
    rows: &[(usize, Fields)],
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<Vec<AudioEntry>, String> {
    if is_cue(file) {
        write_cue_rows(file, rows)?;
        return rows
            .iter()
            .map(|(number, _)| cue_row(file, *number, include_cover_data, rules, options))
            .collect();
    }
    let parent = read_description(file, false, rules, options)?;
    let count = parent.chapters.len();
    if let Some((number, _)) = rows.iter().find(|(n, _)| *n == 0 || *n > count) {
        return Err(format!(
            "Chapter {number} is no longer in {}",
            parent.filename
        ));
    }
    let source = parent.chapters[0].source;
    if source == ChapterSource::Cue {
        write_embedded_cue_rows(file, &parent, rows, rules, options)?;
    } else {
        let mut views: Vec<Fields> = (0..count)
            .map(|index| row(&parent, String::new(), index, count).fields)
            .collect();
        let mut titles: Vec<Option<String>> = vec![None; count];
        let mut changed: Vec<String> = Vec::new();
        for (number, updates) in rows {
            let index = number - 1;
            for (name, values) in updates {
                refuse_whole_file_fields(name)?;
                if name.eq_ignore_ascii_case("TITLE") {
                    titles[index] = Some(values.join("; "));
                    continue;
                }
                put(&mut views[index], name, values.clone());
                if !changed.iter().any(|c| c.eq_ignore_ascii_case(name)) {
                    changed.push(name.clone());
                }
            }
        }
        let plan = consolidate(&parent, &views, &changed);
        let title = |index: usize| {
            titles[index]
                .clone()
                .unwrap_or_else(|| parent.chapters[index].title.clone())
        };
        let chapter_values: Vec<String> = (0..count)
            .map(|index| chapter_value(parent.chapters[index].start * 1_000_000, &title(index)))
            .collect();
        let retitled = titles.iter().any(Option::is_some);
        match source {
            ChapterSource::Mp4 => write_mp4_rows(
                file,
                &parent,
                &plan,
                retitled.then_some(chapter_values.as_slice()),
                &title,
                rules,
                options,
            )?,
            ChapterSource::Matroska => {
                let mut fields = plan.file.clone();
                if retitled {
                    fields.insert(CHAPTERS_FIELD.into(), chapter_values);
                }
                let chapters: Vec<(usize, Fields)> = plan
                    .chapters
                    .iter()
                    .cloned()
                    .enumerate()
                    .filter(|(_, f)| !f.is_empty())
                    .collect();
                if !fields.is_empty() || !chapters.is_empty() {
                    container_formats::write_chapter_rows(file, &fields, chapters, rules, options)?;
                }
            }
            ChapterSource::Id3 => {
                let mut chapters = plan.chapters.clone();
                for (index, title) in titles.iter().enumerate() {
                    if let Some(title) = title {
                        let values = if title.is_empty() {
                            Vec::new()
                        } else {
                            vec![title.clone()]
                        };
                        chapters[index].insert("TITLE".into(), values);
                    }
                }
                write_id3_rows(file, &plan.file, &chapters, rules, options)?;
            }
            ChapterSource::Cue => unreachable!("handled above"),
        }
    }
    let parent = read_description(file, include_cover_data, rules, options)?;
    let rows_after = chapter_rows(&parent);
    rows.iter()
        .map(|(number, _)| {
            rows_after
                .get(number - 1)
                .cloned()
                .ok_or_else(|| format!("Chapter {number} is no longer in {}", parent.filename))
        })
        .collect()
}

/// Rows of a cue sheet in a CUESHEET field: TITLE, ARTIST, COMPOSER, ISRC
/// and the track ReplayGain values are written into the track's lines, other
/// fields are file fields that the rows must agree on.
fn write_embedded_cue_rows(
    file: &Path,
    parent: &AudioEntry,
    rows: &[(usize, Fields)],
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<(), String> {
    let name = key(&parent.fields, CUESHEET_FIELD)
        .cloned()
        .ok_or("The embedded cue sheet is missing")?;
    let text = parent.fields[&name].first().cloned().unwrap_or_default();
    let mut sheet = CueSheet::parse(&text);
    let mut shared = Fields::new();
    let mut tracks: Vec<(usize, Fields)> = Vec::new();
    for (number, updates) in rows {
        let index = cue_track(&sheet, *number)?;
        let mut own = Fields::new();
        for (name, values) in updates {
            refuse_whole_file_fields(name)?;
            if name.eq_ignore_ascii_case("TITLE")
                || CueSheet::track_field(name)
                || name.eq_ignore_ascii_case("TRACK")
            {
                own.insert(name.to_ascii_uppercase(), values.clone());
                continue;
            }
            match shared.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)) {
                Some((_, other)) if other != values => {
                    return Err(conflict(name, &parent.filename))
                }
                Some(_) => {}
                None => {
                    shared.insert(name.clone(), values.clone());
                }
            }
        }
        if !own.is_empty() {
            tracks.push((index, own));
        }
    }
    sheet.apply(&tracks)?;
    let rendered = sheet.render();
    if rendered != text {
        shared.insert(name, vec![rendered]);
    }
    if !shared.is_empty() {
        write_fields_result(file, &shared, false, rules, options)?;
    }
    Ok(())
}

/// MP4 rows: file fields in the iTunes tag, titles in the Nero chapter list
/// and the QuickTime chapter track, and the fields of each chapter in Nero
/// tags (`udta/tags`), all in one write. Nero tags are written when a
/// chapter has fields of its own or the file already has them; each
/// chapter's set also holds its title, as Mp3tag writes it.
fn write_mp4_rows(
    file: &Path,
    parent: &AudioEntry,
    plan: &Plan,
    chapter_values: Option<&[String]>,
    title: &dyn Fn(usize) -> String,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<(), String> {
    let existing = mp4_chapters::scan(file)?.nero_tags;
    let mut sets: Vec<Fields> = parent.chapters.iter().map(|c| c.fields.clone()).collect();
    for (set, changes) in sets.iter_mut().zip(&plan.chapters) {
        for (name, values) in changes {
            put(set, name, values.clone());
        }
    }
    let mut changed = plan.chapters.iter().any(|f| !f.is_empty());
    let nero = if existing.is_some() || sets.iter().any(|s| !s.is_empty()) {
        let mut nero = existing.unwrap_or_default();
        nero.truncate(sets.len());
        for (index, set) in sets.iter_mut().enumerate() {
            put(set, "TITLE", vec![title(index)]);
            nero.set_chapter(index, set)?;
        }
        // File-wide Nero fields follow the file fields (see `sync_global`).
        changed |= nero.sync_global(&plan.file)?;
        Some(nero)
    } else {
        None
    };
    let rewrite_chapters = chapter_values.is_some() || (changed && nero.is_some());
    if plan.file.is_empty() && !rewrite_chapters {
        return Ok(());
    }
    let values: Vec<String> = match chapter_values {
        Some(values) => values.to_vec(),
        None => parent
            .chapters
            .iter()
            .map(|c| chapter_value(c.start * 1_000_000, &c.title))
            .collect(),
    };
    staged_mutation_with_mappings(
        file,
        false,
        rules,
        options,
        |staging, tagged, staged_path| {
            if !plan.file.is_empty() {
                write_tag_fields(staging, tagged, staged_path, &plan.file, rules, options)?;
            }
            if rewrite_chapters {
                mp4_chapters::write_with_tags(staging, staged_path, &values, nero.as_ref())?;
            }
            Ok(())
        },
    )
    .map(|_| ())
}

/// ID3v2 rows: file fields in the tag, and each chapter's changes (TITLE
/// included) in the sub-frames of its `CHAP` frame (0-based in start order),
/// in one write. Other sub-frames, such as pictures, are kept.
fn write_id3_rows(
    file: &Path,
    shared: &Fields,
    chapters: &[Fields],
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<(), String> {
    if shared.is_empty() && chapters.iter().all(Fields::is_empty) {
        return Ok(());
    }
    staged_mutation_with_mappings(
        file,
        false,
        rules,
        options,
        |staging, tagged, staged_path| {
            let kind = tagged.file_type();
            if !shared.is_empty() {
                write_tag_fields(staging, tagged, staged_path, shared, rules, options)?;
            }
            if chapters.iter().all(Fields::is_empty) {
                return Ok(());
            }
            let mut tag = native_tags(staging, kind)?
                .id3
                .ok_or("The ID3v2 tag with the chapters is missing")?;
            let ids: Vec<String> = id3_chapters(&tag).into_iter().map(|(id, _)| id).collect();
            // MPEG saves use the configured revision, as every MPEG save does.
            let target = if kind == FileType::Mpeg {
                let target = tag_policy::id3_target(options);
                mapping::convert_id3_version(&mut tag, &target)?;
                target
            } else {
                mapping::Id3Target::preserving(&tag, options.comment_language())
            };
            let mut expected = Vec::new();
            for (index, updates) in chapters.iter().enumerate() {
                if updates.is_empty() {
                    continue;
                }
                let id = ids.get(index).ok_or("A chapter disappeared while saving")?;
                let native_updates = normalized_updates(
                    &map_write_fields(updates, TagType::Id3v2, rules)?,
                    TagType::Id3v2,
                )?;
                let mut failed = None;
                tag.retain_mut(|frame| {
                    if let Frame::Chapter(chapter) = frame {
                        if chapter.id == id.as_str() && failed.is_none() {
                            let mut own = sub_frames(&chapter.children);
                            match mapping::patch_id3(&mut own, &native_updates, &target) {
                                Ok(()) => {
                                    let mut children = FrameList::new();
                                    for frame in own {
                                        children.insert(frame);
                                    }
                                    chapter.children = children;
                                }
                                Err(error) => failed = Some(error),
                            }
                        }
                    }
                    true
                });
                if let Some(error) = failed {
                    return Err(error);
                }
                expected.push((index, native_updates));
            }
            staging
                .seek(SeekFrom::Start(0))
                .map_err(|e| e.to_string())?;
            mapping::save_id3(staging, kind, &tag, &target)?;
            let written = native_tags(staging, kind)?
                .id3
                .map(|tag| id3_chapters(&tag))
                .unwrap_or_default();
            for (index, updates) in expected {
                let (_, chapter) = written
                    .get(index)
                    .ok_or("The ID3v2 chapters could not be written")?;
                let mut stored = chapter.fields.clone();
                put(&mut stored, "TITLE", vec![chapter.title.clone()]);
                verify_readback(&stored, &updates, TagType::Id3v2, target.latin1)
                    .map_err(|e| format!("Chapter {}: {e}", index + 1))?;
            }
            Ok(())
        },
    )
    .map(|_| ())
}

/// Edit a cue sheet file in place of the original, keeping its encoding.
fn write_cue_rows(cue: &Path, rows: &[(usize, Fields)]) -> Result<(), String> {
    let before = fs::metadata(cue).map_err(|e| e.to_string())?;
    let (mut sheet, text, encoding) = read_sheet(cue)?;
    let indexed: Vec<(usize, Fields)> = rows
        .iter()
        .map(|(number, updates)| Ok((cue_track(&sheet, *number)?, updates.clone())))
        .collect::<Result<_, String>>()?;
    sheet.apply(&indexed)?;
    let rendered = sheet.render();
    if rendered == text {
        return Ok(());
    }
    let directory = cue.parent().ok_or("The cue sheet has no folder")?;
    let mut staged = tempfile::Builder::new()
        .prefix(".opentag-")
        .suffix(".cue")
        .tempfile_in(directory)
        .map_err(|e| e.to_string())?;
    std::io::Write::write_all(
        staged.as_file_mut(),
        &crate::cue::encode(&rendered, encoding),
    )
    .map_err(|e| e.to_string())?;
    staged.as_file().sync_all().map_err(|e| e.to_string())?;
    let (check, _, _) = read_sheet(staged.path())?;
    if check.render() != rendered {
        return Err("The cue sheet could not be written; the original was kept".into());
    }
    fs::set_permissions(staged.path(), before.permissions()).map_err(|e| e.to_string())?;
    let current = fs::metadata(cue).map_err(|e| e.to_string())?;
    if current.len() != before.len() || current.modified().ok() != before.modified().ok() {
        return Err("The cue sheet changed during editing; reload it before saving".into());
    }
    staged.persist(cue).map_err(|e| e.error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_paths_round_trip() {
        let path = subsong_path("/music/book.m4b", 3);
        assert_eq!(subsong_parts(&path), Some(("/music/book.m4b", 3)));
        assert_eq!(subsong_parts("/music/book.m4b"), None);
        assert_eq!(subsong_parts(&subsong_path("/a", 0)), None);
        assert_eq!(subsong_parts("\u{1}2"), None);
        assert_eq!(
            split_path(Path::new(&path)),
            Some((PathBuf::from("/music/book.m4b"), 3))
        );
    }

    #[test]
    fn chapter_values_parse_and_format() {
        assert_eq!(
            parse_chapter_value("1:02:03.5 Title  x").unwrap(),
            (3_723_500_000_000, "Title  x".into())
        );
        assert_eq!(chapter_value(3_723_500_000_000, "T"), "01:02:03.500 T");
        assert!(parse_chapter_value("Title").is_err());
        assert!(parse_chapter_value("0:60 x").is_err());
    }

    #[test]
    fn rows_overlay_chapter_fields_and_lengths() {
        let parent = AudioEntry {
            path: "/a/book.m4b".into(),
            filename: "book.m4b".into(),
            format: "MP4".into(),
            duration: 10.0,
            bitrate: 128,
            sample_rate: 44100,
            channels: 2,
            size: 1000,
            fields: Fields::from([
                ("TITLE".into(), vec!["Book".into()]),
                ("ARTIST".into(), vec!["Author".into()]),
                ("CHAPTERS".into(), vec!["x".into()]),
            ]),
            covers: Vec::new(),
            technical: Fields::from([("_SUBSONG_COUNT".into(), vec!["2".into()])]),
            chapters: vec![
                ChapterInfo {
                    start: 0,
                    end: None,
                    title: "One".into(),
                    fields: Fields::new(),
                    source: ChapterSource::Mp4,
                },
                ChapterInfo {
                    start: 4000,
                    end: None,
                    title: String::new(),
                    fields: Fields::from([("ARTIST".into(), vec!["Guest".into()])]),
                    source: ChapterSource::Mp4,
                },
            ],
        };
        let rows = chapter_rows(&parent);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].path, "/a/book.m4b\u{1}1");
        assert_eq!(rows[0].fields["TITLE"], ["One"]);
        assert_eq!(rows[0].fields["ARTIST"], ["Author"]);
        assert!(!rows[0].fields.contains_key("CHAPTERS"));
        assert_eq!(rows[0].technical["_SUBSONG"], ["0"]);
        assert_eq!(rows[0].technical["_LENGTH"], ["0:04"]);
        // MP4 sub-tracks without numbers are numbered.
        assert_eq!(rows[0].fields["TRACK"], ["1"]);
        assert_eq!(rows[1].fields["TRACK"], ["2"]);
        assert_eq!(rows[1].fields["TOTALTRACKS"], ["2"]);
        assert_eq!(rows[1].duration, 6.0);
        assert!(!rows[1].fields.contains_key("TITLE"));
        assert_eq!(rows[1].fields["ARTIST"], ["Guest"]);
        assert_eq!(rows[1].technical["_SUBSONG_COUNT"], ["2"]);
        let expanded = expand_chapter_rows(vec![parent.clone(), rows[0].clone()]);
        assert_eq!(expanded.len(), 3);
        // A single chapter is not a subsong of its own.
        let mut single = parent.clone();
        single.chapters.clear();
        single.technical.clear();
        attach(&mut single, parent.chapters[..1].to_vec());
        assert!(single.chapters.is_empty() && single.technical.is_empty());
    }

    fn values(pairs: &[(&str, &[&str])]) -> Fields {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
            .collect()
    }

    #[test]
    fn equal_values_are_file_fields_and_differing_ones_belong_to_chapters() {
        let chapter = |fields: Fields| ChapterInfo {
            start: 0,
            end: None,
            title: String::new(),
            fields,
            source: ChapterSource::Id3,
        };
        let parent = AudioEntry {
            path: "/a/show.mp3".into(),
            filename: "show.mp3".into(),
            format: "MP3".into(),
            duration: 9.0,
            bitrate: 0,
            sample_rate: 0,
            channels: 0,
            size: 0,
            fields: values(&[("ARTIST", &["Host"]), ("ALBUM", &["Show"])]),
            covers: Vec::new(),
            technical: Fields::new(),
            chapters: vec![
                chapter(values(&[("ARTIST", &["Guest"])])),
                chapter(Fields::new()),
                chapter(Fields::new()),
            ],
        };
        let view = |pairs: &[(&str, &[&str])]| values(pairs);
        // ALBUM edited the same on every row: a file field. ARTIST made equal:
        // back to the file, out of chapter 1. COMMENT differs: per chapter.
        let views = [
            view(&[
                ("ARTIST", &["Host"]),
                ("ALBUM", &["New"]),
                ("COMMENT", &["a"]),
            ]),
            view(&[("ARTIST", &["Host"]), ("ALBUM", &["New"])]),
            view(&[
                ("ARTIST", &["Host"]),
                ("ALBUM", &["New"]),
                ("COMMENT", &["c"]),
            ]),
        ];
        let plan = consolidate(
            &parent,
            &views,
            &["ALBUM".into(), "ARTIST".into(), "COMMENT".into()],
        );
        assert_eq!(plan.file, values(&[("ALBUM", &["New"])]));
        assert_eq!(
            plan.chapters,
            [
                values(&[("ARTIST", &[]), ("COMMENT", &["a"])]),
                Fields::new(),
                values(&[("COMMENT", &["c"])]),
            ]
        );
        // A file field one row changes leaves the file and every chapter
        // keeps its value.
        let views = [
            view(&[("ALBUM", &["Show"])]),
            view(&[("ALBUM", &["Special"])]),
            view(&[("ALBUM", &["Show"])]),
        ];
        let plan = consolidate(&parent, &views, &["ALBUM".into()]);
        assert_eq!(plan.file, values(&[("ALBUM", &[])]));
        assert_eq!(
            plan.chapters,
            [
                values(&[("ALBUM", &["Show"])]),
                values(&[("ALBUM", &["Special"])]),
                values(&[("ALBUM", &["Show"])]),
            ]
        );
    }
}
