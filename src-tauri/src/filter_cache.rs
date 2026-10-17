//! Backend copy of the filter fields of each listed file.
//!
//! A filter query used to carry every file's fields and column values (about
//! 2.7 MB for 434 files). Each window session now sends a file's fields only
//! when they changed since its previous query (load, save, undo, a pending
//! edit or new column values), and the list order only when it changed. The
//! query itself carries the expression and the column names.
use crate::evaluation::{ColumnSpec, EvaluationSettings};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use tag_core::Fields;

/// Returned when a session lacks a listed file, for example after the backend
/// restarted; the window then resends every file.
pub const CACHE_MISS: &str = "OPENTAG_FILTER_CACHE_MISS";
const SESSIONS: usize = 8;

#[derive(Deserialize)]
pub struct FilterFile {
    pub path: String,
    pub fields: Fields,
    pub raw_fields: Option<Fields>,
    /// Evaluated column values in `columns` order, `None` where not evaluated.
    #[serde(default)]
    pub columns: Vec<Option<String>>,
}

#[derive(Deserialize, PartialEq)]
pub struct FilterColumn {
    pub name: String,
    pub template: String,
}

#[derive(Deserialize)]
pub struct FilterRequest {
    pub session: String,
    /// The listed paths in display order, when they changed.
    pub order: Option<Vec<String>>,
    #[serde(default)]
    pub updates: Vec<FilterFile>,
    #[serde(default)]
    pub columns: Vec<FilterColumn>,
}

/// The matching paths, or why the expression could not be evaluated. Either
/// way the request's files were cached.
#[derive(Serialize, Debug)]
pub struct FilterOutcome {
    pub matches: Vec<String>,
    pub error: Option<String>,
}

struct Cached {
    fields: Fields,
    raw: Option<Fields>,
}

#[derive(Default)]
struct Session {
    order: Vec<String>,
    files: HashMap<String, Cached>,
    columns: Vec<ColumnSpec>,
    used: u64,
}

#[derive(Default)]
pub struct FilterCache {
    sessions: HashMap<String, Session>,
    clock: u64,
}

impl FilterCache {
    /// Apply the request's changes, then return the listed paths matching
    /// `query`. `settings` supplies everything but the column snapshots. The
    /// only error is [`CACHE_MISS`]: nothing else leaves the cache behind
    /// what the window believes it holds.
    pub fn matches(
        &mut self,
        query: &str,
        request: FilterRequest,
        settings: impl FnOnce(Vec<ColumnSpec>) -> Result<EvaluationSettings, String>,
    ) -> Result<FilterOutcome, String> {
        self.clock += 1;
        if !self.sessions.contains_key(&request.session) && self.sessions.len() >= SESSIONS {
            let oldest = self
                .sessions
                .iter()
                .min_by_key(|(_, session)| session.used)
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest {
                self.sessions.remove(&oldest);
            }
        }
        let session = self.sessions.entry(request.session).or_default();
        session.used = self.clock;
        let same_columns = session.columns.len() == request.columns.len()
            && session
                .columns
                .iter()
                .zip(&request.columns)
                .all(|(old, new)| old.name == new.name && old.template == new.template);
        if !same_columns {
            session.columns = request
                .columns
                .into_iter()
                .map(|column| ColumnSpec {
                    name: column.name,
                    template: column.template,
                    values: BTreeMap::new(),
                })
                .collect();
        }
        if let Some(order) = request.order {
            let listed: HashSet<&String> = order.iter().collect();
            session.files.retain(|path, _| listed.contains(path));
            for column in &mut session.columns {
                column.values.retain(|path, _| listed.contains(path));
            }
            session.order = order;
        }
        for update in request.updates {
            for (index, column) in session.columns.iter_mut().enumerate() {
                match update.columns.get(index).cloned().flatten() {
                    Some(value) => column.values.insert(update.path.clone(), value),
                    None => column.values.remove(&update.path),
                };
            }
            session.files.insert(
                update.path,
                Cached {
                    fields: update.fields,
                    raw: update.raw_fields,
                },
            );
        }
        if session
            .order
            .iter()
            .any(|path| !session.files.contains_key(path))
        {
            return Err(CACHE_MISS.into());
        }
        Ok(match evaluate(session, query, settings) {
            Ok(matches) => FilterOutcome {
                matches,
                error: None,
            },
            Err(error) => FilterOutcome {
                matches: Vec::new(),
                error: Some(error),
            },
        })
    }
}

