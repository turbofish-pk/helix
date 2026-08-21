use anyhow::{Context, Result};
use arc_swap::ArcSwap;

// use crate::FileChange;

use helix_ext::git::{
    Commit,
    EntryKind,
    ObjectId,
    Repository,
    RepositoryPath,
    ThreadSafeRepository,
    helix_discover_upwards_opts,
    helix_real_path,
    // IndexAsWorktreeChange, ByteSlice,
    // IndexAsWorktreeEntryStatus, IndexWorktreeItem,
};
use std::{io::Read, path::Path, sync::Arc};

#[inline]
fn get_repo_dir(file: &Path) -> Result<&Path> {
    file.parent().context("file has no parent directory")
}
pub fn get_diff_base(file: &Path) -> Result<Vec<u8>> {
    let file = helix_real_path(file).context("resolve symlinks")?;
    // let file = helix_gix::path::realpath(file).context("resolve symlinks")?;
    // TODO cache repository lookup
    let repo_dir = &file.parent().context("file has no parent directory")?; //get_repo_dir(&file)?;
    let opened = open_repo(repo_dir);

    let repo: Repository = opened // <<---
        .context("failed to open git repo")?
        .into();
    let head = repo.helix_head_commit();

    let head = head?;
    let file_oid = find_file_in_commit(&repo, &head, &file)?;

    // let file_oid = file_oid?;

    let data = repo.helix_find_object_data(file_oid)?;

    // let data = data?;
    // Get the actual data that git would make out of the git object.
    // This will apply the user's git config or attributes like crlf conversions.
    //
    if let Some(work_dir) = repo.work_tree.as_deref() {
        let rela_path: &[u8] = file.strip_prefix(work_dir)?.as_os_str().as_encoded_bytes();
        let mut pipeline = repo.helix_repository_filter_pipeline()?;
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
        None => head_commit.id.helix_to_hex_with_len(),
    };

    Ok(Arc::new(ArcSwap::from_pointee(name.into_boxed_str())))
}

fn open_repo(path: &Path) -> Result<ThreadSafeRepository> {
    let repo_path: RepositoryPath =
        helix_discover_upwards_opts(path).context("failed to discover git repo")?;

    Ok(ThreadSafeRepository::helix_open_opts(
        repo_path.helix_into_repository_dir(),
    )?)
}

/// Finds the object that contains the contents of a file at a specific commit.
fn find_file_in_commit(repo: &Repository, commit: &Commit, file: &Path) -> Result<ObjectId> {
    let repo_dir = repo.work_tree.as_deref().context("repo has no worktree")?;

    // let tree = commit.helix_commit_tree()?;
    let tree_entry = commit
        .helix_commit_tree()?
        .helix_lookup_entry_by_path(file.strip_prefix(repo_dir)?)?
        .context("file is unracked")?;
    // Tree/Commit/Link mean everything is new, so there's no diff to show.
    matches!(
        tree_entry.inner.mode.kind(),
        EntryKind::Blob | EntryKind::BlobExecutable
    )
    .then_some(tree_entry.inner.oid)
    .with_context(|| format!("entry at {} is not a file", file.display()))
}
