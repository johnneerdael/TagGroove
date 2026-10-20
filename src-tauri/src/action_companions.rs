//! Frozen command-start membership for immediate native action renames.
//! This does not simulate actions or claim a human-reviewed destination receipt.
use crate::associated_files::{self, CreatedDirectories, Item, Plan};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use tag_core::{
    actions::{ActionGroup, ActionSpec},
    batch_errors::ErrorCause,
    file_copy::{DirectoryIdentity, FileIdentity},
};

fn can_rename(action: &ActionSpec) -> bool {
    match action {
        ActionSpec::FormatValue { field, .. } => field.eq_ignore_ascii_case("_FILENAME"),
        ActionSpec::CaseConversion { field, .. }
        | ActionSpec::ConvertCodepage { field, .. }
        | ActionSpec::Replace { field, .. }
        | ActionSpec::RegexReplace { field, .. } => {
            field.eq_ignore_ascii_case("_FILENAME") || field.eq_ignore_ascii_case("_ALL")
        }
        ActionSpec::GuessValues { pattern, .. } => tag_core::actions::guess_value_targets(pattern)
            .is_ok_and(|names| names.iter().any(|name| name == "_FILENAME")),
        _ => false,
    }
}
struct Member {
    original: String,
    item: Item,
}
struct Group {
    audio: PathBuf,
    members: Vec<Member>,
    invalid: bool,
    uncertain_audio: bool,
}
#[derive(Default)]
pub(crate) struct Prepared {
    groups: Option<BTreeMap<String, Group>>,
    blocked: std::collections::HashSet<String>,
}
impl Prepared {
    pub(crate) fn prepare(
        paths: &[String],
        groups: &[ActionGroup],
        enabled: bool,
    ) -> Result<Self, String> {
        if !enabled
            || !groups
                .iter()
                .filter(|group| group.enabled)
                .flat_map(|group| &group.actions)
                .any(can_rename)
        {
            return Ok(Self::default());
        }
        let mut originals = Vec::new();
        let mut pairs = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for path in paths {
            if !seen.insert(path.clone()) {
                continue;
            }
            // Retain the runner's per-file read-error behavior. Successfully read
            // inputs still require regular leaves; never resolve an audio symlink
            // into permission to rename a different physical file.
            if let Ok(entry) = crate::settings::read_summary(Path::new(path)) {
                if !fs::symlink_metadata(&entry.path)
                    .map_err(|error| error.to_string())?
                    .is_file()
                {
                    return Err(
                        "Action rename inputs must be regular files, not symbolic links".into(),
                    );
                }
                let source = dunce::canonicalize(&entry.path).map_err(|error| error.to_string())?;
                originals.push(entry.path);
                pairs.push((source.to_string_lossy().into_owned(), source));
            }
        }
        let plan = Plan::new(pairs, true)?;
        let mut captured = BTreeMap::new();
        for (original, group) in originals.into_iter().zip(plan.groups) {
            captured.insert(
                original,
                Group {
                    audio: PathBuf::from(group.audio.source),
                    members: group
                        .associated
                        .into_iter()
                        .map(|item| Member {
                            original: item.source.clone(),
                            item,
                        })
                        .collect(),
                    invalid: false,
                    uncertain_audio: false,
                },
            );
        }
        Ok(Self {
            groups: Some(captured),
            blocked: Default::default(),
        })
    }
    pub(crate) fn active(&self) -> bool {
        self.groups.is_some()
    }
    pub(crate) fn history_sources(&self, paths: &[String]) -> Vec<String> {
        let mut captured = paths.to_vec();
        if let Some(groups) = &self.groups {
            captured.extend(
                groups
                    .values()
                    .flat_map(|group| group.members.iter().map(|member| member.original.clone())),
            );
        }
        captured
    }
    pub(crate) fn audio_uncertain(&self, original: &str) -> bool {
        self.blocked.contains(original)
            || self
                .groups
                .as_ref()
                .and_then(|groups| groups.get(original))
                .is_some_and(|group| group.uncertain_audio)
    }
    pub(crate) fn current_audio(&self, original: &str) -> Option<&Path> {
        self.groups
            .as_ref()?
            .get(original)
            .map(|group| group.audio.as_path())
    }
    pub(crate) fn changes(&self) -> Vec<crate::action_host::PathChange> {
        self.groups
            .iter()
            .flat_map(|groups| groups.values())
            .flat_map(|group| &group.members)
            .filter(|member| member.original != member.item.source)
            .map(|member| crate::action_host::PathChange {
                from: member.original.clone(),
                to: member.item.source.clone(),
            })
            .collect()
    }
    pub(crate) fn validate_directory(&self, source: &Path) -> Result<(), ErrorCause> {
        if let Some(groups) = &self.groups {
            for group in groups.values().filter(|group| {
                group.audio.starts_with(source)
                    || group
                        .members
                        .iter()
                        .any(|member| Path::new(&member.item.source).starts_with(source))
            }) {
                group.validate()?;
            }
        }
        Ok(())
    }
    /// Guard known selected inputs even when companion participation is off.
    pub(crate) fn guard_operation(
        &mut self,
        source: &Path,
        target: &Path,
        inputs: &[(String, PathBuf)],
        operation: &mut dyn FnMut(&Path, &Path) -> Result<(), ErrorCause>,
    ) -> Result<(), ErrorCause> {
        self.validate_directory(source)?;
        let directory = DirectoryIdentity::read(source).map_err(ErrorCause::owned_text)?;
        let mut captured = Vec::new();
        for (key, path) in inputs.iter().filter(|(_, path)| path.starts_with(source)) {
            if self.audio_uncertain(key) {
                return Err(ErrorCause::owned("An earlier audio rollback is incomplete; inspect the reported paths before continuing"));
            }
            captured.push((
                key.clone(),
                path.clone(),
                FileIdentity::read(path).map_err(ErrorCause::owned_text)?,
                associated_files::hash_file(path).map_err(ErrorCause::owned_text)?,
            ));
        }
        if captured.is_empty() {
            return Err(ErrorCause::owned(
                "The action operation has no captured selected input",
            ));
        }
        match operation(source, target) {
            Ok(()) => Ok(()),
            Err(error) => {
                // Failed alias operations can leave files/folders in a private
                // stage. Never let later flushes adopt an outside occupant.
                let directory_intact = DirectoryIdentity::read(source).ok() == Some(directory);
                for (key, path, identity, hash) in captured {
                    let intact = directory_intact
                        && FileIdentity::read(&path).ok() == Some(identity)
                        && associated_files::hash_file(&path).ok() == Some(hash);
                    if !intact {
                        self.blocked.insert(key.clone());
                    }
                    if let Some(group) =
                        self.groups.as_mut().and_then(|groups| groups.get_mut(&key))
                    {
                        if !intact {
                            group.uncertain_audio = true;
                            group.invalid = true;
                        } else if group.validate().is_err() {
                            group.invalid = true;
                        }
                    }
                }
                Err(error)
            }
        }
    }
    pub(crate) fn moved_directory(&mut self, from: &Path, to: &Path) {
        if let Some(groups) = &mut self.groups {
            for group in groups.values_mut() {
                if let Ok(suffix) = group.audio.strip_prefix(from) {
                    group.audio = to.join(suffix);
                }
                for member in &mut group.members {
                    if let Ok(suffix) = Path::new(&member.item.source).strip_prefix(from) {
                        let path = to.join(suffix);
                        member.item.follow_move(path, None);
                    }
                }
            }
        }
    }
    pub(crate) fn rename(
        &mut self,
        original: &str,
        path: &Path,
        target: &Path,
        after: &mut dyn FnMut() -> Result<(), ErrorCause>,
    ) -> Result<(), ErrorCause> {
        let group = self
            .groups
            .as_mut()
            .and_then(|groups| groups.get_mut(original))
            .ok_or_else(|| {
                ErrorCause::owned("The action input has no frozen companion membership")
            })?;
        if dunce::canonicalize(path).map_err(ErrorCause::external)? != group.audio {
            return Err(ErrorCause::owned(
                "The action audio path changed outside its tracked moves",
            ));
        }
        group.validate()?;
        let plan = Plan::for_action(vec![(
            group.audio.to_string_lossy().into_owned(),
            target.to_owned(),
        )])
        .map_err(ErrorCause::owned_text)?;
        // Building a current audio plan must not adopt a newly appearing or
        // replaced companion after the command-start freeze.
        group.validate()?;
        let report = move_group(
            &plan.groups[0],
            &mut tag_core::operations::rename_no_clobber_cause,
            after,
        );
        let failed = report.error.is_some();
        group.invalid |= report.unconfirmed_member || report.uncertain_audio;
        group.uncertain_audio |= report.uncertain_audio;
        for restored in report.restored {
            if let Some(member) = group
                .members
                .iter_mut()
                .find(|member| member.item.source == restored.from)
            {
                member.item.follow_move(restored.to, restored.identity);
            }
        }
        for movement in report.moved {
            if Path::new(&movement.from) == group.audio {
                group.audio = movement.to.clone();
                if failed {
                    group.uncertain_audio = true;
                }
            }
            if let Some(member) = group
                .members
                .iter_mut()
                .find(|member| member.item.source == movement.from)
            {
                member.item.follow_move(movement.to, movement.identity);
            }
            if failed {
                group.invalid = true;
            }
        }
        report.error.map_or(Ok(()), Err)
    }
}
impl Group {
    fn validate(&self) -> Result<(), ErrorCause> {
        if self.invalid {
            return Err(ErrorCause::owned("An earlier companion rollback is incomplete; inspect the reported paths before another rename"));
        }
        let current = associated_files::companions(&self.audio).map_err(ErrorCause::owned_text)?;
        let mut expected = self
            .members
            .iter()
            .map(|member| PathBuf::from(&member.item.source))
            .collect::<Vec<_>>();
        expected.sort();
        if current != expected {
            return Err(ErrorCause::owned(
                "Action companion membership changed after command preparation",
            ));
        }
        for member in &self.members {
            member.item.validate().map_err(ErrorCause::owned_text)?;
        }
        Ok(())
    }
}
struct Movement {
    from: String,
    to: PathBuf,
    identity: Option<FileIdentity>,
}
struct Completed {
    movement: Movement,
    hash: [u8; 32],
    metadata: Option<fs::Metadata>,
    source_parent: DirectoryIdentity,
    target_parent: DirectoryIdentity,
}
struct MoveReport {
    moved: Vec<Movement>,
    restored: Vec<Movement>,
    error: Option<ErrorCause>,
    uncertain_audio: bool,
    unconfirmed_member: bool,
}
fn move_group(
    group: &associated_files::Group,
    operation: &mut dyn FnMut(&Path, &Path) -> Result<(), ErrorCause>,
    after: &mut dyn FnMut() -> Result<(), ErrorCause>,
) -> MoveReport {
    let mut completed: Vec<Completed> = Vec::new();
    let mut directories = CreatedDirectories::default();
    let audio_source = Path::new(&group.audio.source);
    let mut audio_expected = Some(group.audio.native_identity());
    let audio_parent = audio_source
        .parent()
        .and_then(|path| DirectoryIdentity::read(path).ok());
    let mut unconfirmed_member = false;
    let outcome = (|| {
        group.validate().map_err(ErrorCause::owned_text)?;
        for item in group.associated.iter().chain(std::iter::once(&group.audio)) {
            let source = Path::new(&item.source);
            let target = &item.target;
            item.validate().map_err(ErrorCause::owned_text)?;
            if source == target {
                continue;
            }
            let hash = item.content_hash().ok_or_else(|| {
                ErrorCause::owned("An action rename member has no frozen content digest")
            })?;
            directories
                .prepare(target)
                .map_err(ErrorCause::owned_text)?;
            let source_parent = DirectoryIdentity::read(
                source
                    .parent()
                    .ok_or_else(|| ErrorCause::owned("Source has no parent directory"))?,
            )
            .map_err(ErrorCause::owned_text)?;
            let target_parent = DirectoryIdentity::read(
                target
                    .parent()
                    .ok_or_else(|| ErrorCause::owned("Destination has no parent directory"))?,
            )
            .map_err(ErrorCause::owned_text)?;
            let original_identity = item.native_identity();
            if let Err(error) = operation(source, target) {
                // A failed native operation can have moved to a private stage
                // before its own reversal failed. Never infer an unchanged
                // source merely from Err, or adopt an occupant at that name.
                let intact = FileIdentity::read(source).ok() == Some(original_identity)
                    && associated_files::hash_file(source).ok() == Some(hash)
                    && source
                        .parent()
                        .and_then(|path| DirectoryIdentity::read(path).ok())
                        == Some(source_parent.clone());
                unconfirmed_member |= !intact;
                return Err(error);
            }
            completed.push(Completed {
                movement: Movement {
                    from: item.source.clone(),
                    to: target.clone(),
                    identity: None,
                },
                hash,
                metadata: None,
                source_parent,
                target_parent,
            });
            let metadata = fs::symlink_metadata(target).map_err(ErrorCause::external)?;
            let identity = FileIdentity::read(target).map_err(ErrorCause::owned_text)?;
            let last = completed.last_mut().unwrap();
            last.metadata = Some(metadata);
            last.movement.identity = Some(identity);
        }
        after()
    })();
    let Err(error) = outcome else {
        return MoveReport {
            moved: completed.into_iter().map(|item| item.movement).collect(),
            restored: Vec::new(),
            error: None,
            uncertain_audio: false,
            unconfirmed_member: false,
        };
    };
    let had_moves = !completed.is_empty();
    let mut remaining = Vec::new();
    let mut restored = Vec::new();
    let mut failures = Vec::new();
    for item in completed.into_iter().rev() {
        let rollback = (|| {
            if DirectoryIdentity::read(
                Path::new(&item.movement.from)
                    .parent()
                    .ok_or_else(|| ErrorCause::owned("Source has no parent directory"))?,
            )
            .map_err(ErrorCause::owned_text)?
                != item.source_parent
                || DirectoryIdentity::read(
                    item.movement
                        .to
                        .parent()
                        .ok_or_else(|| ErrorCause::owned("Destination has no parent directory"))?,
                )
                .map_err(ErrorCause::owned_text)?
                    != item.target_parent
            {
                return Err(ErrorCause::owned(
                    "An action rename parent changed; retained the moved file",
                ));
            }
            let Some(identity) = item.movement.identity else {
                return Err(ErrorCause::owned(
                    "Moved file identity could not be captured; retained at destination",
                ));
            };
            if FileIdentity::read(&item.movement.to).map_err(ErrorCause::owned_text)? != identity {
                return Err(ErrorCause::owned(
                    "The moved action file changed; retained it at its destination",
                ));
            }
            let metadata = fs::symlink_metadata(&item.movement.to).map_err(ErrorCause::external)?;
            let Some(expected) = item.metadata else {
                return Err(ErrorCause::owned(
                    "Moved file identity could not be captured; retained at destination",
                ));
            };
            if metadata.permissions() != expected.permissions()
                || metadata.created().ok() != expected.created().ok()
            {
                return Err(ErrorCause::owned(
                    "The moved action file changed; retained it at its destination",
                ));
            }
            // Reuse the common bounded reader, comparing the original bytes.
            let probe =
                associated_files::hash_file(&item.movement.to).map_err(ErrorCause::owned_text)?;
            if probe != item.hash {
                return Err(ErrorCause::owned(
                    "The moved action file changed; retained it at its destination",
                ));
            }
            tag_core::operations::rename_no_clobber_cause(
                &item.movement.to,
                Path::new(&item.movement.from),
            )
        })();
        if let Err(problem) = rollback {
            failures.push(ErrorCause::wrap(
                "{value}: {error}",
                [("value", item.movement.to.to_string_lossy().into_owned())],
                "error",
                problem,
            ));
            remaining.push(item.movement);
        } else {
            // A cross-volume forward move and successful reversal can create
            // a new native ID. Carry that proven own result into later actions.
            let identity = FileIdentity::read(Path::new(&item.movement.from)).ok();
            if Path::new(&item.movement.from) == audio_source {
                audio_expected = identity;
            }
            unconfirmed_member |= identity.is_none();
            restored.push(Movement {
                to: PathBuf::from(&item.movement.from),
                from: item.movement.from,
                identity,
            });
        }
    }
    let uncertain_audio = audio_expected.is_none()
        || FileIdentity::read(audio_source).ok() != audio_expected
        || audio_source
            .parent()
            .and_then(|path| DirectoryIdentity::read(path).ok())
            != audio_parent
        || associated_files::hash_file(audio_source).ok() != group.audio.content_hash();
    for (path, problem) in directories.cleanup() {
        failures.push(ErrorCause::wrap(
            "{value}: {error}",
            [("value", path.to_string_lossy().into_owned())],
            "error",
            ErrorCause::external(problem),
        ));
    }
    let error = if failures.is_empty() {
        if had_moves {
            ErrorCause::joined(
                vec![
                    error,
                    ErrorCause::owned("Completed group moves were rolled back"),
                ],
                "; ",
            )
        } else {
            error
        }
    } else {
        let mut parts = vec![
            error,
            ErrorCause::owned("Companion rename rollback incomplete"),
        ];
        parts.extend(failures);
        ErrorCause::joined(parts, "; ")
    };
    MoveReport {
        moved: remaining,
        restored,
        error: Some(error),
        uncertain_audio,
        unconfirmed_member,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(root: &Path) -> (PathBuf, PathBuf) {
        let audio = root.join("old.wav");
        let lyric = root.join("old.lrc");
        fs::write(&audio, b"audio").unwrap();
        fs::write(&lyric, b"lyric").unwrap();
        (audio, lyric)
    }
    #[test]
    fn action_move_successful_reverse_carries_the_new_native_identity_after_copy_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let (audio, lyric) = fixture(&root);
        let plan = Plan::for_action(vec![(
            audio.to_string_lossy().into_owned(),
            root.join("new.wav"),
        )])
        .unwrap();
        let mut frozen = plan.groups[0].associated[0].clone();
        let original = frozen.native_identity();
        let mut operation = |source: &Path, target: &Path| {
            if source == audio {
                return Err(ErrorCause::external(
                    "Injected failure after companion copy-move",
                ));
            }
            fs::copy(source, target).map_err(ErrorCause::external)?;
            fs::remove_file(source).map_err(ErrorCause::external)
        };
        let report = move_group(&plan.groups[0], &mut operation, &mut || Ok(()));
        assert!(report.error.is_some() && report.moved.is_empty());
        assert!(!report.unconfirmed_member && !report.uncertain_audio);
        assert_eq!(report.restored.len(), 1);
        let restored = &report.restored[0];
        assert_eq!(restored.to, lyric);
        assert_ne!(restored.identity, Some(original));
        frozen.follow_move(restored.to.clone(), restored.identity);
        frozen.validate().unwrap();
        assert_eq!(fs::read(&lyric).unwrap(), b"lyric");
    }
    #[test]
    fn action_move_unrecorded_partial_failures_mark_unknown_ownership_without_adopting_occupants() {
        for audio_failure in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let root = dunce::canonicalize(dir.path()).unwrap();
            let (audio, lyric) = fixture(&root);
            let target = root.join("new.wav");
            let plan =
                Plan::for_action(vec![(audio.to_string_lossy().into_owned(), target)]).unwrap();
            let retained = root.join("Retained-original");
            let mut operation = |source: &Path, destination: &Path| {
                if (source == audio) == audio_failure {
                    fs::rename(source, &retained).unwrap();
                    fs::write(source, b"Outside occupant").unwrap();
                    Err(ErrorCause::Rename {
                        value: Box::new(tag_core::operations::RenameFailure::AliasRollback {
                            source: source.to_string_lossy().into_owned(),
                            destination: destination.to_string_lossy().into_owned(),
                            error: "Injected partial native failure".into(),
                            rollback: format!("Retained at {}", retained.display()),
                        }),
                    })
                } else {
                    tag_core::operations::rename_no_clobber_cause(source, destination)
                }
            };
            let report = move_group(&plan.groups[0], &mut operation, &mut || Ok(()));
            assert!(report.error.is_some());
            assert!(report.unconfirmed_member);
            assert_eq!(report.uncertain_audio, audio_failure);
            assert!(
                report.moved.is_empty(),
                "unknown staging locations must not invent successful path mappings"
            );
            assert_eq!(
                fs::read(&retained).unwrap(),
                if audio_failure {
                    b"audio".as_slice()
                } else {
                    b"lyric"
                }
            );
            assert_eq!(
                fs::read(if audio_failure { &audio } else { &lyric }).unwrap(),
                b"Outside occupant"
            );
        }
    }
    #[test]
    fn action_move_failure_restores_members_cleans_only_owned_empty_parents_and_preserves_typed_cause(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let (audio, lyric) = fixture(&root);
        let target = root.join("New/Nested/new.wav");
        let plan =
            Plan::for_action(vec![(audio.to_string_lossy().into_owned(), target.clone())]).unwrap();
        let mut operation = |source: &Path, destination: &Path| {
            if source == audio {
                Err(ErrorCause::Rename {
                    value: Box::new(tag_core::operations::RenameFailure::CannotCreate {
                        path: destination.to_string_lossy().into_owned(),
                        error: "Injected IO failure".into(),
                    }),
                })
            } else {
                tag_core::operations::rename_no_clobber_cause(source, destination)
            }
        };
        let report = move_group(&plan.groups[0], &mut operation, &mut || {
            panic!("readback must not run")
        });
        assert!(report.moved.is_empty());
        let wire = serde_json::to_value(report.error.unwrap()).unwrap();
        assert_eq!(wire["kind"], "joined");
        assert_eq!(wire["parts"][0]["kind"], "rename");
        assert_eq!(wire["parts"][0]["value"]["kind"], "cannot_create");
        assert_eq!(
            wire["parts"][0]["value"]["path"],
            target.to_string_lossy().as_ref()
        );
        assert_eq!(fs::read(&audio).unwrap(), b"audio");
        assert_eq!(fs::read(&lyric).unwrap(), b"lyric");
        assert!(!root.join("New").exists());
    }
    #[test]
    fn action_move_rollback_retains_external_bytes_equal_replacements_and_source_occupants() {
        for change in ["bytes", "replacement", "occupant"] {
            let dir = tempfile::tempdir().unwrap();
            let root = dunce::canonicalize(dir.path()).unwrap();
            let (audio, lyric) = fixture(&root);
            let target = root.join("new.wav");
            let moved = target.with_extension("lrc");
            let plan =
                Plan::for_action(vec![(audio.to_string_lossy().into_owned(), target.clone())])
                    .unwrap();
            let mut operation = |source: &Path, destination: &Path| {
                if source == audio {
                    match change {
                        "bytes" => fs::write(&moved, b"External lyric").unwrap(),
                        "replacement" => {
                            let modified = fs::metadata(&moved).unwrap().modified().unwrap();
                            fs::rename(&moved, root.join("retained")).unwrap();
                            fs::write(&moved, b"lyric").unwrap();
                            fs::File::options()
                                .write(true)
                                .open(&moved)
                                .unwrap()
                                .set_times(fs::FileTimes::new().set_modified(modified))
                                .unwrap();
                        }
                        _ => fs::write(&lyric, b"Outside occupant").unwrap(),
                    }
                    Err(ErrorCause::external("Injected audio failure"))
                } else {
                    tag_core::operations::rename_no_clobber_cause(source, destination)
                }
            };
            let report = move_group(&plan.groups[0], &mut operation, &mut || Ok(()));
            assert!(report
                .error
                .as_ref()
                .unwrap()
                .to_string()
                .contains(moved.to_str().unwrap()));
            assert_eq!(report.moved.len(), 1);
            assert_eq!(report.moved[0].to, moved);
            assert_eq!(fs::read(&audio).unwrap(), b"audio");
            assert_eq!(
                fs::read(&moved).unwrap(),
                if change == "bytes" {
                    b"External lyric".as_slice()
                } else {
                    b"lyric"
                }
            );
            if change == "occupant" {
                assert_eq!(fs::read(&lyric).unwrap(), b"Outside occupant");
            } else {
                assert!(!lyric.exists());
            }
        }
    }
    #[test]
    fn action_move_readback_failure_rolls_back_audio_and_companions_together() {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let (audio, lyric) = fixture(&root);
        let target = root.join("New/new.wav");
        let plan =
            Plan::for_action(vec![(audio.to_string_lossy().into_owned(), target.clone())]).unwrap();
        let report = move_group(
            &plan.groups[0],
            &mut tag_core::operations::rename_no_clobber_cause,
            &mut || Err(ErrorCause::external("Injected readback failure")),
        );
        assert!(report.error.is_some() && report.moved.is_empty());
        assert_eq!(fs::read(&audio).unwrap(), b"audio");
        assert_eq!(fs::read(&lyric).unwrap(), b"lyric");
        assert!(!root.join("New").exists());
    }
}