fn evaluate(
    session: &mut Session,
    query: &str,
    settings: impl FnOnce(Vec<ColumnSpec>) -> Result<EvaluationSettings, String>,
) -> Result<Vec<String>, String> {
    let filter = tag_core::compile_filter(query)?;
    let settings = settings(std::mem::take(&mut session.columns))?;
    let mut matching = Vec::new();
    let result = session
        .order
        .iter()
        .enumerate()
        .try_for_each(|(index, path)| {
            let file = session
                .files
                .get_mut(path)
                .expect("every listed file is cached");
            file.fields
                .insert("_COUNTER".into(), vec![(index + 1).to_string()]);
            file.fields
                .insert("_APP".into(), vec![crate::evaluation::app_name()]);
            let raw = file.raw.as_ref().unwrap_or(&file.fields);
            if settings.matches(&filter, &file.fields, raw, path)? {
                matching.push(path.clone());
            }
            Ok::<_, String>(())
        });
    session.columns = settings.columns;
    result.map(|()| matching)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, title: &str) -> FilterFile {
        FilterFile {
            path: path.into(),
            fields: Fields::from([("TITLE".into(), vec![title.into()])]),
            raw_fields: None,
            columns: vec![Some(format!("col {title}"))],
        }
    }
    fn request(order: Option<&[&str]>, updates: Vec<FilterFile>) -> FilterRequest {
        FilterRequest {
            session: "window".into(),
            order: order.map(|o| o.iter().map(|p| p.to_string()).collect()),
            updates,
            columns: vec![FilterColumn {
                name: "Shown".into(),
                template: "%title%".into(),
            }],
        }
    }
    fn settings(columns: Vec<ColumnSpec>) -> Result<EvaluationSettings, String> {
        Ok(EvaluationSettings {
            columns,
            ..Default::default()
        })
    }

    #[test]
    fn later_queries_reuse_cached_fields_and_order() {
        let mut cache = FilterCache::default();
        let first = cache
            .matches(
                "TITLE HAS a",
                request(Some(&["1", "2"]), vec![file("1", "abc"), file("2", "xyz")]),
                settings,
            )
            .unwrap()
            .matches;
        assert_eq!(first, ["1"]);
        // Only the query: cached fields, order and counter answer it.
        assert_eq!(
            cache
                .matches("%_counter% IS 2", request(None, vec![]), settings)
                .unwrap()
                .matches,
            ["2"]
        );
        // Application and working-folder placeholders come from the context.
        assert_eq!(
            cache
                .matches("_APP HAS taggroove", request(None, vec![]), settings)
                .unwrap()
                .matches,
            ["1", "2"]
        );
        let in_rock = |columns| {
            Ok(EvaluationSettings {
                working_directory: Some("/music/rock".into()),
                ..settings(columns)?
            })
        };
        assert_eq!(
            cache
                .matches("\"%_workingdir%\" IS rock", request(None, vec![]), in_rock)
                .unwrap()
                .matches,
            ["1", "2"]
        );
        // Column snapshots are kept per file.
        assert_eq!(
            cache
                .matches(
                    "\"$columnValues(,)\" IS \"col xyz\"",
                    request(None, vec![]),
                    settings
                )
                .unwrap()
                .matches,
            ["2"]
        );
        // A changed file replaces its cached fields; a new order renumbers.
        assert_eq!(
            cache
                .matches(
                    "TITLE HAS a",
                    request(Some(&["2", "1"]), vec![file("2", "zap")]),
                    settings
                )
                .unwrap()
                .matches,
            ["2", "1"]
        );
        assert_eq!(
            cache
                .matches("%_counter% IS 1", request(None, vec![]), settings)
                .unwrap()
                .matches,
            ["2"]
        );
    }

    #[test]
    fn missing_files_ask_for_a_resend_and_removed_files_are_dropped() {
        let mut cache = FilterCache::default();
        let error = cache
            .matches(
                "TITLE HAS a",
                request(Some(&["1", "2"]), vec![file("1", "a")]),
                settings,
            )
            .unwrap_err();
        assert_eq!(error, CACHE_MISS);
        cache
            .matches("TITLE HAS a", request(Some(&["1"]), vec![]), settings)
            .unwrap();
        // "2" was never cached, and "1" is dropped once it leaves the list.
        cache
            .matches("TITLE HAS a", request(Some(&[]), vec![]), settings)
            .unwrap();
        assert_eq!(
            cache
                .matches("TITLE HAS a", request(Some(&["1"]), vec![]), settings)
                .unwrap_err(),
            CACHE_MISS
        );
        // An invalid expression still keeps the files it carried.
        assert!(cache
            .matches("TITLE HAS (", request(None, vec![file("1", "a")]), settings)
            .unwrap()
            .error
            .is_some());
        assert_eq!(
            cache
                .matches("TITLE HAS a", request(None, vec![]), settings)
                .unwrap()
                .matches,
            ["1"]
        );
    }

    /// Payload sizes and timings for a real folder (read only):
    /// `OPENTAG_FILTER_BENCH=<folder> cargo test --release -p opentag filter_cache_bench -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn filter_cache_bench() {
        let Ok(folder) = std::env::var("OPENTAG_FILTER_BENCH") else {
            return;
        };
        let scan = tag_core::scan_paths(&[folder], true);
        let templates = [
            "%_filename_ext%",
            "$meta_sep(title,' \\\\ ')",
            "$meta_sep(artist,' \\\\ ')",
            "$meta_sep(album,' \\\\ ')",
            "%track%",
            "%year%",
            "$meta_sep(genre,' \\\\ ')",
            "%_length%",
            "$if2(%_codec%,%_filetype%)",
            "[%_bitrate% kbps]",
            "%_folderpath%",
        ];
        let fields = |entry: &tag_core::AudioEntry| {
            let mut fields = entry.fields.clone();
            fields.extend(entry.technical.clone());
            fields
        };
        // What every query sent before: all fields, raw fields and column values.
        let legacy = serde_json::json!({
            "query": "remix",
            "items": scan.entries.iter().enumerate().map(|(index, entry)| {
                let mut fields = fields(entry);
                fields.insert("_COUNTER".into(), vec![(index + 1).to_string()]);
                serde_json::json!({"path": entry.path, "fields": fields, "raw_fields": entry.fields})
            }).collect::<Vec<_>>(),
            "columns": templates.iter().map(|template| serde_json::json!({
                "name": template, "template": template,
                "values": scan.entries.iter().map(|e| (e.path.clone(), "value".to_string())).collect::<BTreeMap<_, _>>(),
            })).collect::<Vec<_>>(),
        });
        let columns = || {
            templates
                .iter()
                .map(|t| serde_json::json!({"name": t, "template": t}))
                .collect::<Vec<_>>()
        };
        let first = serde_json::json!({"query": "remix", "request": {
            "session": "bench",
            "order": scan.entries.iter().map(|e| &e.path).collect::<Vec<_>>(),
            "updates": scan.entries.iter().map(|entry| serde_json::json!({
                "path": entry.path, "fields": fields(entry), "raw_fields": entry.fields,
                "columns": templates.iter().map(|_| "value").collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "columns": columns(),
        }});
        let next = serde_json::json!({"query": "mix", "request": {
            "session": "bench", "order": null, "updates": [], "columns": columns(),
        }});
        let size = |value: &serde_json::Value| value.to_string().len();
        let mut cache = FilterCache::default();
        let run = |cache: &mut FilterCache, value: &serde_json::Value| {
            let started = std::time::Instant::now();
            let text = value.to_string();
            let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
            let request: FilterRequest = serde_json::from_value(parsed["request"].clone()).unwrap();
            let outcome = cache
                .matches(parsed["query"].as_str().unwrap(), request, settings)
                .unwrap();
            assert!(outcome.error.is_none());
            (
                started.elapsed().as_secs_f64() * 1000.0,
                outcome.matches.len(),
            )
        };
        let (first_ms, first_matches) = run(&mut cache, &first);
        let mut later = (0..20)
            .map(|_| run(&mut cache, &next).0)
            .collect::<Vec<_>>();
        later.sort_by(f64::total_cmp);
        println!(
            "files={} legacy_bytes={} first_bytes={} next_bytes={} first_ms={first_ms:.2} \
             first_matches={first_matches} next_median_ms={:.2}",
            scan.entries.len(),
            size(&legacy),
            size(&first),
            size(&next),
            later[later.len() / 2]
        );
    }
}
