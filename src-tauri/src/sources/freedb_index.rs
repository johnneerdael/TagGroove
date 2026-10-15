//! Text search in a local freedb folder (Options → Tag Sources → Local
//! freedb). freedb snapshots are only organised by disc ID, so the user builds
//! an index once: one tab-separated line per disc with its category, disc ID,
//! artist, title, year, genre and track count. Searches stream the index and
//! keep every disc whose artist and title contain all words of the query.
use super::freedb;
use super::model::SearchResult;
use std::io::{BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use tag_core::parallel::Progress;

const HEADER: &str = "# OpenTag local freedb index 1";
/// Results shown for one search; the total still counts every match.
pub const MAX_RESULTS: usize = 500;

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct IndexStatus {
    /// The folder the index was built from, or `None` without an index.
    pub directory: Option<String>,
    pub discs: u64,
}

#[derive(Debug, PartialEq)]
pub struct Found {
    pub results: Vec<SearchResult>,
    pub total: u32,
}

fn clean(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_owned()
}

/// One index line for an xmcd entry.
fn line(category: &str, disc_id: &str, text: &str) -> Option<String> {
    let xmcd = freedb::parse_xmcd(text);
    let dtitle = xmcd.get("DTITLE");
    if dtitle.trim().is_empty() {
        return None;
    }
    let (artist, title) = freedb::split_title(dtitle);
    Some(format!(
        "{category}\t{disc_id}\t{}\t{}\t{}\t{}\t{}\n",
        clean(&artist),
        clean(&title),
        clean(xmcd.get("DYEAR")),
        clean(xmcd.get("DGENRE")),
        xmcd.track_count()
    ))
}

fn is_disc_id(name: &str) -> bool {
    name.len() == 8 && name.chars().all(|c| c.is_ascii_hexdigit())
}

/// Every entry of one file: a single disc (Unix layout, named by its disc ID)
/// or the `#FILENAME=` sections of a Windows-layout `xxtoyy` file.
fn entries(name: &str, text: &str) -> Vec<(String, String)> {
    if is_disc_id(name) {
        return vec![(name.to_ascii_lowercase(), text.to_owned())];
    }
    let mut found = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    for raw in text.lines() {
        if let Some(id) = raw.trim().strip_prefix("#FILENAME=") {
            if let Some((id, lines)) = current.take() {
                found.push((id, lines.join("\n")));
            }
            let id = id.trim().to_ascii_lowercase();
            current = is_disc_id(&id).then(|| (id, Vec::new()));
        } else if let Some((_, lines)) = current.as_mut() {
            lines.push(raw);
        }
    }
    if let Some((id, lines)) = current {
        found.push((id, lines.join("\n")));
    }
    found
}

/// Index every disc below `directory` into `index`, replacing it only when
/// the build completes. Returns the number of discs, or an error when the
/// job was cancelled or the folder could not be read.
pub fn build(directory: &Path, index: &Path, job: &dyn Progress) -> Result<u64, String> {
    freedb::check_local_directory(directory)?;
    let mut categories: Vec<PathBuf> = std::fs::read_dir(directory)
        .map_err(|e| format!("The local freedb folder could not be read: {e}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    categories.sort();
    let mut files = Vec::new();
    for category in &categories {
        let name = category
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        let Ok(listing) = std::fs::read_dir(category) else {
            continue;
        };
        let mut inside: Vec<PathBuf> = listing
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect();
        inside.sort();
        files.extend(inside.into_iter().map(|path| (name.clone(), path)));
    }
    job.start(files.len());
    let folder = index.parent().ok_or("The index has no folder")?;
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let staging = tempfile::NamedTempFile::new_in(folder).map_err(|e| e.to_string())?;
    let mut out = BufWriter::new(staging.as_file());
    let directory_text = directory.to_string_lossy();
    writeln!(out, "{HEADER}\t{}", clean(&directory_text)).map_err(|e| e.to_string())?;
    let mut discs = 0u64;
    for (category, path) in &files {
        if !job.proceed() {
            return Err("Cancelled: the index was not changed.".into());
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if let Ok(bytes) = std::fs::read(path) {
            for (disc_id, text) in entries(&name, &freedb::decode(&bytes)) {
                if let Some(line) = line(category, &disc_id, &text) {
                    out.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
                    discs += 1;
                }
            }
        }
        job.advance();
    }
    out.flush().map_err(|e| e.to_string())?;
    drop(out);
    staging.persist(index).map_err(|e| e.error.to_string())?;
    Ok(discs)
}

fn header_directory(first: &str) -> Option<&str> {
    first.strip_prefix(HEADER)?.strip_prefix('\t')
}

pub fn status(index: &Path) -> IndexStatus {
    let Ok(file) = std::fs::File::open(index) else {
        return IndexStatus {
            directory: None,
            discs: 0,
        };
    };
    let mut lines = std::io::BufReader::new(file).lines();
    let directory = lines
        .next()
        .and_then(Result::ok)
        .and_then(|first| header_directory(&first).map(str::to_owned));
    let discs = if directory.is_some() {
        lines.map_while(Result::ok).count() as u64
    } else {
        0
    };
    IndexStatus { directory, discs }
}

/// Lower-case words without accents, so `cafe` finds `Café`.
fn words(text: &str) -> Vec<String> {
    use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};
    text.nfkd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Discs whose artist and title contain every word of `query`, in index order.
pub fn search(index: &Path, directory: &Path, query: &str) -> Result<Found, String> {
    let wanted = words(query);
    if wanted.is_empty() {
        return Err("Enter words from the artist or album title.".into());
    }
    let rebuild = "Build the text index in Options → Tag Sources → Local freedb to search by text.";
    let file = std::fs::File::open(index).map_err(|_| rebuild.to_string())?;
    let mut lines = std::io::BufReader::new(file).lines();
    let first = lines
        .next()
        .and_then(Result::ok)
        .ok_or_else(|| rebuild.to_string())?;
    let built_for = header_directory(&first).ok_or_else(|| rebuild.to_string())?;
    if Path::new(built_for) != directory {
        return Err("The text index was built for another local freedb folder. Build it again in Options → Tag Sources → Local freedb.".into());
    }
    let mut results = Vec::new();
    let mut total = 0u32;
    for line in lines.map_while(Result::ok) {
        let parts: Vec<&str> = line.split('\t').collect();
        let [category, disc_id, artist, title, year, genre, tracks] = parts[..] else {
            continue;
        };
        let haystack = words(&format!("{artist} {title}"));
        if !wanted.iter().all(|word| {
            haystack
                .iter()
                .any(|candidate| candidate.contains(word.as_str()))
        }) {
            continue;
        }
        total = total.saturating_add(1);
        if results.len() < MAX_RESULTS {
            results.push(SearchResult {
                id: format!("{category}/{disc_id}"),
                title: title.to_owned(),
                artist: artist.to_owned(),
                year: year.to_owned(),
                format: if genre.is_empty() {
                    category.to_owned()
                } else {
                    genre.to_owned()
                },
                track_count: tracks.parse().ok(),
                ..Default::default()
            });
        }
    }
    Ok(Found { results, total })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct Stop(AtomicBool);
    impl Progress for Stop {
        fn proceed(&self) -> bool {
            !self.0.load(Ordering::Relaxed)
        }
        fn advance(&self) {
            self.0.store(true, Ordering::Relaxed);
        }
    }

    fn entry(artist: &str, title: &str, year: &str) -> String {
        format!(
            "# xmcd\nDTITLE={artist} / {title}\nDYEAR={year}\nDGENRE=\nTTITLE0=One\nTTITLE1=Two\n"
        )
    }

    #[test]
    fn index_covers_both_layouts_and_finds_discs_by_words() {
        let dir = tempfile::tempdir().unwrap();
        let freedb = dir.path().join("freedb");
        for category in ["rock", "jazz"] {
            std::fs::create_dir_all(freedb.join(category)).unwrap();
        }
        std::fs::write(
            freedb.join("rock").join("0a000802"),
            entry("Fixture Artist", "Fixture Album", "2001"),
        )
        .unwrap();
        let bundle = format!(
            "#FILENAME=10000001\n{}#FILENAME=1000000F\n{}#FILENAME=zz\nDTITLE=Not / Indexed\n",
            entry("Café Quartet", "Night\tSet", "1999"),
            entry("Other", "Fixture Live", "")
        );
        std::fs::write(freedb.join("jazz").join("10to1f"), bundle).unwrap();
        let index = dir.path().join("config").join("index.tsv");
        assert_eq!(
            status(&index),
            IndexStatus {
                directory: None,
                discs: 0
            }
        );
        assert!(search(&index, &freedb, "fixture")
            .unwrap_err()
            .contains("Build the text index"));
        assert_eq!(
            build(&freedb, &index, &tag_core::parallel::Silent).unwrap(),
            3
        );
        let built = status(&index);
        assert_eq!(built.discs, 3);
        assert_eq!(built.directory.as_deref(), Some(&*freedb.to_string_lossy()));

        let found = search(&index, &freedb, "FIXTURE").unwrap();
        assert_eq!(found.total, 2);
        let ids: Vec<_> = found.results.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["jazz/1000000f", "rock/0a000802"]);
        assert_eq!(found.results[1].year, "2001");
        assert_eq!(found.results[1].track_count, Some(2));
        assert_eq!(found.results[1].format, "rock");
        let accents = search(&index, &freedb, "cafe night").unwrap();
        assert_eq!(accents.results[0].title, "Night Set");
        assert_eq!(accents.results[0].artist, "Café Quartet");
        assert_eq!(search(&index, &freedb, "fixture jazz").unwrap().total, 0);
        assert!(search(&index, &freedb, " - ").is_err());
        assert!(search(&index, &dir.path().join("elsewhere"), "fixture")
            .unwrap_err()
            .contains("another local freedb folder"));

        // A cancelled rebuild keeps the previous index.
        std::fs::write(
            freedb.join("rock").join("0a000803"),
            entry("New", "Disc", ""),
        )
        .unwrap();
        let stop = Stop(AtomicBool::new(false));
        assert!(build(&freedb, &index, &stop)
            .unwrap_err()
            .starts_with("Cancelled"));
        assert_eq!(status(&index).discs, 3);
    }
}
