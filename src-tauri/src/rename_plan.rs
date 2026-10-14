use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{Instant, SystemTime},
};

static COUNTER: AtomicU64 = AtomicU64::new(1);
static PLANS: Mutex<VecDeque<Plan>> = Mutex::new(VecDeque::new());
struct Item {
    path: String,
    target: PathBuf,
    size: u64,
    modified: Option<SystemTime>,
}
struct Plan {
    token: String,
    template: String,
    created: Instant,
    items: Vec<Item>,
}

pub fn record(paths: &[String], template: &str, targets: Vec<PathBuf>) -> Result<String, String> {
    if paths.len() != targets.len() {
        return Err("Incomplete rename preview".into());
    }
    let items = paths
        .iter()
        .zip(targets)
        .map(|(path, target)| {
            let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
            Ok(Item {
                path: path.clone(),
                target,
                size: meta.len(),
                modified: meta.modified().ok(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let token = COUNTER.fetch_add(1, Ordering::Relaxed).to_string();
    let mut plans = PLANS.lock().map_err(|_| "Rename plan lock failed")?;
    plans.retain(|plan| plan.created.elapsed().as_secs() < 300);
    while plans.len() >= 32 {
        plans.pop_front();
    }
    plans.push_back(Plan {
        token: token.clone(),
        template: template.into(),
        created: Instant::now(),
        items,
    });
    Ok(token)
}
pub fn consume(
    token: &str,
    paths: &[String],
    template: &str,
) -> Result<Vec<(String, PathBuf)>, String> {
    let mut plans = PLANS.lock().map_err(|_| "Rename plan lock failed")?;
    let index = plans
        .iter()
        .position(|p| p.token == token)
        .ok_or("Rename preview is no longer available; preview again")?;
    let plan = plans.remove(index).ok_or("Rename plan disappeared")?;
    if plan.created.elapsed().as_secs() >= 300
        || plan.template != template
        || plan.items.iter().map(|i| &i.path).ne(paths.iter())
    {
        return Err("The rename request changed; preview again".into());
    }
    for item in &plan.items {
        let meta = std::fs::metadata(&item.path).map_err(|e| e.to_string())?;
        if meta.len() != item.size || meta.modified().ok() != item.modified {
            return Err("A file changed after preview; preview again".into());
        }
    }
    Ok(plan.items.into_iter().map(|i| (i.path, i.target)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reviewed_targets_survive_dynamic_templates_and_can_only_apply_once() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        std::fs::write(&source, "original").unwrap();
        let paths = vec![source.to_string_lossy().into_owned()];
        let target = dir.path().join("chosen-uuid.wav");
        let token = record(&paths, "$uuid()", vec![target.clone()]).unwrap();
        assert_eq!(consume(&token, &paths, "$uuid()").unwrap()[0].1, target);
        assert!(consume(&token, &paths, "$uuid()").is_err());
    }
    #[test]
    fn changed_sources_and_selection_are_rejected_before_renaming() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        std::fs::write(&source, "original").unwrap();
        let paths = vec![source.to_string_lossy().into_owned()];
        let token = record(&paths, "%title%", vec![dir.path().join("target")]).unwrap();
        std::fs::write(&source, "changed-size").unwrap();
        assert!(consume(&token, &paths, "%title%").is_err());
        assert!(!dir.path().join("target").exists());
    }
}
