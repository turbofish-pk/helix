use anyhow::{bail, Context, Result};
use arc_swap::ArcSwap;

use crate::FileChange;

use gix::{
    helix_real_path, index_worktree::IndexWorktreeItem, ByteSlice, Commit, EntryKind,
    GixDirEntryStatus, IndexAsWorktreeChange, IndexAsWorktreeEntryStatus, ObjectId, Repository,
    RepositoryPath, ThreadSafeRepository,
};
use std::{io::Read, path::Path, sync::Arc};

#[inline]
fn get_repo_dir(file: &Path) -> Result<&Path> {
    file.parent().context("file has no parent directory")
}

pub fn get_diff_base(file: &Path) -> Result<Vec<u8>> {
    // let file = gix::path::realpath(file).context("resolve symlinks")?;
    let file = helix_real_path(file).context("resolve symlinks")?;

    // let file = helix_gix::path::realpath(file).context("resolve symlinks")?;

    // TODO cache repository lookup

    let repo_dir = &file.parent().context("file has no parent directory")?; //get_repo_dir(&file)?;
    let repo: Repository = open_repo(repo_dir) // <<---
        .context("failed to open git repo")?
        .into();
    let head = repo.helix_head_commit()?;
    eprintln!("HEAD ok");
    let file_oid = find_file_in_commit(&repo, &head, &file)?;

    // let file_object = repo.hx_find_object(file_oid)?;
    // let data = file_object.detach().data;
    let data = repo.helix_find_object_data(file_oid)?;
    eprintln!("obj: {} bytes", data.len());

    // Get the actual data that git would make out of the git object.
    // This will apply the user's git config or attributes like crlf conversions.
    //
    if let Some(work_dir) = repo.work_tree.as_deref() {
        // repo.workdir() {
        // let rela_path = file.strip_prefix(work_dir)?;
        // let rela_path = gix::path::try_into_bstr(rela_path)?;
        let rela_path: &[u8] = file.strip_prefix(work_dir)?.as_os_str().as_encoded_bytes();
        let mut pipeline = repo.helix_repository_filter_pipeline()?;
        // let mut worktree_outcome =
        //     pipeline.helix_pipeline_convert_to_worktree(&data, rela_path.as_ref())?;
        let mut worktree_outcome =
            pipeline.helix_pipeline_convert_to_worktree(&data, rela_path.as_ref());
        let mut buf = Vec::with_capacity(data.len());
        worktree_outcome.read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        Ok(data)
    }
}

pub fn get_current_head_name(file: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
    let file = helix_real_path(file).context("resolve symlinks")?;

    let repo_dir = get_repo_dir(&file)?;
    let repo: Repository = open_repo(repo_dir)
        .context("failed to open git repo")?
        .into();
    let head_ref = repo.helix_head_ref()?;
    let head_commit = repo.helix_head_commit()?;

    let name = match head_ref {
        Some(reference) => reference.name().shorten(),
        None => head_commit.id.hx_to_hex_with_len(),
    };

    Ok(Arc::new(ArcSwap::from_pointee(name.into_boxed_str())))
}

pub fn for_each_changed_file(cwd: &Path, f: impl Fn(Result<FileChange>) -> bool) -> Result<()> {
    git_status(&open_repo(cwd)?.into(), f)
}

fn open_repo(path: &Path) -> Result<ThreadSafeRepository> {
    let repo_path: RepositoryPath =
        gix::helix_discover_upwards_opts(path).context("failed to discover git repo")?;

    Ok(ThreadSafeRepository::helix_open_opts(
        repo_path.helix_into_repository_dir(),
    )?)
}

/// Emulates the result of running `git status` from the command line.
fn git_status(repo: &Repository, f: impl Fn(Result<FileChange>) -> bool) -> Result<()> {
    let work_dir = repo
        .work_tree
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("working tree not found"))?
        .to_path_buf();

    let status_platform = repo
        .helix_repository_status()?
        // Here we discard the `status.showUntrackedFiles` config, as it makes little sense in
        // our case to not list new (untracked) files. We could have respected this config
        // if the default value weren't `Collapsed` though, as this default value would render
        // the feature unusable to many.
        .helix_untracked_files() //(UntrackedFiles::Files)
        // Turn on file rename detection, which is off by default.
        .helix_index_worktree_rewrites();

    // No filtering based on path
    let status_iter = status_platform.helix_into_index_worktree_iter()?;

    for item in status_iter {
        let Ok(index_item) = item.map_err(|err| f(Err(err.into()))) else {
            continue;
        };
        let change = match index_item {
            IndexWorktreeItem::Modification {
                rela_path, status, ..
            } => {
                let path = work_dir.join(rela_path.to_path()?);
                match status {
                    IndexAsWorktreeEntryStatus::Conflict => FileChange::Conflict { path },
                    IndexAsWorktreeEntryStatus::Change(IndexAsWorktreeChange::Removed) => {
                        FileChange::Deleted { path }
                    }
                    IndexAsWorktreeEntryStatus::Change(IndexAsWorktreeChange::Modification {
                        ..
                    }) => FileChange::Modified { path },
                    // Files marked with `git add --intent-to-add`. Such files
                    // still show up as new in `git status`, so it's appropriate
                    // to show them the same way as untracked files in the
                    // "changed file" picker. One example of this being used
                    // is Jujutsu, a Git-compatible VCS. It marks all new files
                    // with `--intent-to-add` automatically.
                    IndexAsWorktreeEntryStatus::IntentToAdd => FileChange::Untracked { path },
                    _ => continue,
                }
            }
            IndexWorktreeItem::DirectoryContents { entry, .. }
                if entry.status == GixDirEntryStatus::Untracked =>
            {
                FileChange::Untracked {
                    path: work_dir.join(entry.rela_path.to_os_str().map(Path::new).unwrap()),
                }
            }
            IndexWorktreeItem::Rewrite {
                source,
                dirwalk_entry,
                ..
            } => FileChange::Renamed {
                from_path: work_dir.join(source.helix_rela_path().to_path()?),
                to_path: work_dir.join(dirwalk_entry.rela_path.to_path()?),
            },
            IndexWorktreeItem::DirectoryContents { .. } => continue,
        };
        if !f(Ok(change)) {
            break;
        }
    }

    Ok(())
}

/// Finds the object that contains the contents of a file at a specific commit.
fn find_file_in_commit(repo: &Repository, commit: &Commit, file: &Path) -> Result<ObjectId> {
    let repo_dir = repo.work_tree.as_deref().context("repo has no worktree")?;
    let rel_path = file.strip_prefix(repo_dir)?;
    let tree = commit.helix_commit_tree()?;
    let tree_entry = tree
        .helix_lookup_entry_by_path(rel_path)?
        .context("file is unracked")?;
    match tree_entry.inner.mode.kind() {
        // not a file, everything is new, do not show diff
        mode @ (EntryKind::Tree | EntryKind::Commit | EntryKind::Link) => {
            bail!("entry at {} is not a file but a {mode:?}", file.display())
        }
        // found a file
        EntryKind::Blob | EntryKind::BlobExecutable => Ok(tree_entry.inner.oid),
    }
}
