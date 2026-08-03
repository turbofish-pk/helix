//! `helix_vcs` provides types for working with diffs from a Version Control System (VCS).
//! Currently `git` is the only supported provider for diffs, but this architecture allows
//! for other providers to be added in the future.
mod diff;
pub mod git;
mod status;
pub use crate::git::get_current_head_name;
pub use crate::git::get_diff_base;

use anyhow::{anyhow, bail, Result};
use arc_swap::ArcSwap;
pub use diff::{DiffHandle, Hunk};
pub use status::FileChange;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// Contains all active diff providers. Diff providers are compiled in via features. Currently
/// only `git` is supported.
#[derive(Default, Clone)]
pub struct DiffProviderRegistry {}
// pub type DiffProviderRegistry = DiffProvider;

impl DiffProviderRegistry {
    /// Get the given file from the VCS. This provides the unedited document as a "base"
    /// for a diff to be created.
    pub fn get_diff_base(&self, file: &Path) -> Option<Vec<u8>> {
        match git::get_diff_base(file) {
            Ok(res) => Some(res),
            Err(err) => {
                log::debug!("{err:#?}");
                log::debug!("failed to open diff base for {}", file.display());
                None
            }
        }
    }

    /// Get the current name of the current [HEAD](https://stackoverflow.com/questions/2304087/what-is-head-in-git).
    pub fn get_current_head_name(&self, file: &Path) -> Option<Arc<ArcSwap<Box<str>>>> {
        match git::get_current_head_name(file) {
            Ok(res) => Some(res),
            Err(err) => {
                log::debug!("{err:#?}");
                log::debug!("failed to obtain current head name for {}", file.display());
                None
            }
        }
    }

    /// Fire-and-forget changed file iteration. Runs everything in a background task. Keeps
    /// iteration until `on_change` returns `false`.
    pub fn for_each_changed_file(
        self,
        cwd: PathBuf,
        f: impl Fn(Result<FileChange>) -> bool + Send + 'static,
    ) {
        tokio::task::spawn_blocking(move || {
            if git::for_each_changed_file(&cwd, &f).ok().is_none() {
                f(Err(anyhow!("no diff provider returns success")));
            }
        });
    }
}
