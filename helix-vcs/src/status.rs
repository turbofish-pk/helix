// use std::path::{Path, PathBuf};

// /// States for a file having been changed.
// pub enum FileChange {
//   /// Not tracked by the VCS.
//   Untracked { path: PathBuf },
//   /// File has been modified.
//   Modified { path: PathBuf },
//   /// File modification is in conflict with a different update.
//   Conflict { path: PathBuf },
//   /// File has been deleted.
//   Deleted { path: PathBuf },
//   /// File has been renamed.
//   Renamed { from_path: PathBuf, to_path: PathBuf },
// }

// impl FileChange {
//   #[must_use]
//   pub fn path(&self) -> &Path {
//     match self {
//       Self::Untracked { path } | Self::Modified { path } | Self::Conflict { path } | Self::Deleted { path } => path,
//       Self::Renamed { to_path, .. } => to_path,
//     }
//   }
// }
