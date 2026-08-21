use bitflags::bitflags;

use crate::git::Header::{OfsDelta, RefDelta};

use crate::git::State::AttributesStack;
use crate::git::search::MetadataCollection;
pub use bstr::ByteSlice;
use bstr::{BStr, BString, ByteVec};

use std::{
    borrow::{Borrow, BorrowMut, Cow},
    cell::RefCell,
    cmp,
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    convert,
    ffi::OsStr,
    fmt,
    io::{self, Read},
    ops::{ControlFlow, Deref, DerefMut},
    path::{
        Component::{self, CurDir, Normal, ParentDir, Prefix, RootDir},
        Path, PathBuf,
    },
    sync::{Arc, atomic::Ordering},
};

const DOT_GIT_DIR: &str = ".git";
const MODULES: &str = "modules";

const ATTRS: [&str; 6] = [
    "crlf",
    "ident",
    "filter",
    "eol",
    "text",
    "working-tree-encoding",
];
#[derive(Clone)]
struct GixWorktreeStack {
    stack: GixWorktreeGixFsStack,
    state: State,
    case: Case,
}

#[derive(Clone)]
enum State {
    AttributesStack(Attributes),
}
/// Walks up from `directory` looking for a `.git` entry, returning the first
/// repository found.
///
/// Unlike `git`, this does not stop at filesystem boundaries: discovery is for a
/// locally-opened file, so crossing into a separate partition or bind mount is
/// almost always what the user wants.
pub fn helix_discover_upwards_opts(directory: &Path) -> Result<RepositoryPath, GitError> {
    let error = || GitError::Gen;
    let current_dir = std::env::current_dir().map_err(|_| GitError::Gen)?;
    let dir = hx_path_normalize(directory.into(), &current_dir).ok_or_else(&error)?;
    if !dir.metadata().map_err(|_| GitError::Gen)?.is_dir() {
        return Err(GitError::Gen);
    }
    let mut cursor = dir.into_owned();
    loop {
        if cursor.file_name() != Some(OsStr::new(DOT_GIT_DIR)) {
            cursor.push(DOT_GIT_DIR);
        }
        if let Ok(repository_kind) = hx_is_git(&cursor) {
            return RepositoryPath::hx_from_dot_git_dir(cursor, repository_kind, &current_dir)
                .ok_or_else(error);
        }
        cursor.pop();
        if cursor.as_os_str().is_empty() || cursor.as_os_str() == OsStr::new(".") {
            cursor.clone_from(&current_dir);
        }
        if !cursor.pop() {
            return Err(GitError::Gen);
        }
    }
}

// impl Pipeline<'_> {
impl Pipeline {
    //helix
    pub fn helix_pipeline_convert_to_worktree<'input>(
        &mut self,
        src: &'input [u8],
        rela_path: &BStr,
    ) -> ToWorktreeOutcome<'input, '_> {
        let entry = self.cache.hx_at_entry(rela_path, None).expect("entry");
        self.inner
            .hx_convert_to_worktree(src, rela_path, &mut |_, attrs| {
                entry.hx_platform_matching_attributes(attrs);
            })
            .expect("helix_pipeline_convert_to_worktree")
    }
}

impl Repository {
    pub fn helix_repository_filter_pipeline(&self) -> Result<Pipeline, GitError> {
        Pipeline::new(
            self,
            GixWorktreeStack::new(
                self.work_tree.as_deref().unwrap_or(&self.refs.git_dir),
                AttributesStack(self.repository_config.hx_assemble_attribute_globals()),
                Case::Sensitive,
            ),
        )
        .map_err(|_| GitError::Gen)
    }
    pub fn helix_find_object_data(&self, id: impl Into<ObjectId>) -> Result<Vec<u8>, GitError> {
        Ok(self.hx_find_object(id)?.detach().data)
    }
    fn hx_find_object(&self, id: impl Into<ObjectId>) -> Result<Object<'_>, GitError> {
        let id = id.into(); // 20 bytes long sha1 hash

        let empty_tree_sha1 = ObjectId::Sha1([
            0x4b, 0x82, 0x5d, 0xc6, 0x42, 0xcb, 0x6e, 0xb9, 0xa0, 0x60, 0xe5, 0x4b, 0xf8, 0xd6,
            0x92, 0x88, 0xfb, 0xee, 0x49, 0x04,
        ]);

        if id == empty_tree_sha1 {
            return Ok(Object {
                id,
                kind: ObjectKind::Tree,
                data: Vec::new(),
                repo: self,
            });
        }
        let mut buf = self
            .bufs
            .as_ref()
            .and_then(|bufs| bufs.borrow_mut().pop())
            .unwrap_or_default();

        Ok(Object {
            id,
            kind: self.objects.find(&id, &mut buf)?.kind,
            data: buf,
            repo: self,
        })
    }

    pub fn helix_head_ref(&self) -> Result<Option<Reference<'_>>, GitError> {
        Ok(self.hx_head()?.try_into_referent())
    }
    pub fn helix_head_commit(&self) -> Result<Commit<'_>, GitError> {
        self.hx_head()?.hx_peel_to_commit()
    }

    fn hx_head(&self) -> Result<Head<'_>, GitError> {
        let head = self.hx_find_reference("HEAD")?;
        Ok(match head.inner.target {
            Target::Symbolic(branch) => match self.hx_find_reference(&branch) {
                Ok(r) => GixGitHeadKind::Symbolic(r.detach()),
                Err(GitError::NotFound) => GixGitHeadKind::Unborn(branch),
                Err(_) => return Err(GitError::Gen),
            },
            Target::Object(target) => GixGitHeadKind::Detached {
                target,
                peeled: head.inner.peeled,
            },
        }
        .attach(self))
    }

    fn hx_find_reference<'a, Name>(&self, name: Name) -> Result<Reference<'_>, GitError>
    where
        Name: TryInto<&'a PartialNameRef>,
    {
        self.refs
            .try_find(name)
            .ok()
            .flatten()
            .map(|r| Reference::from_ref(r, self))
            .ok_or(GitError::NotFound)
    }

    #[inline]
    fn free_buf(&self) -> Vec<u8> {
        self.bufs
            .as_ref()
            .and_then(|bufs| bufs.borrow_mut().pop())
            .unwrap_or_default()
    }

    #[inline]
    fn reuse_buffer(&self, data: &mut Vec<u8>) {
        if data.capacity() > 0
            && let Some(bufs) = self.bufs.as_ref()
        {
            bufs.borrow_mut().push(std::mem::take(data));
        }
    }
}

#[derive(Debug)]
pub enum GitError {
    Gen,
    Io,
    NotFound,
    Unborn,
    Error,
    KeyMissing,
    DeltaBaseUnresolved(ObjectId),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for GitError {}

pub struct Repository {
    refs: RefFileStore,
    objects: OdbCache<OdbHandle<Arc<Store>>>, // helix

    pub work_tree: Option<PathBuf>, // helix
    bufs: Option<RefCell<Vec<Vec<u8>>>>,
    repository_config: Cache,
}
#[derive(Clone)]
pub struct ThreadSafeRepository {
    refs: RefFileStore,
    objects: std::sync::Arc<Store>,
    work_tree: Option<PathBuf>,
    thread_safe_repo_config: Cache,
}
// helix transitively
#[derive(Clone)]
pub struct Tree<'repo> {
    data: Vec<u8>,
    repo: &'repo Repository,
}
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RepositoryPath {
    LinkedWorkTree { work_dir: PathBuf, git_dir: PathBuf },
    WorkTree(PathBuf),
    Repository(PathBuf),
}

// helix
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct GixDirEntry {
    pub rela_path: BString,
}

impl ThreadSafeRepository {
    // helix
    pub fn helix_open_opts(path: impl Into<PathBuf>) -> Result<Self, GitError> {
        let (repository_path, repository_kind) = {
            let path = path.into();
            let candidate = Cow::Borrowed(&path);
            match hx_is_git(candidate.as_ref()) {
                Ok(kind) => (candidate.into_owned(), kind),
                Err(_) => {
                    return Err(GitError::Gen);
                }
            }
        };

        let cwd = std::env::current_dir().map_err(|_| GitError::Gen)?;
        let (git_dir, mut work_tree_dir) =
            RepositoryPath::hx_from_dot_git_dir(repository_path, repository_kind, &cwd)
                .expect("we have sanitized path with is_git()")
                .hx_into_repository_and_work_tree_directories();

        let refs = RefFileStore::at(git_dir.clone());
        let _head = refs.find("HEAD").ok();

        let config = Cache::hx_from_stage_one(StageOne::new(&git_dir), &git_dir)?;

        if work_tree_dir.is_none() {
            work_tree_dir = Some(
                git_dir
                    .parent()
                    .expect("parent is always available")
                    .to_owned(),
            );
        }

        Ok(ThreadSafeRepository {
            objects: Arc::new(
                Store::at_opts(git_dir.join("objects"), cwd.clone()).map_err(|_| GitError::Gen)?,
            ),
            refs,
            work_tree: work_tree_dir,
            thread_safe_repo_config: config,
        })
    }
}

impl RewriteSource {
    #[must_use]
    pub fn helix_rela_path(&self) -> &bstr::BStr {
        match self {
            RewriteSource::RewriteFromIndex {
                source_rela_path, ..
            } => source_rela_path.as_ref(),
            RewriteSource::CopyFromDirectoryEntry {
                source_dirwalk_entry,
            } => source_dirwalk_entry.rela_path.as_ref(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum GitRepositoryKind {
    PossiblyBare,
    WorkTree { linked_git_dir: Option<PathBuf> },
    WorkTreeGitDir { work_dir: PathBuf },
    Submodule { git_dir: PathBuf },
    SubmoduleGitDir,
}

impl<'repo> Tree<'repo> {
    /// helix ++++
    pub fn helix_lookup_entry_by_path(
        &self,
        relative_path: impl AsRef<std::path::Path>,
    ) -> Result<Option<GitRepoEntry>, GitError> {
        let mut inner = self.repo.free_buf();

        inner.clear();
        let mut buf = Buffer {
            inner,
            _repo: self.repo,
        };

        buf.extend_from_slice(&self.data);

        let mut iter = relative_path
            .as_ref()
            .components()
            .map(|component| component.as_os_str().as_encoded_bytes())
            .peekable();

        let mut data = ObjectData::new(&buf, ObjectKind::Tree);

        loop {
            data = match next_entry(&mut iter, data) {
                ControlFlow::Continue(oid) => self.repo.find(&oid, &mut buf)?,
                ControlFlow::Break(entry) => {
                    break Ok(entry.map(|entry| GitRepoEntry {
                        inner: entry.into(),
                        // repo: self.repo,
                    }));
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct GitRepoEntry {
    pub inner: TreeEntry,
}

pub struct Platform;
// always returns and absolute path
pub fn helix_real_path(path: impl AsRef<Path>) -> Result<PathBuf, GitError> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Err(GitError::Gen);
    }

    let mut real_path = PathBuf::new();
    if path.is_relative() {
        real_path.push(std::env::current_dir().map_err(|_| GitError::Gen)?);
    }

    let mut num_symlinks = 0;
    let mut path_backing: PathBuf;
    let mut components = path.components();

    let mut symlink_checks = 0;
    while let Some(component) = components.next() {
        match component {
            part @ (RootDir | Prefix(_)) => real_path.push(part),
            CurDir => {}
            ParentDir => {
                if !real_path.pop() {
                    return Err(GitError::Gen);
                }
            }
            Normal(part) => {
                real_path.push(part);
                symlink_checks += 1;
                if real_path.is_symlink() {
                    num_symlinks += 1;
                    if num_symlinks > 32 {
                        return Err(GitError::Gen);
                    }
                    let mut link_destination =
                        std::fs::read_link(real_path.as_path()).map_err(|_| GitError::Gen)?;
                    if link_destination.is_relative() {
                        real_path.pop();
                    }
                    link_destination.extend(components);
                    path_backing = link_destination;
                    components = path_backing.components();
                }
                if symlink_checks > 2048 {
                    return Err(GitError::Gen);
                }
            }
        }
    }
    Ok(real_path)
}

#[derive(Clone, PartialEq)]
pub enum RewriteSource {
    RewriteFromIndex { source_rela_path: BString },
    CopyFromDirectoryEntry { source_dirwalk_entry: GixDirEntry },
}

#[derive(Clone)]
struct Head<'repo> {
    kind: GixGitHeadKind,
    repo: &'repo Repository,
}

#[derive(Clone, Copy)]
struct Id<'r> {
    inner: ObjectId,
    repo: &'r Repository,
}

#[derive(Clone)]
struct Object<'repo> {
    id: ObjectId,
    kind: ObjectKind,
    data: Vec<u8>,
    repo: &'repo Repository,
}

impl Drop for Object<'_> {
    fn drop(&mut self) {
        self.repo.reuse_buffer(&mut self.data);
    }
}

#[derive(Clone)]
struct ObjectDetached {
    data: Vec<u8>,
}

#[derive(Clone)]
struct Blob<'repo> {
    data: Vec<u8>,
    repo: &'repo Repository,
}

impl Drop for Blob<'_> {
    fn drop(&mut self) {
        self.repo.reuse_buffer(&mut self.data);
    }
}

impl Drop for Tree<'_> {
    fn drop(&mut self) {
        self.repo.reuse_buffer(&mut self.data);
    }
}

#[derive(Clone)]
pub struct Commit<'repo> {
    pub id: ObjectId,
    data: Vec<u8>,
    repo: &'repo Repository,
}

impl Drop for Commit<'_> {
    fn drop(&mut self) {
        self.repo.reuse_buffer(&mut self.data);
    }
}

#[derive(Clone)]
pub struct Reference<'r> {
    inner: RawReference,
    repo: &'r Repository,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
enum GitRepositoryPathKind {
    Submodule,
    LinkedWorktree,
    Common,
}

impl<'repo> Id<'repo> {
    fn hx_object(&self) -> Result<Object<'repo>, GitError> {
        self.repo.hx_find_object(self.inner)
    }
}

impl Deref for Id<'_> {
    type Target = oid;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'repo> Id<'repo> {
    fn from_id(id: impl Into<ObjectId>, repo: &'repo Repository) -> Self {
        Id {
            inner: id.into(),
            repo,
        }
    }
}

impl<'repo> From<Id<'repo>> for ObjectId {
    fn from(v: Id<'repo>) -> Self {
        v.inner
    }
}

impl RepositoryPath {
    fn hx_from_dot_git_dir(
        path: PathBuf,
        repository_kind: GitRepositoryKind,
        current_dir: &std::path::Path,
    ) -> Option<Self> {
        let cwd = current_dir;
        let normalize_on_trailing_dot_dot = |dir: PathBuf| -> Option<PathBuf> {
            if matches!(
                dir.components().next_back(),
                Some(std::path::Component::ParentDir)
            ) {
                hx_path_normalize(dir.into(), cwd)?.into_owned()
            } else {
                dir
            }
            .into()
        };

        match repository_kind {
            GitRepositoryKind::Submodule { git_dir } => RepositoryPath::LinkedWorkTree {
                git_dir: hx_path_normalize(git_dir.into(), cwd)?.into_owned(),
                work_dir: without_dot_git_dir(normalize_on_trailing_dot_dot(path)?),
            },
            GitRepositoryKind::SubmoduleGitDir | GitRepositoryKind::PossiblyBare => {
                RepositoryPath::Repository(path)
            }
            GitRepositoryKind::WorkTreeGitDir { work_dir } => RepositoryPath::LinkedWorkTree {
                git_dir: path,
                work_dir,
            },
            GitRepositoryKind::WorkTree { linked_git_dir } => {
                if let Some(git_dir) = linked_git_dir {
                    RepositoryPath::LinkedWorkTree {
                        git_dir,
                        work_dir: without_dot_git_dir(normalize_on_trailing_dot_dot(path)?),
                    }
                } else {
                    let mut dir = normalize_on_trailing_dot_dot(path)?;
                    dir.pop(); // ".git" suffix
                    let work_dir = if dir.as_os_str().is_empty() {
                        PathBuf::from(".")
                    } else {
                        dir
                    };
                    RepositoryPath::WorkTree(work_dir)
                }
            }
        }
        .into()
    }
    fn hx_into_repository_and_work_tree_directories(self) -> (PathBuf, Option<PathBuf>) {
        match self {
            RepositoryPath::LinkedWorkTree { work_dir, git_dir } => (git_dir, Some(work_dir)),
            RepositoryPath::WorkTree(working_tree) => {
                (working_tree.join(DOT_GIT_DIR), Some(working_tree))
            }
            RepositoryPath::Repository(repository) => (repository, None),
        }
    }
    // helix
    #[must_use]
    pub fn helix_into_repository_dir(self) -> PathBuf {
        match self {
            RepositoryPath::LinkedWorkTree { git_dir, .. } => git_dir,
            RepositoryPath::WorkTree(working_tree) => working_tree.join(DOT_GIT_DIR),
            RepositoryPath::Repository(repository) => repository,
        }
    }
}

impl AsRef<std::path::Path> for RepositoryPath {
    fn as_ref(&self) -> &std::path::Path {
        match self {
            RepositoryPath::WorkTree(path)
            | RepositoryPath::Repository(path)
            | RepositoryPath::LinkedWorkTree {
                work_dir: _,
                git_dir: path,
            } => path,
        }
    }
}

fn is_bare(git_dir_candidate: &Path) -> bool {
    !(git_dir_candidate.join("index").exists()
        || (git_dir_candidate.file_name() == Some(OsStr::new(".git"))))
}

fn hx_is_git(git_dir: &Path) -> Result<GitRepositoryKind, GitError> {
    let git_dir_metadata = git_dir.metadata().map_err(|_| GitError::Gen)?;
    let cwd = std::env::current_dir().map_err(|_| GitError::Gen)?;
    hx_is_git_with_metadata(git_dir, &git_dir_metadata, &cwd)
}

fn hx_is_git_with_metadata(
    git_dir: &Path,
    git_dir_metadata: &std::fs::Metadata,
    cwd: &Path,
) -> Result<GitRepositoryKind, GitError> {
    #[derive(Eq, PartialEq)]
    enum Kind {
        MaybeRepo,
        Submodule,
        LinkedWorkTreeDir,
        WorkTreeGitDir { work_dir: std::path::PathBuf },
    }

    let dot_git = if git_dir_metadata.is_file() {
        let private_git_dir = hx_from_gitdir_file(git_dir)?;
        Cow::Owned(private_git_dir)
    } else {
        Cow::Borrowed(git_dir)
    };

    let (common_dir, kind) = if git_dir_metadata.is_file() {
        let common_dir = dot_git.join("commondir");
        match fx_from_plain_file(&common_dir) {
            Some(Err(_)) => {
                return Err(GitError::Gen);
            }
            Some(Ok(common_dir)) => {
                let common_dir = dot_git.join(common_dir);
                (Cow::Owned(common_dir), Kind::LinkedWorkTreeDir)
            }
            None => (dot_git.clone(), Kind::Submodule),
        }
    } else {
        let common_dir = dot_git.join("commondir");
        let worktree_and_common_dir = fx_from_plain_file(&common_dir)
            .and_then(Result::ok)
            .and_then(|cd| {
                hx_from_plain_file_relative_to_file(&dot_git.join("gitdir"))
                    .and_then(Result::ok)
                    .map(|worktree_gitfile| (without_dot_git_dir(worktree_gitfile), cd))
            });
        match worktree_and_common_dir {
            Some((work_dir, common_dir)) => {
                let common_dir = dot_git.join(common_dir);
                (Cow::Owned(common_dir), Kind::WorkTreeGitDir { work_dir })
            }
            None => (dot_git.clone(), Kind::MaybeRepo),
        }
    };

    {
        let objects_path = common_dir.join("objects");
        if !objects_path.is_dir() {
            return Err(GitError::Gen);
        }
    }
    {
        let refs_path = common_dir.join("refs");
        if !refs_path.is_dir() {
            return Err(GitError::Gen);
        }
    }
    Ok(match kind {
        Kind::LinkedWorkTreeDir => GitRepositoryKind::WorkTree {
            linked_git_dir: Some(dot_git.into_owned()),
        },
        Kind::WorkTreeGitDir { work_dir } => GitRepositoryKind::WorkTreeGitDir { work_dir },
        Kind::Submodule => GitRepositoryKind::Submodule {
            git_dir: dot_git.into_owned(),
        },
        Kind::MaybeRepo => {
            let conformed_git_dir = if git_dir == Path::new(".") {
                hx_realpath_opts(git_dir, cwd)
                    .map(Cow::Owned)
                    .unwrap_or(Cow::Borrowed(git_dir))
            } else {
                hx_normalize(git_dir.into(), cwd).unwrap_or(Cow::Borrowed(git_dir))
            };
            if is_bare(conformed_git_dir.as_ref())
                || conformed_git_dir.extension() == Some(OsStr::new("git"))
            {
                GitRepositoryKind::PossiblyBare
            } else if repository_kind(conformed_git_dir.as_ref())
                .is_some_and(|kind| matches!(kind, GitRepositoryPathKind::Submodule))
            {
                GitRepositoryKind::SubmoduleGitDir
            } else if conformed_git_dir.file_name() == Some(OsStr::new(".git")) {
                GitRepositoryKind::WorkTree {
                    linked_git_dir: None,
                }
            } else {
                GitRepositoryKind::PossiblyBare
            }
        }
    })
}

fn hx_read_regular_file_content_with_size_limit(
    path: &std::path::Path,
) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let max_file_size = 1024 * 64; // NOTE: git allows 1MB here
    let file_size = file.metadata()?.len();
    if file_size > max_file_size {
        return Err(std::io::Error::other("error"));
    }
    let mut buf = Vec::with_capacity(512);
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn hx_read_plain_file_content(path: &std::path::Path) -> Option<std::io::Result<Vec<u8>>> {
    let mut buf = match hx_read_regular_file_content_with_size_limit(path) {
        Ok(buf) => buf,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(err) => return Some(Err(err)),
    };
    let trimmed_len = buf.trim_end().len();
    buf.truncate(trimmed_len);
    if buf.is_empty() {
        return Some(Err(std::io::Error::other("error")));
    }
    Some(Ok(buf))
}

fn repository_kind(git_dir: &Path) -> Option<GitRepositoryPathKind> {
    if git_dir.file_name() == Some(OsStr::new(DOT_GIT_DIR)) {
        return Some(GitRepositoryPathKind::Common);
    }

    let mut last_comp = None;
    git_dir.components().rev().skip(1).any(|c| {
        if c.as_os_str() == OsStr::new(DOT_GIT_DIR) {
            true
        } else {
            last_comp = Some(c.as_os_str());
            false
        }
    });
    let last_comp = last_comp?;
    if last_comp == OsStr::new(MODULES) {
        GitRepositoryPathKind::Submodule.into()
    } else if last_comp == OsStr::new("worktrees") {
        GitRepositoryPathKind::LinkedWorktree.into()
    } else {
        None
    }
}

fn fx_from_plain_file(path: &std::path::Path) -> Option<std::io::Result<PathBuf>> {
    hx_read_plain_file_content(path).map(|res| res.map(from_bstring))
}

fn hx_from_plain_file_relative_to_file(path: &std::path::Path) -> Option<std::io::Result<PathBuf>> {
    hx_read_plain_file_content(path).map(|res| {
        res.and_then(|buf| {
            let plain_path = from_bstring(buf);
            if !plain_path.is_relative() {
                return Ok(plain_path);
            }
            match path.parent() {
                Some(parent) => Ok(parent.join(plain_path)),
                _ => Err(std::io::Error::other("error")),
            }
        })
    })
}
fn hx_path_normalize<'a>(path: Cow<'a, Path>, current_dir: &Path) -> Option<Cow<'a, Path>> {
    use std::path::Component::ParentDir;

    if !path.components().any(|c| matches!(c, ParentDir)) {
        return Some(path);
    }
    let mut current_dir_opt = Some(current_dir);
    let was_relative = path.is_relative();
    let components = path.components();
    let mut path = PathBuf::new();
    for component in components {
        if let ParentDir = component {
            let path_was_dot = path == Path::new(".");
            if path.as_os_str().is_empty() || path_was_dot {
                path.push(current_dir_opt.take()?);
            }
            if !path.pop() {
                return None;
            }
        } else {
            path.push(component);
        }
    }

    if (path.as_os_str().is_empty() || path == current_dir) && was_relative {
        Cow::Borrowed(Path::new("."))
    } else {
        path.into()
    }
    .into()
}

fn without_dot_git_dir(mut path: PathBuf) -> PathBuf {
    if path.file_name().and_then(std::ffi::OsStr::to_str) == Some(DOT_GIT_DIR) {
        path.pop();
    }
    path
}

fn hx_from_gitdir_file(path: &std::path::Path) -> Result<PathBuf, GitError> {
    let buf = hx_read_regular_file_content_with_size_limit(path).map_err(|_| GitError::Gen)?;
    let mut gitdir = gitdir(&buf)?;
    if let Some(parent) = path.parent() {
        gitdir = parent.join(gitdir);
    }
    Ok(gitdir)
}
fn gitdir(input: &[u8]) -> Result<PathBuf, GitError> {
    let path = input
        .strip_prefix(b"gitdir: ")
        .ok_or(GitError::Gen)?
        .as_bstr();
    let path = path.trim_end().as_bstr();
    if path.is_empty() {
        return Err(GitError::Gen);
    }
    Ok(try_from_bstr(path).map_err(|_| GitError::Gen)?.into_owned())
}

impl GixWorktreeStack {
    fn new(worktree_root: impl Into<PathBuf>, state: State, case: Case) -> Self {
        GixWorktreeStack {
            stack: GixWorktreeGixFsStack::new(worktree_root.into()),
            state,
            case,
        }
    }

    // helix
    fn hx_at_entry<'r>(
        &mut self,
        relative: impl Into<&'r BStr>,
        mode: Option<IndexMode>,
    ) -> std::io::Result<GixWorktreeStackPlatform<'_>> {
        self.stack.hx_make_relative_path_current(relative.into());
        Ok(GixWorktreeStackPlatform {
            parent: self,
            is_dir: mode.map(|m| m.hx_is_sparse() || m.hx_is_submodule()),
        })
    }
}

#[must_use]
struct GixWorktreeStackPlatform<'a> {
    parent: &'a GixWorktreeStack,
    is_dir: Option<bool>,
}
impl GixWorktreeStackPlatform<'_> {
    fn hx_platform_matching_attributes(&self, out: &mut search::Outcome) -> bool {
        let attrs = self.parent.state.hx_attributes_or_panic();
        let relative_path =
            hx_to_unix_separators_on_windows(into_bstr(self.parent.stack.hx_current_relative()));
        attrs.hx_matching_attributes(relative_path.as_bstr(), self.parent.case, self.is_dir, out)
    }
}

#[derive(Clone)]
struct GixWorktreeGixFsStack {
    current: PathBuf,
    current_relative: PathBuf,
    valid_components: usize,
    current_is_directory: bool,
}

// must remain public
trait ToNormalPathComponents {
    fn to_normal_path_components(&self) -> impl Iterator<Item = Result<&OsStr, GitError>>;
}

fn component_to_os_str(component: Component<'_>) -> Result<&OsStr, GitError> {
    match component {
        Component::Normal(os_str) => Ok(os_str),
        _ => Err(GitError::Gen),
    }
}

impl ToNormalPathComponents for &BStr {
    fn to_normal_path_components(&self) -> impl Iterator<Item = Result<&OsStr, GitError>> {
        self.split(|b| *b == b'/')
            .filter_map(bytes_component_to_os_str)
    }
}

impl ToNormalPathComponents for &str {
    fn to_normal_path_components(&self) -> impl Iterator<Item = Result<&OsStr, GitError>> {
        self.split('/')
            .filter_map(|c| bytes_component_to_os_str(c.as_bytes()))
    }
}

impl ToNormalPathComponents for &BString {
    fn to_normal_path_components(&self) -> impl Iterator<Item = Result<&OsStr, GitError>> {
        self.split(|b| *b == b'/')
            .filter_map(bytes_component_to_os_str)
    }
}

fn bytes_component_to_os_str(component: &[u8]) -> Option<Result<&OsStr, GitError>> {
    if component.is_empty() {
        return None;
    }
    let component = match hx_try_from_byte_slice(component.as_bstr()).map_err(|_| GitError::Gen) {
        Ok(c) => c,
        Err(_) => return Some(Err(GitError::Gen)),
    };
    let component = component.components().next()?;
    Some(component_to_os_str(component))
}

impl GixWorktreeGixFsStack {
    #[must_use]
    fn hx_current_relative(&self) -> &Path {
        &self.current_relative
    }
}

impl GixWorktreeGixFsStack {
    #[must_use]
    fn new(root: PathBuf) -> Self {
        GixWorktreeGixFsStack {
            current: root.clone(),
            current_relative: PathBuf::with_capacity(128),
            valid_components: 0,
            current_is_directory: true,
        }
    }

    fn hx_make_relative_path_current(&mut self, relative: impl ToNormalPathComponents) {
        let mut components = relative.to_normal_path_components().peekable();
        let mut existing_components = self.current_relative.components();
        let mut matching_components = 0;
        while let (Some(existing_comp), Some(new_comp)) =
            (existing_components.next(), components.peek())
        {
            match new_comp {
                Ok(new_comp) => {
                    if existing_comp.as_os_str() == *new_comp {
                        components.next();
                        matching_components += 1;
                    } else {
                        break;
                    }
                }
                Err(_err) => {}
            }
        }

        for _ in 0..self.valid_components - matching_components {
            self.current.pop();
            self.current_relative.pop();
            self.current_is_directory = true;
        }
        self.valid_components = matching_components;

        if !self.current_is_directory && components.peek().is_some() {
            self.current_is_directory = true;
        }

        while let Some(comp) = components.next() {
            let comp = comp.map_err(|_| GitError::Gen).unwrap();
            let is_last_component = components.peek().is_none();
            self.current_is_directory = !is_last_component;
            self.current.push(comp);
            self.current_relative.push(comp);
            self.valid_components += 1;
        }
    }
}
// }

type AttributeMatchGroup = Search;

#[derive(Default, Clone)]
struct Attributes {
    globals: AttributeMatchGroup,
    stack: AttributeMatchGroup,
    collection: MetadataCollection,
}

impl State {
    fn hx_attributes_or_panic(&self) -> &Attributes {
        match self {
            State::AttributesStack(attributes) => attributes,
        }
    }
}

impl Attributes {
    #[must_use]
    fn new(globals: AttributeMatchGroup, collection: search::MetadataCollection) -> Self {
        Attributes {
            globals,
            stack: Default::default(),

            collection,
        }
    }
    fn hx_matching_attributes(
        &self,
        relative_path: &BStr,
        case: Case,
        is_dir: Option<bool>,
        out: &mut search::Outcome,
    ) -> bool {
        out.initialize(&self.collection);

        let groups = [&self.globals, &self.stack];
        let mut has_match = false;
        groups.iter().rev().any(|group| {
            has_match |= group.pattern_matching_relative_path(relative_path, case, is_dir, out);
            out.is_done()
        });
        has_match
    }
}

use std::ffi::OsString;

struct StageOne {
    git_dir_config: GitConfigurationFile<'static>,
    buf: Vec<u8>,
}

impl StageOne {
    // helix
    fn new(dir: &std::path::Path) -> Self {
        let mut buf = Vec::with_capacity(512);
        StageOne {
            git_dir_config: hx_load_config(&dir.join("config"), &mut buf).unwrap(),
            buf,
        }
    }
}

impl Cache {
    #[allow(clippy::too_many_arguments)]
    fn hx_from_stage_one(
        StageOne {
            git_dir_config,
            mut buf,
        }: StageOne,
        git_dir: &std::path::Path,
        // git_install_dir: Option<&std::path::Path>,
        // home: Option<&std::path::Path>,
    ) -> Result<Self, GitError> {
        let git_install_dir = std::env::current_exe()
            .and_then(|exe| {
                exe.parent()
                    .map(ToOwned::to_owned)
                    .ok_or_else(|| std::io::Error::other("no parent for current executable"))
            })
            .ok();
        let home = std::env::var_os("HOME")
            .map(Into::into)
            .or_else(std::env::home_dir);

        let options = ConfigFileOptions {
            includes: {
                FileIncludesOptions::hx_follow(
                    InterpolationContext {
                        git_install_dir: git_install_dir.as_deref(),
                        home_dir: home.as_deref(),
                        home_for_user: Some(home_for_user),
                    },
                    IncludesConditionalContext {
                        git_dir: git_dir.into(),
                    },
                )
            },
        };

        let config = {
            let mut metas = [
                GixConfigSourceKind::GitInstallation,
                GixConfigSourceKind::System,
                GixConfigSourceKind::Global,
            ]
            .iter()
            .flat_map(|kind| kind.sources())
            .filter_map(|source| {
                source
                    .hx_storage_location(&mut Self::make_source_env())
                    .map(|p| (source, p.into_owned()))
            })
            .map(|(_, path)| Metadata { path: Some(path) });

            let mut globals = GitConfigurationFile::from_paths_metadata_buf(
                &mut metas,
                &mut buf,
                ConfigFileOptions {
                    includes: FileIncludesOptions::hx_no_follow(),
                },
            )
            .map_err(|_| GitError::Io)?
            .unwrap_or_default();

            let local_meta = git_dir_config.meta_owned();
            globals.append(git_dir_config);
            globals.resolve_includes({
                FileIncludesOptions::hx_follow(
                    InterpolationContext {
                        git_install_dir: git_install_dir.as_deref(),
                        home_dir: home.as_deref(),
                        home_for_user: Some(home_for_user),
                    },
                    IncludesConditionalContext {
                        git_dir: git_dir.into(),
                    },
                )
            })?;
            globals.append(GitConfigurationFile::from_env(options.includes)?.unwrap_or_default());
            globals.set_meta(local_meta);
            globals
        };

        Ok(Cache {
            resolved: config.into(),
        })
    }
    fn make_source_env() -> impl FnMut(&str) -> Option<OsString> {
        |name| match name {
            "XDG_CONFIG_HOME" => var("XDG_CONFIG_HOME"),
            "HOME" => hx_home_dir().map(Into::into),
            _ => None,
        }
    }
    fn hx_trusted_file_path(
        &self,
        key: impl AsKey,
    ) -> Option<Result<Cow<'_, std::path::Path>, GitError>> {
        let config: &GitConfigurationFile<'_> = &self.resolved;
        let path = config.hx_path_filter(key)?;
        if path.is_empty() {
            let _key = key.as_key();
            return None;
        }
        let install_dir = hx_install_dir().ok();
        let home = hx_home_dir();
        let ctx = hx_interpolate_context(install_dir.as_deref(), home.as_deref());
        let is_optional = path.is_optional;
        let res = path.hx_interpolate(ctx);
        if is_optional
            && let Ok(path) = &res
            && path.metadata().is_err()
        {
            return None;
        }
        Some(res)
    }

    fn hx_assemble_attribute_globals(&self) -> Attributes {
        let configured_or_user_attributes = match self
            .hx_trusted_file_path(Core::ATTRIBUTES_FILE)
            .transpose()
            .expect("ok")
        {
            Some(attributes) => Some(attributes),
            None => xdg_config_path("attributes").map(Cow::Owned),
        };

        let attribute_files = AttributesSource::System
            .storage_location()
            .into_iter()
            .chain(configured_or_user_attributes);

        let mut _buf = Vec::new();
        let mut collection = search::MetadataCollection::default();
        Attributes::new(
            Search::new_globals(attribute_files, &mut _buf, &mut collection).expect("ok"),
            collection,
        )
    }
}
fn xdg_config_path(resource_file_name: &str) -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .or_else(std::env::home_dir)
                .map(|home| home.join(".config"))
        })?;

    Some(base.join("git").join(resource_file_name))
}

fn hx_interpolate_context<'a>(
    git_install_dir: Option<&'a Path>,
    home_dir: Option<&'a Path>,
) -> InterpolationContext<'a> {
    InterpolationContext {
        git_install_dir,
        home_dir,
        home_for_user: Some(home_for_user),
    }
}

trait ApplyLeniency {
    fn with_leniency(self) -> Self;
}

impl<T, E> ApplyLeniency for Result<Option<T>, E> {
    fn with_leniency(self) -> Self {
        match self {
            Ok(v) => Ok(v),
            Err(_) => Ok(None),
        }
    }
}

fn hx_load_config(
    config_path: &std::path::PathBuf,
    buf: &mut Vec<u8>,
) -> Result<GitConfigurationFile<'static>, GitError> {
    let mut file = match std::fs::File::open(config_path) {
        Ok(f) => f,
        _ => {
            return Ok(GitConfigurationFile::new(Metadata {
                path: Some(config_path.into()),
            }));
        }
    };

    buf.clear();
    if std::io::copy(&mut file, buf).is_err() {
        buf.clear();
    }

    GitConfigurationFile::from_bytes_owned(
        buf,
        Metadata {
            path: Some(config_path.into()),
        },
        ConfigFileOptions {
            includes: FileIncludesOptions::hx_no_follow(),
        },
    )
}

impl Core {
    const ATTRIBUTES_FILE: PathKey = PathKey::new_path("attributesFile", &ConfigTree::CORE);

    const AUTO_CRLF: AutoCrlf = AutoCrlf::new("autocrlf", &ConfigTree::CORE);
}

impl Section for Core {
    fn name(&self) -> &'static str {
        "core"
    }
}

type AutoCrlf = Any;

impl AutoCrlf {
    fn try_into_autocrlf(&'static self, value: Cow<'_, BStr>) -> Result<EolAutoCrlf, GitError> {
        if value.as_ref() == "input" {
            return Ok(EolAutoCrlf::Input);
        }
        let value = Boolean::try_from(value.as_ref()).map_err(|_| GitError::Gen)?;
        Ok(if value.into() {
            EolAutoCrlf::Enabled
        } else {
            EolAutoCrlf::Disabled
        })
    }
}

#[derive(Copy, Clone, Default)]
struct Core;

use arc_swap::ArcSwap;
use encoding_rs::EncoderResult;
use filetime::FileTime;
use memmap2::{Mmap as MMap, Mmap};
use sha1_checked::{Builder, CollisionResult, Digest};
use smallvec::SmallVec;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::io::{Error as IoError, Write};
use std::iter::FusedIterator;
use std::ops::Range;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU16, AtomicU32, AtomicUsize};
use std::time::{SystemTime, SystemTimeError};

#[derive(Copy, Clone)]
struct Any {
    name: &'static str,
    section: &'static dyn Section,
}

impl Any {
    const fn new(name: &'static str, section: &'static dyn Section) -> Self {
        Any { name, section }
    }
}

impl Debug for Any {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.logical_name(), f)
    }
}

impl std::fmt::Display for Any {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.logical_name())
    }
}

impl Key for Any {
    fn name(&self) -> &str {
        self.name
    }

    fn section(&self) -> &dyn Section {
        self.section
    }
}

impl AsKey for Any {
    fn as_key(&self) -> KeyRef<'_> {
        self.hx_try_as_key().expect("infallible")
    }

    fn hx_try_as_key(&self) -> Option<KeyRef<'_>> {
        let section_name = self
            .section
            .parent()
            .map_or_else(|| self.section.name(), Section::name);
        let subsection_name = if self.section.parent().is_some() {
            Some(self.section.name().into())
        } else {
            None
        };
        let value_name = self.name;
        KeyRef {
            section_name,
            subsection_name,
            value_name,
        }
        .into()
    }
}

type PathKey = Any;

impl PathKey {
    const fn new_path(name: &'static str, section: &'static dyn Section) -> Self {
        Self::new(name, section)
    }
}

trait Section {
    fn name(&self) -> &str;
    fn parent(&self) -> Option<&dyn Section> {
        None
    }
}

trait Key: std::fmt::Debug {
    fn name(&self) -> &str;

    fn section(&self) -> &dyn Section;

    fn logical_name(&self) -> String {
        let section = self.section();
        let mut buf = String::new();
        buf.push_str(section.name());
        buf.push('.');
        buf.push_str(self.name());
        buf
    }
}

impl AsKey for &dyn Key {
    fn as_key(&self) -> KeyRef<'_> {
        self.hx_try_as_key().expect("infallible")
    }

    fn hx_try_as_key(&self) -> Option<KeyRef<'_>> {
        let section_name = self
            .section()
            .parent()
            .map_or_else(|| self.section().name(), Section::name);
        let subsection_name = if self.section().parent().is_some() {
            Some(self.section().name().into())
        } else {
            None
        };
        let value_name = self.name();
        KeyRef {
            section_name,
            subsection_name,
            value_name,
        }
        .into()
    }
}

#[derive(Copy, Clone, Default)]
struct ConfigTree;

impl ConfigTree {
    const CORE: Core = Core;
}

#[derive(Clone)]
struct Cache {
    resolved: std::sync::Arc<GitConfigurationFile<'static>>,
}

#[derive(Clone)]
enum GixGitHeadKind {
    Symbolic(RawReference),
    Unborn(FullName),
    Detached {
        target: ObjectId,
        peeled: Option<ObjectId>,
    },
}

impl GixGitHeadKind {
    fn attach(self, repo: &Repository) -> Head<'_> {
        Head { kind: self, repo }
    }
}

impl<'repo> Head<'repo> {
    fn try_into_referent(self) -> Option<Reference<'repo>> {
        match self.kind {
            GixGitHeadKind::Symbolic(r) => r.attach_reference(self.repo).into(),
            _ => None,
        }
    }
}

impl<'repo> Head<'repo> {
    fn hx_try_peel_to_id(&mut self) -> Result<Option<Id<'repo>>, GitError> {
        Ok(Some(match &mut self.kind {
            GixGitHeadKind::Unborn(_name) => return Ok(None),
            GixGitHeadKind::Detached {
                peeled: Some(peeled),
                ..
            } => (*peeled).attach_object_id(self.repo),
            GixGitHeadKind::Detached {
                peeled: None,
                target,
            } => {
                let id = target.attach_object_id(self.repo);
                if id.hx_object().map_err(|_| GitError::Gen)?.kind == ObjectKind::Commit {
                    id
                } else {
                    return Ok(None); // NOTE(pk)
                }
            }
            GixGitHeadKind::Symbolic(r) => {
                let mut nr = r.clone().attach_reference(self.repo);
                let peeled = nr.peel_to_id();
                *r = nr.detach();
                peeled.map_err(|_| GitError::Gen)?
            }
        }))
    }

    fn hx_peel_to_object(&mut self) -> Result<Object<'repo>, GitError> {
        let id = self.hx_try_peel_to_id()?.ok_or(GitError::Unborn)?;
        id.hx_object().map_err(|_| GitError::Gen)
    }

    fn hx_peel_to_commit(&mut self) -> Result<Commit<'repo>, GitError> {
        self.hx_peel_to_object()?.try_into_commit()
    }
}

impl<'repo> Object<'repo> {
    fn try_into_commit(self) -> Result<Commit<'repo>, GitError> {
        self.try_into().map_err(|_| GitError::Gen)
    }

    fn try_into_tree(self) -> Result<Tree<'repo>, GitError> {
        self.try_into().map_err(|_| GitError::Gen)
    }
}

impl Object<'_> {
    #[must_use]
    fn detach(self) -> ObjectDetached {
        self.into()
    }
}

impl<'repo> Commit<'repo> {
    // helix
    pub fn helix_commit_tree(&self) -> Result<Tree<'repo>, GitError> {
        match self.hx_tree_id()?.hx_object()?.try_into_tree() {
            Ok(tree) => Ok(tree),
            Err(_) => Err(GitError::Gen),
        }
    }
    // helix transitive
    fn hx_tree_id(&self) -> Result<Id<'repo>, GitError> {
        CommitRefIter::hx_from_bytes(&self.data)
            .tree_id()
            .map(|id| Id::from_id(id, self.repo))
    }
}

impl<'repo> From<Object<'repo>> for ObjectDetached {
    fn from(mut v: Object<'repo>) -> Self {
        ObjectDetached {
            data: std::mem::take(&mut v.data),
        }
    }
}

impl<'repo> From<Commit<'repo>> for ObjectDetached {
    fn from(mut v: Commit<'repo>) -> Self {
        ObjectDetached {
            data: std::mem::take(&mut v.data),
        }
    }
}

impl<'repo> From<Blob<'repo>> for ObjectDetached {
    fn from(mut v: Blob<'repo>) -> Self {
        ObjectDetached {
            data: std::mem::take(&mut v.data),
        }
    }
}

impl<'repo> From<Tree<'repo>> for ObjectDetached {
    fn from(mut v: Tree<'repo>) -> Self {
        ObjectDetached {
            data: std::mem::take(&mut v.data),
        }
    }
}

impl<'repo> TryFrom<Object<'repo>> for Commit<'repo> {
    type Error = GitError;
    fn try_from(mut value: Object<'repo>) -> Result<Self, GitError> {
        let repo = value.repo;
        match value.kind {
            ObjectKind::Commit => Ok(Commit {
                id: value.id,
                repo,
                data: std::mem::take(&mut value.data),
            }),
            _ => Err(GitError::Gen),
        }
    }
}

impl<'repo> TryFrom<Object<'repo>> for Tree<'repo> {
    // type Error = Object<'repo>;
    type Error = GitError;

    fn try_from(mut value: Object<'repo>) -> Result<Self, GitError> {
        let repo = value.repo;
        match value.kind {
            ObjectKind::Tree => Ok(Tree {
                repo,
                data: std::mem::take(&mut value.data),
            }),
            _ => Err(GitError::Gen),
        }
    }
}

impl<'repo> TryFrom<Object<'repo>> for Blob<'repo> {
    // type Error = Object<'repo>;
    type Error = GitError;

    fn try_from(mut value: Object<'repo>) -> Result<Self, GitError> {
        let repo = value.repo;
        match value.kind {
            ObjectKind::Blob => Ok(Blob {
                repo,
                data: std::mem::take(&mut value.data),
            }),
            _ => Err(GitError::Gen),
        }
    }
}

impl Reference<'_> {
    // helix
    #[must_use]
    pub fn name(&self) -> &FullNameRef {
        self.inner.name.as_ref()
    }

    fn detach(self) -> RawReference {
        self.inner
    }
}

impl<'repo> Reference<'repo> {
    fn from_ref(reference: RawReference, repo: &'repo Repository) -> Self {
        Reference {
            inner: reference,
            repo,
        }
    }
}

impl<'repo> Reference<'repo> {
    fn peel_to_id(&mut self) -> Result<Id<'repo>, GitError> {
        let oid = self.inner.peel_to_id(&self.repo.refs, &self.repo.objects)?;
        Ok(Id::from_id(oid, self.repo))
    }
}

#[derive(Clone)]
struct Buffer<'repo> {
    inner: Vec<u8>,
    _repo: &'repo Repository,
}

impl From<Buffer<'_>> for Vec<u8> {
    fn from(mut value: Buffer<'_>) -> Self {
        std::mem::take(&mut value.inner)
    }
}

impl Deref for Buffer<'_> {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Buffer<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl From<ThreadSafeRepository> for Repository {
    // helix
    fn from(thread_safe_repo: ThreadSafeRepository) -> Self {
        let mut objects = OdbCache::from(thread_safe_repo.objects.to_handle());

        let new_pack_cache: Arc<NewPackCacheFn> =
            Arc::new(|| Box::<StaticLinkedList<64>>::default());
        objects.pack_cache = Some(RefCell::new(new_pack_cache()));
        objects.new_pack_cache = Some(new_pack_cache);

        Repository {
            bufs: Some(RefCell::new(Vec::with_capacity(4))),
            work_tree: thread_safe_repo.work_tree,
            objects,
            refs: thread_safe_repo.refs,
            repository_config: thread_safe_repo.thread_safe_repo_config,
        }
    }
}

impl Find for Repository {
    fn try_find<'a>(
        &self,
        id: &oid,
        buffer: &'a mut Vec<u8>,
    ) -> Result<Option<ObjectData<'a>>, GitError> {
        if id == ObjectId::empty_tree() {
            buffer.clear();
            return Ok(Some(ObjectData {
                kind: ObjectKind::Tree,
                data: &[],
            }));
        }
        // self.objects.try_find(id, buffer)
        TraitPackFind::try_find(&self.objects, id, buffer)
    }
}

fn hx_install_dir() -> std::io::Result<PathBuf> {
    std::env::current_exe().and_then(|exe| {
        exe.parent()
            .map(ToOwned::to_owned)
            .ok_or_else(|| std::io::Error::other("no parent for current executable"))
    })
}

impl ObjectId {
    fn attach_object_id(self, repo: &Repository) -> Id<'_> {
        Id::from_id(self, repo)
    }
}

// was vendor.rs:978, `impl RawReference` -- inherent impls may live in any module
// of the defining crate, so this belongs next to `Reference` rather than in `vendor`.
impl RawReference {
    fn attach_reference(self, repo: &Repository) -> Reference<'_> {
        Reference::from_ref(self, repo)
    }
}

#[derive(Clone)]
pub struct Pipeline {
    //<'repo> {
    inner: FilterPipeline,
    cache: GixWorktreeStack,
    // repo: &'repo Repository,
}

impl<'repo> Pipeline {
    //<'repo> {
    fn new(repo: &'repo Repository, cache: GixWorktreeStack) -> Result<Self, GitError> {
        Ok(Pipeline {
            inner: FilterPipeline::new(FilterPipelineOptions {
                drivers: repo
                    .repository_config
                    .resolved
                    .sections_by_name("filter")
                    .into_iter()
                    .flatten()
                    .filter(|_s| true)
                    .filter_map(|s| {
                        s.header().subsection_name().map(|name| {
                            Ok(Driver {
                                name: name.to_owned(),
                            })
                        })
                    })
                    .collect::<Result<Vec<_>, GitError>>()?,
                eol_config: EolConfiguration {
                    auto_crlf: repo
                        .repository_config
                        .resolved
                        .string("core.autocrlf")
                        .map(|value| Core::AUTO_CRLF.try_into_autocrlf(value))
                        .transpose()
                        .with_leniency()
                        .map_err(|_| GitError::Gen)?
                        .unwrap_or_default(),
                },
            }),
            cache,
            // repo,
        })
    }
}

#[derive(Debug, Default, Clone)]
struct GixCommandContext {}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Ord, PartialOrd, Hash)]
#[repr(u16)]
pub enum EntryKind {
    Tree = 0o040000u16,
    Blob = 0o100644,
    BlobExecutable = 0o100755,
    Link = 0o120000,
    Commit = 0o160000,
}
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
enum ObjectKind {
    Tree,
    Blob,
    Commit,
}
#[derive(PartialEq, Eq, Ord, PartialOrd, Clone, Copy)]
#[non_exhaustive]
pub enum ObjectId {
    Sha1([u8; 20]),
}

fn mmap_read_only(path: &Path) -> std::io::Result<memmap2::Mmap> {
    let file = std::fs::File::open(path)?;
    #[allow(unsafe_code)]
    unsafe {
        memmap2::MmapOptions::new().map_copy_read_only(&file)
    }
}

type ParseResult<T> = Result<T, ()>;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Event<'a> {
    Comment(Comment<'a>),
    SectionHeader(SectionHeader<'a>),
    SectionValueName(SectionValueName<'a>),
    Value(Cow<'a, BStr>),
    Newline(Cow<'a, BStr>),
    ValueNotDone(Cow<'a, BStr>),
    ValueDone(Cow<'a, BStr>),
    Whitespace(Cow<'a, BStr>),
    KeyValueSeparator,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct ConfigSection<'a> {
    header: SectionHeader<'a>,
    events: Vec<Event<'a>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
struct Comment<'a> {
    tag: u8,
    text: Cow<'a, BStr>,
}

fn parse_from_bytes<'i>(
    mut input: &'i [u8],
    dispatch: &mut dyn FnMut(Event<'i>),
) -> Result<(), GitError> {
    let bom = unicode_bom::Bom::from(input);
    input = &input[bom.len()..];

    loop {
        let before = input;
        if let Ok(comment) = comment(&mut input) {
            dispatch(Event::Comment(comment));
        } else if let Ok(whitespace) = take_spaces1(&mut input) {
            dispatch(Event::Whitespace(Cow::Borrowed(whitespace)));
        } else if let Ok(newline) = take_newlines1(&mut input) {
            dispatch(Event::Newline(Cow::Borrowed(newline)));
        } else if !input.starts_with(b"[") {
            let mut node = ParseNode::SectionHeader;
            key_value_pair(&mut input, &mut node, dispatch).map_err(|()| GitError::Gen)?;
        }
        if input.len() == before.len() {
            break;
        }
    }

    if input.is_empty() {
        return Ok(());
    }

    let mut node = ParseNode::SectionHeader;
    while !input.is_empty() {
        section(&mut input, &mut node, dispatch).map_err(|_| GitError::Gen)?;
    }
    Ok(())
}

fn comment<'i>(i: &mut &'i [u8]) -> ParseResult<Comment<'i>> {
    let Some((&tag, rest)) = i.split_first() else {
        return Err(());
    };
    if tag != b';' && tag != b'#' {
        return Err(());
    }
    let end = rest.find_byte(b'\n').unwrap_or(rest.len());
    let text = rest[..end].as_bstr();
    *i = &rest[end..];
    Ok(Comment {
        tag,
        text: Cow::Borrowed(text),
    })
}

fn section<'i>(
    i: &mut &'i [u8],
    node: &mut ParseNode,
    dispatch: &mut dyn FnMut(Event<'i>),
) -> ParseResult<()> {
    let header = section_header(i)?;
    dispatch(Event::SectionHeader(header));

    loop {
        let before = *i;

        if let Ok(v) = take_spaces1(i) {
            dispatch(Event::Whitespace(Cow::Borrowed(v.as_bstr())));
        }
        if let Ok(v) = take_newlines1(i) {
            dispatch(Event::Newline(Cow::Borrowed(v.as_bstr())));
        }

        key_value_pair(i, node, dispatch)?;

        if let Ok(comment) = comment(i) {
            dispatch(Event::Comment(comment));
        }

        if i.len() == before.len() {
            break;
        }
    }

    Ok(())
}

fn section_header<'i>(i: &mut &'i [u8]) -> ParseResult<SectionHeader<'i>> {
    let mut c = *i;
    c = c.strip_prefix(b"[").ok_or(())?;
    let name = {
        let rest = c;
        let name_len = rest.iter().take_while(|b| is_section_char(**b)).count();
        c = &rest[name_len..];
        rest[..name_len].as_bstr()
    };

    if let Some(rest) = c.strip_prefix(b"]") {
        if name.is_empty() {
            return Err(());
        }
        *i = rest;
        return match name.find_byte(b'.') {
            Some(index) => Ok(SectionHeader {
                name: SectionName(Cow::Borrowed(name[..index].as_bstr())),
                separator: name.get(index..=index).map(|s| Cow::Borrowed(s.as_bstr())),
                subsection_name: name.get(index + 1..).map(|s| Cow::Borrowed(s.as_bstr())),
            }),
            None => Ok(SectionHeader {
                name: SectionName(Cow::Borrowed(name.as_bstr())),
                separator: None,
                subsection_name: None,
            }),
        };
    }

    let whitespace = take_spaces1(&mut c)?;
    let Some(rest) = c.strip_prefix(b"\"") else {
        return Err(());
    };
    c = rest;
    let subsection_name = quoted_sub_section(&mut c)?;
    let Some(rest) = c.strip_prefix(b"\"]") else {
        return Err(());
    };
    *i = rest;
    Ok(SectionHeader {
        name: SectionName(Cow::Borrowed(name)),
        separator: Some(Cow::Borrowed(whitespace)),
        subsection_name: Some(subsection_name),
    })
}

fn is_section_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-' || c == b'.'
}

fn quoted_sub_section<'i>(i: &mut &'i [u8]) -> ParseResult<Cow<'i, BStr>> {
    let mut c = *i;
    let input = c;
    let mut out: Option<Vec<u8>> = None;
    let mut borrowed_len = 0usize;
    while let Some(&b) = c.first() {
        match b {
            b'"' => break,
            b'\n' => return Err(()),
            b'\\' => {
                let escaped = *c.get(1).ok_or(())?;
                if escaped == b'\n' {
                    return Err(());
                }
                let out = out.get_or_insert_with(|| input[..borrowed_len].to_vec());
                out.push(escaped);
                c = &c[2..];
                borrowed_len = input.len() - c.len();
            }
            _ => {
                if let Some(out) = out.as_mut() {
                    out.push(b);
                }
                c = &c[1..];
                borrowed_len = input.len() - c.len();
            }
        }
    }
    *i = c;
    Ok(match out {
        Some(out) => Cow::Owned(out.into()),
        None => Cow::Borrowed(input[..borrowed_len].as_bstr()),
    })
}

fn config_name<'i>(i: &mut &'i [u8]) -> ParseResult<&'i BStr> {
    if !i.first().is_some_and(u8::is_ascii_alphabetic) {
        return Err(());
    }
    let len = i
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric() || **c == b'-')
        .count();
    let (name, rest) = i.split_at(len);
    *i = rest;
    Ok(name.as_bstr())
}

fn key_value_pair<'i>(
    i: &mut &'i [u8],
    node: &mut ParseNode,
    dispatch: &mut dyn FnMut(Event<'i>),
) -> ParseResult<()> {
    *node = ParseNode::Name;
    let Ok(name) = config_name(i) else {
        return Ok(());
    };

    dispatch(Event::SectionValueName(SectionValueName(Cow::Borrowed(
        name,
    ))));

    if let Ok(whitespace) = take_spaces1(i) {
        dispatch(Event::Whitespace(Cow::Borrowed(whitespace)));
    }

    *node = ParseNode::Value;
    config_value(i, dispatch)
}

fn config_value<'i>(i: &mut &'i [u8], dispatch: &mut dyn FnMut(Event<'i>)) -> ParseResult<()> {
    if let Some(rest) = i.strip_prefix(b"=") {
        *i = rest;
        dispatch(Event::KeyValueSeparator);
        if let Ok(whitespace) = take_spaces1(i) {
            dispatch(Event::Whitespace(Cow::Borrowed(whitespace)));
        }
        value(i, dispatch)
    } else {
        dispatch(Event::Value(Cow::Borrowed("".into())));
        Ok(())
    }
}

fn value<'i>(i: &mut &'i [u8], dispatch: &mut dyn FnMut(Event<'i>)) -> ParseResult<()> {
    let input = *i;
    let mut cursor = 0usize;
    let mut value_start = 0usize;
    let mut value_end = None;
    let mut is_in_quotes = false;
    let mut partial_value_found = false;

    while cursor < input.len() {
        match input[cursor] {
            b'\n' => {
                value_end = Some(cursor);
                break;
            }
            b';' | b'#' if !is_in_quotes => {
                value_end = Some(cursor);
                break;
            }
            b'\\' => {
                let escape_index = cursor;
                cursor += 1;
                let mut consumed = 1usize;
                let Some(mut b) = input.get(cursor).copied() else {
                    let value = input[value_start..escape_index].as_bstr();
                    dispatch(Event::ValueNotDone(Cow::Borrowed(value)));
                    dispatch(Event::ValueDone(Cow::Borrowed("".into())));
                    *i = &[];
                    return Ok(());
                };
                if b == b'\r' {
                    cursor += 1;
                    b = *input.get(cursor).ok_or(())?;
                    if b != b'\n' {
                        return Err(());
                    }
                    consumed += 1;
                }
                match b {
                    b'\n' => {
                        partial_value_found = true;
                        let value = input[value_start..escape_index].as_bstr();
                        dispatch(Event::ValueNotDone(Cow::Borrowed(value)));
                        let nl_start = escape_index + 1;
                        let nl = input[nl_start..nl_start + consumed].as_bstr();
                        dispatch(Event::Newline(Cow::Borrowed(nl)));
                        cursor += 1;
                        value_start = cursor;
                        value_end = None;
                    }
                    b'n' | b't' | b'\\' | b'b' | b'"' => cursor += 1,
                    _ => return Err(()),
                }
            }
            b'"' => {
                is_in_quotes = !is_in_quotes;
                cursor += 1;
            }
            _ => cursor += 1,
        }
    }
    if is_in_quotes {
        return Err(());
    }

    let end = value_end.unwrap_or(cursor);
    if end == value_start {
        dispatch(Event::Value(Cow::Borrowed("".into())));
        *i = &input[cursor..];
        return Ok(());
    }

    let value_end_no_trailing_whitespace = input[value_start..end]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(idx, b)| (!b.is_ascii_whitespace()).then_some(value_start + idx + 1))
        .unwrap_or(value_start);
    let value = input[value_start..value_end_no_trailing_whitespace].as_bstr();
    if partial_value_found {
        dispatch(Event::ValueDone(Cow::Borrowed(value)));
    } else {
        dispatch(Event::Value(Cow::Borrowed(value)));
    }
    *i = &input[value_end_no_trailing_whitespace..];
    Ok(())
}

fn take_spaces1<'i>(i: &mut &'i [u8]) -> ParseResult<&'i BStr> {
    let len = i.iter().take_while(|c| **c == b' ' || **c == b'\t').count();
    if len == 0 {
        return Err(());
    }
    let (spaces, rest) = i.split_at(len);
    *i = rest;
    Ok(spaces.as_bstr())
}

fn take_newlines1<'i>(i: &mut &'i [u8]) -> ParseResult<&'i BStr> {
    let mut c = *i;
    let input = c;
    let mut cursor = 0usize;
    while cursor < input.len() {
        if input[cursor..].starts_with(b"\r\n") {
            cursor += 2;
        } else if input[cursor] == b'\n' {
            cursor += 1;
        } else {
            break;
        }
    }
    if cursor == 0 {
        return Err(());
    }
    c = &input[cursor..];
    *i = c;
    Ok(input[..cursor].as_bstr())
}

impl Event<'_> {
    #[must_use]
    fn to_bstr_lossy(&self) -> &BStr {
        match self {
            Self::ValueNotDone(e)
            | Self::Whitespace(e)
            | Self::Newline(e)
            | Self::Value(e)
            | Self::ValueDone(e) => e.as_ref(),
            Self::KeyValueSeparator => "=".into(),
            Self::SectionValueName(k) => k.0.as_ref(),
            Self::SectionHeader(h) => h.name.0.as_ref(),
            Self::Comment(c) => c.text.as_ref(),
        }
    }

    #[must_use]
    fn to_owned(&self) -> Event<'static> {
        match self {
            Event::Comment(e) => Event::Comment(e.to_owned()),
            Event::SectionHeader(e) => Event::SectionHeader(e.to_owned()),
            Event::SectionValueName(e) => Event::SectionValueName(e.to_owned()),
            Event::Value(e) => Event::Value(Cow::Owned(e.clone().into_owned())),
            Event::ValueNotDone(e) => Event::ValueNotDone(Cow::Owned(e.clone().into_owned())),
            Event::ValueDone(e) => Event::ValueDone(Cow::Owned(e.clone().into_owned())),
            Event::Newline(e) => Event::Newline(Cow::Owned(e.clone().into_owned())),
            Event::Whitespace(e) => Event::Whitespace(Cow::Owned(e.clone().into_owned())),
            Event::KeyValueSeparator => Event::KeyValueSeparator,
        }
    }
}

impl Comment<'_> {
    #[must_use]
    fn to_owned(&self) -> Comment<'static> {
        Comment {
            tag: self.tag,
            text: Cow::Owned(self.text.as_ref().into()),
        }
    }
}

impl<'a> SectionHeader<'a> {
    fn new(
        name: impl Into<Cow<'a, str>>,
        subsection: impl Into<Option<Cow<'a, BStr>>>,
    ) -> Result<SectionHeader<'a>, GitError> {
        let name: SectionName = SectionName(validated_name(into_cow_bstr(name.into()))?);
        if let Some(subsection_name) = subsection.into() {
            Ok(SectionHeader {
                name,
                separator: Some(Cow::Borrowed(" ".into())),
                subsection_name: Some(validated_subsection(subsection_name)?),
            })
        } else {
            Ok(SectionHeader {
                name,
                separator: None,
                subsection_name: None,
            })
        }
    }
}

#[must_use]
fn is_valid_subsection(name: &BStr) -> bool {
    name.find_byteset(b"\n\0").is_none()
}

fn validated_subsection(name: Cow<'_, BStr>) -> Result<Cow<'_, BStr>, GitError> {
    is_valid_subsection(name.as_ref())
        .then_some(name)
        .ok_or(GitError::Gen)
}

fn validated_name(name: Cow<'_, BStr>) -> Result<Cow<'_, BStr>, GitError> {
    name.iter()
        .all(|bytec| bytec.is_ascii_alphanumeric() || *bytec == b'-')
        .then_some(name)
        .ok_or(GitError::Gen)
}

impl SectionHeader<'_> {
    // gix functions
    #[must_use]
    fn subsection_name(&self) -> Option<&BStr> {
        self.subsection_name.as_deref()
    }

    #[must_use]
    fn to_bstring(&self) -> BString {
        let mut buf = Vec::new();
        self.write_to(&mut buf).expect("io error impossible");
        buf.into()
    }

    fn write_to(&self, mut out: impl std::io::Write) -> std::io::Result<()> {
        out.write_all(b"[")?;
        out.write_all(&self.name)?;

        if let (Some(sep), Some(subsection)) = (&self.separator, &self.subsection_name) {
            let sep = sep.as_ref();
            out.write_all(sep)?;
            if sep == "." {
                out.write_all(subsection.as_ref())?;
            } else {
                out.write_all(b"\"")?;
                out.write_all(escape_subsection(subsection.as_ref()).as_ref())?;
                out.write_all(b"\"")?;
            }
        }

        out.write_all(b"]")
    }

    #[must_use]
    fn to_owned(&self) -> SectionHeader<'static> {
        SectionHeader {
            name: self.name.to_owned(),
            separator: self.separator.clone().map(|v| Cow::Owned(v.into_owned())),
            subsection_name: self
                .subsection_name
                .clone()
                .map(|v| Cow::Owned(v.into_owned())),
        }
    }
}

fn escape_subsection(name: &BStr) -> Cow<'_, BStr> {
    if name.find_byteset(b"\\\"").is_none() {
        return name.into();
    }
    let mut buf = Vec::with_capacity(name.len());
    for b in name.iter().copied() {
        match b {
            b'\\' => buf.push_str(br"\\"),
            b'"' => buf.push_str(br#"\""#),
            _ => buf.push(b),
        }
    }
    BString::from(buf).into()
}

impl Display for SectionHeader<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.to_bstring(), f)
    }
}

impl From<SectionHeader<'_>> for BString {
    fn from(header: SectionHeader<'_>) -> Self {
        header.to_bstring()
    }
}

impl From<&SectionHeader<'_>> for BString {
    fn from(header: &SectionHeader<'_>) -> Self {
        header.to_bstring()
    }
}

impl<'a> From<SectionHeader<'a>> for Event<'a> {
    fn from(header: SectionHeader<'_>) -> Event<'_> {
        Event::SectionHeader(header)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct SectionHeader<'a> {
    name: SectionName<'a>,
    separator: Option<Cow<'a, BStr>>,
    subsection_name: Option<Cow<'a, BStr>>,
}

fn is_valid_name(n: &bstr::BStr) -> bool {
    !n.is_empty() && n.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'-')
}

fn is_valid_value_name(n: &bstr::BStr) -> bool {
    is_valid_name(n) && n[0].is_ascii_alphabetic()
}

/// Wrapper struct for section header names, like `remote`, since these are case-insensitive.
#[derive(Clone, Eq, Debug, Default)]
struct SectionName<'a>(Cow<'a, bstr::BStr>);

impl<'a> SectionName<'a> {
    fn from_str_unchecked(s: &'a str) -> Self {
        SectionName(Cow::Borrowed(s.into()))
    }

    #[must_use]
    fn to_owned(&self) -> SectionName<'static> {
        SectionName(Cow::Owned(self.0.clone().into_owned()))
    }
}

impl PartialEq for SectionName<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl std::fmt::Display for SectionName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl PartialOrd for SectionName<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SectionName<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let a = self.0.iter().map(u8::to_ascii_lowercase);
        let b = other.0.iter().map(u8::to_ascii_lowercase);
        a.cmp(b)
    }
}

impl std::hash::Hash for SectionName<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for b in self.0.iter() {
            b.to_ascii_lowercase().hash(state);
        }
    }
}

impl<'a> std::convert::TryFrom<&'a str> for SectionName<'a> {
    type Error = GitError;

    fn try_from(s: &'a str) -> Result<Self, GitError> {
        Self::try_from(Cow::Borrowed(bstr::ByteSlice::as_bstr(s.as_bytes())))
    }
}

impl std::convert::TryFrom<String> for SectionName<'static> {
    type Error = GitError;

    fn try_from(s: String) -> Result<Self, GitError> {
        Self::try_from(Cow::Owned(bstr::BString::from(s)))
    }
}

impl<'a> std::convert::TryFrom<Cow<'a, bstr::BStr>> for SectionName<'a> {
    type Error = GitError;

    fn try_from(s: Cow<'a, bstr::BStr>) -> Result<Self, GitError> {
        if is_valid_name(s.as_ref()) {
            Ok(Self(s))
        } else {
            Err(GitError::Gen)
        }
    }
}

impl std::ops::Deref for SectionName<'_> {
    type Target = bstr::BStr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::convert::AsRef<str> for SectionName<'_> {
    fn as_ref(&self) -> &str {
        std::str::from_utf8(self.0.as_ref())
            .expect("only valid UTF8 makes it through our validation")
    }
}

/// Wrapper struct for value names, like `path` in `include.path`, since keys are case-insensitive.
#[derive(Clone, Eq, Debug, Default)]
struct SectionValueName<'a>(Cow<'a, bstr::BStr>);

impl<'a> SectionValueName<'a> {
    fn from_str_unchecked(s: &'a str) -> Self {
        SectionValueName(Cow::Borrowed(s.into()))
    }

    #[must_use]
    fn to_owned(&self) -> SectionValueName<'static> {
        SectionValueName(Cow::Owned(self.0.clone().into_owned()))
    }
}

impl PartialEq for SectionValueName<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl std::fmt::Display for SectionValueName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl PartialOrd for SectionValueName<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SectionValueName<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let a = self.0.iter().map(u8::to_ascii_lowercase);
        let b = other.0.iter().map(u8::to_ascii_lowercase);
        a.cmp(b)
    }
}

impl std::hash::Hash for SectionValueName<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for b in self.0.iter() {
            b.to_ascii_lowercase().hash(state);
        }
    }
}

impl<'a> std::convert::TryFrom<&'a str> for SectionValueName<'a> {
    type Error = GitError;

    fn try_from(s: &'a str) -> Result<Self, GitError> {
        Self::try_from(Cow::Borrowed(bstr::ByteSlice::as_bstr(s.as_bytes())))
    }
}

impl std::convert::TryFrom<String> for SectionValueName<'static> {
    type Error = GitError;

    fn try_from(s: String) -> Result<Self, GitError> {
        Self::try_from(Cow::Owned(bstr::BString::from(s)))
    }
}

impl<'a> std::convert::TryFrom<Cow<'a, bstr::BStr>> for SectionValueName<'a> {
    type Error = GitError;

    fn try_from(s: Cow<'a, bstr::BStr>) -> Result<Self, GitError> {
        if is_valid_value_name(s.as_ref()) {
            Ok(Self(s))
        } else {
            Err(GitError::Gen)
        }
    }
}

impl std::ops::Deref for SectionValueName<'_> {
    type Target = bstr::BStr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::convert::AsRef<str> for SectionValueName<'_> {
    fn as_ref(&self) -> &str {
        std::str::from_utf8(self.0.as_ref())
            .expect("only valid UTF8 makes it through our validation")
    }
}

fn into_cow_bstr(c: Cow<'_, str>) -> Cow<'_, BStr> {
    match c {
        Cow::Borrowed(s) => Cow::Borrowed(s.into()),
        Cow::Owned(s) => Cow::Owned(s.into()),
    }
}

#[must_use]
fn normalize(mut input: Cow<'_, BStr>) -> Cow<'_, BStr> {
    if input.as_ref() == "\"\"" {
        return Cow::Borrowed("".into());
    }
    while input.len() >= 3
        && input[0] == b'"'
        && input[input.len() - 1] == b'"'
        && input[input.len() - 2] != b'\\'
    {
        match &mut input {
            Cow::Borrowed(input) => *input = &input[1..input.len() - 1],
            Cow::Owned(input) => {
                input.pop();
                input.remove(0);
            }
        }
        if input.as_ref() == "\"\"" {
            return Cow::Borrowed("".into());
        }
    }

    if input.find_byteset(br#"\""#).is_none() {
        return input;
    }
    let mut out: BString = Vec::with_capacity(input.len()).into();
    let mut bytes = input.iter().copied();
    while let Some(c) = bytes.next() {
        match c {
            b'\\' => match bytes.next() {
                Some(b'n') => out.push(b'\n'),
                Some(b't') => out.push(b'\t'),
                Some(b'b') => {
                    out.pop();
                }
                Some(c) => {
                    out.push(c);
                }
                None => break,
            },
            b'"' => {}
            _ => out.push(c),
        }
    }
    Cow::Owned(out)
}

#[must_use]
fn normalize_bstr<'a>(input: impl Into<&'a BStr>) -> Cow<'a, BStr> {
    normalize(Cow::Borrowed(input.into()))
}

#[must_use]
fn normalize_bstring(input: impl Into<BString>) -> Cow<'static, BStr> {
    normalize(Cow::Owned(input.into()))
}

trait AsKey: Copy {
    fn as_key(&self) -> KeyRef<'_>;

    fn hx_try_as_key(&self) -> Option<KeyRef<'_>>;
}

impl AsKey for &str {
    fn as_key(&self) -> KeyRef<'_> {
        self.hx_try_as_key()
            .unwrap_or_else(|| panic!("'{self}' is not a valid configuration key"))
    }

    fn hx_try_as_key(&self) -> Option<KeyRef<'_>> {
        KeyRef::hx_parse_unvalidated((*self).into())
    }
}

#[derive(Debug, PartialEq, Ord, PartialOrd, Eq, Hash, Clone, Copy)]
struct KeyRef<'a> {
    section_name: &'a str,
    subsection_name: Option<&'a BStr>,
    value_name: &'a str,
}

impl KeyRef<'_> {
    #[must_use]
    fn hx_parse_unvalidated(input: &BStr) -> Option<KeyRef<'_>> {
        let mut tokens = input.splitn(2, |b| *b == b'.');
        let section_name = tokens.next()?;
        let subsection_or_key = tokens.next()?;
        let mut tokens = subsection_or_key.rsplitn(2, |b| *b == b'.');
        let (subsection_name, value_name) = match (tokens.next(), tokens.next()) {
            (Some(key), Some(subsection)) => (Some(subsection.into()), key),
            (Some(key), None) => (None, key),
            (None, Some(_)) => unreachable!("iterator can't restart producing items"),
            (None, None) => return None,
        };

        Some(KeyRef {
            section_name: section_name.to_str().ok()?,
            subsection_name,
            value_name: value_name.to_str().ok()?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
enum Source {
    GitInstallation,
    System,
    Git,
    User,
}

#[derive(Eq, Clone, Debug, Default)]
struct GitConfigurationFile<'event> {
    frontmatter_events: SmallVec<[Event<'event>; 8]>,
    frontmatter_post_section: HashMap<SectionId, SmallVec<[Event<'event>; 8]>>,
    section_lookup_tree: HashMap<SectionName<'event>, Vec<SectionBodyIdsLut<'event>>>,
    sections: HashMap<SectionId, ConfigFileSection<'event>>,
    section_id_counter: usize,
    section_order: VecDeque<SectionId>,
    meta: std::sync::Arc<Metadata>,
}

type FrontMatterEvents<'a> = SmallVec<[Event<'a>; 8]>;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
struct Events<'a> {
    frontmatter: SmallVec<[Event<'a>; 8]>,
    sections: Vec<ConfigSection<'a>>,
}

impl Events<'static> {
    fn from_bytes_owned<'a>(
        input: &'a [u8],
        _filter: Option<fn(&Event<'a>) -> bool>,
    ) -> Result<Events<'static>, GitError> {
        from_bytes(input, &|e| e.to_owned(), None)
    }
}

impl<'a> Events<'a> {
    fn from_bytes(
        input: &'a [u8],
        filter: Option<fn(&Event<'a>) -> bool>,
    ) -> Result<Events<'a>, GitError> {
        from_bytes(input, &std::convert::identity, filter)
    }

    #[allow(clippy::should_implement_trait)]
    fn from_str(input: &'a str) -> Result<Events<'a>, GitError> {
        Self::from_bytes(input.as_bytes(), None)
    }
}

impl<'a> TryFrom<&'a str> for Events<'a> {
    type Error = GitError;

    fn try_from(value: &'a str) -> Result<Self, GitError> {
        Self::from_str(value)
    }
}

impl<'a> TryFrom<&'a [u8]> for Events<'a> {
    type Error = GitError;

    fn try_from(value: &'a [u8]) -> Result<Self, GitError> {
        Events::from_bytes(value, None)
    }
}

fn from_bytes<'a, 'b>(
    input: &'a [u8],
    convert: &dyn Fn(Event<'a>) -> Event<'b>,
    _filter: Option<fn(&Event<'a>) -> bool>,
) -> Result<Events<'b>, GitError> {
    let mut header = None;
    let mut events = Vec::with_capacity(256);
    let mut frontmatter = SmallVec::new();
    let mut sections = Vec::new();
    parse_from_bytes(input, &mut |e: Event<'_>| match e {
        Event::SectionHeader(next_header) => {
            match header.take() {
                None => {
                    frontmatter = std::mem::take(&mut events).into_iter().collect();
                }
                Some(prev_header) => {
                    sections.push(ConfigSection {
                        header: prev_header,
                        events: std::mem::take(&mut events),
                    });
                }
            }
            header = match convert(Event::SectionHeader(next_header)) {
                Event::SectionHeader(h) => h,
                _ => unreachable!("BUG: convert must not change the event type, just the lifetime"),
            }
            .into();
        }
        event => {
            events.push(convert(event));
        }
    })?;

    match header {
        None => {
            frontmatter = events.into_iter().collect();
        }
        Some(prev_header) => {
            sections.push(ConfigSection {
                header: prev_header,
                events: std::mem::take(&mut events),
            });
        }
    }
    Ok(Events {
        frontmatter,
        sections,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
enum GixConfigSourceKind {
    GitInstallation,
    System,
    Global,
}

impl GixConfigSourceKind {
    #[must_use]
    fn sources(self) -> &'static [Source] {
        match self {
            GixConfigSourceKind::GitInstallation => &[Source::GitInstallation] as &[_],
            GixConfigSourceKind::System => &[Source::System],
            GixConfigSourceKind::Global => &[Source::Git, Source::User],
        }
    }
}

impl Source {
    fn hx_storage_location(
        self,
        env_var: &mut dyn FnMut(&str) -> Option<OsString>,
    ) -> Option<Cow<'static, Path>> {
        use Source::{Git, GitInstallation, System, User};

        match self {
            GitInstallation => hx_installation_config().map(Into::into),
            System => {
                if env_var("GIT_CONFIG_NOSYSTEM")
                    .map(Boolean::try_from)
                    .transpose()
                    .ok()
                    .flatten()
                    .is_some_and(|b| b.0)
                {
                    None
                } else {
                    env_var("GIT_CONFIG_SYSTEM")
                        .map(|p| Cow::Owned(p.into()))
                        .or_else(|| system_prefix().map(|p| p.join("etc/gitconfig").into()))
                }
            }
            Git => match env_var("GIT_CONFIG_GLOBAL") {
                Some(global_override) => Some(PathBuf::from(global_override).into()),
                None => xdg_config("config", env_var).map(Cow::Owned),
            },
            User => env_var("GIT_CONFIG_GLOBAL")
                .map(|global_override| PathBuf::from(global_override).into())
                .or_else(|| {
                    env_var("HOME")
                        .map(PathBuf::from)
                        .or_else(|| {
                            if cfg!(windows) {
                                std::env::home_dir()
                            } else {
                                None
                            }
                        })
                        .map(|mut p| {
                            p.push(".gitconfig");
                            p.into()
                        })
                }),
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum ParseNode {
    SectionHeader,
    Name,
    Value,
}

impl Display for ParseNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SectionHeader => write!(f, "section header"),
            Self::Name => write!(f, "name"),
            Self::Value => write!(f, "value"),
        }
    }
}

#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct Boolean(bool);

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct ConfigPath<'a> {
    value: Cow<'a, bstr::BStr>,
    is_optional: bool,
}

impl TryFrom<OsString> for Boolean {
    type Error = GitError;

    fn try_from(value: OsString) -> Result<Self, GitError> {
        let value = os_str_into_bstr(&value).map_err(|_| GitError::Gen)?;
        Self::try_from(value)
    }
}

impl TryFrom<&BStr> for Boolean {
    type Error = GitError;

    fn try_from(value: &BStr) -> Result<Self, GitError> {
        if parse_true(value) {
            Ok(Boolean(true))
        } else if parse_false(value) {
            Ok(Boolean(false))
        } else {
            use bstr::ByteSlice;
            use std::str::FromStr;
            if let Some(integer) = value.to_str().ok().and_then(|s| i64::from_str(s).ok()) {
                Ok(Boolean(integer != 0))
            } else {
                Err(GitError::Gen)
            }
        }
    }
}

impl TryFrom<Cow<'_, BStr>> for Boolean {
    type Error = GitError;
    fn try_from(c: Cow<'_, BStr>) -> Result<Self, GitError> {
        Self::try_from(c.as_ref())
    }
}

impl Display for Boolean {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl From<Boolean> for bool {
    fn from(b: Boolean) -> Self {
        b.0
    }
}

fn parse_true(value: &BStr) -> bool {
    value.eq_ignore_ascii_case(b"yes")
        || value.eq_ignore_ascii_case(b"on")
        || value.eq_ignore_ascii_case(b"true")
}

fn parse_false(value: &BStr) -> bool {
    value.eq_ignore_ascii_case(b"no")
        || value.eq_ignore_ascii_case(b"off")
        || value.eq_ignore_ascii_case(b"false")
        || value.is_empty()
}

#[derive(Clone, Copy)]
struct InterpolationContext<'a> {
    git_install_dir: Option<&'a std::path::Path>,
    home_dir: Option<&'a std::path::Path>,
    home_for_user: Option<fn(&str) -> Option<PathBuf>>,
}

impl Default for InterpolationContext<'_> {
    fn default() -> Self {
        InterpolationContext {
            git_install_dir: None,
            home_dir: None,
            home_for_user: Some(home_for_user),
        }
    }
}

#[must_use]
fn home_for_user(name: &str) -> Option<PathBuf> {
    let cname = std::ffi::CString::new(name).ok()?;
    #[allow(unsafe_code)]
    let pwd = unsafe { libc::getpwnam(cname.as_ptr()) };
    if pwd.is_null() {
        None
    } else {
        use std::os::unix::ffi::OsStrExt;
        #[allow(unsafe_code)]
        let cstr = unsafe { std::ffi::CStr::from_ptr((*pwd).pw_dir) };
        Some(std::ffi::OsStr::from_bytes(cstr.to_bytes()).into())
    }
}

impl std::ops::Deref for ConfigPath<'_> {
    type Target = BStr;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref()
    }
}

impl AsRef<[u8]> for ConfigPath<'_> {
    fn as_ref(&self) -> &[u8] {
        self.value.as_ref()
    }
}

impl AsRef<BStr> for ConfigPath<'_> {
    fn as_ref(&self) -> &BStr {
        self.value.as_ref()
    }
}

impl<'a> From<Cow<'a, BStr>> for ConfigPath<'a> {
    fn from(value: Cow<'a, BStr>) -> Self {
        const OPTIONAL_PREFIX: &[u8] = b":(optional)";

        if value.starts_with(OPTIONAL_PREFIX) {
            let stripped = match value {
                Cow::Borrowed(b) => Cow::Borrowed(&b[OPTIONAL_PREFIX.len()..]),
                Cow::Owned(mut b) => {
                    b.drain(..OPTIONAL_PREFIX.len());
                    Cow::Owned(b)
                }
            };
            ConfigPath {
                value: stripped,
                is_optional: true,
            }
        } else {
            ConfigPath {
                value,
                is_optional: false,
            }
        }
    }
}

impl<'a> ConfigPath<'a> {
    fn hx_interpolate(
        self,
        InterpolationContext {
            git_install_dir,
            home_dir,
            home_for_user,
        }: InterpolationContext<'_>,
    ) -> Result<Cow<'a, std::path::Path>, GitError> {
        if self.is_empty() {
            return Err(GitError::Gen); // -> Ok(None)
        }

        const PREFIX: &[u8] = b"%(prefix)/";
        const USER_HOME: &[u8] = b"~/";
        if self.starts_with(PREFIX) {
            let git_install_dir = git_install_dir.ok_or(GitError::Gen)?; // -> Ok(None)
            let (_prefix, path_without_trailing_slash) = self.split_at(PREFIX.len());
            let path_without_trailing_slash =
                hx_try_from_bstring(path_without_trailing_slash).map_err(|_| GitError::Error)?; // -> Error
            Ok(git_install_dir.join(path_without_trailing_slash).into())
        } else if self.starts_with(USER_HOME) {
            let home_path = home_dir.ok_or(GitError::Gen)?; // -> Ok(None)
            let (_prefix, val) = self.split_at(USER_HOME.len());
            let val = hx_try_from_byte_slice(val).map_err(|_| GitError::Error)?; // -> Error
            Ok(home_path.join(val).into())
        } else if self.starts_with(b"~") && self.contains(&b'/') {
            self.interpolate_user(home_for_user.ok_or(GitError::Gen)?) // -> Ok(None)
        } else {
            Ok(from_bstr(self.value))
        }
    }

    fn interpolate_user(
        self,
        home_for_user: fn(&str) -> Option<PathBuf>,
    ) -> Result<Cow<'a, std::path::Path>, GitError> {
        let (_prefix, val) = self.split_at("/".len());
        let i = val.iter().position(|&e| e == b'/').ok_or(GitError::Gen)?;
        let (username, path_with_leading_slash) = val.split_at(i);
        let username = std::str::from_utf8(username).map_err(|_| GitError::Gen)?;
        let home = home_for_user(username).ok_or(GitError::Gen)?;
        let path_past_user_prefix = hx_try_from_byte_slice(&path_with_leading_slash["/".len()..])
            .map_err(|_| GitError::Gen)?;
        Ok(home.join(path_past_user_prefix).into())
    }
}

fn escape_value(value: &BStr) -> BString {
    let starts_with_whitespace = value.first().is_some_and(u8::is_ascii_whitespace);
    let ends_with_whitespace = value
        .get(value.len().saturating_sub(1))
        .is_some_and(u8::is_ascii_whitespace);
    let contains_comment_indicators = value.find_byteset(b";#").is_some();
    let quote = starts_with_whitespace || ends_with_whitespace || contains_comment_indicators;

    let mut buf: BString = Vec::with_capacity(value.len()).into();
    if quote {
        buf.push(b'"');
    }

    for b in value.iter().copied() {
        match b {
            b'\n' => buf.push_str(r"\n"),
            b'\t' => buf.push_str(r"\t"),
            b'"' => buf.push_str(r#"\""#),
            b'\\' => buf.push_str(r"\\"),
            _ => buf.push(b),
        }
    }

    if quote {
        buf.push(b'"');
    }
    buf
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
struct Whitespace<'a> {
    pre_key: Option<Cow<'a, BStr>>,
    pre_sep: Option<Cow<'a, BStr>>,
    post_sep: Option<Cow<'a, BStr>>,
}

impl Default for Whitespace<'_> {
    fn default() -> Self {
        Whitespace {
            pre_key: Some(b"\t".as_bstr().into()),
            pre_sep: Some(b" ".as_bstr().into()),
            post_sep: Some(b" ".as_bstr().into()),
        }
    }
}

impl<'a> Whitespace<'a> {
    fn key_value_separators(&self) -> Vec<Event<'a>> {
        let mut out = Vec::with_capacity(3);
        if let Some(ws) = &self.pre_sep {
            out.push(Event::Whitespace(ws.clone()));
        }
        out.push(Event::KeyValueSeparator);
        if let Some(ws) = &self.post_sep {
            out.push(Event::Whitespace(ws.clone()));
        }
        out
    }

    fn from_body(s: &ConfigFileBody<'a>) -> Self {
        let key_pos =
            s.0.iter()
                .enumerate()
                .find_map(|(idx, e)| matches!(e, Event::SectionValueName(_)).then(|| idx));
        key_pos
            .map(|key_pos| {
                let pre_key = s.0[..key_pos].iter().next_back().and_then(|e| match e {
                    Event::Whitespace(s) => Some(s.clone()),
                    _ => None,
                });
                let from_key = &s.0[key_pos..];
                let (pre_sep, post_sep) = from_key
                    .iter()
                    .enumerate()
                    .find_map(|(idx, e)| matches!(e, Event::KeyValueSeparator).then(|| idx))
                    .map(|sep_pos| {
                        (
                            from_key.get(sep_pos - 1).and_then(|e| match e {
                                Event::Whitespace(ws) => Some(ws.clone()),
                                _ => None,
                            }),
                            from_key.get(sep_pos + 1).and_then(|e| match e {
                                Event::Whitespace(ws) => Some(ws.clone()),
                                _ => None,
                            }),
                        )
                    })
                    .unwrap_or_default();
                Whitespace {
                    pre_key,
                    pre_sep,
                    post_sep,
                }
            })
            .unwrap_or_default()
    }
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
struct SectionMut<'a, 'event> {
    section: &'a mut ConfigFileSection<'event>,
    implicit_newline: bool,
    whitespace: Whitespace<'event>,
    newline: SmallVec<[u8; 2]>,
}

impl<'event> SectionMut<'_, 'event> {
    fn push<'b>(
        &mut self,
        value_name: SectionValueName<'event>,
        value: Option<&'b BStr>,
    ) -> &mut Self {
        self.push_with_comment_inner(value_name, value, None);
        self
    }

    fn push_with_comment_inner(
        &mut self,
        value_name: SectionValueName<'event>,
        value: Option<&BStr>,
        comment: Option<&BStr>,
    ) {
        let body = &mut self.section.body.0;
        if let Some(ws) = &self.whitespace.pre_key {
            body.push(Event::Whitespace(ws.clone()));
        }

        body.push(Event::SectionValueName(value_name));
        match value {
            Some(value) => {
                body.extend(self.whitespace.key_value_separators());
                body.push(Event::Value(escape_value(value).into()));
            }
            None => body.push(Event::Value(Cow::Borrowed("".into()))),
        }
        if let Some(comment) = comment {
            body.push(Event::Whitespace(Cow::Borrowed(" ".into())));
            body.push(Event::Comment(Comment {
                tag: b'#',
                text: Cow::Owned({
                    let mut c = Vec::with_capacity(comment.len());
                    let mut bytes = comment.iter().peekable();
                    if !bytes.peek().is_none_or(|b| b.is_ascii_whitespace()) {
                        c.insert(0, b' ');
                    }
                    c.extend(bytes.map(|b| if *b == b'\n' { b' ' } else { *b }));
                    c.into()
                }),
            }));
        }
        if self.implicit_newline {
            body.push(Event::Newline(BString::from(self.newline.to_vec()).into()));
        }
    }

    fn push_newline(&mut self) -> &mut Self {
        self.section
            .body
            .0
            .push(Event::Newline(Cow::Owned(BString::from(
                self.newline.to_vec(),
            ))));
        self
    }
}

impl<'a, 'event> SectionMut<'a, 'event> {
    fn new(section: &'a mut ConfigFileSection<'event>, newline: SmallVec<[u8; 2]>) -> Self {
        let whitespace = Whitespace::from_body(&section.body);
        Self {
            section,
            implicit_newline: true,
            whitespace,
            newline,
        }
    }
}

impl<'event> Deref for SectionMut<'_, 'event> {
    type Target = ConfigFileSection<'event>;

    fn deref(&self) -> &Self::Target {
        self.section
    }
}

#[derive(Clone, Copy, Default)]
struct ConfigFileOptions<'a> {
    includes: FileIncludesOptions<'a>,
}

impl GitConfigurationFile<'static> {
    // gix config
    fn from_env(
        includes: FileIncludesOptions,
    ) -> Result<Option<GitConfigurationFile<'static>>, GitError> {
        use std::env;
        let count: usize = match env::var("GIT_CONFIG_COUNT") {
            Ok(v) => v.parse().map_err(|_| GitError::Gen)?,
            Err(_) => return Ok(None),
        };

        if count == 0 {
            return Ok(None);
        }

        let meta = Metadata { path: None };
        let mut config = GitConfigurationFile::new(meta);
        for i in 0..count {
            let key = os_string_into_bstring(
                env::var_os(format!("GIT_CONFIG_KEY_{i}")).ok_or(GitError::Gen)?,
            )
            .map_err(|_| GitError::Gen)?;
            let value = env::var_os(format!("GIT_CONFIG_VALUE_{i}")).ok_or(GitError::Gen)?;
            let key = KeyRef::hx_parse_unvalidated(key.as_ref()).ok_or(GitError::Gen)?;

            config
                .section_mut_or_create_new(key.section_name, key.subsection_name)?
                .push(
                    SectionValueName::try_from(key.value_name.to_owned())?,
                    Some(
                        os_str_into_bstr(&value)
                            .map_err(|_| GitError::Gen)?
                            .as_bytes()
                            .into(),
                    ),
                );
        }

        let mut buf = Vec::new();
        resolve_includes(&mut config, &mut buf, includes)?;
        Ok(Some(config))
    }
}

impl GitConfigurationFile<'static> {
    fn from_paths_metadata_buf(
        path_meta: &mut dyn Iterator<Item = Metadata>,
        buf: &mut Vec<u8>,
        options: ConfigFileOptions<'_>,
    ) -> Result<Option<Self>, GitError> {
        let mut target = None;
        let mut seen = BTreeSet::default();
        for (path, mut meta) in path_meta.filter_map(|mut meta| meta.path.take().map(|p| (p, meta)))
        {
            if !seen.insert(path.clone()) {
                continue;
            }

            buf.clear();
            match std::io::copy(
                &mut match std::fs::File::open(&path) {
                    Ok(f) => f,
                    Err(_) => {
                        continue;
                    }
                },
                buf,
            ) {
                Ok(_) => {}
                Err(_) => {
                    return Err(GitError::Io);
                }
            }
            meta.path = Some(path);

            let config = Self::from_bytes_owned(buf, meta, options)?;
            match &mut target {
                None => {
                    target = Some(config);
                }
                Some(target) => {
                    target.append(config);
                }
            }
        }
        Ok(target)
    }
}

impl<'a> GitConfigurationFile<'a> {
    fn new(meta: impl Into<Arc<Metadata>>) -> Self {
        Self {
            frontmatter_events: Default::default(),
            frontmatter_post_section: Default::default(),
            section_lookup_tree: Default::default(),
            sections: Default::default(),
            section_id_counter: 0,
            section_order: Default::default(),
            meta: meta.into(),
        }
    }

    fn from_parse_events_no_includes(
        Events {
            frontmatter,
            sections,
        }: Events<'a>,
        meta: impl Into<Arc<Metadata>>,
    ) -> Self {
        let meta = meta.into();
        let mut this = GitConfigurationFile::new(Arc::clone(&meta));

        this.frontmatter_events = frontmatter;

        this.sections.reserve(sections.len());
        this.section_order.reserve(sections.len());
        for section in sections {
            this.push_section_internal(ConfigFileSection {
                header: section.header,
                body: ConfigFileBody(section.events),
                meta: Arc::clone(&meta),
                id: Default::default(),
            });
        }
        this
    }
}

impl GitConfigurationFile<'static> {
    fn from_bytes_owned(
        input_and_buf: &mut Vec<u8>,
        meta: impl Into<Arc<Metadata>>,
        options: ConfigFileOptions<'_>,
    ) -> Result<Self, GitError> {
        let mut config = Self::from_parse_events_no_includes(
            Events::from_bytes_owned(input_and_buf, None).map_err(|_| GitError::Gen)?,
            meta,
        );

        resolve_includes(&mut config, input_and_buf, options.includes)
            .map_err(|_| GitError::Gen)?;
        Ok(config)
    }
}

impl GitConfigurationFile<'_> {
    fn string(&self, key: impl AsKey) -> Option<Cow<'_, BStr>> {
        self.hx_string_filter(key)
    }

    fn hx_string_filter(&self, key: impl AsKey) -> Option<Cow<'_, BStr>> {
        let key = key.hx_try_as_key()?;
        self.hx_raw_value_filter_by(key.section_name, key.subsection_name, key.value_name)
            .ok()
    }

    fn hx_path_filter(&self, key: impl AsKey) -> Option<ConfigPath<'_>> {
        let key = key.hx_try_as_key()?;
        self.hx_path_filter_by(key.section_name, key.subsection_name, key.value_name)
    }

    fn hx_path_filter_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
    ) -> Option<ConfigPath<'_>> {
        self.hx_raw_value_filter_by(section_name.as_ref(), subsection_name, value_name.as_ref())
            .ok()
            .map(ConfigPath::from)
    }
}

impl<'event> GitConfigurationFile<'event> {
    fn section_mut_or_create_new<'a>(
        &'a mut self,
        name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
    ) -> Result<SectionMut<'a, 'event>, GitError> {
        self.section_mut_or_create_new_filter(name, subsection_name, |_| true)
    }

    fn section_mut_or_create_new_filter<'a>(
        &'a mut self,
        name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<SectionMut<'a, 'event>, GitError> {
        self.section_mut_or_create_new_filter_inner(name.as_ref(), subsection_name, filter)
    }

    fn section_mut_or_create_new_filter_inner<'a>(
        &'a mut self,
        name: &str,
        subsection_name: Option<&BStr>,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<SectionMut<'a, 'event>, GitError> {
        match self
            .hx_section_ids_by_name_and_subname(name.as_ref(), subsection_name)
            .ok()
            .and_then(|it| {
                it.rev()
                    .find(|id| self.sections.get(id).is_some_and(|s| filter(s.meta())))
            }) {
            Some(id) => {
                let nl = self.detect_newline_style_smallvec();
                Ok(self
                    .sections
                    .get_mut(&id)
                    .expect("BUG: Section did not have id from lookup")
                    .to_mut(nl))
            }
            None => self.new_section(
                name.to_owned(),
                subsection_name.map(|n| Cow::Owned(n.to_owned())),
            ),
        }
    }

    fn new_section(
        &mut self,
        name: impl Into<Cow<'event, str>>,
        subsection: impl Into<Option<Cow<'event, BStr>>>,
    ) -> Result<SectionMut<'_, 'event>, GitError> {
        let id = self.push_section_internal(ConfigFileSection::new(
            name.into(),
            subsection.into(),
            Arc::clone(&self.meta),
        )?);
        let nl = self.detect_newline_style_smallvec();
        let mut section = self
            .sections
            .get_mut(&id)
            .expect("each id yields a section")
            .to_mut(nl);
        section.push_newline();
        Ok(section)
    }

    fn append(&mut self, other: Self) -> &mut Self {
        self.append_or_insert(other, None)
    }

    fn append_or_insert(
        &mut self,
        mut other: Self,
        mut insert_after: Option<SectionId>,
    ) -> &mut Self {
        let nl = self.detect_newline_style_smallvec();
        fn extend_and_assure_newline<'a>(
            lhs: &mut FrontMatterEvents<'a>,
            rhs: FrontMatterEvents<'a>,
            nl: &impl AsRef<[u8]>,
        ) {
            if !ends_with_newline(lhs.as_ref(), nl, true)
                && !rhs
                    .first()
                    .is_none_or(|e| e.to_bstr_lossy().starts_with(nl.as_ref()))
            {
                lhs.push(Event::Newline(Cow::Owned(nl.as_ref().into())));
            }
            lhs.extend(rhs);
        }
        #[allow(clippy::unnecessary_lazy_evaluations)]
        let our_last_section_before_append = insert_after.or_else(|| {
            (self.section_id_counter != 0).then(|| SectionId(self.section_id_counter - 1))
        });

        for id in std::mem::take(&mut other.section_order) {
            let section = other.sections.remove(&id).expect("present");

            let new_id = match insert_after {
                Some(id) => {
                    let new_id = self.insert_section_after(section, id);
                    insert_after = Some(new_id);
                    new_id
                }
                None => self.push_section_internal(section),
            };

            if let Some(post_matter) = other.frontmatter_post_section.remove(&id) {
                self.frontmatter_post_section.insert(new_id, post_matter);
            }
        }

        if other.frontmatter_events.is_empty() {
            return self;
        }

        match our_last_section_before_append {
            Some(last_id) => extend_and_assure_newline(
                self.frontmatter_post_section.entry(last_id).or_default(),
                other.frontmatter_events,
                &nl,
            ),
            None => {
                extend_and_assure_newline(
                    &mut self.frontmatter_events,
                    other.frontmatter_events,
                    &nl,
                );
            }
        }
        self
    }
}

impl GitConfigurationFile<'_> {
    fn hx_raw_value_filter_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
    ) -> Result<Cow<'_, BStr>, GitError> {
        self.hx_raw_value_filter_inner(section_name.as_ref(), subsection_name, value_name.as_ref())
    }

    fn hx_raw_value_filter_inner(
        &self,
        section_name: &str,
        subsection_name: Option<&BStr>,
        value_name: &str,
    ) -> Result<Cow<'_, BStr>, GitError> {
        let section_ids = self.hx_section_ids_by_name_and_subname(section_name, subsection_name)?;
        for section_id in section_ids.rev() {
            let section = self.sections.get(&section_id).expect("known section id");
            if let Some(v) = section.value(value_name) {
                return Ok(v);
            }
        }

        Err(GitError::KeyMissing)
    }
}

impl<'event> GitConfigurationFile<'event> {
    #[must_use]
    fn sections_by_name<'a>(
        &'a self,
        name: &'a str,
    ) -> Option<impl Iterator<Item = &'a ConfigFileSection<'event>> + 'a> {
        self.section_ids_by_name(name).ok().map(move |ids| {
            ids.map(move |id| {
                self.sections
                    .get(&id)
                    .expect("section doesn't have id from from lookup")
            })
        })
    }

    fn set_meta(&mut self, meta: impl Into<Arc<Metadata>>) -> &mut Self {
        self.meta = meta.into();
        self
    }

    #[must_use]
    fn meta_owned(&self) -> std::sync::Arc<Metadata> {
        Arc::clone(&self.meta)
    }

    fn sections(&self) -> impl Iterator<Item = &ConfigFileSection<'event>> + '_ {
        self.section_order.iter().map(move |id| &self.sections[id])
    }

    fn detect_newline_style(&self) -> &BStr {
        self.frontmatter_events
            .iter()
            .find_map(extract_newline)
            .or_else(|| {
                self.sections()
                    .find_map(|s| s.body.as_ref().iter().find_map(extract_newline))
            })
            .unwrap_or_else(|| platform_newline())
    }

    fn detect_newline_style_smallvec(&self) -> SmallVec<[u8; 2]> {
        self.detect_newline_style().as_bytes().into()
    }
}

impl FromStr for GitConfigurationFile<'static> {
    type Err = GitError;

    fn from_str(s: &str) -> Result<Self, GitError> {
        Events::from_bytes_owned(s.as_bytes(), None).map(|events| {
            GitConfigurationFile::from_parse_events_no_includes(events, Metadata::api())
        })
    }
}

impl<'a> TryFrom<&'a str> for GitConfigurationFile<'a> {
    type Error = GitError;

    fn try_from(s: &'a str) -> Result<GitConfigurationFile<'a>, GitError> {
        Events::from_str(s)
            .map(|events| Self::from_parse_events_no_includes(events, Metadata::api()))
    }
}

impl<'a> TryFrom<&'a BStr> for GitConfigurationFile<'a> {
    type Error = GitError;

    fn try_from(value: &'a BStr) -> Result<GitConfigurationFile<'a>, GitError> {
        Events::from_bytes(value, None)
            .map(|events| Self::from_parse_events_no_includes(events, Metadata::api()))
    }
}

impl PartialEq for GitConfigurationFile<'_> {
    fn eq(&self, other: &Self) -> bool {
        fn find_key<'a>(
            mut it: impl Iterator<Item = &'a Event<'a>>,
        ) -> Option<&'a SectionValueName<'a>> {
            it.find_map(|e| match e {
                Event::SectionValueName(k) => Some(k),
                _ => None,
            })
        }
        fn collect_value<'a>(it: impl Iterator<Item = &'a Event<'a>>) -> Cow<'a, BStr> {
            let mut partial_value = BString::default();
            let mut value = None;

            for event in it {
                match event {
                    Event::SectionValueName(_) => break,
                    Event::Value(v) => {
                        value = v.clone().into();
                        break;
                    }
                    Event::ValueNotDone(v) => partial_value.push_str(v.as_ref()),
                    Event::ValueDone(v) => {
                        partial_value.push_str(v.as_ref());
                        value = Some(partial_value.into());
                        break;
                    }
                    _ => (),
                }
            }
            value.map(normalize).unwrap_or_default()
        }
        if self.section_order.len() != other.section_order.len() {
            return false;
        }

        for (lhs, rhs) in self
            .section_order
            .iter()
            .zip(&other.section_order)
            .map(|(lhs, rhs)| (&self.sections[lhs], &other.sections[rhs]))
        {
            if !(lhs.header.name == rhs.header.name
                && lhs.header.subsection_name == rhs.header.subsection_name)
            {
                return false;
            }

            let (mut lhs, mut rhs) = (lhs.body.0.iter(), rhs.body.0.iter());
            while let (Some(lhs_key), Some(rhs_key)) = (find_key(&mut lhs), find_key(&mut rhs)) {
                if lhs_key != rhs_key {
                    return false;
                }
                if collect_value(&mut lhs) != collect_value(&mut rhs) {
                    return false;
                }
            }
        }
        true
    }
}

impl GitConfigurationFile<'static> {
    fn resolve_includes(
        &mut self,
        // options: init::ConfigFileOptions<'_>,
        includes: FileIncludesOptions,
    ) -> Result<(), GitError> {
        if includes.max_depth == 0 {
            return Ok(());
        }
        let mut buf = Vec::new();
        resolve_includes(self, &mut buf, includes)
    }
}

fn resolve_includes(
    config: &mut GitConfigurationFile<'static>,
    buf: &mut Vec<u8>,
    includes: FileIncludesOptions,
) -> Result<(), GitError> {
    resolve_includes_recursive(config, 0, buf, includes)
}

fn resolve_includes_recursive(
    target_config: &mut GitConfigurationFile<'static>,
    depth: u8,
    buf: &mut Vec<u8>,
    includes: FileIncludesOptions,
) -> Result<(), GitError> {
    if depth == includes.max_depth {
        return if includes.err_on_max_depth_exceeded {
            Err(GitError::Gen)
        } else {
            Ok(())
        };
    }

    for id in target_config.section_order.clone() {
        let section = &target_config.sections[&id];
        let header = &section.header;
        let header_name = header.name.as_ref();
        let mut paths = None;
        if header_name == "include" && header.subsection_name.is_none() {
            paths = Some(gather_paths(section, id));
        } else if header_name == "includeIf"
            && let Some(condition) = &header.subsection_name
        {
            let target_config_path = section.meta.path.as_deref();
            if include_condition_match(condition.as_ref(), target_config_path, includes)? {
                paths = Some(gather_paths(section, id));
            }
        }
        if let Some(paths) = paths {
            insert_includes_recursively(paths, target_config, depth, includes, buf)?;
        }
    }
    Ok(())
}

fn insert_includes_recursively(
    section_ids_and_include_paths: Vec<(SectionId, ConfigPath<'_>)>,
    target_config: &mut GitConfigurationFile<'static>,
    depth: u8,
    includes: FileIncludesOptions,
    buf: &mut Vec<u8>,
) -> Result<(), GitError> {
    for (section_id, config_path) in section_ids_and_include_paths {
        let meta = Arc::clone(&target_config.sections[&section_id].meta);
        let target_config_path = meta.path.as_deref();
        let config_path = match resolve_path(config_path, target_config_path, includes)? {
            Some(p) => p,
            None => continue,
        };
        if !config_path.is_file() {
            continue;
        }

        buf.clear();
        std::io::copy(
            &mut std::fs::File::open(&config_path).map_err(|_| GitError::Gen)?,
            buf,
        )
        .map_err(|_| GitError::Gen)?;
        let config_meta = Metadata {
            path: Some(config_path),
        };
        let no_follow_options = ConfigFileOptions {
            includes: FileIncludesOptions::hx_no_follow(),
        };

        let mut include_config =
            GitConfigurationFile::from_bytes_owned(buf, config_meta, no_follow_options)
                .map_err(|_| GitError::Gen)?;
        resolve_includes_recursive(&mut include_config, depth + 1, buf, includes)?;

        target_config.append_or_insert(include_config, Some(section_id));
    }
    Ok(())
}

fn gather_paths(
    section: &ConfigFileSection<'_>,
    id: SectionId,
) -> Vec<(SectionId, ConfigPath<'static>)> {
    section
        .body
        .values("path")
        .into_iter()
        .map(|path| (id, ConfigPath::from(Cow::Owned(path.into_owned()))))
        .collect()
}

fn include_condition_match(
    condition: &BStr,
    target_config_path: Option<&Path>,
    options: FileIncludesOptions<'_>,
) -> Result<bool, GitError> {
    let mut tokens = condition.splitn(2, |b| *b == b':');
    let (prefix, condition) = match (tokens.next(), tokens.next()) {
        (Some(a), Some(b)) => (a, b),
        _ => return Ok(false),
    };
    let condition = condition.as_bstr();
    let _mode: WildmatchMode = WildmatchMode::empty();

    match prefix {
        b"gitdir" => gitdir_matches(
            condition,
            target_config_path,
            options,
            WildmatchMode::empty(),
        ),
        b"gitdir/i" => gitdir_matches(
            condition,
            target_config_path,
            options,
            WildmatchMode::IGNORE_CASE,
        ),
        _ => Ok(false),
    }
}

fn gitdir_matches(
    condition_path: &BStr,
    target_config_path: Option<&Path>,
    FileIncludesOptions {
        conditional: IncludesConditionalContext { git_dir, .. },
        interpolate: context,
        err_on_missing_config_path,
        ..
    }: FileIncludesOptions<'_>,
    wildmatch_mode: WildmatchMode,
) -> Result<bool, GitError> {
    if git_dir.is_none() {
        return Ok(false);
    }
    let git_dir = hx_to_unix_separators_on_windows(into_bstr(git_dir.ok_or(GitError::Gen)?));

    let mut pattern_path: Cow<'_, _> = {
        let path = match check_interpolation_result(
            ConfigPath::from(Cow::Borrowed(condition_path)).hx_interpolate(context),
        )? {
            Some(p) => p,
            None => return Ok(false),
        };
        into_bstr(path).into_owned().into()
    };
    if pattern_path != condition_path {
        pattern_path = hx_to_unix_separators_on_windows(pattern_path);
    }

    if let Some(relative_pattern_path) = pattern_path.strip_prefix(b"./") {
        if !err_on_missing_config_path && target_config_path.is_none() {
            return Ok(false);
        }
        let parent_dir = target_config_path
            .ok_or(GitError::Gen)?
            .parent()
            .expect("config path can never be /");
        let mut joined_path = hx_to_unix_separators_on_windows(into_bstr(parent_dir)).into_owned();
        joined_path.push(b'/');
        joined_path.extend_from_slice(relative_pattern_path);
        pattern_path = joined_path.into();
    }

    if pattern_path.iter().next() != Some(&(std::path::MAIN_SEPARATOR as u8))
        && !from_bstr(pattern_path.clone()).is_absolute()
    {
        let mut prefixed = pattern_path.into_owned();
        prefixed.insert_str(0, "**/");
        pattern_path = prefixed.into();
    }
    if pattern_path.ends_with(b"/") {
        let mut suffixed = pattern_path.into_owned();
        suffixed.push_str("**");
        pattern_path = suffixed.into();
    }

    let match_mode = WildmatchMode::NO_MATCH_SLASH_LITERAL | wildmatch_mode;
    let is_match = wildmatch(pattern_path.as_bstr(), git_dir.as_bstr(), match_mode);
    if is_match {
        return Ok(true);
    }

    let expanded_git_dir = into_bstr(realpath(from_byte_slice(&git_dir))?);
    Ok(wildmatch(
        pattern_path.as_bstr(),
        expanded_git_dir.as_bstr(),
        match_mode,
    ))
}

fn check_interpolation_result(
    res: Result<Cow<'_, std::path::Path>, GitError>,
) -> Result<Option<Cow<'_, std::path::Path>>, GitError> {
    match res {
        Ok(good) => Ok(good.into()),
        Err(err) => match err {
            GitError::Gen => Ok(None),
            _ => Err(GitError::Gen),
        },
    }
}

fn resolve_path(
    path: ConfigPath<'_>,
    target_config_path: Option<&Path>,
    FileIncludesOptions {
        interpolate: context,
        err_on_missing_config_path,
        ..
    }: FileIncludesOptions<'_>,
) -> Result<Option<PathBuf>, GitError> {
    let path = match check_interpolation_result(path.hx_interpolate(context))? {
        Some(p) => p,
        None => return Ok(None),
    };
    let path: PathBuf = if path.is_relative() {
        if !err_on_missing_config_path && target_config_path.is_none() {
            return Ok(None);
        }
        target_config_path
            .ok_or(GitError::Gen)?
            .parent()
            .expect("path is a config file which naturally lives in a directory")
            .join(path)
    } else {
        path.into()
    };
    Ok(Some(path))
}

#[derive(Clone, Copy)]
struct FileIncludesOptions<'a> {
    max_depth: u8,
    err_on_max_depth_exceeded: bool,
    err_on_missing_config_path: bool,
    interpolate: InterpolationContext<'a>,
    conditional: IncludesConditionalContext<'a>,
}

impl<'a> FileIncludesOptions<'a> {
    #[must_use]
    fn hx_no_follow() -> Self {
        FileIncludesOptions {
            max_depth: 0,
            err_on_max_depth_exceeded: false,
            err_on_missing_config_path: false,
            interpolate: InterpolationContext {
                git_install_dir: None,
                home_dir: None,
                home_for_user: Some(home_for_user),
            },
            conditional: Default::default(),
        }
    }
    #[must_use]
    fn hx_follow(
        interpolate: InterpolationContext<'a>,
        conditional: IncludesConditionalContext<'a>,
    ) -> Self {
        FileIncludesOptions {
            max_depth: 10,
            err_on_max_depth_exceeded: true,
            err_on_missing_config_path: true,
            interpolate,
            conditional,
        }
    }
}

impl Default for FileIncludesOptions<'_> {
    fn default() -> Self {
        FileIncludesOptions {
            max_depth: 0,
            err_on_max_depth_exceeded: false,
            err_on_missing_config_path: false,
            interpolate: InterpolationContext {
                git_install_dir: None,
                home_dir: None,
                home_for_user: Some(home_for_user),
            },
            conditional: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Default)]
struct IncludesConditionalContext<'a> {
    git_dir: Option<&'a std::path::Path>,
}

impl Metadata {
    #[must_use]
    fn api() -> Self {
        Metadata { path: None }
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Metadata::api()
    }
}

impl<'event> GitConfigurationFile<'event> {
    fn push_section_internal(&mut self, mut section: ConfigFileSection<'event>) -> SectionId {
        let new_section_id = SectionId(self.section_id_counter);
        section.id = new_section_id;
        self.sections.insert(new_section_id, section);
        let header = &self.sections[&new_section_id].header;
        let lookup = self
            .section_lookup_tree
            .entry(header.name.clone())
            .or_default();

        let mut found_node = false;
        if let Some(subsection_name) = header.subsection_name.clone() {
            for node in lookup.iter_mut() {
                if let SectionBodyIdsLut::NonTerminal(subsections) = node {
                    found_node = true;
                    subsections
                        .entry(subsection_name.clone())
                        .or_default()
                        .push(new_section_id);
                    break;
                }
            }
            if !found_node {
                let mut map = HashMap::new();
                map.insert(subsection_name, vec![new_section_id]);
                lookup.push(SectionBodyIdsLut::NonTerminal(map));
            }
        } else {
            for node in lookup.iter_mut() {
                if let SectionBodyIdsLut::Terminal(vec) = node {
                    found_node = true;
                    vec.push(new_section_id);
                    break;
                }
            }
            if !found_node {
                lookup.push(SectionBodyIdsLut::Terminal(vec![new_section_id]));
            }
        }
        self.section_order.push_back(new_section_id);
        self.section_id_counter += 1;
        new_section_id
    }

    fn insert_section_after(
        &mut self,
        mut section: ConfigFileSection<'event>,
        before: SectionId,
    ) -> SectionId {
        let lookup_section_order = {
            let section_order = &self.section_order;
            move |section_id| {
                section_order
                    .iter()
                    .enumerate()
                    .find_map(|(idx, id)| (*id == section_id).then_some(idx))
                    .expect("before-section exists")
            }
        };

        let before_order = lookup_section_order(before);
        let new_section_id = SectionId(self.section_id_counter);
        section.id = new_section_id;
        self.sections.insert(new_section_id, section);
        let header = &self.sections[&new_section_id].header;
        let lookup = self
            .section_lookup_tree
            .entry(header.name.clone())
            .or_default();

        let mut found_node = false;
        if let Some(subsection_name) = header.subsection_name.clone() {
            for node in lookup.iter_mut() {
                if let SectionBodyIdsLut::NonTerminal(subsections) = node {
                    found_node = true;
                    let sections_with_name_and_subsection_name =
                        subsections.entry(subsection_name.clone()).or_default();
                    let insert_pos = find_insert_pos_by_order(
                        sections_with_name_and_subsection_name,
                        before_order,
                        lookup_section_order,
                    );
                    sections_with_name_and_subsection_name.insert(insert_pos, new_section_id);
                    break;
                }
            }
            if !found_node {
                let mut map = HashMap::new();
                map.insert(subsection_name, vec![new_section_id]);
                lookup.push(SectionBodyIdsLut::NonTerminal(map));
            }
        } else {
            for node in lookup.iter_mut() {
                if let SectionBodyIdsLut::Terminal(sections_with_name) = node {
                    found_node = true;
                    let insert_pos = find_insert_pos_by_order(
                        sections_with_name,
                        before_order,
                        lookup_section_order,
                    );
                    sections_with_name.insert(insert_pos, new_section_id);
                    break;
                }
            }
            if !found_node {
                lookup.push(SectionBodyIdsLut::Terminal(vec![new_section_id]));
            }
        }

        self.section_order.insert(before_order + 1, new_section_id);
        self.section_id_counter += 1;
        new_section_id
    }

    fn hx_section_ids_by_name_and_subname<'a>(
        &'a self,
        section_name: &'a str,
        subsection_name: Option<&BStr>,
    ) -> Result<impl ExactSizeIterator<Item = SectionId> + DoubleEndedIterator + 'a, GitError> {
        let section_name = SectionName::from_str_unchecked(section_name);
        let section_ids = self
            .section_lookup_tree
            .get(&section_name)
            .ok_or(GitError::Gen)?;
        let mut maybe_ids = None;
        if let Some(subsection_name) = subsection_name {
            for node in section_ids {
                if let SectionBodyIdsLut::NonTerminal(subsection_lookup) = node {
                    maybe_ids = subsection_lookup
                        .get(subsection_name)
                        .map(|v| v.iter().copied());
                    break;
                }
            }
        } else {
            for node in section_ids {
                if let SectionBodyIdsLut::Terminal(subsection_lookup) = node {
                    maybe_ids = Some(subsection_lookup.iter().copied());
                    break;
                }
            }
        }
        maybe_ids.ok_or(GitError::Gen)
    }

    fn section_ids_by_name<'a>(
        &'a self,
        section_name: &'a str,
    ) -> Result<impl Iterator<Item = SectionId> + 'a, GitError> {
        let section_name = SectionName::from_str_unchecked(section_name);
        match self.section_lookup_tree.get(&section_name) {
            Some(lookup) => {
                let mut lut = Vec::with_capacity(self.section_order.len());
                for node in lookup {
                    match node {
                        SectionBodyIdsLut::Terminal(v) => lut.extend(v.iter().copied()),
                        SectionBodyIdsLut::NonTerminal(v) => {
                            lut.extend(v.values().flatten().copied())
                        }
                    }
                }

                Ok(self
                    .section_order
                    .iter()
                    .filter(move |a| lut.contains(a))
                    .copied())
            }
            None => Err(GitError::Gen),
        }
    }
}

fn find_insert_pos_by_order(
    sections_with_name: &[SectionId],
    before_order: usize,
    lookup_section_order: impl Fn(SectionId) -> usize,
) -> usize {
    let mut insert_pos = sections_with_name.len(); // push back by default
    for (idx, candidate_id) in sections_with_name.iter().enumerate() {
        let candidate_order = lookup_section_order(*candidate_id);
        match candidate_order.cmp(&before_order) {
            cmp::Ordering::Less => {}
            cmp::Ordering::Equal => {
                insert_pos = idx + 1; // insert right after this one
                break;
            }
            cmp::Ordering::Greater => {
                insert_pos = idx; // insert before this one
                break;
            }
        }
    }
    insert_pos
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Debug, Default)]
struct ConfigFileBody<'event>(Vec<Event<'event>>);

impl ConfigFileBody<'_> {
    #[must_use]
    fn value(&self, value_name: impl AsRef<str>) -> Option<Cow<'_, BStr>> {
        self.hx_value_implicit(value_name.as_ref()).flatten()
    }

    #[must_use]
    fn hx_value_implicit(&self, value_name: &str) -> Option<Option<Cow<'_, BStr>>> {
        let key = SectionValueName::from_str_unchecked(value_name);
        let (_key_range, range) = self.hx_key_and_value_range_by(&key)?;
        let range = match range {
            None => return Some(None),
            Some(range) => range,
        };
        let mut concatenated = BString::default();

        for event in &self.0[range] {
            match event {
                Event::Value(v) => {
                    return Some(Some(normalize_bstr(v.as_ref())));
                }
                Event::ValueNotDone(v) => {
                    concatenated.push_str(v.as_ref());
                }
                Event::ValueDone(v) => {
                    concatenated.push_str(v.as_ref());
                    return Some(Some(normalize_bstring(concatenated)));
                }
                _ => (),
            }
        }
        None
    }

    #[must_use]
    fn values(&self, value_name: &str) -> Vec<Cow<'_, BStr>> {
        let key = &SectionValueName::from_str_unchecked(value_name);
        let mut values = Vec::new();
        let mut expect_value = false;
        let mut concatenated_value = BString::default();

        for event in &self.0 {
            match event {
                Event::SectionValueName(event_key) if event_key == key => expect_value = true,
                Event::Value(v) if expect_value => {
                    expect_value = false;
                    values.push(normalize_bstr(v.as_ref()));
                }
                Event::ValueNotDone(v) if expect_value => {
                    concatenated_value.push_str(v.as_ref());
                }
                Event::ValueDone(v) if expect_value => {
                    expect_value = false;
                    concatenated_value.push_str(v.as_ref());
                    values.push(normalize_bstring(std::mem::take(&mut concatenated_value)));
                }
                _ => (),
            }
        }

        values
    }
}

impl ConfigFileBody<'_> {
    fn as_ref(&self) -> &[Event<'_>] {
        &self.0
    }

    fn hx_key_and_value_range_by(
        &self,
        value_name: &SectionValueName<'_>,
    ) -> Option<(Range<usize>, Option<Range<usize>>)> {
        let mut value_range = Range::default();
        let mut key_start = None;
        for (i, e) in self.0.iter().enumerate().rev() {
            match e {
                Event::SectionValueName(k) => {
                    if k == value_name {
                        key_start = Some(i);
                        break;
                    }
                    value_range = Range::default();
                }
                Event::Value(_) => {
                    (value_range.start, value_range.end) = (i, i);
                }
                Event::ValueNotDone(_) | Event::ValueDone(_) => {
                    if value_range.end == 0 {
                        value_range.end = i;
                    } else {
                        value_range.start = i;
                    }
                }
                _ => (),
            }
        }
        key_start.map(|key_start| {
            #[allow(clippy::range_plus_one)]
            let value_range = value_range.start..value_range.end + 1;
            let key_range = key_start..value_range.end;
            (
                key_range,
                (value_range.start != key_start + 1).then_some(value_range),
            )
        })
    }
}

struct BodyIter<'event>(std::vec::IntoIter<Event<'event>>);

impl<'event> IntoIterator for ConfigFileBody<'event> {
    type Item = (SectionValueName<'event>, Cow<'event, BStr>);

    type IntoIter = BodyIter<'event>;

    fn into_iter(self) -> Self::IntoIter {
        BodyIter(self.0.into_iter())
    }
}

impl<'event> Iterator for BodyIter<'event> {
    type Item = (SectionValueName<'event>, Cow<'event, BStr>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut key = None;
        let mut partial_value = BString::default();
        let mut value = None;

        for event in self.0.by_ref() {
            match event {
                Event::SectionValueName(k) => key = Some(k),
                Event::Value(v) => {
                    value = Some(v);
                    break;
                }
                Event::ValueNotDone(v) => partial_value.push_str(v.as_ref()),
                Event::ValueDone(v) => {
                    partial_value.push_str(v.as_ref());
                    value = Some(partial_value.into());
                    break;
                }
                _ => (),
            }
        }

        key.zip(value.map(normalize))
    }
}

impl FusedIterator for BodyIter<'_> {}

impl<'a> Deref for ConfigFileSection<'a> {
    type Target = ConfigFileBody<'a>;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

impl<'a> ConfigFileSection<'a> {
    fn new(
        name: impl Into<Cow<'a, str>>,
        subsection: impl Into<Option<Cow<'a, BStr>>>,
        meta: impl Into<Arc<Metadata>>,
    ) -> Result<Self, GitError> {
        Ok(ConfigFileSection {
            header: SectionHeader::new(name, subsection)?,
            body: Default::default(),
            meta: meta.into(),
            id: SectionId::default(),
        })
    }
}

impl<'a> ConfigFileSection<'a> {
    // gix
    #[must_use]
    fn header(&self) -> &SectionHeader<'a> {
        &self.header
    }

    #[must_use]
    fn meta(&self) -> &Metadata {
        &self.meta
    }

    fn to_mut(&mut self, newline: SmallVec<[u8; 2]>) -> SectionMut<'_, 'a> {
        SectionMut::new(self, newline)
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash)]
struct Metadata {
    path: Option<PathBuf>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct ConfigFileSection<'a> {
    header: SectionHeader<'a>,
    body: ConfigFileBody<'a>,
    meta: std::sync::Arc<Metadata>,
    id: SectionId,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, PartialOrd, Ord, Debug)]
struct SectionId(usize);

impl Default for SectionId {
    fn default() -> Self {
        SectionId(usize::MAX)
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
enum SectionBodyIdsLut<'a> {
    Terminal(Vec<SectionId>),
    NonTerminal(HashMap<Cow<'a, BStr>, Vec<SectionId>>),
}

fn ends_with_newline(e: &[Event<'_>], nl: impl AsRef<[u8]>, default: bool) -> bool {
    if e.is_empty() {
        return default;
    }
    e.iter()
        .rev()
        .take_while(|e| e.to_bstr_lossy().iter().all(u8::is_ascii_whitespace))
        .find_map(|e| e.to_bstr_lossy().contains_str(nl.as_ref()).then_some(true))
        .unwrap_or(false)
}

fn extract_newline<'a>(e: &'a Event<'_>) -> Option<&'a BStr> {
    Some(match e {
        Event::Newline(b) => {
            let nl = b.as_ref();

            if nl.contains(&b'\r') {
                "\r\n".into()
            } else {
                "\n".into()
            }
        }
        _ => return None,
    })
}

fn platform_newline() -> &'static BStr {
    "\n".into()
}

type IndexId = usize;
type StateId = u32;
type Generation = u32;
type AtomicGeneration = AtomicU32;

struct Snapshot {
    indices: Vec<IndexLookup>,
    loose_dbs: std::sync::Arc<Vec<LooseStore>>,
    marker: SlotIndexMarker,
}

struct Store {
    write: parking_lot::Mutex<()>,
    path: PathBuf,
    current_dir: PathBuf,
    index: ArcSwap<SlotMapIndex>,
    files: Vec<MutableIndexAndPack>,
    num_handles_stable: AtomicUsize,
    num_handles_unstable: AtomicUsize,
}

impl Store {
    // helix transitively
    fn at_opts(objects_dir: PathBuf, current_dir: PathBuf) -> std::io::Result<Self> {
        const MAX_SLOTS: usize = (1 << 15) - 1;
        if !objects_dir.is_dir() {
            return Err(IoError::other(format!(
                "'{}' wasn't a directory",
                objects_dir.display()
            )));
        }
        let mut db_paths = resolve(objects_dir.clone(), &current_dir).map_err(IoError::other)?;
        db_paths.insert(0, objects_dir.clone());
        let num_slots = Store::collect_indices_and_mtime_sorted_by_size(db_paths)
            .map_err(IoError::other)?
            .len();

        if num_slots > MAX_SLOTS {
            return Err(IoError::other(format!(
                "Cannot use more than 2^15-1 slots, got {num_slots}"
            )));
        }

        let candidate = ((num_slots as f32 * 1.1_f32) as usize).max(32_usize);

        let slot_count = if candidate > MAX_SLOTS {
            num_slots
        } else {
            candidate
        };

        Ok(Store {
            current_dir,
            write: Default::default(),
            path: objects_dir,
            files: Vec::from_iter(
                std::iter::repeat_with(MutableIndexAndPack::default).take(slot_count),
            ),
            index: ArcSwap::new(Arc::new(SlotMapIndex::default())),
            num_handles_stable: AtomicUsize::new(Default::default()),
            num_handles_unstable: AtomicUsize::new(Default::default()),
        })
    }

    fn load_pack(
        &self,
        id: PackId,
        marker: SlotIndexMarker,
    ) -> std::io::Result<Option<Arc<DataFile>>> {
        let index = self.index.load();
        if index.generation != marker.generation {
            return Ok(None);
        }
        fn load_pack(path: &Path, id: PackId) -> std::io::Result<Arc<DataFile>> {
            DataFile::at(path)
                .map(|mut pack| {
                    pack.id = id.to_intrinsic_pack_id();
                    Arc::new(pack)
                })
                .map_err(|_| std::io::Error::other("failed to open pack data file"))
        }

        let slot = &self.files[id.index];
        let slot_files = &**slot.files.load();
        if slot.generation.load(Ordering::SeqCst) > marker.generation {
            return Ok(None);
        }
        if let Some(_pack_index) = id.multipack_index {
            Ok(None)
        } else {
            match slot_files {
                Some(IndexAndPacks::Index(bundle)) => {
                    if let Some(pack) = bundle.data.loaded() {
                        Ok(Some(pack.clone()))
                    } else {
                        let _lock = slot.write.lock();
                        let mut files = slot.files.load_full();
                        let files_mut = Arc::make_mut(&mut files);
                        let pack = match files_mut {
                            Some(IndexAndPacks::Index(bundle)) => {
                                bundle.data.load_with_recovery(|path| load_pack(path, id))?
                            }
                            None => {
                                unreachable!(
                                    "BUG: must set this handle to be stable to avoid slots to be cleared/changed"
                                )
                            }
                        };
                        slot.files.store(files);
                        Ok(pack)
                    }
                }
                None => {
                    unreachable!(
                        "BUG: must set this handle to be stable to avoid slots to be cleared/changed"
                    )
                }
            }
        }
    }

    fn load_one_index(&self, marker: SlotIndexMarker) -> Result<Option<Snapshot>, GitError> {
        let index = self.index.load();
        if !index.is_initialized() {
            return self.consolidate_with_disk_state(
                true,  /* needs_init */
                false, /*load one new index*/
            );
        }

        if marker.generation != index.generation
            || marker.state_id != index.state_id()
            || self.load_next_index(index)
        {
            Ok(Some(self.collect_snapshot()))
        } else {
            self.consolidate_with_disk_state(
                false, /* needs init */
                true,  /*load one new index*/
            )
        }
    }

    fn load_next_index(&self, mut index: arc_swap::Guard<Arc<SlotMapIndex>>) -> bool {
        'retry_with_changed_index: loop {
            let previous_state_id = index.state_id();
            'retry_with_next_slot_index: loop {
                match index.next_index_to_load.fetch_update(
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                    |current| (current != index.slot_indices.len()).then_some(current + 1),
                ) {
                    Ok(slot_map_index) => {
                        let _ongoing_operation =
                            IncOnNewAndDecOnDrop::new(&index.num_indices_currently_being_loaded);
                        let slot = &self.files[index.slot_indices[slot_map_index]];
                        let _lock = slot.write.lock();
                        if slot.generation.load(Ordering::SeqCst) > index.generation {
                            continue 'retry_with_next_slot_index;
                        }
                        let mut bundle = slot.files.load_full();
                        let bundle_mut = Arc::make_mut(&mut bundle);
                        if let Some(files) = bundle_mut.as_mut() {
                            let res = {
                                let res = files.load_index();
                                slot.files.store(bundle);
                                index.loaded_indices.fetch_add(1, Ordering::SeqCst);
                                res
                            };
                            match res {
                                Ok(()) => {
                                    break 'retry_with_next_slot_index;
                                }
                                Err(_err) => {
                                    continue 'retry_with_next_slot_index;
                                }
                            }
                        }
                    }
                    Err(_nothing_more_to_load) => {
                        std::thread::yield_now();
                        while index
                            .num_indices_currently_being_loaded
                            .load(Ordering::SeqCst)
                            != 0
                        {
                            std::thread::yield_now();
                        }
                        break 'retry_with_next_slot_index;
                    }
                }
            }
            if previous_state_id == index.state_id() {
                let potentially_new_index = self.index.load();
                if std::ptr::eq(Arc::as_ptr(&potentially_new_index), Arc::as_ptr(&index)) {
                    return false;
                }
                index = potentially_new_index;
                continue 'retry_with_changed_index;
            }
            return true;
        }
    }

    fn consolidate_with_disk_state(
        &self,
        needs_init: bool,
        load_new_index: bool,
    ) -> Result<Option<Snapshot>, GitError> {
        let index = self.index.load();
        let previous_index_state = Arc::as_ptr(&index) as usize;

        let write = self.write.lock();
        let objects_directory = &self.path;

        let index = self.index.load();
        if previous_index_state != Arc::as_ptr(&index) as usize {
            return Ok(Some(self.collect_snapshot()));
        }

        let was_uninitialized = !index.is_initialized();

        if !was_uninitialized && needs_init {
            return Ok(Some(self.collect_snapshot()));
        }

        let db_paths: Vec<_> = std::iter::once(objects_directory.to_owned())
            .chain(resolve(objects_directory.clone(), &self.current_dir)?)
            .collect();

        let loose_dbs = if was_uninitialized
            || db_paths.len() != index.loose_dbs.len()
            || db_paths
                .iter()
                .zip(index.loose_dbs.iter().map(|ldb| &ldb.path))
                .any(|(lhs, rhs)| lhs != rhs)
        {
            Arc::new(db_paths.iter().map(LooseStore::at).collect::<Vec<_>>())
        } else {
            Arc::clone(&index.loose_dbs)
        };

        let indices_by_modification_time =
            Self::collect_indices_and_mtime_sorted_by_size(db_paths)?;
        let mut idx_by_index_path: BTreeMap<_, _> = index
            .slot_indices
            .iter()
            .filter_map(|&idx| {
                let f = &self.files[idx];
                Option::as_ref(&f.files.load()).map(|f| (f.index_path().to_owned(), idx))
            })
            .collect();

        let mut new_slot_map_indices = Vec::new(); // these indices into the slot map still exist there/didn't change
        let mut index_paths_to_add = if was_uninitialized {
            VecDeque::with_capacity(indices_by_modification_time.len())
        } else {
            Default::default()
        };

        let mut num_loaded_indices = 0;
        for (index_info, mtime) in indices_by_modification_time
            .into_iter()
            .map(|(a, b, _)| (a, b))
        {
            match idx_by_index_path.remove(index_info.path()) {
                Some(slot_idx) => {
                    let slot = &self.files[slot_idx];
                    if Self::assure_slot_matches_index(&write, slot, index_info, index.generation) {
                        num_loaded_indices += 1;
                    }
                    new_slot_map_indices.push(slot_idx);
                }
                None => index_paths_to_add.push_back((index_info, mtime, None)),
            }
        }
        let needs_stable_indices = self.maintain_stable_indices(&write);

        let mut next_possibly_free_index = index
            .slot_indices
            .iter()
            .max()
            .map_or(0, |idx| (idx + 1) % self.files.len());
        let mut num_indices_checked = 0;
        let mut needs_generation_change = false;
        let mut slot_indices_to_remove: Vec<_> = idx_by_index_path.into_values().collect();
        while let Some((mut index_info, _mtime, move_from_slot_idx)) =
            index_paths_to_add.pop_front()
        {
            'increment_slot_index: loop {
                if num_indices_checked == self.files.len() {
                    return Err(GitError::Gen);
                }
                if new_slot_map_indices.contains(&next_possibly_free_index) {
                    next_possibly_free_index = (next_possibly_free_index + 1) % self.files.len();
                    num_indices_checked += 1;
                    continue 'increment_slot_index;
                }
                let slot_index = next_possibly_free_index;
                let slot = &self.files[slot_index];
                next_possibly_free_index = (next_possibly_free_index + 1) % self.files.len();
                num_indices_checked += 1;
                match move_from_slot_idx {
                    Some(move_from_slot_idx) => {
                        if slot_index == move_from_slot_idx {
                            continue 'increment_slot_index;
                        }
                        match Self::try_set_index_slot(
                            &write,
                            slot,
                            index_info,
                            index.generation,
                            needs_stable_indices,
                        ) {
                            Ok(dest_was_empty) => {
                                slot_indices_to_remove.push(move_from_slot_idx);
                                new_slot_map_indices.push(slot_index);
                                if !dest_was_empty {
                                    needs_generation_change = true;
                                }
                                break 'increment_slot_index;
                            }
                            Err(unused_index_info) => index_info = unused_index_info,
                        }
                    }
                    None => match Self::try_set_index_slot(
                        &write,
                        slot,
                        index_info,
                        index.generation,
                        needs_stable_indices,
                    ) {
                        Ok(dest_was_empty) => {
                            new_slot_map_indices.push(slot_index);
                            if !dest_was_empty {
                                needs_generation_change = true;
                            }
                            break 'increment_slot_index;
                        }
                        Err(unused_index_info) => index_info = unused_index_info,
                    },
                }
            }
        }

        let generation = if needs_generation_change {
            index.generation.checked_add(1).ok_or(GitError::Gen)?
        } else {
            index.generation
        };
        let index_unchanged = index.slot_indices == new_slot_map_indices;
        if generation != index.generation {
            assert!(
                !index_unchanged,
                "if the generation changed, the slot index must have changed for sure"
            );
        }
        if !index_unchanged || loose_dbs != index.loose_dbs {
            let new_index = Arc::new(SlotMapIndex {
                slot_indices: new_slot_map_indices,
                loose_dbs,
                generation,
                next_index_to_load: if index_unchanged {
                    Arc::clone(&index.next_index_to_load)
                } else {
                    Default::default()
                },
                loaded_indices: if index_unchanged {
                    Arc::clone(&index.loaded_indices)
                } else {
                    Arc::new(num_loaded_indices.into())
                },
                num_indices_currently_being_loaded: Default::default(),
            });
            self.index.store(new_index);
        }

        for slot in slot_indices_to_remove
            .into_iter()
            .map(|idx| &self.files[idx])
        {
            let _lock = slot.write.lock();
            let mut files = slot.files.load_full();
            let files_mut = Arc::make_mut(&mut files);
            if needs_stable_indices {
                if let Some(files) = files_mut.as_mut() {
                    files.trash();
                }
            } else {
                slot.generation.store(generation, Ordering::SeqCst);
                *files_mut = None;
            }
            slot.files.store(files);
        }

        let new_index = self.index.load();
        Ok(if index.state_id() == new_index.state_id() {
            None
        } else {
            if load_new_index {
                self.load_next_index(new_index);
            }
            Some(self.collect_snapshot())
        })
    }

    fn collect_indices_and_mtime_sorted_by_size(
        db_paths: Vec<PathBuf>,
    ) -> Result<Vec<(Either, SystemTime, u64)>, GitError> {
        let mut indices_by_modification_time = Vec::with_capacity(0);
        for db_path in db_paths {
            let packs = db_path.join("pack");
            let entries = match std::fs::read_dir(packs) {
                Ok(e) => e,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => return Err(GitError::Gen),
            };
            let indices = entries
                .filter_map(Result::ok)
                .filter_map(|e| e.metadata().map(|md| (e.path(), md)).ok())
                .filter(|(_, md)| md.file_type().is_file())
                .filter(|(p, _)| {
                    p.extension() == Some(OsStr::new("idx")) && p.with_extension("pack").is_file()
                })
                .map(|(p, md)| {
                    md.modified()
                        .map_err(|_| GitError::Gen)
                        .map(|mtime| (p, mtime, md.len()))
                })
                .collect::<Result<Vec<_>, _>>()?;

            indices_by_modification_time.extend(indices.into_iter().filter_map(|(p, a, b)| {
                (p.file_name() != Some(OsStr::new("multi-pack-index"))).then_some((
                    Either::IndexPath(p),
                    a,
                    b,
                ))
            }));
        }
        indices_by_modification_time.sort_by(|l, r| l.2.cmp(&r.2).reverse());
        Ok(indices_by_modification_time)
    }

    #[allow(clippy::too_many_arguments)]
    fn try_set_index_slot(
        lock: &parking_lot::MutexGuard<'_, ()>,
        dest_slot: &MutableIndexAndPack,
        index_info: Either,
        current_generation: Generation,
        needs_stable_indices: bool,
    ) -> Result<bool, Either> {
        let (dest_slot_was_empty, generation) = match &**dest_slot.files.load() {
            Some(bundle) => {
                if bundle.index_path() == index_info.path()
                    || (bundle.is_disposable() && needs_stable_indices)
                {
                    return Err(index_info);
                }
                (false, current_generation + 1)
            }
            None => (true, current_generation),
        };
        Self::set_slot_to_index(lock, dest_slot, index_info, generation);
        Ok(dest_slot_was_empty)
    }

    fn set_slot_to_index(
        _lock: &parking_lot::MutexGuard<'_, ()>,
        slot: &MutableIndexAndPack,
        index_info: Either,
        generation: Generation,
    ) {
        let _lock = slot.write.lock();
        let mut files = slot.files.load_full();
        let files_mut = Arc::make_mut(&mut files);
        slot.generation.store(generation, Ordering::SeqCst);
        *files_mut = Some(index_info.into_index_and_packs());
        slot.files.store(files);
    }

    fn assure_slot_matches_index(
        _lock: &parking_lot::MutexGuard<'_, ()>,
        slot: &MutableIndexAndPack,
        index_info: Either,
        current_generation: Generation,
    ) -> bool {
        match Option::as_ref(&slot.files.load()) {
            Some(bundle) => {
                assert_eq!(
                    bundle.index_path(),
                    index_info.path(),
                    "Parallel writers cannot change the file the slot points to."
                );
                if bundle.is_disposable() {
                    let _lock = slot.write.lock();
                    let mut files = slot.files.load_full();
                    let files_mut = Arc::make_mut(&mut files)
                        .as_mut()
                        .expect("BUG: cannot change from something to nothing, would be race");
                    files_mut.put_back();

                    slot.generation.store(current_generation, Ordering::SeqCst);
                    slot.files.store(files);
                }
                bundle.index_is_loaded()
            }
            None => {
                unreachable!(
                    "BUG: a slot can never be deleted if we have it recorded in the index WHILE changing said index. There shouldn't be a race"
                )
            }
        }
    }

    fn maintain_stable_indices(&self, _guard: &parking_lot::MutexGuard<'_, ()>) -> bool {
        self.num_handles_stable.load(Ordering::SeqCst) > 0
    }

    fn collect_snapshot(&self) -> Snapshot {
        let index = self.index.load();
        loop {
            if index
                .num_indices_currently_being_loaded
                .deref()
                .load(Ordering::SeqCst)
                != 0
            {
                std::thread::yield_now();
                continue;
            }
            let marker = index.marker();
            let indices = if index.is_initialized() {
                index
                    .slot_indices
                    .iter()
                    .map(|idx| (*idx, &self.files[*idx]))
                    .filter_map(|(id, file)| {
                        let lookup = match (**file.files.load()).as_ref()? {
                            IndexAndPacks::Index(bundle) => SingleOrMultiIndex::Single {
                                index: bundle.index.loaded()?.clone(),
                                data: bundle.data.loaded().cloned(),
                            },
                        };
                        IndexLookup { file: lookup, id }.into()
                    })
                    .collect()
            } else {
                Vec::new()
            };

            return Snapshot {
                indices,
                loose_dbs: Arc::clone(&index.loose_dbs),
                marker,
            };
        }
    }

    fn register_handle(&self) -> HandleMode {
        self.num_handles_unstable.fetch_add(1, Ordering::Relaxed);
        HandleMode::DeletedPacksAreInaccessible
    }
    fn remove_handle(&self, mode: HandleMode) {
        match mode {
            HandleMode::KeepDeletedPacksAvailable => {
                let _lock = self.write.lock();
                self.num_handles_stable.fetch_sub(1, Ordering::SeqCst)
            }
            HandleMode::DeletedPacksAreInaccessible => {
                self.num_handles_unstable.fetch_sub(1, Ordering::Relaxed)
            }
        };
    }
    fn upgrade_handle(&self, mode: HandleMode) -> HandleMode {
        if let HandleMode::DeletedPacksAreInaccessible = mode {
            let _lock = self.write.lock();
            self.num_handles_stable.fetch_add(1, Ordering::SeqCst);
            self.num_handles_unstable.fetch_sub(1, Ordering::SeqCst);
        }
        HandleMode::KeepDeletedPacksAvailable
    }

    fn to_handle(self: &Arc<Self>) -> OdbHandle<Arc<Store>> {
        let token = self.register_handle();
        OdbHandle {
            store: self.clone(),

            token: Some(token),
            inflate: RefCell::new(Default::default()),
            snapshot: RefCell::new(self.collect_snapshot()),
            max_recursion_depth: 32,
            packed_object_count: Default::default(),
        }
    }
}

struct OdbCache<S> {
    inner: S,
    new_pack_cache: Option<Arc<NewPackCacheFn>>,
    pack_cache: Option<RefCell<Box<PackCache>>>,
}

#[derive(Default)]
struct SlotMapIndex {
    slot_indices: Vec<usize>,
    loose_dbs: std::sync::Arc<Vec<LooseStore>>,

    generation: Generation,
    next_index_to_load: std::sync::Arc<AtomicUsize>,
    loaded_indices: std::sync::Arc<AtomicUsize>,
    num_indices_currently_being_loaded: std::sync::Arc<AtomicU16>,
}

impl SlotMapIndex {
    fn state_id(self: &Arc<SlotMapIndex>) -> StateId {
        let hash = hash::crc32(&(Arc::as_ptr(self) as usize).to_be_bytes());
        hash::crc32_update(
            hash,
            &self.loaded_indices.load(Ordering::SeqCst).to_be_bytes(),
        )
    }

    fn marker(self: &Arc<SlotMapIndex>) -> SlotIndexMarker {
        SlotIndexMarker {
            generation: self.generation,
            state_id: self.state_id(),
        }
    }

    fn is_initialized(&self) -> bool {
        !self.loose_dbs.is_empty()
    }
}

struct IncOnNewAndDecOnDrop<'a>(&'a AtomicU16);

impl<'a> IncOnNewAndDecOnDrop<'a> {
    fn new(v: &'a AtomicU16) -> Self {
        v.fetch_add(1, Ordering::SeqCst);
        Self(v)
    }
}

impl Drop for IncOnNewAndDecOnDrop<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

enum Either {
    IndexPath(PathBuf),
}

impl Either {
    fn path(&self) -> &Path {
        match self {
            Either::IndexPath(p) => p,
        }
    }

    fn into_index_and_packs(self) -> IndexAndPacks {
        match self {
            Either::IndexPath(path) => IndexAndPacks::new_single(path),
        }
    }
}

impl Eq for Either {}

impl PartialEq<Self> for Either {
    fn eq(&self, other: &Self) -> bool {
        self.path().eq(other.path())
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd<Self> for Either {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.path().cmp(other.path()))
    }
}

impl Ord for Either {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path().cmp(other.path())
    }
}

enum SingleOrMultiIndex {
    Single {
        index: std::sync::Arc<IndexFile>,
        data: Option<Arc<DataFile>>,
    },
}

enum IntraPackLookup<'a> {
    Single(&'a IndexFile),
}

impl IntraPackLookup<'_> {
    fn pack_offset_by_id(&self, id: &oid) -> Option<Offset> {
        match self {
            IntraPackLookup::Single(index) => index
                .lookup(id)
                .map(|entry_index| index.pack_offset_at_index(entry_index)),
        }
    }
}
fn base_resolver<'a>(
    pack: &'a DataFile,
    index_file: &'a IntraPackLookup<'a>,
    external: Option<(ObjectId, &'a [u8], ObjectKind)>,
) -> impl Fn(&oid, &mut Vec<u8>) -> Option<DataDecodeEntryResolvedBase> + 'a {
    move |id, out| {
        index_file
            .pack_offset_by_id(id)
            .and_then(|pack_offset| {
                pack.entry(pack_offset)
                    .ok()
                    .map(DataDecodeEntryResolvedBase::InPack)
            })
            .or_else(|| {
                let (base_id, data, kind) = external?;
                (id == base_id).then(|| {
                    out.clear();
                    out.extend_from_slice(data);
                    DataDecodeEntryResolvedBase::OutOfPack {
                        kind,
                        end: out.len(),
                    }
                })
            })
    }
}

struct IndexLookup {
    file: SingleOrMultiIndex,
    id: IndexId,
}

struct IndexForObjectInPack {
    pack_id: PackId,
    pack_offset: u64,
}

struct IndexLookupOutcome<'a> {
    object_index: IndexForObjectInPack,
    index_file: IntraPackLookup<'a>,
    pack: &'a mut Option<Arc<DataFile>>,
}

impl IndexLookup {
    fn lookup(&mut self, object_id: &oid) -> Option<IndexLookupOutcome<'_>> {
        let id = self.id;
        match &mut self.file {
            SingleOrMultiIndex::Single { index, data } => {
                index.lookup(object_id).map(move |idx| IndexLookupOutcome {
                    object_index: IndexForObjectInPack {
                        pack_id: PackId {
                            index: id,
                            multipack_index: None,
                        },
                        pack_offset: index.pack_offset_at_index(idx),
                    },
                    index_file: IntraPackLookup::Single(index),
                    pack: data,
                })
            }
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
struct SlotIndexMarker {
    generation: Generation,
    state_id: StateId,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct PackId {
    index: IndexId,
    multipack_index: Option<PackIndex>,
}

impl PackId {
    fn to_intrinsic_pack_id(self) -> FilePackId {
        assert!(
            self.index < (1 << 15),
            "There shouldn't be more than 2^15 indices"
        );
        match self.multipack_index {
            None => self.index as FilePackId,
            Some(midx) => (self.index as FilePackId | (1 << 15)) | (midx << 16) as FilePackId,
        }
    }
}

#[derive(Clone)]
struct OnDiskFile<T: Clone> {
    path: std::sync::Arc<PathBuf>,
    state: OnDiskFileState<T>,
}

#[derive(Clone)]
enum OnDiskFileState<T: Clone> {
    Unloaded,
    Loaded(T),
    Garbage(T),
    Missing,
}

impl<T: Clone> OnDiskFile<T> {
    fn is_loaded(&self) -> bool {
        matches!(
            self.state,
            OnDiskFileState::Loaded(_) | OnDiskFileState::Garbage(_)
        )
    }

    fn is_disposable(&self) -> bool {
        matches!(
            self.state,
            OnDiskFileState::Garbage(_) | OnDiskFileState::Missing
        )
    }

    fn load_strict(
        &mut self,
        load: impl FnOnce(&Path) -> std::io::Result<T>,
    ) -> std::io::Result<()> {
        match self.state {
            OnDiskFileState::Unloaded | OnDiskFileState::Missing => match load(&self.path) {
                Ok(v) => {
                    self.state = OnDiskFileState::Loaded(v);
                    Ok(())
                }
                Err(err) => {
                    self.state = OnDiskFileState::Missing;
                    Err(err)
                }
            },
            OnDiskFileState::Loaded(_) | OnDiskFileState::Garbage(_) => Ok(()),
        }
    }
    fn load_with_recovery(
        &mut self,
        load: impl FnOnce(&Path) -> std::io::Result<T>,
    ) -> std::io::Result<Option<T>> {
        match &mut self.state {
            OnDiskFileState::Loaded(v) | OnDiskFileState::Garbage(v) => Ok(Some(v.clone())),
            OnDiskFileState::Missing => Ok(None),
            OnDiskFileState::Unloaded => match load(&self.path) {
                Ok(v) => {
                    self.state = OnDiskFileState::Loaded(v.clone());
                    Ok(Some(v))
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    self.state = OnDiskFileState::Missing;
                    Ok(None)
                }
                Err(err) => Err(err),
            },
        }
    }

    fn loaded(&self) -> Option<&T> {
        match &self.state {
            OnDiskFileState::Loaded(v) | OnDiskFileState::Garbage(v) => Some(v),
            OnDiskFileState::Unloaded | OnDiskFileState::Missing => None,
        }
    }

    fn put_back(&mut self) {
        match std::mem::replace(&mut self.state, OnDiskFileState::Missing) {
            OnDiskFileState::Garbage(v) => self.state = OnDiskFileState::Loaded(v),
            OnDiskFileState::Missing => self.state = OnDiskFileState::Unloaded,
            other @ (OnDiskFileState::Loaded(_) | OnDiskFileState::Unloaded) => self.state = other,
        }
    }

    fn trash(&mut self) {
        match std::mem::replace(&mut self.state, OnDiskFileState::Missing) {
            OnDiskFileState::Loaded(v) => self.state = OnDiskFileState::Garbage(v),
            other @ (OnDiskFileState::Garbage(_)
            | OnDiskFileState::Unloaded
            | OnDiskFileState::Missing) => {
                self.state = other;
            }
        }
    }
}

#[derive(Clone)]
struct IndexFileBundle {
    index: OnDiskFile<Arc<IndexFile>>,
    data: OnDiskFile<Arc<DataFile>>,
}

#[derive(Clone)]
enum IndexAndPacks {
    Index(IndexFileBundle),
}

impl IndexAndPacks {
    fn index_path(&self) -> &Path {
        match self {
            IndexAndPacks::Index(index) => &index.index.path,
        }
    }

    fn put_back(&mut self) {
        match self {
            IndexAndPacks::Index(bundle) => {
                bundle.index.put_back();
                bundle.data.put_back();
            }
        }
    }

    fn trash(&mut self) {
        match self {
            IndexAndPacks::Index(bundle) => {
                bundle.index.trash();
                bundle.data.trash();
            }
        }
    }

    fn index_is_loaded(&self) -> bool {
        match self {
            Self::Index(bundle) => bundle.index.is_loaded(),
        }
    }

    fn is_disposable(&self) -> bool {
        match self {
            Self::Index(bundle) => bundle.index.is_disposable() || bundle.data.is_disposable(),
        }
    }

    fn load_index(&mut self) -> std::io::Result<()> {
        match self {
            IndexAndPacks::Index(bundle) => bundle.index.load_strict(|path| {
                IndexFile::at(path)
                    .map(Arc::new)
                    .map_err(std::io::Error::other)
            }),
        }
    }

    fn new_single(index_path: PathBuf) -> Self {
        let data_path = index_path.with_extension("pack");
        Self::Index(IndexFileBundle {
            index: OnDiskFile {
                path: index_path.into(),
                state: OnDiskFileState::Unloaded,
            },
            data: OnDiskFile {
                path: data_path.into(),
                state: OnDiskFileState::Unloaded,
            },
        })
    }
}

#[derive(Default)]
struct MutableIndexAndPack {
    files: ArcSwap<Option<IndexAndPacks>>,
    write: parking_lot::Mutex<()>,
    generation: AtomicGeneration,
}

const HEADER_MAX_SIZE: usize = 64;

#[derive(Clone, PartialEq, Eq)]
struct LooseStore {
    path: PathBuf,
}

impl LooseStore {
    fn at(objects_directory: impl Into<PathBuf>) -> LooseStore {
        LooseStore {
            path: objects_directory.into(),
        }
    }
}

fn hash_path(id: &oid, mut root: PathBuf) -> PathBuf {
    let mut hex = [0u8; 40];
    let hex = id.hex_to_buf(hex.as_mut());
    root.push(&hex[..2]);
    root.push(&hex[2..]);
    root
}

impl LooseStore {
    fn contains(&self, id: &oid) -> bool {
        hash_path(id, self.path.clone()).is_file()
    }

    fn try_find<'a>(
        &self,
        id: &oid,
        out: &'a mut Vec<u8>,
    ) -> Result<Option<ObjectData<'a>>, GitError> {
        self.find_inner(id, out)
    }

    fn find_inner<'a>(
        &self,
        id: &oid,
        out: &'a mut Vec<u8>,
    ) -> Result<Option<ObjectData<'a>>, GitError> {
        let path = hash_path(id, self.path.clone());
        let Some(map) = self.map_loose_object(&path)? else {
            return Ok(None);
        };
        let mut header = [0_u8; HEADER_MAX_SIZE];

        let mut inflate = zlib::Inflate::default();
        let (status, consumed_in, consumed_out) =
            inflate.once(&map, &mut header).map_err(|_| GitError::Gen)?;
        if status == zlib::Status::BufError {
            return Err(GitError::Gen);
        }

        let (kind, size, header_size) = decode_loose_header(&header[..consumed_out])?;
        let size_usize = usize::try_from(size).map_err(|_| GitError::Gen)?;
        let decompressed_body_prefix_len =
            consumed_out.checked_sub(header_size).ok_or(GitError::Gen)?;

        if decompressed_body_prefix_len > size_usize {
            return Err(GitError::Gen);
        }

        out.clear();
        if status == zlib::Status::StreamEnd {
            if consumed_out as u64 != size + header_size as u64 {
                return Err(GitError::Gen);
            }
            out.extend_from_slice(&header[header_size..consumed_out]);
        } else {
            out.resize(size_usize, 0);
            out[..decompressed_body_prefix_len].copy_from_slice(&header[header_size..consumed_out]);

            let mut input = &map[consumed_in..];
            let num_decompressed_bytes = zlib::stream::inflate::read(
                &mut input,
                &mut inflate.state,
                &mut out[decompressed_body_prefix_len..],
            )
            .map_err(|_| GitError::Gen)?;

            if num_decompressed_bytes as u64 + decompressed_body_prefix_len as u64 != size {
                return Err(GitError::Gen);
            }
        }
        Ok(Some(ObjectData { kind, data: out }))
    }

    fn map_loose_object(&self, path: &std::path::Path) -> Result<Option<memmap2::Mmap>, GitError> {
        let map = match mmap_read_only(path) {
            Ok(map) => map,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => {
                return Err(GitError::Gen);
            }
        };

        if map.is_empty() {
            return Err(GitError::Gen);
        }
        Ok(Some(map))
    }
}

fn resolve(
    objects_directory: PathBuf,
    current_dir: &std::path::Path,
) -> Result<Vec<PathBuf>, GitError> {
    let mut dirs = vec![(0, objects_directory.clone())];
    let mut out = Vec::new();
    let mut seen = vec![hx_realpath_opts(&objects_directory, current_dir)?];
    while let Some((depth, dir)) = dirs.pop() {
        match std::fs::read(dir.join("info").join("alternates")) {
            Ok(input) => {
                for path in content(&input)? {
                    let path = objects_directory.join(path);
                    let path_canonicalized = hx_realpath_opts(&path, current_dir)?;
                    if seen.contains(&path_canonicalized) {
                        return Err(GitError::Gen);
                    }
                    seen.push(path_canonicalized);
                    dirs.push((depth + 1, path));
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err(GitError::Gen),
        }
        if depth != 0 {
            out.push(dir);
        }
    }
    Ok(out)
}

fn content(input: &[u8]) -> Result<Vec<PathBuf>, GitError> {
    let mut out = Vec::new();
    for line in input.split(|b| *b == b'\n') {
        let line = line.as_bstr();
        if line.is_empty() || line.starts_with(b"#") {
            continue;
        }
        out.push(
            try_from_bstr(if line.starts_with(b"\"") {
                undo(line)?.0
            } else {
                Cow::Borrowed(line)
            })
            .map_err(|_| GitError::Gen)?
            .into_owned(),
        );
    }
    Ok(out)
}

type PackCache = dyn TraitDecodeEntry + Send + 'static;
type NewPackCacheFn = dyn Fn() -> Box<PackCache> + Send + Sync + 'static;

impl<S> From<S> for OdbCache<S>
where
    S: TraitPackFind,
{
    fn from(store: S) -> Self {
        Self {
            inner: store,
            pack_cache: None,
            new_pack_cache: None,
        }
    }
}

impl<S: Clone> Clone for OdbCache<S> {
    fn clone(&self) -> Self {
        OdbCache {
            inner: self.inner.clone(),
            new_pack_cache: self.new_pack_cache.clone(),
            pack_cache: self
                .new_pack_cache
                .as_ref()
                .map(|create| RefCell::new(create())),
        }
    }
}

impl<S> Deref for OdbCache<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<S> DerefMut for OdbCache<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<S> TraitPackFind for OdbCache<S>
where
    S: TraitPackFind,
{
    fn try_find<'a>(
        &self,
        id: &oid,
        buffer: &'a mut Vec<u8>,
    ) -> Result<Option<ObjectData<'a>>, GitError> {
        match self.pack_cache.as_ref().map(RefCell::borrow_mut) {
            Some(mut pack_cache) => self.try_find_cached(id, buffer, &mut *pack_cache),
            None => self.try_find_cached(id, buffer, &mut Never),
        }
    }
    fn try_find_cached<'a>(
        &self,
        id: &oid,
        buffer: &'a mut Vec<u8>,
        pack_cache: &mut dyn TraitDecodeEntry,
    ) -> Result<Option<ObjectData<'a>>, GitError> {
        self.inner.try_find_cached(id.as_ref(), buffer, pack_cache)
    }
}

impl<S> Find for OdbCache<S>
where
    S: TraitPackFind,
{
    fn try_find<'a>(
        &self,
        id: &oid,
        buffer: &'a mut Vec<u8>,
    ) -> Result<Option<ObjectData<'a>>, GitError> {
        TraitPackFind::try_find(self, id, buffer)
    }
}

// gix
struct OdbHandle<S>
where
    S: Deref<Target = Store> + Clone,
{
    store: S,
    max_recursion_depth: usize,

    token: Option<HandleMode>,
    snapshot: RefCell<Snapshot>,
    inflate: RefCell<zlib::Inflate>,
    packed_object_count: RefCell<Option<u64>>,
}

impl<S> OdbHandle<S>
where
    S: Deref<Target = Store> + Clone,
{
    fn try_find_cached_inner<'a, 'b>(
        &'b self,
        id: &'b oid,
        buffer: &'a mut Vec<u8>,
        inflate: &mut zlib::Inflate,
        pack_cache: &mut dyn TraitDecodeEntry,
        snapshot: &mut Snapshot,
        recursion: usize,
    ) -> Result<Option<ObjectData<'a>>, GitError> {
        if recursion >= self.max_recursion_depth {
            return Err(GitError::Gen);
        }

        'outer: loop {
            {
                let marker = snapshot.marker;
                for (idx, index) in snapshot.indices.iter_mut().enumerate() {
                    if let Some(IndexLookupOutcome {
                        object_index:
                            IndexForObjectInPack {
                                pack_id,
                                pack_offset,
                            },
                        index_file,
                        pack: possibly_pack,
                    }) = index.lookup(id)
                    {
                        let pack = match possibly_pack {
                            Some(pack) => pack,
                            None => match self
                                .store
                                .load_pack(pack_id, marker)
                                .map_err(|_| GitError::Gen)?
                            {
                                Some(pack) => {
                                    *possibly_pack = Some(pack);
                                    possibly_pack.as_deref().expect("just put it in")
                                }
                                None => match self.store.load_one_index(snapshot.marker)? {
                                    Some(new_snapshot) => {
                                        *snapshot = new_snapshot;
                                        self.clear_cache();
                                        continue 'outer;
                                    }
                                    None => {
                                        return Ok(None);
                                    }
                                },
                            },
                        };
                        let entry = pack.entry(pack_offset)?;
                        let res = pack.decode_entry(
                            entry,
                            buffer,
                            inflate,
                            &base_resolver(pack, &index_file, None),
                            pack_cache,
                        );
                        let res = match res {
                            Ok(r) => Ok(ObjectData {
                                kind: r.kind,
                                data: buffer.as_slice(),
                            }),
                            Err(GitError::DeltaBaseUnresolved(base_id)) => self
                                .decode_with_external_base(
                                    id, base_id, idx, buffer, inflate, pack_cache, snapshot,
                                    recursion,
                                ),
                            Err(err) => Err(err),
                        }?;

                        if idx != 0 {
                            snapshot.indices.swap(0, idx);
                        }
                        return Ok(Some(res));
                    }
                }
            }

            for lodb in snapshot.loose_dbs.iter() {
                if lodb.contains(id) {
                    return lodb.try_find(id, buffer).map_err(|_| GitError::Gen);
                }
            }

            match self.store.load_one_index(snapshot.marker)? {
                Some(new_snapshot) => {
                    *snapshot = new_snapshot;
                    self.clear_cache();
                }
                None => return Ok(None),
            }
        }
    }
    /// Recovery path for a delta whose base lives outside the current pack: resolve
    /// the base via a full lookup, then decode the original entry again with those
    /// bytes injected. The recursive call may swap indices or replace the snapshot,
    /// so `id` is located from scratch rather than reusing the earlier borrows. (claude)
    #[allow(clippy::too_many_arguments)]
    fn decode_with_external_base<'a>(
        &self,
        id: &oid,
        base_id: ObjectId,
        idx: usize,
        buffer: &'a mut Vec<u8>,
        inflate: &mut zlib::Inflate,
        pack_cache: &mut dyn TraitDecodeEntry,
        snapshot: &mut Snapshot,
        recursion: usize,
    ) -> Result<ObjectData<'a>, GitError> {
        let mut buf = Vec::new();
        let obj_kind = self
            .try_find_cached_inner(
                &base_id,
                &mut buf,
                inflate,
                pack_cache,
                snapshot,
                recursion + 1,
            )
            .map_err(|_| GitError::Gen)?
            .ok_or(GitError::Gen)?
            .kind;

        let IndexLookupOutcome {
            object_index: IndexForObjectInPack { pack_offset, .. },
            index_file,
            pack: possibly_pack,
        } = if let Some(res) = snapshot.indices[idx].lookup(id) {
            res
        } else {
            snapshot
                .indices
                .iter_mut()
                .find_map(|index| index.lookup(id))
                .unwrap_or_else(|| {
                    panic!("{id} not found in any index after resolving its base object")
                })
        };

        let pack = possibly_pack
            .as_ref()
            .expect("pack to still be available like just now");
        let entry = pack.entry(pack_offset)?;
        pack.decode_entry(
            entry,
            buffer,
            inflate,
            &base_resolver(pack, &index_file, Some((base_id, buf.as_slice(), obj_kind))),
            pack_cache,
        )
        .map(move |r| ObjectData {
            kind: r.kind,
            data: buffer.as_slice(),
        })
    }
    fn clear_cache(&self) {
        self.packed_object_count.borrow_mut().take();
    }
}

impl<S> TraitPackFind for OdbHandle<S>
where
    S: Deref<Target = Store> + Clone,
{
    fn try_find_cached<'a>(
        &self,
        id: &oid,
        buffer: &'a mut Vec<u8>,
        pack_cache: &mut dyn TraitDecodeEntry,
    ) -> Result<Option<ObjectData<'a>>, GitError> {
        let mut snapshot = self.snapshot.borrow_mut();
        let mut inflate = self.inflate.borrow_mut();
        self.try_find_cached_inner(id, buffer, &mut inflate, pack_cache, &mut snapshot, 0)
            .map_err(|_| GitError::Gen)
    }
}

impl<S> Find for OdbHandle<S>
where
    S: Deref<Target = Store> + Clone,
    Self: TraitPackFind,
{
    fn try_find<'a>(
        &self,
        id: &oid,
        buffer: &'a mut Vec<u8>,
    ) -> Result<Option<ObjectData<'a>>, GitError> {
        TraitPackFind::try_find(self, id, buffer)
    }
}

#[derive(Clone)]
enum HandleMode {
    DeletedPacksAreInaccessible,
    KeepDeletedPacksAvailable,
}

impl<S> Drop for OdbHandle<S>
where
    S: Deref<Target = Store> + Clone,
{
    fn drop(&mut self) {
        if let Some(token) = self.token.take() {
            self.store.remove_handle(token);
        }
    }
}

impl<S> Clone for OdbHandle<S>
where
    S: Deref<Target = Store> + Clone,
{
    fn clone(&self) -> Self {
        OdbHandle {
            store: self.store.clone(),

            token: {
                let token = self.store.register_handle();
                match self.token.as_ref().expect("token is always set here ") {
                    HandleMode::DeletedPacksAreInaccessible => token,
                    HandleMode::KeepDeletedPacksAvailable => self.store.upgrade_handle(token),
                }
                .into()
            },
            inflate: RefCell::new(Default::default()),
            snapshot: RefCell::new(self.store.collect_snapshot()),
            max_recursion_depth: self.max_recursion_depth,
            packed_object_count: Default::default(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct TreeEntry {
    pub mode: EntryMode,
    filename: bstr::BString,
    pub oid: ObjectId,
}

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct EntryMode {
    internal: u16,
}

#[derive(PartialEq, Eq, Debug, Hash, Clone)]
struct SignedData<'a> {
    data: &'a [u8],
    signature_range: Range<usize>,
}

impl SignedData<'_> {
    #[must_use]
    fn to_bstring(&self) -> BString {
        let mut buf = BString::from(&self.data[..self.signature_range.start]);
        buf.extend_from_slice(&self.data[self.signature_range.end..]);
        buf
    }
}

impl From<SignedData<'_>> for BString {
    fn from(value: SignedData<'_>) -> Self {
        value.to_bstring()
    }
}

#[derive(Copy, Clone)]
enum SignatureKind {
    Author,
    Committer,
}

#[derive(Default, Copy, Clone)]
enum CommitState {
    #[default]
    Tree,
    Parents,
    Signature {
        of: SignatureKind,
    },
    Encoding,
    ExtraHeaders,
    Message,
}

impl<'a> CommitRefIter<'a> {
    #[must_use]
    fn hx_from_bytes(data: &'a [u8]) -> CommitRefIter<'a> {
        CommitRefIter {
            data,
            state: CommitState::default(),
        }
    }
}

impl CommitRefIter<'_> {
    fn tree_id(&mut self) -> Result<ObjectId, GitError> {
        let tree_id = self.next().ok_or(GitError::Gen)??;
        Token::try_into_id(tree_id).ok_or(GitError::Gen)
    }
}

impl<'a> CommitRefIter<'a> {
    #[inline]
    fn next_inner(mut i: &'a [u8], state: &mut CommitState) -> Result<(&'a [u8], Token), GitError> {
        let input = &mut i;
        match Self::next_inner_(input, state) {
            Ok(token) => Ok((*input, token)),
            Err(_) => Err(GitError::Gen),
        }
    }

    fn next_inner_(input: &mut &'a [u8], state: &mut CommitState) -> Result<Token, GitError> {
        Ok(match state {
            CommitState::Tree => {
                let tree = header_field(input, b"tree", |value| hex_hash(value))?;
                *state = CommitState::Parents;
                Token::Tree {
                    id: ObjectId::hx_from_hex(tree).expect("parsing validation"),
                }
            }
            CommitState::Parents => {
                if input.starts_with(b"parent ") {
                    let parent = header_field(input, b"parent", |value| hex_hash(value))?;
                    Token::Parent {
                        id: ObjectId::hx_from_hex(parent).expect("parsing validation"),
                    }
                } else {
                    *state = CommitState::Signature {
                        of: SignatureKind::Author,
                    };
                    Self::next_inner_(input, state)?
                }
            }
            CommitState::Signature { of } => {
                let who = *of;
                let field_name = match of {
                    SignatureKind::Author => {
                        *of = SignatureKind::Committer;
                        &b"author"[..]
                    }
                    SignatureKind::Committer => {
                        *state = CommitState::Encoding;
                        &b"committer"[..]
                    }
                };
                let _signature = header_field(input, field_name, signature)?;
                match who {
                    SignatureKind::Author => Token::Author,
                    SignatureKind::Committer => Token::Committer,
                }
            }
            CommitState::Encoding => {
                *state = CommitState::ExtraHeaders;
                if input.starts_with(b"encoding ") {
                    let _encoding = header_field(input, b"encoding", Ok)?;
                    Token::Encoding
                } else {
                    Self::next_inner_(input, state)?
                }
            }
            CommitState::ExtraHeaders => {
                if input.starts_with(b"\n") {
                    *state = CommitState::Message;
                    Self::next_inner_(input, state)?
                } else {
                    let before = *input;
                    {
                        let _extra_header = any_header_field_multi_line(input)
                            .map(|(k, o)| (k.as_bstr(), Cow::Owned(o)))
                            .or_else(|_| {
                                *input = before;
                                any_header_field(input)
                                    .map(|(k, o)| (k.as_bstr(), Cow::Borrowed(o.as_bstr())))
                            })?;
                        Token::ExtraHeader
                    }
                }
            }
            CommitState::Message => Token::Message,
        })
    }
}

impl Iterator for CommitRefIter<'_> {
    type Item = Result<Token, GitError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }
        match Self::next_inner(self.data, &mut self.state) {
            Ok((data, token)) => {
                self.data = data;
                Some(Ok(token))
            }
            Err(_) => {
                self.data = &[];
                Some(Err(GitError::Gen))
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
enum Token {
    Tree { id: ObjectId },
    Parent { id: ObjectId },
    Author,
    Committer,
    Encoding,
    ExtraHeader,
    Message,
}

impl Token {
    #[must_use]
    fn try_into_id(self) -> Option<ObjectId> {
        match self {
            Token::Tree { id } | Token::Parent { id } => Some(id),
            _ => None,
        }
    }
}

impl<'a> ObjectData<'a> {
    #[must_use]
    fn new(data: &'a [u8], kind: ObjectKind) -> ObjectData<'a> {
        ObjectData { kind, data }
    }
}

impl From<EntryRef<'_>> for TreeEntry {
    fn from(other: EntryRef<'_>) -> TreeEntry {
        let EntryRef {
            mode,
            filename,
            oid,
        } = other;
        TreeEntry {
            mode,
            filename: filename.to_owned(),
            oid: oid.into(),
        }
    }
}

impl<'a> From<&'a TreeEntry> for EntryRef<'a> {
    fn from(other: &'a TreeEntry) -> EntryRef<'a> {
        let TreeEntry {
            mode,
            filename,
            oid,
        } = other;
        EntryRef {
            mode: *mode,
            filename: filename.as_ref(),
            oid,
        }
    }
}

impl TryFrom<u32> for EntryMode {
    type Error = GitError;
    fn try_from(mode: u32) -> Result<Self, GitError> {
        Ok(match mode {
            0o40000 | 0o120000 | 0o160000 => EntryMode {
                internal: mode as u16,
            },
            blob_mode if blob_mode & 0o100000 == 0o100000 => EntryMode {
                internal: mode as u16,
            },
            _ => return Err(GitError::Gen),
        })
    }
}

impl EntryMode {
    #[must_use]
    const fn value(self) -> u16 {
        if self.internal & IFMT == 0o140000 {
            0o040000
        } else {
            self.internal
        }
    }

    fn as_bytes<'a>(&self, backing: &'a mut [u8; 6]) -> &'a BStr {
        if self.internal == 0 {
            std::slice::from_ref(&b'0')
        } else {
            for (idx, backing_octet) in backing.iter_mut().enumerate() {
                let bit_pos = 3 /* because base 8 and 2^3 == 8*/ * (6 - idx - 1);
                let oct_mask = 0b111 << bit_pos;
                let digit = (self.internal & oct_mask) >> bit_pos;
                *backing_octet = b'0' + digit as u8;
            }
            if backing[1] == b'4' {
                if backing[0] == b'1' {
                    backing[0] = b'0';
                    &backing[0..6]
                } else {
                    &backing[1..6]
                }
            } else {
                &backing[0..6]
            }
        }
        .into()
    }

    fn extract_from_bytes(i: &[u8]) -> Option<(Self, &'_ [u8])> {
        let mut mode = 0;
        if i.is_empty() {
            return None;
        }

        let space_pos = if i.get(6) == Some(&b' ') && i.get(5) != Some(&b' ') {
            for b in i.iter().take(6) {
                let b = u16::from(b.wrapping_sub(b'0'));
                if b > 7 {
                    return None;
                }
                mode = (mode << 3) + b;
            }
            6
        } else {
            let mut idx = 0;
            let mut space_pos = 0;

            while idx < i.len() {
                let b = u16::from(i[idx].wrapping_sub(b'0'));
                if b == u16::from(b' '.wrapping_sub(b'0')) {
                    space_pos = idx;
                    break;
                }
                if b > 7 {
                    return None;
                }
                if idx > 6 {
                    return None;
                }
                mode = (mode << 3) + b;
                idx += 1;
            }

            space_pos
        };

        if mode == 0o040000 && i[0] == b'0' {
            mode += 0o100000;
        }
        Some((Self { internal: mode }, &i[(space_pos + 1)..]))
    }

    #[must_use]
    fn from_bytes(i: &[u8]) -> Option<Self> {
        Self::extract_from_bytes(i).map(|(mode, _rest)| mode)
    }
}

impl std::fmt::Debug for EntryMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EntryMode(0o{})", self.as_bytes(&mut Default::default()))
    }
}

impl std::fmt::Octal for EntryMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_bytes(&mut Default::default()))
    }
}

impl From<EntryKind> for EntryMode {
    fn from(value: EntryKind) -> Self {
        EntryMode {
            internal: value as u16,
        }
    }
}

impl From<EntryMode> for EntryKind {
    fn from(value: EntryMode) -> Self {
        value.kind()
    }
}

const IFMT: u16 = 0o170000;

impl EntryMode {
    #[must_use]
    pub const fn kind(&self) -> EntryKind {
        let etype = self.value() & IFMT;
        if etype == 0o100000 {
            if self.value() & 0o000100 == 0o000100 {
                EntryKind::BlobExecutable
            } else {
                EntryKind::Blob
            }
        } else if etype == EntryKind::Link as u16 {
            EntryKind::Link
        } else if etype == EntryKind::Tree as u16 {
            EntryKind::Tree
        } else {
            EntryKind::Commit
        }
    }

    #[must_use]
    const fn is_tree(&self) -> bool {
        self.value() & IFMT == EntryKind::Tree as u16
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct EntryRef<'a> {
    mode: EntryMode,
    filename: &'a bstr::BStr,
    oid: &'a oid,
}

impl PartialOrd for EntryRef<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EntryRef<'_> {
    fn cmp(&self, b: &Self) -> cmp::Ordering {
        let a = self;
        let common = a.filename.len().min(b.filename.len());
        a.filename[..common]
            .cmp(&b.filename[..common])
            .then_with(|| {
                let a = a
                    .filename
                    .get(common)
                    .or_else(|| a.mode.is_tree().then_some(&b'/'));
                let b = b
                    .filename
                    .get(common)
                    .or_else(|| b.mode.is_tree().then_some(&b'/'));
                a.cmp(&b)
            })
    }
}

impl PartialOrd for TreeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TreeEntry {
    fn cmp(&self, b: &Self) -> cmp::Ordering {
        let a = self;
        let common = a.filename.len().min(b.filename.len());
        a.filename[..common]
            .cmp(&b.filename[..common])
            .then_with(|| {
                let a = a
                    .filename
                    .get(common)
                    .or_else(|| a.mode.is_tree().then_some(&b'/'));
                let b = b
                    .filename
                    .get(common)
                    .or_else(|| b.mode.is_tree().then_some(&b'/'));
                a.cmp(&b)
            })
    }
}

fn next_entry<'a, I, P>(
    components: &mut core::iter::Peekable<I>,
    tree: ObjectData<'a>,
) -> core::ops::ControlFlow<Option<EntryRef<'a>>, ObjectId>
where
    I: Iterator<Item = P>,
    P: PartialEq<BStr>,
{
    if !tree.kind.is_tree() {
        return ControlFlow::Break(None);
    }

    let Some(component) = components.next() else {
        return ControlFlow::Break(None);
    };

    let Some(entry) = TreeRefIter::from_bytes(tree.data)
        .filter_map(Result::ok)
        .find(|entry| component.eq(entry.filename))
    else {
        return ControlFlow::Break(None);
    };

    if components.peek().is_none() {
        ControlFlow::Break(Some(entry))
    } else {
        ControlFlow::Continue(entry.oid.to_owned())
    }
}

impl<'a> TreeRefIter<'a> {
    #[must_use]
    fn from_bytes(data: &'a [u8]) -> TreeRefIter<'a> {
        TreeRefIter { data }
    }
}

impl<'a> Iterator for TreeRefIter<'a> {
    type Item = Result<EntryRef<'a>, GitError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }
        if let Some((data_left, entry)) = tree_decode_fast_entry(self.data, 20) {
            self.data = data_left;
            Some(Ok(entry))
        } else {
            self.data = &[];
            Some(Err(GitError::Gen))
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for EntryMode {
    type Error = GitError;

    fn try_from(mode: &'a [u8]) -> Result<Self, GitError> {
        EntryMode::from_bytes(mode).ok_or(GitError::Gen)
    }
}

fn tree_decode_fast_entry(i: &[u8], hash_len: usize) -> Option<(&[u8], EntryRef<'_>)> {
    let (mode, i) = EntryMode::extract_from_bytes(i)?;
    let (filename, i) = i.split_at(i.find_byte(0)?);
    let i = &i[1..];
    let (oid, i) = match i.len() {
        len if len < hash_len => return None,
        _ => i.split_at(hash_len),
    };
    Some((
        i,
        EntryRef {
            mode,
            filename: filename.as_bstr(),
            oid: oid::try_from_bytes(oid)
                .unwrap_or_else(|_| panic!("we counted exactly {hash_len} bytes")),
        },
    ))
}

trait Find {
    fn try_find<'a>(
        &self,
        id: &oid,
        buffer: &'a mut Vec<u8>,
    ) -> Result<Option<ObjectData<'a>>, GitError>;
}

trait FindExt: Find {
    fn find<'a>(&self, id: &oid, buffer: &'a mut Vec<u8>) -> Result<ObjectData<'a>, GitError> {
        self.try_find(id, buffer)
            .map_err(|_| GitError::Gen)?
            .ok_or(GitError::NotFound)
    }
}

impl<T: Find + ?Sized> FindExt for T {}

#[must_use]
fn loose_header(kind: ObjectKind, size: u64) -> smallvec::SmallVec<[u8; 28]> {
    let mut v = smallvec::SmallVec::new();
    let _ = v.write_all(kind.as_bytes());
    let _ = v.write_all(SPACE);
    let _ = v.write_all(itoa::Buffer::new().format(size).as_bytes());
    let _ = v.write_all(b"\0");
    v
}

const SPACE: &[u8; 1] = b" ";
const SPACE_OR_NL: &[u8] = b" \n";

type OdbParseResult<T> = Result<T, GitError>;

fn any_header_field_multi_line<'a>(i: &mut &'a [u8]) -> OdbParseResult<(&'a [u8], BString)> {
    let mut c = *i;
    let input = c;
    let name_end = c
        .find_byteset(SPACE_OR_NL)
        .filter(|pos| *pos > 0)
        .ok_or(GitError::Gen)?;
    if c.get(name_end) != Some(&b' ') {
        return Err(GitError::Gen);
    }

    c = &c[name_end + 1..];
    let first_line_end = c.find_byte(b'\n').ok_or(GitError::Gen)?;
    c = &c[first_line_end + 1..];

    let mut continuation_end = name_end + 1 + first_line_end + 1;
    let mut continuation_count = 0usize;
    while c.first() == Some(&b' ') {
        let line_end = c.find_byte(b'\n').ok_or(GitError::Gen)?;
        continuation_end += line_end + 1;
        c = &c[line_end + 1..];
        continuation_count += 1;
    }
    if continuation_count == 0 {
        return Err(GitError::Gen);
    }

    let bytes = input[name_end + 1..continuation_end].as_bstr();
    let mut out = BString::from(Vec::with_capacity(bytes.len()));
    let mut lines = bytes.lines_with_terminator();
    out.push_str(lines.next().expect("first line"));
    for line in lines {
        out.push_str(&line[1..]);
    }
    *i = &input[continuation_end..];
    Ok((input[..name_end].as_bstr(), out))
}

fn header_field<'a, T>(
    i: &mut &'a [u8],
    name: &'static [u8],
    parse_value: impl FnOnce(&'a [u8]) -> OdbParseResult<T>,
) -> OdbParseResult<T> {
    let c = *i;
    let Some(rest) = c
        .strip_prefix(name)
        .and_then(|rest| rest.strip_prefix(SPACE))
    else {
        return Err(GitError::Gen);
    };
    let Some(nl) = rest.find_byte(b'\n') else {
        return Err(GitError::Gen);
    };
    let value = parse_value(&rest[..nl])?;
    *i = &rest[nl + 1..];
    Ok(value)
}

fn any_header_field<'a>(i: &mut &'a [u8]) -> OdbParseResult<(&'a [u8], &'a [u8])> {
    let mut c = *i;
    let input = c;
    let name_end = c
        .find_byteset(SPACE_OR_NL)
        .filter(|pos| *pos > 0)
        .ok_or(GitError::Gen)?;
    if c.get(name_end) != Some(&b' ') {
        return Err(GitError::Gen);
    }
    c = &c[name_end + 1..];
    if let Some(value_end) = c.find_byte(b'\n') {
        let value = &c[..value_end];
        let rest = &c[value_end + 1..];
        *i = rest;
        Ok((&input[..name_end], value))
    } else {
        Err(GitError::Gen)
    }
}

fn hex_hash(i: &[u8]) -> OdbParseResult<&BStr> {
    if i.len() != 40 || !i.iter().all(u8::is_ascii_hexdigit) {
        return Err(GitError::Gen);
    }
    Ok(i.as_bstr())
}

fn signature(mut i: &[u8]) -> OdbParseResult<GixActorSignatureRef<'_>> {
    let signature =
        GixActorSignatureRef::from_bytes_consuming(&mut i).map_err(|_| GitError::Gen)?;
    if i.is_empty() {
        Ok(signature)
    } else {
        Err(GitError::Gen)
    }
}

#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
struct IdentityRef<'a> {
    name: &'a bstr::BStr,
    email: &'a bstr::BStr,
}

#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
struct GixActorSignatureRef<'a> {
    name: &'a bstr::BStr,
    email: &'a bstr::BStr,
    time: &'a str,
}

impl<'a> GixActorSignatureRef<'a> {
    fn from_bytes_consuming(data: &mut &'a [u8]) -> Result<GixActorSignatureRef<'a>, GitError> {
        decode(data)
    }
}

fn decode<'a>(i: &mut &'a [u8]) -> Result<GixActorSignatureRef<'a>, GitError> {
    let identity = identity(i)?;
    if i.first() == Some(&b' ') {
        *i = &i[1..];
    }

    let time_len = i.iter().position(|b| !is_time_byte(*b)).unwrap_or(i.len());
    let (time, rest) = i.split_at(time_len);
    *i = rest;
    #[allow(unsafe_code)]
    let time = unsafe { std::str::from_utf8_unchecked(time) };

    Ok(GixActorSignatureRef {
        name: identity.name,
        email: identity.email,
        time,
    })
}

fn identity<'a>(i: &mut &'a [u8]) -> Result<IdentityRef<'a>, GitError> {
    let eol_idx = i.find_byte(b'\n').unwrap_or(i.len());
    let right_delim_idx = i[..eol_idx].rfind_byte(b'>').ok_or(GitError::Gen)?;
    let i_name_and_email = &i[..right_delim_idx];
    let skip_from_right = i_name_and_email
        .iter()
        .rev()
        .take_while(|b| **b == b'>')
        .count();
    let left_delim_idx = i_name_and_email.find_byte(b'<').ok_or(GitError::Gen)?;
    let skip_from_left = i[left_delim_idx..]
        .iter()
        .take_while(|b| **b == b'<')
        .count();
    let mut name = i[..left_delim_idx].as_bstr();
    name = name.strip_suffix(b" ").unwrap_or(name).as_bstr();

    let email = i
        .get(left_delim_idx + skip_from_left..right_delim_idx - skip_from_right)
        .ok_or(GitError::Gen)?
        .as_bstr();
    *i = i.get(right_delim_idx + 1..).unwrap_or(&[]);
    Ok(IdentityRef { name, email })
}

fn is_time_byte(b: u8) -> bool {
    matches!(b, b'+' | b'-' | b'0'..=b'9' | b' ' | b'\t')
}

impl ObjectKind {
    fn from_bytes(s: &[u8]) -> Result<ObjectKind, GitError> {
        Ok(match s {
            b"tree" => ObjectKind::Tree,
            b"blob" => ObjectKind::Blob,
            b"commit" => ObjectKind::Commit,

            _ => return Err(GitError::Gen),
        })
    }

    #[must_use]
    fn as_bytes(&self) -> &[u8] {
        match self {
            ObjectKind::Tree => b"tree",
            ObjectKind::Commit => b"commit",
            ObjectKind::Blob => b"blob",
        }
    }

    #[must_use]
    fn is_tree(&self) -> bool {
        matches!(self, ObjectKind::Tree)
    }
}

impl fmt::Display for ObjectKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(std::str::from_utf8(self.as_bytes()).expect("Converting Kind name to utf8"))
    }
}

#[derive(Copy, Clone)]
struct CommitRefIter<'a> {
    data: &'a [u8],
    state: CommitState,
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
struct TreeRefIter<'a> {
    data: &'a [u8],
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
struct ObjectData<'a> {
    kind: ObjectKind,
    data: &'a [u8],
}

fn decode_loose_header(input: &[u8]) -> Result<(ObjectKind, u64, usize), GitError> {
    let kind_end = input.find_byte(0x20).ok_or(GitError::Gen)?;
    let kind = ObjectKind::from_bytes(&input[..kind_end])?;
    let size_end = input.find_byte(0x0).ok_or(GitError::Gen)?;
    let size_bytes = &input[kind_end + 1..size_end];
    let size = to_signed(size_bytes).map_err(|_| GitError::Gen)?;
    Ok((kind, size, size_end + 1))
}

fn hx_compute_hash(object_kind: ObjectKind, data: &[u8]) -> Result<ObjectId, GitError> {
    let object_size = data.len() as u64;
    let mut hasher: HasherKind = HasherKind::new_sha1();
    hasher.hx_update(&loose_header(object_kind, object_size));
    hasher.hx_update(data);
    hasher.hx_try_finalize()
}

#[derive(Clone)]
enum HasherKind {
    Sha1(sha1_checked::Sha1),
}

impl HasherKind {
    fn new_sha1() -> Self {
        Self::Sha1(Builder::default().safe_hash(false).build())
    }
}

impl HasherKind {
    // gix-object
    fn hx_update(&mut self, bytes: &[u8]) {
        match self {
            HasherKind::Sha1(sha1) => sha1.update(bytes),
        }
    }

    #[inline]
    fn hx_try_finalize(self) -> Result<ObjectId, GitError> {
        match self {
            HasherKind::Sha1(sha1) => match sha1.try_finalize() {
                CollisionResult::Ok(digest) => Ok(ObjectId::Sha1(digest.into())),
                CollisionResult::Mitigated(_) => {
                    #[allow(unsafe_code)]
                    unsafe {
                        std::hint::unreachable_unchecked()
                    }
                }
                CollisionResult::Collision(_digest) => Err(GitError::Gen),
            },
        }
    }
}

#[allow(clippy::derived_hash_with_manual_eq)]
impl Hash for ObjectId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.as_slice());
    }
}

impl ObjectId {
    fn hx_from_hex(buffer: &[u8]) -> Result<ObjectId, GitError> {
        match buffer.len() {
            40 => Ok({
                ObjectId::Sha1({
                    let mut buf = [0; 20];
                    faster_hex::hex_decode(buffer, &mut buf).map_err(|err| match err {
                        faster_hex::Error::InvalidChar | faster_hex::Error::Overflow => {
                            GitError::Gen
                        }
                        faster_hex::Error::InvalidLength(_) => {
                            unreachable!("BUG: This is already checked")
                        }
                    })?;
                    buf
                })
            }),
            _ => Err(GitError::Gen),
        }
    }
}

impl FromStr for ObjectId {
    type Err = GitError;

    fn from_str(s: &str) -> Result<Self, GitError> {
        Self::hx_from_hex(s.as_bytes())
    }
}

impl ObjectId {
    #[inline]
    #[must_use]
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Sha1(b) => b.as_ref(),
        }
    }

    #[inline]
    #[must_use]
    const fn empty_tree() -> ObjectId {
        ObjectId::Sha1([
            0x4b, 0x82, 0x5d, 0xc6, 0x42, 0xcb, 0x6e, 0xb9, 0xa0, 0x60, 0xe5, 0x4b, 0xf8, 0xd6,
            0x92, 0x88, 0xfb, 0xee, 0x49, 0x04,
        ])
    }
}

impl ObjectId {
    #[must_use]
    fn from_bytes_or_panic(bytes: &[u8]) -> Self {
        match bytes.len() {
            20 => Self::Sha1(bytes.try_into().expect("prior length validation")),
            other => panic!("BUG: unsupported hash len: {other}"),
        }
    }
}

impl ObjectId {
    #[inline]
    fn new_sha1(id: [u8; 20]) -> Self {
        ObjectId::Sha1(id)
    }

    #[inline]
    fn from_20_bytes(b: &[u8]) -> ObjectId {
        let mut id = [0; 20];
        id.copy_from_slice(b);
        ObjectId::Sha1(id)
    }
}

impl std::fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectId::Sha1(_hash) => f.write_str("Sha1(")?,
        }
        for b in self.as_bytes() {
            write!(f, "{b:02x}")?;
        }
        f.write_str(")")
    }
}

impl From<[u8; 20]> for ObjectId {
    fn from(v: [u8; 20]) -> Self {
        Self::new_sha1(v)
    }
}

impl From<&oid> for ObjectId {
    fn from(v: &oid) -> Self {
        ObjectId::from_20_bytes(v.as_bytes())
    }
}

impl TryFrom<&[u8]> for ObjectId {
    type Error = GitError;

    fn try_from(bytes: &[u8]) -> Result<Self, GitError> {
        Ok(oid::try_from_bytes(bytes)?.into())
    }
}

impl Deref for ObjectId {
    type Target = oid;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl AsRef<oid> for ObjectId {
    fn as_ref(&self) -> &oid {
        oid::from_bytes_unchecked(self.as_slice())
    }
}

impl Borrow<oid> for ObjectId {
    fn borrow(&self) -> &oid {
        self.as_ref()
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl PartialEq<&oid> for ObjectId {
    fn eq(&self, other: &&oid) -> bool {
        self.as_ref() == *other
    }
}

#[derive(PartialEq, Eq, Ord, PartialOrd)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct oid {
    bytes: [u8],
}

#[allow(clippy::derived_hash_with_manual_eq)]
impl std::hash::Hash for oid {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(self.as_bytes());
    }
}

#[derive(PartialEq, Eq, Hash, Ord, PartialOrd)]
struct HexDisplay<'a> {
    inner: &'a oid,
    hex_len: usize,
}

impl std::fmt::Display for HexDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut hex = [0u8; 40];
        let hex = self.inner.hex_to_buf(hex.as_mut());
        let max_len = hex.len();
        f.write_str(&hex[..self.hex_len.min(max_len)])
    }
}

impl oid {
    #[inline]
    fn try_from_bytes(digest: &[u8]) -> Result<&Self, GitError> {
        match digest.len() {
            20 => Ok(
                #[allow(unsafe_code)]
                unsafe {
                    &*(std::ptr::from_ref::<[u8]>(digest) as *const oid)
                },
            ),
            _ => Err(GitError::Gen),
        }
    }

    #[must_use]
    fn from_bytes_unchecked(value: &[u8]) -> &Self {
        Self::from_bytes(value)
    }

    fn from_bytes(value: &[u8]) -> &Self {
        #[allow(unsafe_code)]
        unsafe {
            &*(std::ptr::from_ref::<[u8]>(value) as *const oid)
        }
    }
}

impl oid {
    #[inline]
    #[must_use]
    fn first_byte(&self) -> u8 {
        self.bytes[0]
    }

    #[inline]
    #[must_use]
    fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[inline]
    #[must_use]
    pub fn helix_to_hex_with_len(&self) -> String {
        HexDisplay {
            inner: self,
            hex_len: 8,
        }
        .to_string()
    }

    #[inline]
    #[must_use]
    fn to_hex(&self) -> HexDisplay<'_> {
        HexDisplay {
            inner: self,
            hex_len: self.bytes.len() * 2,
        }
    }

    #[inline]
    #[must_use]
    fn hex_to_buf<'a>(&self, buf: &'a mut [u8]) -> &'a mut str {
        let num_hex_bytes = self.bytes.len() * 2;
        faster_hex::hex_encode(&self.bytes, &mut buf[..num_hex_bytes])
            .expect("buffer size must be at least twice the hash digest size in bytes")
    }

    #[inline]
    fn write_hex_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        let mut hex = [0u8; 40];
        let hex_len = self.hex_to_buf(&mut hex).len();
        out.write_all(&hex[..hex_len])
    }
}

impl AsRef<oid> for &oid {
    fn as_ref(&self) -> &oid {
        self
    }
}

impl<'a> TryFrom<&'a [u8]> for &'a oid {
    type Error = GitError;

    fn try_from(value: &'a [u8]) -> Result<Self, GitError> {
        oid::try_from_bytes(value)
    }
}

impl ToOwned for oid {
    type Owned = ObjectId;

    fn to_owned(&self) -> Self::Owned {
        ObjectId::Sha1(self.bytes.try_into().expect("no bug in hash detection"))
    }
}

impl<'a> From<&'a [u8; 20]> for &'a oid {
    fn from(v: &'a [u8; 20]) -> Self {
        oid::from_bytes(v.as_ref())
    }
}

impl std::fmt::Display for &oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = [0u8; 40];
        f.write_str(self.hex_to_buf(&mut buf))
    }
}

impl PartialEq<ObjectId> for &oid {
    fn eq(&self, other: &ObjectId) -> bool {
        *self == other.as_ref()
    }
}

trait TraitFileData: Deref<Target = [u8]> {}

impl<T> TraitFileData for T where T: Deref<Target = [u8]> {}

trait TraitDecodeEntry {
    fn put(
        &mut self,
        pack_id: u32,
        offset: u64,
        data: &[u8],
        kind: ObjectKind,
        compressed_size: usize,
    );
    fn get(&mut self, pack_id: u32, offset: u64, out: &mut Vec<u8>) -> Option<(ObjectKind, usize)>;
}

#[derive(Default)]
struct Never;

impl TraitDecodeEntry for Never {
    fn put(
        &mut self,
        _pack_id: u32,
        _offset: u64,
        _data: &[u8],
        _kind: ObjectKind,
        _compressed_size: usize,
    ) {
    }
    fn get(
        &mut self,
        _pack_id: u32,
        _offset: u64,
        _out: &mut Vec<u8>,
    ) -> Option<(ObjectKind, usize)> {
        None
    }
}

impl<T: TraitDecodeEntry + ?Sized> TraitDecodeEntry for Box<T> {
    fn put(
        &mut self,
        pack_id: u32,
        offset: u64,
        data: &[u8],
        kind: ObjectKind,
        compressed_size: usize,
    ) {
        self.deref_mut()
            .put(pack_id, offset, data, kind, compressed_size);
    }

    fn get(&mut self, pack_id: u32, offset: u64, out: &mut Vec<u8>) -> Option<(ObjectKind, usize)> {
        self.deref_mut().get(pack_id, offset, out)
    }
}

struct LRUCacheEntry {
    pack_id: u32,
    offset: u64,
    data: Vec<u8>,
    kind: ObjectKind,
    compressed_size: usize,
}

struct StaticLinkedList<const SIZE: usize> {
    inner: uluru::LRUCache<LRUCacheEntry, SIZE>,
    last_evicted: Vec<u8>,
    debug: cache::Debug,
    mem_used: usize,
    mem_limit: usize,
}

impl<const SIZE: usize> StaticLinkedList<SIZE> {
    #[must_use]
    fn new(mem_limit: usize) -> Self {
        StaticLinkedList {
            inner: uluru::LRUCache::default(),
            last_evicted: Vec::new(),
            debug: cache::Debug::new(format!("StaticLinkedList<{SIZE}>")),
            mem_used: 0,
            mem_limit: if mem_limit == 0 {
                usize::MAX
            } else {
                mem_limit
            },
        }
    }
}

impl<const SIZE: usize> Default for StaticLinkedList<SIZE> {
    fn default() -> Self {
        Self::new(96 * 1024 * 1024)
    }
}

impl<const SIZE: usize> TraitDecodeEntry for StaticLinkedList<SIZE> {
    fn put(
        &mut self,
        pack_id: u32,
        offset: u64,
        data: &[u8],
        kind: ObjectKind,
        compressed_size: usize,
    ) {
        if data.len() > self.mem_limit {
            return;
        }
        let mem_free = self.mem_limit - self.mem_used;
        if data.len() > mem_free {
            let free_list_cap = self.last_evicted.len();
            self.last_evicted = Vec::new();
            if data.len() > mem_free + free_list_cap {
                self.inner.clear();
                self.mem_used = 0;
            } else {
                self.mem_used -= free_list_cap;
            }
        }
        self.debug.put();
        let mut v = std::mem::take(&mut self.last_evicted);
        self.mem_used -= v.capacity();
        if set_vec_to_slice(&mut v, data).is_none() {
            return;
        }
        self.mem_used += v.capacity();
        if let Some(previous) = self.inner.insert(LRUCacheEntry {
            offset,
            pack_id,
            data: v,
            kind,
            compressed_size,
        }) {
            self.last_evicted = previous.data;
        }
    }

    fn get(&mut self, pack_id: u32, offset: u64, out: &mut Vec<u8>) -> Option<(ObjectKind, usize)> {
        let res = self.inner.lookup(|e: &mut LRUCacheEntry| {
            if e.pack_id == pack_id && e.offset == offset {
                set_vec_to_slice(&mut *out, &e.data)?;
                Some((e.kind, e.compressed_size))
            } else {
                None
            }
        });
        if res.is_some() {
            self.debug.hit();
        } else {
            self.debug.miss();
        }
        res
    }
}

fn set_vec_to_slice<V: BorrowMut<Vec<u8>>>(mut vec: V, source: &[u8]) -> Option<V> {
    let out = vec.borrow_mut();
    out.clear();
    out.try_reserve(source.len()).ok()?;
    out.extend_from_slice(source);
    Some(vec)
}

trait TraitPackFind {
    fn try_find<'a>(
        &self,
        id: &oid,
        buffer: &'a mut Vec<u8>,
    ) -> Result<Option<ObjectData<'a>>, GitError> {
        self.try_find_cached(id, buffer, &mut Never)
    }

    fn try_find_cached<'a>(
        &self,
        id: &oid,
        buffer: &'a mut Vec<u8>,
        pack_cache: &mut dyn TraitDecodeEntry,
    ) -> Result<Option<ObjectData<'a>>, GitError>;
}

impl<T> TraitPackFind for &T
where
    T: TraitPackFind,
{
    fn try_find_cached<'a>(
        &self,
        id: &oid,
        buffer: &'a mut Vec<u8>,
        pack_cache: &mut dyn TraitDecodeEntry,
    ) -> Result<Option<ObjectData<'a>>, GitError> {
        (*self).try_find_cached(id, buffer, pack_cache)
    }
}

#[derive(Default, PartialEq, Eq, Ord, PartialOrd, Debug, Hash, Clone, Copy)]
enum IndexVersion {
    V1 = 1,
    #[default]
    V2 = 2,
}

type EntryIndex = u32;

const FAN_LEN: usize = 256;

struct IndexFile<T = MMap> {
    data: T,

    version: IndexVersion,
    num_objects: u32,
    fan: [u32; FAN_LEN],
    hash_len: usize,
}

const V2_SIGNATURE: &[u8] = b"\xfftOc";
const N32_SIZE: usize = size_of::<u32>();

impl IndexFile<MMap> {
    fn at(path: impl AsRef<Path>) -> Result<Self, GitError> {
        Self::at_inner(path.as_ref())
    }

    fn at_inner(path: &Path) -> Result<Self, GitError> {
        let data = mmap_read_only(path).map_err(|_| GitError::Gen)?;
        Self::from_data(data)
    }
}

impl<T> IndexFile<T>
where
    T: TraitFileData,
{
    fn from_data(data: T) -> Result<Self, GitError> {
        let idx_len = data.len();
        let hash_len = 20;

        let footer_size = hash_len * 2;
        if idx_len < FAN_LEN * N32_SIZE + footer_size {
            return Err(GitError::Gen);
        }
        let (kind, fan, num_objects) = {
            let (kind, d) = {
                let (sig, d) = data.split_at(V2_SIGNATURE.len());
                if sig == V2_SIGNATURE {
                    (IndexVersion::V2, d)
                } else {
                    (IndexVersion::V1, &data[..])
                }
            };
            let d = {
                if let IndexVersion::V2 = kind {
                    let (vd, dr) = d.split_at(N32_SIZE);
                    let version = read_u32(vd);
                    if version != IndexVersion::V2 as u32 {
                        return Err(GitError::Gen);
                    }
                    dr
                } else {
                    d
                }
            };
            let (fan, bytes_read) = read_fan(d);
            let (_, _d) = d.split_at(bytes_read);
            let num_objects = fan[FAN_LEN - 1];

            (kind, fan, num_objects)
        };
        validate_fan(&fan)?;
        validate_size(&data, kind, num_objects, hash_len)?;
        Ok(Self {
            data,
            // path,
            version: kind,
            num_objects,
            fan,
            hash_len,
        })
    }
}

fn read_fan(d: &[u8]) -> ([u32; FAN_LEN], usize) {
    let mut fan = [0; FAN_LEN];
    for (c, f) in d.chunks_exact(N32_SIZE).zip(fan.iter_mut()) {
        *f = read_u32(c);
    }
    (fan, FAN_LEN * N32_SIZE)
}

fn validate_fan(fan: &[u32; FAN_LEN]) -> Result<(), GitError> {
    if !fan_is_monotonically_increasing(fan) {
        return Err(GitError::Gen);
    }
    Ok(())
}

fn validate_size(
    data: &[u8],
    kind: IndexVersion,
    num_objects: u32,
    hash_len: usize,
) -> Result<(), GitError> {
    let num_objects = num_objects as usize;
    let footer_size = hash_len * 2;
    let expected_size = match kind {
        IndexVersion::V1 => FAN_LEN
            .checked_mul(N32_SIZE)
            .and_then(|size| size.checked_add(num_objects.checked_mul(N32_SIZE + hash_len)?))
            .and_then(|size| size.checked_add(footer_size))
            .ok_or(GitError::Gen)?,
        IndexVersion::V2 => {
            let v2_header_size = V2_SIGNATURE.len() + N32_SIZE + FAN_LEN * N32_SIZE;
            let oid_bytes = num_objects.checked_mul(hash_len).ok_or(GitError::Gen)?;
            let table_bytes = num_objects.checked_mul(N32_SIZE).ok_or(GitError::Gen)?;
            let offset32_start = v2_header_size
                .checked_add(oid_bytes)
                .and_then(|size| size.checked_add(table_bytes))
                .ok_or(GitError::Gen)?;
            let offset32_end = offset32_start
                .checked_add(table_bytes)
                .ok_or(GitError::Gen)?;
            if offset32_end > data.len() {
                return Err(GitError::Gen);
            }
            let (large_offsets, max_large_offset_index) = data[offset32_start..offset32_end]
                .chunks_exact(N32_SIZE)
                .filter_map(|offset| {
                    let offset = read_u32(offset);
                    (offset & (1 << 31) != 0).then_some((offset ^ (1 << 31)) as usize)
                })
                .fold((0usize, 0usize), |(count, max_index), index| {
                    (count + 1, max_index.max(index))
                });
            v2_header_size
                .checked_add(oid_bytes)
                .and_then(|size| size.checked_add(table_bytes))
                .and_then(|size| size.checked_add(table_bytes))
                .and_then(|size| size.checked_add(large_offsets.checked_mul(size_of::<u64>())?))
                .and_then(|size| size.checked_add(footer_size))
                .ok_or(GitError::Gen)
                .and_then(|expected_size| {
                    if large_offsets > 0 && max_large_offset_index >= large_offsets {
                        return Err(GitError::Gen);
                    }
                    Ok(expected_size)
                })?
        }
    };
    if data.len() != expected_size {
        return Err(GitError::Gen);
    }
    Ok(())
}

const N64_SIZE: usize = size_of::<u64>();
const V1_HEADER_SIZE: usize = FAN_LEN * N32_SIZE;
const V2_HEADER_SIZE: usize = N32_SIZE * 2 + FAN_LEN * N32_SIZE;
const N32_HIGH_BIT: u32 = 1 << 31;

impl<T> IndexFile<T>
where
    T: TraitFileData,
{
    fn oid_at_index(&self, index: EntryIndex) -> &oid {
        let index = index as usize;
        let start = match self.version {
            IndexVersion::V2 => V2_HEADER_SIZE + index * self.hash_len,
            IndexVersion::V1 => V1_HEADER_SIZE + index * (N32_SIZE + self.hash_len) + N32_SIZE,
        };
        oid::from_bytes_unchecked(&self.data[start..][..self.hash_len])
    }

    fn pack_offset_at_index(&self, index: EntryIndex) -> Offset {
        let index = index as usize;
        match self.version {
            IndexVersion::V2 => {
                let start = self.offset_pack_offset_v2() + index * N32_SIZE;
                self.pack_offset_from_offset_v2(
                    &self.data[start..][..N32_SIZE],
                    self.offset_pack_offset64_v2(),
                )
            }
            IndexVersion::V1 => {
                let start = V1_HEADER_SIZE + index * (N32_SIZE + self.hash_len);
                u64::from(read_u32(&self.data[start..][..N32_SIZE]))
            }
        }
    }

    fn lookup(&self, id: impl AsRef<oid>) -> Option<EntryIndex> {
        lookup(id.as_ref(), &self.fan, &|idx| self.oid_at_index(idx))
    }

    #[inline]
    fn offset_crc32_v2(&self) -> usize {
        V2_HEADER_SIZE + self.num_objects as usize * self.hash_len
    }

    #[inline]
    fn offset_pack_offset_v2(&self) -> usize {
        self.offset_crc32_v2() + self.num_objects as usize * N32_SIZE
    }

    #[inline]
    fn offset_pack_offset64_v2(&self) -> usize {
        self.offset_pack_offset_v2() + self.num_objects as usize * N32_SIZE
    }

    #[inline]
    fn pack_offset_from_offset_v2(&self, offset: &[u8], pack64_offset: usize) -> Offset {
        let ofs32 = read_u32(offset);
        if (ofs32 & N32_HIGH_BIT) == N32_HIGH_BIT {
            let from = pack64_offset + (ofs32 ^ N32_HIGH_BIT) as usize * N64_SIZE;
            read_u64(&self.data[from..][..N64_SIZE])
        } else {
            u64::from(ofs32)
        }
    }
}

fn lookup<'a>(
    id: &oid,
    fan: &[u32; FAN_LEN],
    oid_at_index: &dyn Fn(EntryIndex) -> &'a oid,
) -> Option<EntryIndex> {
    let first_byte = id.first_byte() as usize;
    let mut upper_bound = fan[first_byte];
    let mut lower_bound = if first_byte != 0 {
        fan[first_byte - 1]
    } else {
        0
    };

    while lower_bound < upper_bound {
        let mid = u32::midpoint(lower_bound, upper_bound);
        let mid_sha = oid_at_index(mid);

        use std::cmp::Ordering::{Equal, Greater, Less};
        match id.cmp(mid_sha) {
            Less => upper_bound = mid,
            Equal => return Some(mid),
            Greater => lower_bound = mid + 1,
        }
    }
    None
}

type PackIndex = u32;

#[inline]
fn read_u32(b: &[u8]) -> u32 {
    u32::from_be_bytes(b.try_into().unwrap())
}

#[inline]
fn read_u64(b: &[u8]) -> u64 {
    u64::from_be_bytes(b.try_into().unwrap())
}

#[inline]
fn fan_is_monotonically_increasing(fan: &[u32]) -> bool {
    !fan.windows(2).any(|window| window[0] > window[1])
}

type Offset = u64;
type FilePackId = u32;

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
struct DataEntry {
    header: Header,
    decompressed_size: u64,
    data_offset: Offset,
    encoded_header_size: u16,
}

#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone)]

enum DataDecodeEntryResolvedBase {
    InPack(DataEntry),
    OutOfPack { kind: ObjectKind, end: usize },
}

#[derive(Debug)]
struct Delta {
    data: Range<usize>,
    base_size: usize,
    result_size: usize,

    decompressed_size: usize,
    data_offset: Offset,
}

#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone)]

struct PackOutcome {
    kind: ObjectKind,
    num_deltas: u32,
    decompressed_size: u64,
    compressed_size: usize,
    object_size: u64,
}

impl PackOutcome {
    fn from_object_entry(kind: ObjectKind, entry: &DataEntry, compressed_size: usize) -> Self {
        Self {
            kind,
            num_deltas: 0,
            decompressed_size: entry.decompressed_size,
            compressed_size,
            object_size: entry.decompressed_size,
        }
    }
}

impl<T> DataFile<T>
where
    T: TraitFileData,
{
    fn decoded_object_size(&self, size: u64) -> Result<usize, GitError> {
        decoded_object_size(size)
    }

    fn decompress_entry(
        &self,
        entry: &DataEntry,
        inflate: &mut zlib::Inflate,
        out: &mut [u8],
    ) -> Result<usize, GitError> {
        let size: usize = entry
            .decompressed_size
            .try_into()
            .map_err(|_| GitError::Gen)?;
        if out.len() < size {
            return Err(GitError::Gen);
        }
        self.decompress_entry_from_data_offset(entry.data_offset, inflate, &mut out[..size])
    }

    fn entry(&self, offset: Offset) -> Result<DataEntry, GitError> {
        let pack_offset: usize = offset.try_into().expect("offset representable by machine");
        if pack_offset > self.data.len() {
            return Err(GitError::Gen);
        }

        let object_data = &self.data[pack_offset..];
        DataEntry::from_bytes(object_data, offset, self.hash_len)
    }

    fn decompress_entry_from_data_offset(
        &self,
        data_offset: Offset,
        inflate: &mut zlib::Inflate,
        out: &mut [u8],
    ) -> Result<usize, GitError> {
        let (consumed_in, _consumed_out) =
            self.decompress_complete_entry_from_data_offset(data_offset, inflate, out)?;
        Ok(consumed_in)
    }

    fn decompress_complete_entry_from_data_offset(
        &self,
        data_offset: Offset,
        inflate: &mut zlib::Inflate,
        out: &mut [u8],
    ) -> Result<(usize, usize), GitError> {
        let (status, consumed_in, consumed_out) =
            self.decompress_entry_from_data_offset_unchecked(data_offset, inflate, out)?;
        if status != zlib::Status::StreamEnd || consumed_out != out.len() {
            return Err(GitError::Gen);
        }
        Ok((consumed_in, consumed_out))
    }

    fn decompress_entry_from_data_offset_unchecked(
        &self,
        data_offset: Offset,
        inflate: &mut zlib::Inflate,
        out: &mut [u8],
    ) -> Result<(zlib::Status, usize, usize), GitError> {
        let offset: usize = data_offset
            .try_into()
            .expect("offset representable by machine");
        if offset >= self.data.len() {
            return Err(GitError::Gen);
        }

        inflate.reset();
        inflate.once(&self.data[offset..], out)
    }

    fn decode_entry(
        &self,
        entry: DataEntry,
        out: &mut Vec<u8>,
        inflate: &mut zlib::Inflate,
        resolve: &dyn Fn(&oid, &mut Vec<u8>) -> Option<DataDecodeEntryResolvedBase>,
        delta_cache: &mut dyn TraitDecodeEntry,
    ) -> Result<PackOutcome, GitError> {
        match entry.header {
            Header::Tree | Header::Blob | Header::Commit => {
                let size = self.decoded_object_size(entry.decompressed_size)?;
                if let Some(additional) = size.checked_sub(out.len()) {
                    out.try_reserve(additional).map_err(|_| GitError::Gen)?;
                }
                out.resize(size, 0);
                self.decompress_entry(&entry, inflate, out.as_mut_slice())
                    .map(|consumed_input| {
                        PackOutcome::from_object_entry(
                            entry.header.as_kind().expect("a non-delta entry"),
                            &entry,
                            consumed_input,
                        )
                    })
            }
            Header::OfsDelta { .. } | Header::RefDelta { .. } => {
                self.resolve_deltas(entry, resolve, inflate, out, delta_cache)
            }
        }
    }

    fn resolve_deltas(
        &self,
        last: DataEntry,
        resolve: &dyn Fn(&oid, &mut Vec<u8>) -> Option<DataDecodeEntryResolvedBase>,
        inflate: &mut zlib::Inflate,
        out: &mut Vec<u8>,
        cache: &mut dyn TraitDecodeEntry,
    ) -> Result<PackOutcome, GitError> {
        let mut chain = SmallVec::<[Delta; 10]>::default();
        let first_entry = last.clone();
        let mut cursor = last;
        let mut base_buffer_size: Option<usize> = None;
        let mut object_kind: Option<ObjectKind> = None;
        let mut consumed_input: Option<usize> = None;

        let mut total_delta_data_size: u64 = 0;
        while cursor.header.is_delta() {
            if let Some((kind, packed_size)) = cache.get(self.id, cursor.data_offset, out) {
                base_buffer_size = Some(out.len());
                object_kind = Some(kind);
                if total_delta_data_size == 0 {
                    consumed_input = Some(packed_size);
                }
                break;
            }
            total_delta_data_size = total_delta_data_size
                .checked_add(cursor.decompressed_size)
                .ok_or(GitError::Gen)?;

            let decompressed_size = self.decoded_object_size(cursor.decompressed_size)?;
            chain.push(Delta {
                data: Range {
                    start: 0,
                    end: decompressed_size,
                },
                base_size: 0,
                result_size: 0,
                decompressed_size,
                data_offset: cursor.data_offset,
            });
            cursor = match cursor.header {
                Header::OfsDelta { base_distance } => self.entry(
                    cursor
                        .checked_base_pack_offset(base_distance)
                        .ok_or(GitError::Gen)?,
                )?,
                Header::RefDelta { base_id } => match resolve(base_id.as_ref(), out) {
                    Some(DataDecodeEntryResolvedBase::InPack(entry)) => entry,
                    Some(DataDecodeEntryResolvedBase::OutOfPack { end, kind }) => {
                        base_buffer_size = Some(end);
                        object_kind = Some(kind);
                        break;
                    }
                    None => {
                        return Err(GitError::DeltaBaseUnresolved(base_id));
                    }
                },
                _ => unreachable!("cursor.is_delta() only allows deltas here"),
            };
        }

        if chain.is_empty() {
            return Ok(PackOutcome::from_object_entry(
                object_kind.expect("object kind as set by cache"),
                &first_entry,
                consumed_input.expect("consumed bytes as set by cache"),
            ));
        }

        let total_delta_data_size: usize = total_delta_data_size
            .try_into()
            .map_err(|_| GitError::Gen)?;

        let chain_len = chain.len();
        let (first_buffer_end, second_buffer_end) = {
            let delta_start = base_buffer_size.unwrap_or(0);

            let delta_range = Range {
                start: delta_start,
                end: delta_start
                    .checked_add(total_delta_data_size)
                    .ok_or(GitError::Gen)?,
            };
            out.try_reserve(delta_range.end.saturating_sub(out.len()))
                .map_err(|_| GitError::Gen)?;
            out.resize(delta_range.end, 0);

            let mut instructions = &mut out[delta_range.clone()];
            let mut relative_delta_start = 0;
            let mut biggest_result_size = 0;
            for (delta_idx, delta) in chain.iter_mut().rev().enumerate() {
                let (consumed_from_data_offset, consumed_out) = self
                    .decompress_complete_entry_from_data_offset(
                        delta.data_offset,
                        inflate,
                        &mut instructions[..delta.decompressed_size],
                    )?;
                let is_last_delta_to_be_applied = delta_idx + 1 == chain_len;
                if is_last_delta_to_be_applied {
                    consumed_input = Some(consumed_from_data_offset);
                }

                let current_delta = &instructions[..consumed_out];
                let (base_size, offset) = decode_header_size(current_delta)?;
                let mut bytes_consumed_by_header = offset;
                biggest_result_size = biggest_result_size.max(base_size);
                delta.base_size = self.decoded_object_size(base_size)?;

                let (result_size, offset) = decode_header_size(&current_delta[offset..])?;
                bytes_consumed_by_header += offset;
                biggest_result_size = biggest_result_size.max(result_size);
                delta.result_size = self.decoded_object_size(result_size)?;

                delta.data.start = relative_delta_start + bytes_consumed_by_header;
                delta.data.end = relative_delta_start + consumed_out;
                relative_delta_start += delta.decompressed_size;

                instructions = &mut instructions[delta.decompressed_size..];
            }

            if base_buffer_size.is_none() {
                biggest_result_size = biggest_result_size.max(cursor.decompressed_size);
            }
            let biggest_result_size = self.decoded_object_size(biggest_result_size)?;
            let first_buffer_size = biggest_result_size;
            let second_buffer_size = first_buffer_size;
            let out_size = first_buffer_size
                .checked_add(second_buffer_size)
                .and_then(|size| size.checked_add(total_delta_data_size))
                .ok_or(GitError::Gen)?;
            out.try_reserve(out_size.saturating_sub(out.len()))
                .map_err(|_| GitError::Gen)?;
            out.resize(out_size, 0);

            let second_buffer_end = {
                let end = first_buffer_size
                    .checked_add(second_buffer_size)
                    .ok_or(GitError::Gen)?;
                out.copy_within(delta_range, end);
                end
            };

            if base_buffer_size.is_none() {
                let base_entry = cursor;

                object_kind = base_entry.header.as_kind();
                let base_size = self.decoded_object_size(base_entry.decompressed_size)?;
                let out_base = &mut out[..base_size];
                self.decompress_entry_from_data_offset(base_entry.data_offset, inflate, out_base)?;
            }

            (first_buffer_size, second_buffer_end)
        };

        let (buffers, instructions) = out.split_at_mut(second_buffer_end);
        let (mut source_buf, mut target_buf) = buffers.split_at_mut(first_buffer_end);

        let mut last_result_size = None;
        for (
            delta_idx,
            Delta {
                data,
                base_size,
                result_size,
                ..
            },
        ) in chain.into_iter().rev().enumerate()
        {
            let data = &mut instructions[data];
            if delta_idx + 1 == chain_len {
                last_result_size = Some(result_size);
            }
            apply(
                &source_buf[..base_size],
                &mut target_buf[..result_size],
                data,
            )?;
            std::mem::swap(&mut source_buf, &mut target_buf);
        }

        let last_result_size = last_result_size.expect("at least one delta chain item");
        if chain_len % 2 == 1 {
            target_buf[..last_result_size].copy_from_slice(&source_buf[..last_result_size]);
        }

        out.truncate(last_result_size);

        let object_kind = object_kind
            .expect("a base object as root of any delta chain that we are here to resolve");
        let consumed_input = consumed_input.expect("at least one decompressed delta object");
        cache.put(
            self.id,
            first_entry.data_offset,
            out.as_slice(),
            object_kind,
            consumed_input,
        );
        Ok(PackOutcome {
            kind: object_kind,
            num_deltas: chain_len as u32,
            decompressed_size: first_entry.decompressed_size,
            compressed_size: consumed_input,
            object_size: last_result_size as u64,
        })
    }
}

fn decoded_object_size(size: u64) -> Result<usize, GitError> {
    let size: usize = size.try_into().map_err(|_| GitError::Gen)?;
    Ok(size)
}

impl DataFile<MMap> {
    fn at(path: impl AsRef<Path>) -> Result<Self, GitError> {
        Self::at_inner(path.as_ref())
    }

    fn at_inner(path: &Path) -> Result<Self, GitError> {
        let data = mmap_read_only(path).map_err(|_| GitError::Gen)?;
        Self::from_data(data, path.to_owned())
    }
}

impl<T> DataFile<T>
where
    T: TraitFileData,
{
    fn from_data(data: T, path: PathBuf) -> Result<Self, GitError> {
        let hash_len = 20;
        let pack_len = data.len();
        let id = hash::crc32(path.as_os_str().to_string_lossy().as_bytes());
        if pack_len < N32_SIZE * 3 + hash_len {
            return Err(GitError::Gen);
        }
        let (_kind, _num_objects) = decode_header(
            &data[..12]
                .try_into()
                .expect("enough data after previous check"),
        )?;
        Ok(Self { data, id, hash_len })
    }
}

const COMMIT: u8 = 1;
const TREE: u8 = 2;
const BLOB: u8 = 3;
const OFS_DELTA: u8 = 6;
const REF_DELTA: u8 = 7;

impl DataEntry {
    #[must_use]
    fn checked_base_pack_offset(&self, distance: u64) -> Option<Offset> {
        Header::verified_base_pack_offset(self.pack_offset(), distance)
    }

    #[must_use]
    fn pack_offset(&self) -> Offset {
        self.data_offset - self.header_size() as u64
    }
    #[must_use]
    fn header_size(&self) -> usize {
        if self.encoded_header_size == 0 {
            self.header.size(self.decompressed_size)
        } else {
            self.encoded_header_size.into()
        }
    }
}

impl DataEntry {
    fn from_bytes(d: &[u8], pack_offset: Offset, hash_len: usize) -> Result<DataEntry, GitError> {
        let (type_id, size, mut consumed) = parse_header_info(d)?;

        let object = match type_id {
            OFS_DELTA => {
                let (distance, leb_bytes) = parse_leb64(&d[consumed..])?;
                let delta = OfsDelta {
                    base_distance: distance,
                };
                consumed += leb_bytes;
                delta
            }
            REF_DELTA => {
                let delta = RefDelta {
                    base_id: ObjectId::from_bytes_or_panic(
                        d.get(consumed..consumed + hash_len).ok_or(GitError::Gen)?,
                    ),
                };
                consumed += hash_len;
                delta
            }
            BLOB => Header::Blob,
            TREE => Header::Tree,
            COMMIT => Header::Commit,

            _ => return Err(GitError::Gen),
        };
        Ok(DataEntry {
            header: object,
            decompressed_size: size,
            data_offset: pack_offset + consumed as u64,
            encoded_header_size: consumed
                .try_into()
                .expect("pack entry headers fit into u16"),
        })
    }
}

#[inline]
fn parse_header_info(data: &[u8]) -> Result<(u8, u64, usize), GitError> {
    let mut c = *data.first().ok_or(GitError::Gen)?;
    let mut i = 1;
    let type_id = (c >> 4) & 0b0000_0111;
    let mut size = u64::from(c) & 0b0000_1111;
    let mut shift = 4u32;
    while c & 0b1000_0000 != 0 {
        c = *data.get(i).ok_or(GitError::Gen)?;
        i += 1;
        let component = u64::from(c & 0b0111_1111)
            .checked_shl(shift)
            .ok_or(GitError::Gen)?;
        size = size.checked_add(component).ok_or(GitError::Gen)?;
        shift += 7;
    }
    Ok((type_id, size, i))
}

fn parse_leb64(data: &[u8]) -> Result<(u64, usize), GitError> {
    let mut i = 0;
    let mut c = *data.first().ok_or(GitError::Gen)?;
    i += 1;
    let mut value = u64::from(c) & 0x7f;
    while c & 0x80 != 0 {
        c = *data.get(i).ok_or(GitError::Gen)?;
        i += 1;
        value = value
            .checked_add(1)
            .and_then(|value| value.checked_shl(7))
            .and_then(|value| value.checked_add(u64::from(c) & 0x7f))
            .ok_or(GitError::Gen)?;
    }
    Ok((value, i))
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
enum Header {
    Commit,
    Tree,
    Blob,
    RefDelta { base_id: ObjectId },
    OfsDelta { base_distance: u64 },
}

impl Header {
    #[must_use]
    fn verified_base_pack_offset(pack_offset: Offset, distance: u64) -> Option<Offset> {
        if distance == 0 {
            return None;
        }
        pack_offset.checked_sub(distance)
    }
    #[must_use]
    fn as_kind(&self) -> Option<ObjectKind> {
        Some(match self {
            Header::Tree => ObjectKind::Tree,
            Header::Blob => ObjectKind::Blob,
            Header::Commit => ObjectKind::Commit,
            Header::RefDelta { .. } | Header::OfsDelta { .. } => return None,
        })
    }
    #[must_use]
    fn as_type_id(&self) -> u8 {
        use Header::{Blob, Commit, OfsDelta, RefDelta, Tree};
        match self {
            Blob => BLOB,
            Tree => TREE,
            Commit => COMMIT,
            OfsDelta { .. } => OFS_DELTA,
            RefDelta { .. } => REF_DELTA,
        }
    }
    #[must_use]
    fn is_delta(&self) -> bool {
        matches!(self, Header::OfsDelta { .. } | Header::RefDelta { .. })
    }
}

impl Header {
    fn write_to(
        &self,
        decompressed_size_in_bytes: u64,
        out: &mut dyn io::Write,
    ) -> io::Result<usize> {
        let mut size = decompressed_size_in_bytes;
        let mut written = 1;
        let mut c: u8 = (self.as_type_id() << 4) | (size as u8 & 0b0000_1111);
        size >>= 4;
        while size != 0 {
            out.write_all(&[c | 0b1000_0000])?;
            written += 1;
            c = size as u8 & 0b0111_1111;
            size >>= 7;
        }
        out.write_all(&[c])?;

        use Header::{Blob, Commit, OfsDelta, RefDelta, Tree};
        match self {
            RefDelta { base_id: oid } => {
                out.write_all(oid.as_slice())?;
                written += oid.as_slice().len();
            }
            OfsDelta { base_distance } => {
                let mut buf = [0u8; 10];
                let buf = leb64_encode(*base_distance, &mut buf);
                out.write_all(buf)?;
                written += buf.len();
            }
            Blob | Tree | Commit => {}
        }
        Ok(written)
    }

    #[must_use]
    fn size(&self, decompressed_size: u64) -> usize {
        self.write_to(decompressed_size, &mut io::sink())
            .expect("io::sink() to never fail")
    }
}

#[inline]
fn leb64_encode(mut n: u64, buf: &mut [u8; 10]) -> &[u8] {
    let mut bytes_written = 1;
    buf[buf.len() - 1] = n as u8 & 0b0111_1111;
    for out in buf.iter_mut().rev().skip(1) {
        n >>= 7;
        if n == 0 {
            break;
        }
        n -= 1;
        *out = 0b1000_0000 | (n as u8 & 0b0111_1111);
        bytes_written += 1;
    }

    &buf[buf.len() - bytes_written..]
}

fn decode_header(data: &[u8; 12]) -> Result<(PackVersion, u32), GitError> {
    let mut ofs = 0;
    if &data[ofs..ofs + b"PACK".len()] != b"PACK" {
        return Err(GitError::Gen);
    }
    ofs += N32_SIZE;
    let kind = match read_u32(&data[ofs..ofs + N32_SIZE]) {
        2 => PackVersion::V2,
        3 => PackVersion::V3,
        _ => return Err(GitError::Gen),
    };
    ofs += N32_SIZE;
    let num_objects = read_u32(&data[ofs..ofs + N32_SIZE]);

    Ok((kind, num_objects))
}

#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
enum PackVersion {
    #[default]
    V2,
    V3,
}

struct DataFile<T = MMap> {
    data: T,
    id: FilePackId,
    hash_len: usize,
}

fn decode_header_size(d: &[u8]) -> Result<(u64, usize), GitError> {
    let mut shift = 0;
    let mut size = 0u64;
    let mut consumed = 0;
    for cmd in d {
        if shift >= u64::BITS {
            return Err(GitError::Gen);
        }
        consumed += 1;
        size |= (u64::from(*cmd) & 0x7f) << shift;
        shift += 7;
        if *cmd & 0x80 == 0 {
            return Ok((size, consumed));
        }
    }
    Err(GitError::Gen)
}

fn apply(base: &[u8], mut target: &mut [u8], data: &[u8]) -> Result<(), GitError> {
    fn next_byte(data: &[u8], i: &mut usize) -> Result<u8, GitError> {
        let byte = *data.get(*i).ok_or(GitError::Gen)?;
        *i += 1;
        Ok(byte)
    }

    let mut i = 0;
    while let Some(cmd) = data.get(i) {
        i += 1;
        match cmd {
            cmd if cmd & 0b1000_0000 != 0 => {
                let (mut ofs, mut size): (u32, u32) = (0, 0);
                if cmd & 0b0000_0001 != 0 {
                    ofs = u32::from(next_byte(data, &mut i)?);
                }
                if cmd & 0b0000_0010 != 0 {
                    ofs |= u32::from(next_byte(data, &mut i)?) << 8;
                }
                if cmd & 0b0000_0100 != 0 {
                    ofs |= u32::from(next_byte(data, &mut i)?) << 16;
                }
                if cmd & 0b0000_1000 != 0 {
                    ofs |= u32::from(next_byte(data, &mut i)?) << 24;
                }
                if cmd & 0b0001_0000 != 0 {
                    size = u32::from(next_byte(data, &mut i)?);
                }
                if cmd & 0b0010_0000 != 0 {
                    size |= u32::from(next_byte(data, &mut i)?) << 8;
                }
                if cmd & 0b0100_0000 != 0 {
                    size |= u32::from(next_byte(data, &mut i)?) << 16;
                }
                if size == 0 {
                    size = 0x10000; // 65536
                }
                let ofs = ofs as usize;
                let end = ofs.checked_add(size as usize).ok_or(GitError::Gen)?;
                std::io::Write::write(&mut target, base.get(ofs..end).ok_or(GitError::Gen)?)
                    .map_err(|_| GitError::Gen)?;
            }
            0 => {
                return Err(GitError::Gen);
            }
            size => {
                let end = i.checked_add(*size as usize).ok_or(GitError::Gen)?;
                std::io::Write::write(&mut target, data.get(i..end).ok_or(GitError::Gen)?)
                    .map_err(|_| GitError::Gen)?;
                i = end;
            }
        }
    }

    if !target.is_empty() {
        return Err(GitError::Gen);
    }

    Ok(())
}

impl IndexMode {
    #[must_use]
    fn hx_is_sparse(&self) -> bool {
        *self == Self::DIR
    }

    #[must_use]
    fn hx_is_submodule(&self) -> bool {
        *self == Self::DIR | Self::SYMLINK
    }
}

impl From<EntryMode> for IndexMode {
    fn from(value: EntryMode) -> Self {
        let value: u16 = value.value();
        Self::from_bits_truncate(u32::from(value))
    }
}

impl TryFrom<SystemTime> for Time {
    type Error = SystemTimeError;
    fn try_from(s: SystemTime) -> Result<Self, SystemTimeError> {
        let d = s.duration_since(std::time::UNIX_EPOCH)?;
        Ok(Time {
            secs: d.as_secs() as u32,
            nsecs: d.subsec_nanos(),
        })
    }
}

impl From<Time> for SystemTime {
    fn from(s: Time) -> Self {
        std::time::UNIX_EPOCH + std::time::Duration::new(s.secs.into(), s.nsecs)
    }
}

#[derive(Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd, Clone, Copy)]
struct Time {
    secs: u32,
    nsecs: u32,
}

impl From<FileTime> for Time {
    fn from(value: FileTime) -> Self {
        Time {
            secs: value
                .unix_seconds()
                .try_into()
                .expect("can't represent non-unix times"),
            nsecs: value.nanoseconds(),
        }
    }
}

impl PartialEq<FileTime> for Time {
    fn eq(&self, other: &FileTime) -> bool {
        *self == Time::from(*other)
    }
}

impl PartialOrd<FileTime> for Time {
    fn partial_cmp(&self, other: &FileTime) -> Option<cmp::Ordering> {
        self.partial_cmp(&Time::from(*other))
    }
}

//
// use bitflags::bitflags;
//
// bitflags! {
//     #[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
//     struct Mode: u32 {
//         const DIR = 0o040000;
//         const FILE = 0o100644;
//         const FILE_EXECUTABLE = 0o100755;
//         const SYMLINK = 0o120000;
//         const COMMIT = 0o160000;
//     }
// }

#[derive(Debug, Clone)]
struct Driver {
    name: BString,
}

fn hx_apply(src: &[u8], buf: &mut Vec<u8>) -> Result<bool, GitError> {
    const HASH_LEN: usize = ": ".len() + 40;
    let mut id = None;
    let mut ofs = 0;
    while let Some(pos) = src[ofs..].find(b"$Id$") {
        let id = match id {
            None => {
                let new_id = hx_compute_hash(ObjectKind::Blob, src)?;
                id = new_id.into();
                // pre-allocate for one ID
                clear_and_set_capacity(buf, src.len() + HASH_LEN)?;
                new_id
            }
            Some(id) => id,
        };

        buf.push_str(&src[ofs..][..pos + 3]);
        buf.push_str(b": ");
        id.write_hex_to(&mut *buf)
            .expect("writes to memory always work");
        buf.push(b'$');

        ofs += pos + 4;
    }
    if id.is_some() {
        buf.push_str(&src[ofs..]);
    }
    Ok(id.is_some())
}

fn eol_convert_to_worktree(
    src: &[u8],
    digest: AttributesDigest,
    buf: &mut Vec<u8>,
    config: EolConfiguration,
) -> Result<bool, GitError> {
    let stats = Stats::from_bytes(src);

    if src.is_empty()
        || digest.to_eol(config) != Some(EolMode::CrLf)
        || !stats.will_convert_lf_to_crlf(digest, config)
    {
        return Ok(false);
    }

    clear_and_set_capacity(buf, src.len() + stats.lone_lf)?;

    let mut ofs = 0;
    while let Some(pos) = src[ofs..].find_byteset(b"\r\n") {
        match src[ofs + pos] {
            b'\r' => {
                if src.get(ofs + pos + 1) == Some(&b'\n') {
                    buf.push_str(&src[ofs..][..pos + 2]);
                    ofs += pos + 2;
                } else {
                    buf.push_str(&src[ofs..][..=pos]);
                    ofs += pos + 1;
                }
            }
            b'\n' => {
                buf.push_str(&src[ofs..][..pos]);
                buf.push_str(b"\r\n");
                ofs += pos + 1;
            }
            _ => unreachable!("would only find one of two possible values"),
        }
    }
    buf.push_str(&src[ofs..]);
    Ok(true)
}

impl AttributesDigest {
    #[must_use]
    fn to_eol(&self, config: EolConfiguration) -> Option<EolMode> {
        Some(match self {
            AttributesDigest::Binary => return None,
            AttributesDigest::TextInput | AttributesDigest::TextAutoInput => EolMode::Lf,
            AttributesDigest::TextCrlf | AttributesDigest::TextAutoCrlf => EolMode::CrLf,
            AttributesDigest::Text | AttributesDigest::TextAuto => match &config.auto_crlf {
                EolAutoCrlf::Enabled => EolMode::CrLf,
                EolAutoCrlf::Input | EolAutoCrlf::Disabled => EolMode::Lf,
            },
        })
    }

    #[must_use]
    fn is_auto_text(&self) -> bool {
        matches!(
            self,
            AttributesDigest::TextAuto
                | AttributesDigest::TextAutoCrlf
                | AttributesDigest::TextAutoInput
        )
    }
}

impl EolConfiguration {
    #[must_use]
    fn to_eol(&self) -> EolMode {
        match self.auto_crlf {
            EolAutoCrlf::Enabled => EolMode::CrLf,
            EolAutoCrlf::Input | EolAutoCrlf::Disabled => EolMode::Lf,
        }
    }
}

impl Stats {
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut bytes = bytes.iter().peekable();
        let mut null = 0;
        let mut lone_cr = 0;
        let mut lone_lf = 0;
        let mut crlf = 0;
        let mut printable = 0;
        let mut non_printable = 0;
        while let Some(b) = bytes.next() {
            if *b == b'\r' {
                match bytes.peek() {
                    Some(n) if **n == b'\n' => {
                        bytes.next();
                        crlf += 1;
                    }
                    _ => lone_cr += 1,
                }
                continue;
            }
            if *b == b'\n' {
                lone_lf += 1;
                continue;
            }
            if *b == 127 {
                non_printable += 1;
            } else if *b < 32 {
                match *b {
          8 /* \b */ | b'\t' | 27 /* \033 */ | 12 /* \014 */ => printable += 1,
          0 => {
            non_printable += 1;
            null += 1;
          }
          _ => non_printable += 1,
        }
            } else {
                printable += 1;
            }
        }

        Self {
            null,
            lone_cr,
            lone_lf,
            crlf,
            printable,
            non_printable,
        }
    }

    fn is_binary(&self) -> bool {
        self.lone_cr > 0 || self.null > 0 || (self.printable >> 7) < self.non_printable
    }

    fn will_convert_lf_to_crlf(&self, digest: AttributesDigest, config: EolConfiguration) -> bool {
        if digest.to_eol(config) != Some(EolMode::CrLf)
            || self.lone_lf == 0
            || digest.is_auto_text() && (self.is_binary() || self.lone_cr > 0 || self.crlf > 0)
        {
            return false;
        }
        true
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum EolMode {
    Lf,
    CrLf,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
enum EolAutoCrlf {
    Input,
    Enabled,
    #[default]
    Disabled,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum AttributesDigest {
    Binary,
    Text,
    TextInput,
    TextCrlf,
    TextAuto,
    TextAutoCrlf,
    TextAutoInput,
}

impl From<EolMode> for AttributesDigest {
    fn from(value: EolMode) -> Self {
        match value {
            EolMode::Lf => AttributesDigest::TextInput,
            EolMode::CrLf => AttributesDigest::TextCrlf,
        }
    }
}

impl From<EolAutoCrlf> for AttributesDigest {
    fn from(value: EolAutoCrlf) -> Self {
        match value {
            EolAutoCrlf::Input => AttributesDigest::TextAutoInput,
            EolAutoCrlf::Enabled => AttributesDigest::TextAutoCrlf,
            EolAutoCrlf::Disabled => AttributesDigest::Binary,
        }
    }
}

#[derive(Default, Debug, Copy, Clone)]
struct EolConfiguration {
    auto_crlf: EolAutoCrlf,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct Stats {
    null: usize,
    lone_cr: usize,
    lone_lf: usize,
    crlf: usize,
    printable: usize,
    non_printable: usize,
}

fn encode_to_worktree(
    src_utf8: &[u8],
    worktree_encoding: &'static encoding_rs::Encoding,
    buf: &mut Vec<u8>,
) -> Result<(), GitError> {
    let mut encoder = worktree_encoding.new_encoder();
    let buf_len = encoder
        .max_buffer_length_from_utf8_if_no_unmappables(src_utf8.len())
        .ok_or(GitError::Gen)?;
    buf.clear();
    buf.resize(buf_len, 0);
    let src = std::str::from_utf8(src_utf8).map_err(|_| GitError::Gen)?;
    let (res, _read, written) = encoder.encode_from_utf8_without_replacement(src, buf, true);
    match res {
        EncoderResult::InputEmpty => {
            buf.truncate(written);
        }
        EncoderResult::OutputFull => {
            unreachable!(
                "we assure that the output buffer is big enough as per the encoder's estimate"
            )
        }
        EncoderResult::Unmappable(_) => {
            return Err(GitError::Gen);
        }
    }
    Ok(())
}

pub enum MaybeDelayed<'a> {
    Immediate(Box<dyn std::io::Read + 'a>),
}

struct ReadFilterOutput;

impl std::io::Read for ReadFilterOutput {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

#[derive(Default)]
struct CommandState {
    context: GixCommandContext,
}

impl CommandState {
    fn new(context: GixCommandContext) -> Self {
        Self { context }
    }
}

impl CommandState {
    fn hx_apply_delayed(&mut self) -> Result<Option<MaybeDelayed<'_>>, GitError> {
        Ok(Some(MaybeDelayed::Immediate(Box::new(ReadFilterOutput))))
    }
}

impl Clone for CommandState {
    fn clone(&self) -> Self {
        CommandState {
            context: self.context.clone(),
        }
    }
}

#[derive(Default, Clone)]
struct FilterPipelineOptions {
    drivers: Vec<Driver>,
    eol_config: EolConfiguration,
}

impl FilterPipeline {
    #[must_use]
    fn new(options: FilterPipelineOptions) -> Self {
        let mut attrs = search::Outcome::default();
        attrs.initialize_with_selection(&Default::default(), ATTRS);
        FilterPipeline {
            attrs,
            processes: CommandState::new(GixCommandContext {}),
            options,
            bufs: Default::default(),
        }
    }
}

impl FilterPipeline {
    // helix
    fn hx_convert_to_worktree<'input>(
        &mut self,
        src: &'input [u8],
        rela_path: &BStr,
        attributes: &mut dyn FnMut(&BStr, &mut search::Outcome),
    ) -> Result<ToWorktreeOutcome<'input, '_>, GitError> {
        let Configuration {
            driver,
            digest,
            encoding,
            apply_ident_filter,
        } = Configuration::at_path(
            rela_path,
            &self.options.drivers,
            &mut self.attrs,
            attributes,
            self.options.eol_config,
        )?;

        let mut bufs = self.bufs.use_foreign_src(src);
        let (src, dest) = bufs.src_and_dest();
        if apply_ident_filter && hx_apply(src, dest)? {
            bufs.swap();
        }

        let (src, dest) = bufs.src_and_dest();
        if eol_convert_to_worktree(src, digest, dest, self.options.eol_config)? {
            bufs.swap();
        }

        if let Some(encoding) = encoding {
            let (src, dest) = bufs.src_and_dest();
            encode_to_worktree(src, encoding, dest)?;
            bufs.swap();
        }

        if let Some(_driver) = driver
            && let Some(maybe_delayed) = self.processes.hx_apply_delayed()?
        {
            return Ok(ToWorktreeOutcome::Process(maybe_delayed));
        }

        Ok(match bufs.ro_src {
            Some(src) => ToWorktreeOutcome::Unchanged(src),
            None => ToWorktreeOutcome::Buffer(bufs.src),
        })
    }
}

pub enum ToWorktreeOutcome<'input, 'pipeline> {
    Unchanged(&'input [u8]),
    Buffer(&'pipeline [u8]),
    Process(MaybeDelayed<'pipeline>),
}

impl std::io::Read for ToWorktreeOutcome<'_, '_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            ToWorktreeOutcome::Unchanged(b) => b.read(buf),
            ToWorktreeOutcome::Buffer(b) => b.read(buf),
            ToWorktreeOutcome::Process(MaybeDelayed::Immediate(r)) => r.read(buf),
        }
    }
}

struct Configuration<'a> {
    driver: Option<&'a Driver>,
    digest: AttributesDigest,
    encoding: Option<&'static encoding_rs::Encoding>,
    apply_ident_filter: bool,
}

impl<'driver> Configuration<'driver> {
    fn at_path(
        rela_path: &BStr,
        drivers: &'driver [Driver],
        attrs: &mut search::Outcome,
        attributes: &mut dyn FnMut(&BStr, &mut search::Outcome),
        config: EolConfiguration,
    ) -> Result<Configuration<'driver>, GitError> {
        fn extract_driver<'a>(
            drivers: &'a [Driver],
            attr: &search::Match<'_>,
        ) -> Option<&'a Driver> {
            if let StateRef::Value(name) = attr.assignment.state {
                drivers.iter().find(|d| d.name == name.as_bstr())
            } else {
                None
            }
        }

        fn extract_encoding(
            attr: &search::Match<'_>,
        ) -> Result<Option<&'static encoding_rs::Encoding>, GitError> {
            match attr.assignment.state {
                StateRef::Set | StateRef::Unset => Err(GitError::Gen),
                StateRef::Value(name) => encoding_rs::Encoding::for_label(name.as_bstr())
                    .ok_or(GitError::Gen)
                    .map(|encoding| {
                        if encoding == encoding_rs::UTF_8 {
                            None
                        } else {
                            Some(encoding)
                        }
                    }),
                StateRef::Unspecified => Ok(None),
            }
        }

        fn extract_crlf(attr: &search::Match<'_>) -> Option<AttributesDigest> {
            match attr.assignment.state {
                StateRef::Unspecified => None,
                StateRef::Set => Some(AttributesDigest::Text),
                StateRef::Unset => Some(AttributesDigest::Binary),
                StateRef::Value(v) => {
                    if v.as_bstr() == "input" {
                        Some(AttributesDigest::TextInput)
                    } else if v.as_bstr() == "auto" {
                        Some(AttributesDigest::TextAuto)
                    } else {
                        None
                    }
                }
            }
        }

        fn extract_eol(attr: &search::Match<'_>) -> Option<EolMode> {
            match attr.assignment.state {
                StateRef::Unspecified | StateRef::Unset | StateRef::Set => None,
                StateRef::Value(v) => {
                    if v.as_bstr() == "lf" {
                        Some(EolMode::Lf)
                    } else if v.as_bstr() == "crlf" {
                        Some(EolMode::CrLf)
                    } else {
                        None
                    }
                }
            }
        }

        attributes(rela_path, attrs);
        let attrs: SmallVec<[_; ATTRS.len()]> = attrs.iter_selected().collect();
        let apply_ident_filter = attrs[1].assignment.state.is_set();
        let driver = extract_driver(drivers, &attrs[2]);
        let encoding = extract_encoding(&attrs[5])?;

        let mut digest = extract_crlf(&attrs[4]);
        if digest.is_none() {
            digest = extract_crlf(&attrs[0]);
        }

        if digest != Some(AttributesDigest::Binary) {
            let eol = extract_eol(&attrs[3]);
            digest = match digest {
                Some(AttributesDigest::TextAuto) if eol == Some(EolMode::Lf) => {
                    Some(AttributesDigest::TextAutoInput)
                }
                Some(AttributesDigest::TextAuto) if eol == Some(EolMode::CrLf) => {
                    Some(AttributesDigest::TextAutoCrlf)
                }
                _ => match eol {
                    Some(EolMode::CrLf) => Some(AttributesDigest::TextCrlf),
                    Some(EolMode::Lf) => Some(AttributesDigest::TextInput),
                    _ => digest,
                },
            };
        }

        let digest: Option<AttributesDigest> = match digest {
            None => Some(config.auto_crlf.into()),

            Some(AttributesDigest::Text) => Some(config.to_eol().into()),
            _ => digest,
        };

        Ok(Configuration {
            driver,
            digest: digest.expect("always set by now"),
            encoding,
            apply_ident_filter,
        })
    }
}

#[derive(Clone)]
struct FilterPipeline {
    options: FilterPipelineOptions,
    attrs: search::Outcome,
    processes: CommandState,
    bufs: Buffers,
}

fn clear_and_set_capacity(buf: &mut Vec<u8>, cap: usize) -> Result<(), GitError> {
    buf.clear();
    if buf.capacity() < cap {
        buf.try_reserve(cap).map_err(|_| GitError::Gen)?;
    }
    Ok(())
}

const STAR: u8 = b'*';
const BACKSLASH: u8 = b'\\';
const SLASH: u8 = b'/';
const BRACKET_OPEN: u8 = b'[';
const BRACKET_CLOSE: u8 = b']';
const COLON: u8 = b':';
const NEGATE_CLASS: u8 = b'!';
const RECURSION_LIMIT: usize = 64;

use kstring::{KString, KStringRef};

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
    pub struct IndexMode: u32 {
        const DIR = 0o040000;
        const FILE = 0o100644;
        const FILE_EXECUTABLE = 0o100755;
        const SYMLINK = 0o120000;
        const COMMIT = 0o160000;
    }
}

mod sec {
    use std::fmt::{Display, Formatter};

    bitflags::bitflags! {

        #[derive(Debug)]
        struct ReadWrite: u8 {
            const READ = 1 << 0;
            const WRITE = 1 << 1;
        }
    }

    impl Display for ReadWrite {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            std::fmt::Debug::fmt(self, f)
        }
    }
}

fn os_str_into_bstr(path: &OsStr) -> Result<&BStr, GitError> {
    let path = try_into_bstr(Cow::Borrowed(path.as_ref()))?;
    match path {
        Cow::Borrowed(path) => Ok(path),
        Cow::Owned(_) => unreachable!("borrowed cows stay borrowed"),
    }
}

fn os_string_into_bstring(path: OsString) -> Result<BString, GitError> {
    let path = try_into_bstr(Cow::Owned(path.into()))?;
    match path {
        Cow::Borrowed(_path) => unreachable!("borrowed cows stay borrowed"),
        Cow::Owned(path) => Ok(path),
    }
}

fn try_into_bstr<'a>(path: impl Into<Cow<'a, Path>>) -> Result<Cow<'a, BStr>, GitError> {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let path = path.into();
    let path_str = match path {
        Cow::Owned(path) => Cow::Owned({
            let p: BString = { path.into_os_string().into_vec().into() };
            p
        }),
        Cow::Borrowed(path) => Cow::Borrowed({
            let p: &BStr = { path.as_os_str().as_bytes().into() };
            p
        }),
    };
    Ok(path_str)
}

fn into_bstr<'a>(path: impl Into<Cow<'a, Path>>) -> Cow<'a, BStr> {
    try_into_bstr(path).expect("prefix path doesn't contain ill-formed UTF-8")
}

fn hx_try_from_byte_slice(input: &[u8]) -> Result<&Path, GitError> {
    let p = {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        OsStr::from_bytes(input).as_ref()
    };

    Ok(p)
}

fn try_from_bstr<'a>(input: impl Into<Cow<'a, BStr>>) -> Result<Cow<'a, Path>, GitError> {
    let input = input.into();
    match input {
        Cow::Borrowed(input) => hx_try_from_byte_slice(input).map(Cow::Borrowed),
        Cow::Owned(input) => hx_try_from_bstring(input).map(Cow::Owned),
    }
}

fn from_bstr<'a>(input: impl Into<Cow<'a, BStr>>) -> Cow<'a, Path> {
    try_from_bstr(input).expect("prefix path doesn't contain ill-formed UTF-8")
}

fn hx_try_from_bstring(input: impl Into<BString>) -> Result<PathBuf, GitError> {
    let input = input.into();
    #[cfg(unix)]
    let p = {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(input.into()).into()
    };
    Ok(p)
}

fn from_bstring(input: impl Into<BString>) -> PathBuf {
    hx_try_from_bstring(input).expect("well-formed UTF-8 on windows")
}

#[must_use]
fn from_byte_slice(input: &[u8]) -> &Path {
    hx_try_from_byte_slice(input).expect("well-formed UTF-8 on windows")
}

fn to_native_path_on_windows<'a>(path: impl Into<Cow<'a, BStr>>) -> Cow<'a, std::path::Path> {
    from_bstr(path)
}

fn hx_to_unix_separators_on_windows<'a>(path: impl Into<Cow<'a, BStr>>) -> Cow<'a, BStr> {
    path.into()
}

#[must_use]
fn hx_normalize<'a>(path: Cow<'a, Path>, current_dir: &Path) -> Option<Cow<'a, Path>> {
    use std::path::Component::ParentDir;

    if !path.components().any(|c| matches!(c, ParentDir)) {
        return Some(path);
    }
    let mut current_dir_opt = Some(current_dir);
    let was_relative = path.is_relative();
    let components = path.components();
    let mut path = PathBuf::new();
    for component in components {
        if let ParentDir = component {
            let path_was_dot = path == Path::new(".");
            if path.as_os_str().is_empty() || path_was_dot {
                path.push(current_dir_opt.take()?);
            }
            if !path.pop() {
                return None;
            }
        } else {
            path.push(component);
        }
    }

    if (path.as_os_str().is_empty() || path == current_dir) && was_relative {
        Cow::Borrowed(Path::new("."))
    } else {
        path.into()
    }
    .into()
}

fn realpath(path: impl AsRef<Path>) -> Result<PathBuf, GitError> {
    let path = path.as_ref();
    let cwd = path
        .is_relative()
        .then(std::env::current_dir)
        .unwrap_or_else(|| Ok(PathBuf::default()))
        // .unwrap_or_default() // PathBuf::new()
        .map_err(|_| GitError::Gen)?;
    hx_realpath_opts(path, &cwd)
}

fn hx_realpath_opts(path: &Path, cwd: &Path) -> Result<PathBuf, GitError> {
    const MAX_SYMLINK_CHECKS: usize = 2048;
    const MAX_SYMLINKS: u8 = 32;
    let mut real_path = PathBuf::new();
    if path.is_relative() {
        real_path.push(cwd);
    }

    let mut num_symlinks = 0;
    let mut path_backing: PathBuf;
    let mut components = path.components();
    let mut symlink_checks = 0;

    while let Some(component) = components.next() {
        match component {
            part @ (RootDir | Prefix(_)) => real_path.push(part),
            CurDir => {}
            ParentDir => {
                if !real_path.pop() {
                    return Err(GitError::Gen);
                }
            }
            Normal(part) => {
                real_path.push(part);
                symlink_checks += 1;
                if real_path.is_symlink() {
                    num_symlinks += 1;
                    if num_symlinks > MAX_SYMLINKS {
                        return Err(GitError::Gen);
                    }
                    let mut link_destination =
                        std::fs::read_link(real_path.as_path()).map_err(|_| GitError::Gen)?;

                    if !link_destination.is_absolute() {
                        real_path.pop();
                    }
                    link_destination.extend(components);
                    path_backing = link_destination;
                    components = path_backing.components();
                }
                if symlink_checks > MAX_SYMLINK_CHECKS {
                    return Err(GitError::Gen);
                }
            }
        }
    }
    Ok(real_path)
}

static GIT_HIGHEST_SCOPE_CONFIG_PATH: LazyLock<Option<BString>> = LazyLock::new(exe_info);
const NULL_DEVICE: &str = "/dev/null";

fn exe_info() -> Option<BString> {
    let mut cmd = Command::new("git");

    cmd.args(["config", "-lz", "--show-origin", "--name-only"])
        .current_dir("/")
        .env_remove("GIT_CONFIG")
        .env_remove("GIT_DISCOVERY_ACROSS_FILESYSTEM")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_COMMON_DIR")
        .env("GIT_DIR", NULL_DEVICE) // Avoid getting local-scope config.
        .env("GIT_WORK_TREE", NULL_DEVICE) // Avoid confusion when debugging.
        .stdin(Stdio::null())
        .stderr(Stdio::null());

    let cmd_output = match cmd.output() {
        Ok(out) => out.stdout,

        Err(_) => return None,
    };

    first_file_from_config_with_origin(cmd_output.as_slice().into()).map(ToOwned::to_owned)
}

fn first_file_from_config_with_origin(source: &BStr) -> Option<&BStr> {
    let file = source.strip_prefix(b"file:")?;
    let end_pos = file.find_byte(b'\0')?;
    file[..end_pos].as_bstr().into()
}

fn hx_installation_config() -> Option<&'static Path> {
    static PATH: LazyLock<Option<BString>> =
        LazyLock::new(|| GIT_HIGHEST_SCOPE_CONFIG_PATH.clone());
    PATH.as_ref()
        .map(AsRef::as_ref)
        .and_then(|p| hx_try_from_byte_slice(p).ok())
}

fn xdg_config(file: &str, env_var: &mut dyn FnMut(&str) -> Option<OsString>) -> Option<PathBuf> {
    env_var("XDG_CONFIG_HOME")
        .map(|home| {
            let mut p = PathBuf::from(home);
            p.push("git");
            p.push(file);
            p
        })
        .or_else(|| {
            env_var("HOME").map(|home| {
                let mut p = PathBuf::from(home);
                p.push(".config");
                p.push("git");
                p.push(file);
                p
            })
        })
}

#[must_use]
fn system_prefix() -> Option<&'static Path> {
    Path::new("/").into()
}

fn hx_home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(Into::into)
        .or_else(std::env::home_dir)
}

fn var(name: &str) -> Option<OsString> {
    if name == "HOME" {
        hx_home_dir().map(PathBuf::into_os_string)
    } else {
        std::env::var_os(name)
    }
}

#[derive(Default, Clone, Copy)]
struct NoopHasher(u64);

impl std::hash::Hasher for NoopHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        self.0 = u64::from_ne_bytes(bytes[..8].try_into().unwrap());
    }
}

fn undo(input: &BStr) -> Result<(Cow<'_, BStr>, usize), GitError> {
    if !input.starts_with(b"\"") {
        return Ok((input.into(), input.len()));
    }

    if input.len() < 2 {
        return Err(GitError::Gen);
    }

    let mut input = &input[1..];
    let mut consumed = 1;
    let mut out = BString::default();

    fn consume_one_past(input: &mut &BStr, position: usize) -> Result<u8, GitError> {
        *input = input.get(position + 1..).ok_or(GitError::Gen)?.as_bstr();

        let next = *input.first().ok_or(GitError::Gen)?;

        *input = input.get(1..).unwrap_or_default().as_bstr();

        Ok(next)
    }

    loop {
        if let Some(position) = input.find_byteset(b"\"\\") {
            out.extend_from_slice(&input[..position]);
            consumed += position + 1;

            match input[position] {
                b'"' => break,
                b'\\' => {
                    let next = consume_one_past(&mut input, position)?;
                    consumed += 1;

                    match next {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'a' => out.push(7),
                        b'b' => out.push(8),
                        b'v' => out.push(0xb),
                        b'f' => out.push(0xc),
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'0' | b'1' | b'2' | b'3' => {
                            let mut buf = [next; 3];

                            input
                                .get(..2)
                                .ok_or(GitError::Gen)?
                                .read_exact(&mut buf[1..])
                                .expect("impossible to fail as numbers match");

                            let byte =
                                to_unsigned_with_radix(&buf, 8).map_err(|_| GitError::Gen)?;

                            out.push(byte);
                            input = &input[2..];
                            consumed += 2;
                        }
                        _ => {
                            return Err(GitError::Gen);
                        }
                    }
                }
                _ => unreachable!("cannot find character that we didn't search for"),
            }
        } else {
            out.extend_from_slice(input);
            consumed += input.len();
            break;
        }
    }

    Ok((out.into(), consumed))
}

// Compatibility shims: this module used to be a nested tree (`store_impl::file`,
// `store_impl::packed`, ...). Everything is flat now, but these re-exports keep
// the old `refs::file::*` / `refs::packed::*` paths working for callers.
mod file {
    mod raw_ext {}
}

mod packed {}

#[derive(Debug, Clone)]
struct RefFileStore {
    git_dir: PathBuf,
    common_dir: Option<PathBuf>,
    packed_buffer_mmap_threshold: u64,
    packed: MutableSharedBuffer,
}

impl RefFileStore {
    #[must_use]
    fn common_dir_resolved(&self) -> &Path {
        self.common_dir.as_deref().unwrap_or(&self.git_dir)
    }
}

impl RefFileStore {
    #[must_use]
    fn at(git_dir: PathBuf) -> Self {
        RefFileStore {
            git_dir,
            packed_buffer_mmap_threshold: 32 * 1024,
            common_dir: None,
            packed: SharedFileSnapshotMut::new().into(),
        }
    }
}

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Hash, Clone)]
struct LooseReference {
    name: FullName,
    target: Target,
}

enum MaybeUnsafeState {
    Id(ObjectId),
    UnvalidatedPath(BString),
}

impl TryFrom<MaybeUnsafeState> for Target {
    type Error = GitError;

    fn try_from(v: MaybeUnsafeState) -> Result<Self, GitError> {
        Ok(match v {
            MaybeUnsafeState::Id(id) => Target::Object(id),
            MaybeUnsafeState::UnvalidatedPath(name) => {
                Target::Symbolic(match validate_reference_name(name.as_ref()) {
                    Ok(_) => FullName(name),
                    Err(_) => {
                        return Err(GitError::Gen);
                    }
                })
            }
        })
    }
}

impl LooseReference {
    fn try_from_path(name: FullName, path_contents: &[u8]) -> Result<Self, GitError> {
        Ok(LooseReference {
            name,
            target: parse(path_contents)
                .map_err(|()| GitError::Gen)?
                .try_into()?,
        })
    }
}

fn parse(mut i: &[u8]) -> Result<MaybeUnsafeState, ()> {
    if let Some(rest) = i.strip_prefix(b"ref: ") {
        i = rest;
        while i.first() == Some(&b' ') {
            i = &i[1..];
        }
        let path_end = i
            .iter()
            .position(|b| *b == b'\0' || *b == b'\r' || *b == b'\n')
            .unwrap_or(i.len());
        let path = i[..path_end].into();
        Ok(MaybeUnsafeState::UnvalidatedPath(path))
    } else {
        let hex = hex_hash_consuming(&mut i)?;
        if i.first().is_some_and(u8::is_ascii_hexdigit) {
            return Err(());
        }
        Ok(MaybeUnsafeState::Id(
            ObjectId::hx_from_hex(hex).expect("prior validation"),
        ))
    }
}

impl RefFileStore {
    fn try_find<'a, Name>(&self, partial: Name) -> Result<Option<RawReference>, GitError>
    where
        Name: TryInto<&'a PartialNameRef>,
    {
        let packed = self.assure_packed_refs_uptodate()?;
        self.find_one_with_verified_input(
            partial.try_into().map_err(|_| GitError::Gen)?,
            packed.as_ref().map(|b| &***b),
        )
    }

    fn try_find_packed<'a, Name>(
        &self,
        partial: Name,
        packed: Option<&PackedBuffer>,
    ) -> Result<Option<RawReference>, GitError>
    where
        Name: TryInto<&'a PartialNameRef>,
    {
        self.find_one_with_verified_input(partial.try_into().map_err(|_| GitError::Gen)?, packed)
    }

    fn find_one_with_verified_input(
        &self,
        partial_name: &PartialNameRef,
        packed: Option<&PackedBuffer>,
    ) -> Result<Option<RawReference>, GitError> {
        fn decompose_if(mut r: RawReference, input_changed_to_precomposed: bool) -> RawReference {
            if input_changed_to_precomposed {
                let decomposed = r
                    .name
                    .0
                    .to_str()
                    .ok()
                    .map(|name| str_decompose(name.into()));
                if let Some(Cow::Owned(decomposed)) = decomposed {
                    r.name.0 = decomposed.into();
                }
            }
            r
        }
        let mut buf = BString::default();
        let mut precomposed_partial_name_storage = packed.filter(|_| false).and_then(|_| {
            let precomposed = partial_name.0.to_str().ok()?;
            let precomposed = str_precompose(precomposed.into());
            match precomposed {
                Cow::Owned(precomposed) => Some(PartialName(precomposed.into())),
                Cow::Borrowed(_) => None,
            }
        });
        let precomposed_partial_name = precomposed_partial_name_storage
            .as_ref()
            .map(std::convert::AsRef::as_ref);
        for consider_pseudo_ref in [true, false] {
            if !consider_pseudo_ref && !is_pseudo_ref(partial_name.as_bstr()) {
                break;
            }
            'try_directories: for inbetween in &["", "tags", "heads", "remotes"] {
                match self.find_inner(
                    inbetween,
                    partial_name,
                    precomposed_partial_name,
                    &mut buf,
                    consider_pseudo_ref,
                ) {
                    Ok(Some(r)) => {
                        return Ok(Some(decompose_if(r, precomposed_partial_name.is_some())));
                    }
                    Ok(None) => {
                        if consider_pseudo_ref && is_pseudo_ref(partial_name.as_bstr()) {
                            break 'try_directories;
                        }
                        continue;
                    }
                    Err(_) => return Err(GitError::Gen),
                }
            }
        }
        if partial_name.as_bstr() == "HEAD" {
            Ok(None)
        } else {
            if let Some(mut precomposed) = precomposed_partial_name_storage {
                precomposed = precomposed.join("HEAD".into()).expect("HEAD is valid name");
                precomposed_partial_name_storage = Some(precomposed);
            }
            self.find_inner(
                "remotes",
                partial_name
                    .to_owned()
                    .join("HEAD".into())
                    .expect("HEAD is valid name")
                    .as_ref(),
                precomposed_partial_name_storage
                    .as_ref()
                    .map(std::convert::AsRef::as_ref),
                &mut buf,
                true, /* consider-pseudo-ref */
            )
            .map(|res| res.map(|r| decompose_if(r, precomposed_partial_name_storage.is_some())))
        }
    }

    fn find_inner(
        &self,
        inbetween: &str,
        partial_name: &PartialNameRef,
        precomposed_partial_name: Option<&PartialNameRef>,
        path_buf: &mut BString,
        consider_pseudo_ref: bool,
    ) -> Result<Option<RawReference>, GitError> {
        let full_name = precomposed_partial_name
            .unwrap_or(partial_name)
            .construct_full_name_ref(inbetween, path_buf, consider_pseudo_ref);
        let content_buf = match self.ref_contents(full_name) {
            Ok(content_buf) => content_buf,
            Err(err) if err.kind() == io::ErrorKind::NotADirectory => return Ok(None),
            Err(_) => {
                return Err(GitError::Gen);
            }
        };

        match content_buf {
            None => Ok(None),
            Some(content) => Ok(Some(
                LooseReference::try_from_path(full_name.to_owned(), &content)
                    .map(Into::into)
                    .map(|r: RawReference| r)
                    .map_err(|_| GitError::Gen)?,
            )),
        }
    }
}

impl RefFileStore {
    fn to_base_dir_and_relative_name<'a>(
        &self,
        name: &'a FullNameRef,
        is_reflog: bool,
    ) -> (Cow<'_, Path>, &'a FullNameRef) {
        let commondir = self.common_dir_resolved();
        let linked_git_dir =
            |worktree_name: &BStr| commondir.join("worktrees").join(from_bstr(worktree_name));
        name.category_and_short_name()
            .map(|(c, sn)| {
                use Category::{
                    Bisect, LinkedPseudoRef, LinkedRef, LocalBranch, MainPseudoRef, MainRef, Note,
                    PseudoRef, RemoteBranch, Rewritten, Tag, WorktreePrivate,
                };
                let sn = FullNameRef::new_unchecked(sn);
                match c {
                    LinkedPseudoRef {
                        name: worktree_name,
                    } => {
                        if is_reflog {
                            (linked_git_dir(worktree_name).into(), sn)
                        } else {
                            (commondir.into(), name)
                        }
                    }
                    Tag | LocalBranch | RemoteBranch | Note => (commondir.into(), name),
                    MainRef | MainPseudoRef => (commondir.into(), sn),
                    LinkedRef {
                        name: worktree_name,
                    } => {
                        if sn.category().is_some_and(|cat| cat.is_worktree_private()) {
                            if is_reflog {
                                (linked_git_dir(worktree_name).into(), sn)
                            } else {
                                (commondir.into(), name)
                            }
                        } else {
                            (commondir.into(), sn)
                        }
                    }
                    PseudoRef | Bisect | Rewritten | WorktreePrivate => {
                        (self.git_dir.as_path().into(), name)
                    }
                }
            })
            .unwrap_or((commondir.into(), name))
    }

    fn reference_path_with_base<'b>(
        &self,
        name: &'b FullNameRef,
    ) -> (Cow<'_, Path>, Cow<'b, Path>) {
        let (base, name) = self.to_base_dir_and_relative_name(name, false);
        (base, to_native_path_on_windows(name.as_bstr()))
    }

    fn ref_contents(&self, name: &FullNameRef) -> io::Result<Option<Vec<u8>>> {
        let (base, relative_path) = self.reference_path_with_base(name);
        let ref_path = base.join(&relative_path);
        match std::fs::File::open(&ref_path) {
            Ok(mut file) => {
                let mut buf = Vec::with_capacity(128);
                if let Err(err) = file.read_to_end(&mut buf) {
                    return if ref_path.is_dir() {
                        Ok(None)
                    } else {
                        Err(err)
                    };
                }
                Ok(buf.into())
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),

            Err(err) => Err(err),
        }
    }
}

impl RefFileStore {
    fn find<'a, Name>(&self, partial: Name) -> Result<RawReference, GitError>
    where
        Name: TryInto<&'a PartialNameRef>,
    {
        let packed = self
            .assure_packed_refs_uptodate()
            .map_err(|_| GitError::Gen)?;
        self.find_existing_inner(partial, packed.as_ref().map(|b| &***b))
    }

    fn find_existing_inner<'a, Name>(
        &self,
        partial: Name,
        packed: Option<&PackedBuffer>,
    ) -> Result<RawReference, GitError>
    where
        Name: TryInto<&'a PartialNameRef>,
    {
        let path = partial.try_into().map_err(|_| GitError::Gen)?;
        match self.find_one_with_verified_input(path, packed) {
            Ok(Some(r)) => Ok(r),
            _ => Err(GitError::Gen),
        }
    }
}

impl RefFileStore {
    fn open_packed_buffer(&self) -> Result<Option<PackedBuffer>, GitError> {
        match PackedBuffer::open(self.packed_refs_path(), self.packed_buffer_mmap_threshold) {
            Ok(buf) => Ok(Some(buf)),
            Err(GitError::NotFound) => Ok(None),
            Err(_) => Err(GitError::Gen),
        }
    }

    #[must_use]
    fn packed_refs_path(&self) -> PathBuf {
        self.common_dir_resolved().join("packed-refs")
    }

    fn assure_packed_refs_uptodate(&self) -> Result<Option<SharedBufferSnapshot>, GitError> {
        self.packed.recent_snapshot(
            || {
                self.packed_refs_path()
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
            },
            || self.open_packed_buffer(),
        )
    }
}

type SharedBufferSnapshot = SharedFileSnapshot<PackedBuffer>;
type MutableSharedBuffer = std::sync::Arc<SharedFileSnapshotMut<PackedBuffer>>;

impl RawReference {
    // helix
    fn peel_to_id(
        &mut self,
        store: &RefFileStore,
        objects: &dyn Find,
    ) -> Result<ObjectId, GitError> {
        let packed = store
            .assure_packed_refs_uptodate()
            .map_err(|_| GitError::Gen)?;
        {
            let this = &mut *self;
            let packed = packed.as_ref().map(|b| &***b);
            if let Some(peeled) = this.peeled {
                this.target = Target::Object(peeled);
                Ok(peeled)
            } else {
                let oid = this.follow_to_object_packed(store, packed)?;
                let mut buf = Vec::new();
                let peeled_id = {
                    objects.try_find(&oid, &mut buf)?.ok_or(GitError::Gen)?;
                    oid
                };

                this.peeled = Some(peeled_id);
                this.target = Target::Object(peeled_id);
                Ok(peeled_id)
            }
        }
    }

    // helix
    fn follow_to_object_packed(
        &mut self,
        store: &RefFileStore,
        packed: Option<&PackedBuffer>,
    ) -> Result<ObjectId, GitError> {
        match self.target {
            Target::Object(id) => Ok(id),
            Target::Symbolic(_) => {
                let mut seen = BTreeSet::new();
                let cursor = &mut *self;
                while let Some(next) = cursor.follow_packed(store, packed) {
                    let next = next?;
                    if seen.contains(&next.name) {
                        return Err(GitError::Gen);
                    }
                    *cursor = next;
                    seen.insert(cursor.name.clone());
                    const MAX_REF_DEPTH: usize = 5;
                    if seen.len() == MAX_REF_DEPTH {
                        return Err(GitError::Gen);
                    }
                }
                let oid = self.target.try_id().expect("peeled ref").to_owned();
                Ok(oid)
            }
        }
    }

    // helix
    fn follow_packed(
        &self,
        store: &RefFileStore,
        packed: Option<&PackedBuffer>,
    ) -> Option<Result<RawReference, GitError>> {
        match &self.target {
            Target::Object(_) => None,
            Target::Symbolic(full_name) => {
                match store.try_find_packed(full_name.as_ref(), packed) {
                    Ok(Some(next)) => Some(Ok(next)),
                    Ok(None) => Some(Err(GitError::Gen)),
                    Err(_) => Some(Err(GitError::Gen)),
                }
            }
        }
    }
}

#[derive(Debug)]
enum Backing {
    InMemory(Vec<u8>),
    Mapped(Mmap),
}

#[derive(Debug)]
struct PackedBuffer {
    data: Backing,
    offset: usize,
}

#[derive(Debug, PartialEq, Eq)]
struct PackedReference<'a> {
    name: &'a FullNameRef,
    target: &'a BStr,
    object: Option<&'a BStr>,
}

impl PackedReference<'_> {
    #[must_use]
    fn target(&self) -> ObjectId {
        ObjectId::hx_from_hex(self.target).expect("parser validation")
    }
}

struct Iter<'a> {
    cursor: &'a [u8],
    current_line: usize,
    prefix: Option<BString>,
}

#[derive(Debug, PartialEq, Eq)]
enum Peeled {
    Unspecified,
    Partial,
    Fully,
}

#[derive(Debug, PartialEq, Eq)]
struct PackedRefsHeader {
    peeled: Peeled,
    sorted: bool,
}

impl Default for PackedRefsHeader {
    fn default() -> Self {
        PackedRefsHeader {
            peeled: Peeled::Unspecified,
            sorted: false,
        }
    }
}

fn until_line_end_without_separator<'a>(input: &mut &'a [u8]) -> Result<&'a BStr, ()> {
    let line_end = input
        .iter()
        .position(|b| *b == b'\r' || *b == b'\n')
        .ok_or(())?;
    let out = input[..line_end].as_bstr();
    let mut maybe_start_of_newline = &input[line_end..];
    newline(&mut maybe_start_of_newline)?;
    *input = maybe_start_of_newline;
    Ok(out)
}

fn header(input: &mut &[u8]) -> Result<PackedRefsHeader, ()> {
    let Some(rest) = input.strip_prefix(b"# pack-refs with: ") else {
        return Err(());
    };
    *input = rest;
    let traits = until_line_end_without_separator(input)?;
    let mut peeled = Peeled::Unspecified;
    let mut sorted = false;
    for token in traits.split_str(b" ") {
        if token == b"fully-peeled" {
            peeled = Peeled::Fully;
        } else if token == b"peeled" {
            peeled = Peeled::Partial;
        } else if token == b"sorted" {
            sorted = true;
        }
    }
    Ok(PackedRefsHeader { peeled, sorted })
}

fn reference<'a>(input: &mut &'a [u8]) -> Result<PackedReference<'a>, ()> {
    let target = hex_hash_consuming(input)?;
    let Some(rest) = input.strip_prefix(b" ") else {
        return Err(());
    };
    *input = rest;
    let name = until_line_end_without_separator(input)?
        .try_into()
        .map_err(|_| ())?;

    let object = if let Some(rest) = input.strip_prefix(b"^") {
        *input = rest;
        let object = hex_hash_consuming(input)?;
        newline(input)?;
        Some(object)
    } else {
        None
    };

    Ok(PackedReference {
        name,
        target,
        object,
    })
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<PackedReference<'a>, GitError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor.is_empty() {
            return None;
        }

        let start = self.cursor;
        if let Ok(parsed) = reference(&mut self.cursor) {
            self.current_line += 1;
            if let Some(ref prefix) = self.prefix
                && !parsed.name.as_bstr().starts_with_str(prefix)
            {
                self.cursor = &[];
                return None;
            }
            Some(Ok(parsed))
        } else {
            self.cursor = start;
            let (_, next_cursor) = self
                .cursor
                .find_byte(b'\n')
                .map_or((self.cursor, &[][..]), |pos| self.cursor.split_at(pos + 1));
            self.cursor = next_cursor;
            self.current_line += 1;

            Some(Err(GitError::Gen))
        }
    }
}

impl<'a> Iter<'a> {
    fn new(packed: &'a [u8]) -> Result<Self, GitError> {
        Self::new_with_prefix(packed, None)
    }

    fn new_with_prefix(packed: &'a [u8], prefix: Option<BString>) -> Result<Self, GitError> {
        if packed.is_empty() {
            Ok(Iter {
                cursor: packed,
                prefix,
                current_line: 1,
            })
        } else if packed[0] == b'#' {
            let mut input = packed;
            header(&mut input).map_err(|()| GitError::Gen)?;
            let refs = input;
            Ok(Iter {
                cursor: refs,
                prefix,
                current_line: 2,
            })
        } else {
            Ok(Iter {
                cursor: packed,
                prefix,
                current_line: 1,
            })
        }
    }
}

impl AsRef<[u8]> for PackedBuffer {
    fn as_ref(&self) -> &[u8] {
        &self.data.as_ref()[self.offset..]
    }
}

impl AsRef<[u8]> for Backing {
    fn as_ref(&self) -> &[u8] {
        match self {
            Backing::InMemory(data) => data,
            Backing::Mapped(map) => map,
        }
    }
}

impl PackedBuffer {
    fn open_with_backing(backing: Backing, _path: PathBuf) -> Result<Self, GitError> {
        let (backing, offset) = {
            let (offset, sorted) = {
                let mut input = backing.as_ref();
                if *input.first().unwrap_or(&b' ') == b'#' {
                    let hdr = header(&mut input).map_err(|()| GitError::Gen)?;
                    let offset = backing.as_ref().len() - input.len();
                    (offset, hdr.sorted)
                } else {
                    (0, false)
                }
            };
            if sorted {
                (backing, offset)
            } else {
                let mut entries =
                    Iter::new(&backing.as_ref()[offset..])?.collect::<Result<Vec<_>, _>>()?;
                entries.sort_by_key(|e| e.name.as_bstr());
                let mut serialized = Vec::<u8>::new();
                for entry in entries {
                    serialized.extend_from_slice(entry.target);
                    serialized.push(b' ');
                    serialized.extend_from_slice(entry.name.as_bstr());
                    serialized.push(b'\n');
                    if let Some(object) = entry.object {
                        serialized.push(b'^');
                        serialized.extend_from_slice(object);
                        serialized.push(b'\n');
                    }
                }
                (Backing::InMemory(serialized), 0)
            }
        };
        Ok(PackedBuffer {
            offset,
            data: backing,
        })
    }
    fn open(path: PathBuf, use_memory_map_if_larger_than_bytes: u64) -> Result<Self, GitError> {
        let len = std::fs::metadata(&path)
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    GitError::NotFound
                } else {
                    GitError::Gen
                }
            })?
            .len();

        let backing = if len <= use_memory_map_if_larger_than_bytes {
            Backing::InMemory(std::fs::read(&path).map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    GitError::NotFound
                } else {
                    GitError::Gen
                }
            })?)
        } else {
            let file = std::fs::File::open(&path).map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    GitError::NotFound
                } else {
                    GitError::Gen
                }
            })?;

            Backing::Mapped(
                #[allow(unsafe_code)]
                unsafe {
                    memmap2::MmapOptions::new()
                        .map_copy_read_only(&file)
                        .map_err(|_| GitError::Gen)?
                },
            )
        };

        Self::open_with_backing(backing, path).map_err(|_| GitError::Gen)
    }
}

impl<'a> From<&'a FullNameRef> for &'a BStr {
    fn from(name: &'a FullNameRef) -> Self {
        &name.0
    }
}

impl<'a> From<&'a FullNameRef> for FullName {
    fn from(value: &'a FullNameRef) -> Self {
        FullName(value.as_bstr().into())
    }
}

impl std::fmt::Display for FullName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::Display for FullNameRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl FullNameRef {
    #[must_use]
    fn as_partial_name(&self) -> &PartialNameRef {
        PartialNameRef::new_unchecked(self.0.as_bstr())
    }

    #[must_use]
    fn as_bstr(&self) -> &BStr {
        &self.0
    }

    #[must_use]
    pub fn shorten(&self) -> String {
        self.category_and_short_name()
            .map_or_else(|| self.0.as_bstr(), |(_, short)| short)
            .to_string()
    }

    #[must_use]
    fn category(&self) -> Option<Category<'_>> {
        self.category_and_short_name().map(|(cat, _)| cat)
    }

    #[must_use]
    fn category_and_short_name(&self) -> Option<(Category<'_>, &BStr)> {
        let name = self.0.as_bstr();
        for category in &[Category::Tag, Category::LocalBranch, Category::RemoteBranch] {
            if let Some(shortened) = name.strip_prefix(category.prefix().as_bytes()) {
                return Some((*category, shortened.as_bstr()));
            }
        }

        for category in &[
            Category::Note,
            Category::Bisect,
            Category::WorktreePrivate,
            Category::Rewritten,
        ] {
            if name.starts_with(category.prefix().as_ref()) {
                return Some((
                    *category,
                    name.strip_prefix(b"refs/")
                        .expect("we checked for refs/* above")
                        .as_bstr(),
                ));
            }
        }

        if is_pseudo_ref(name) {
            Some((Category::PseudoRef, name))
        } else if let Some(shortened) =
            name.strip_prefix(Category::MainPseudoRef.prefix().as_bytes())
        {
            if shortened.starts_with_str("refs/") {
                (Category::MainRef, shortened.as_bstr()).into()
            } else {
                is_pseudo_ref(shortened.into())
                    .then(|| (Category::MainPseudoRef, shortened.as_bstr()))
            }
        } else if let Some(shortened_with_worktree_name) = name.strip_prefix(
            Category::LinkedPseudoRef { name: "".into() }
                .prefix()
                .as_bytes(),
        ) {
            let (name, shortened) = shortened_with_worktree_name.find_byte(b'/').map(|pos| {
                (
                    shortened_with_worktree_name[..pos].as_bstr(),
                    shortened_with_worktree_name[pos + 1..].as_bstr(),
                )
            })?;
            if shortened.starts_with_str("refs/") {
                (Category::LinkedRef { name }, shortened.as_bstr()).into()
            } else {
                is_pseudo_ref(shortened)
                    .then(|| (Category::LinkedPseudoRef { name }, shortened.as_bstr()))
            }
        } else {
            None
        }
    }
}

impl Borrow<FullNameRef> for FullName {
    #[inline]
    fn borrow(&self) -> &FullNameRef {
        FullNameRef::new_unchecked(self.0.as_bstr())
    }
}

impl AsRef<FullNameRef> for FullName {
    fn as_ref(&self) -> &FullNameRef {
        self.borrow()
    }
}

impl ToOwned for FullNameRef {
    type Owned = FullName;

    fn to_owned(&self) -> Self::Owned {
        FullName(self.0.to_owned())
    }
}

type Error = GitError;

impl Category<'_> {
    #[must_use]
    fn prefix(&self) -> &BStr {
        match self {
            Category::Tag => b"refs/tags/".as_bstr(),
            Category::LocalBranch => b"refs/heads/".as_bstr(),
            Category::RemoteBranch => b"refs/remotes/".as_bstr(),
            Category::Note => b"refs/notes/".as_bstr(),
            Category::MainPseudoRef => b"main-worktree/".as_bstr(),
            Category::MainRef => b"main-worktree/refs/".as_bstr(),
            Category::PseudoRef => b"".as_bstr(),
            Category::LinkedPseudoRef { .. } | Category::LinkedRef { .. } => {
                b"worktrees/".as_bstr()
            }
            Category::Bisect => b"refs/bisect/".as_bstr(),
            Category::Rewritten => b"refs/rewritten/".as_bstr(),
            Category::WorktreePrivate => b"refs/worktree/".as_bstr(),
        }
    }

    #[must_use]
    fn is_worktree_private(&self) -> bool {
        matches!(
            self,
            Category::MainPseudoRef
                | Category::PseudoRef
                | Category::LinkedPseudoRef { .. }
                | Category::WorktreePrivate
                | Category::Rewritten
                | Category::Bisect
        )
    }
}

impl FullNameRef {
    fn new_unchecked(v: &BStr) -> &Self {
        #[allow(unsafe_code)]
        unsafe {
            std::mem::transmute(v)
        }
    }
}

impl PartialNameRef {
    fn new_unchecked(v: &BStr) -> &Self {
        #[allow(unsafe_code)]
        unsafe {
            std::mem::transmute(v)
        }
    }
}

impl PartialNameRef {
    fn looks_like_full_name(&self, consider_pseudo_ref: bool) -> bool {
        let name = self.0.as_bstr();
        name.starts_with_str("refs/")
            || name.starts_with(Category::MainPseudoRef.prefix())
            || name.starts_with(Category::LinkedPseudoRef { name: "".into() }.prefix())
            || (consider_pseudo_ref && is_pseudo_ref(name))
    }
    fn construct_full_name_ref<'buf>(
        &self,
        inbetween: &str,
        buf: &'buf mut BString,
        consider_pseudo_ref: bool,
    ) -> &'buf FullNameRef {
        buf.clear();
        if !self.looks_like_full_name(consider_pseudo_ref) {
            buf.push_str("refs/");
        }
        if !inbetween.is_empty() {
            buf.push_str(inbetween);
            buf.push_byte(b'/');
        }
        buf.extend_from_slice(&self.0);
        FullNameRef::new_unchecked(buf.as_bstr())
    }
}

impl PartialNameRef {
    #[must_use]
    fn as_bstr(&self) -> &BStr {
        &self.0
    }
}

impl PartialName {
    fn join(self, component: &BStr) -> Result<Self, Error> {
        let mut b = self.0;
        b.push_byte(b'/');
        b.extend(component.as_bytes());
        validate_reference_name_partial(b.as_ref())?;
        Ok(PartialName(b))
    }
}

impl<'a> convert::TryFrom<&'a BStr> for &'a FullNameRef {
    type Error = GitError;

    fn try_from(v: &'a BStr) -> Result<Self, GitError> {
        Ok(FullNameRef::new_unchecked(validate_reference_name(v)?))
    }
}

impl<'a> From<&'a FullNameRef> for &'a PartialNameRef {
    fn from(v: &'a FullNameRef) -> Self {
        PartialNameRef::new_unchecked(v.0.as_bstr())
    }
}

impl<'a> TryFrom<&'a OsStr> for &'a PartialNameRef {
    type Error = GitError;

    fn try_from(v: &'a OsStr) -> Result<Self, GitError> {
        let v = os_str_into_bstr(v).map_err(|_| GitError::Gen)?;
        Ok(PartialNameRef::new_unchecked(
            validate_reference_name_partial(v.as_bstr())?,
        ))
    }
}

impl Borrow<PartialNameRef> for PartialName {
    #[inline]
    fn borrow(&self) -> &PartialNameRef {
        PartialNameRef::new_unchecked(self.0.as_bstr())
    }
}

impl AsRef<PartialNameRef> for PartialName {
    fn as_ref(&self) -> &PartialNameRef {
        self.borrow()
    }
}

impl ToOwned for PartialNameRef {
    type Owned = PartialName;

    fn to_owned(&self) -> Self::Owned {
        PartialName(self.0.to_owned())
    }
}

impl<'a> convert::TryFrom<&'a BStr> for &'a PartialNameRef {
    type Error = GitError;

    fn try_from(v: &'a BStr) -> Result<Self, GitError> {
        Ok(PartialNameRef::new_unchecked(
            validate_reference_name_partial(v)?,
        ))
    }
}

impl<'a> convert::TryFrom<&'a PartialName> for &'a PartialNameRef {
    type Error = GitError;

    fn try_from(v: &'a PartialName) -> Result<Self, GitError> {
        Ok(PartialNameRef::new_unchecked(v.0.as_bstr()))
    }
}

impl<'a> convert::TryFrom<&'a str> for &'a PartialNameRef {
    type Error = GitError;

    fn try_from(v: &'a str) -> Result<Self, GitError> {
        let v = v.as_bytes().as_bstr();
        Ok(PartialNameRef::new_unchecked(
            validate_reference_name_partial(v)?,
        ))
    }
}

impl<'a> convert::TryFrom<&'a FullName> for &'a PartialNameRef {
    type Error = GitError;

    fn try_from(v: &'a FullName) -> Result<Self, GitError> {
        Ok(v.as_ref().as_partial_name())
    }
}

fn is_pseudo_ref(name: &BStr) -> bool {
    name.iter().all(|b| b.is_ascii_uppercase() || *b == b'_')
}

fn hex_hash_consuming<'a>(i: &mut &'a [u8]) -> ParseResult<&'a BStr> {
    let len = 40;
    let Some(hex) = i.get(..len) else {
        return Err(());
    };
    if !hex.iter().all(u8::is_ascii_hexdigit) {
        return Err(());
    }
    *i = &i[len..];
    Ok(hex.as_bstr())
}

fn newline<'a>(i: &mut &'a [u8]) -> ParseResult<&'a [u8]> {
    if let Some(rest) = i.strip_prefix(b"\r\n") {
        let out = &i[..2];
        *i = rest;
        Ok(out)
    } else if let Some(rest) = i.strip_prefix(b"\n") {
        let out = &i[..1];
        *i = rest;
        Ok(out)
    } else {
        Err(())
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
struct RawReference {
    name: FullName,
    target: Target,
    peeled: Option<ObjectId>,
}

impl From<RawReference> for LooseReference {
    fn from(value: RawReference) -> Self {
        LooseReference {
            name: value.name,
            target: value.target,
        }
    }
}

impl From<LooseReference> for RawReference {
    fn from(value: LooseReference) -> Self {
        RawReference {
            name: value.name,
            target: value.target,
            peeled: None,
        }
    }
}

impl<'p> From<PackedReference<'p>> for RawReference {
    fn from(value: PackedReference<'p>) -> Self {
        RawReference {
            name: value.name.into(),
            target: Target::Object(value.target()),
            peeled: value
                .object
                .map(|hex| ObjectId::hx_from_hex(hex).expect("parser validation")),
        }
    }
}

impl Target {
    #[must_use]
    fn try_id(&self) -> Option<&oid> {
        match self {
            Target::Symbolic(_) => None,
            Target::Object(oid) => Some(oid),
        }
    }
}

impl From<ObjectId> for Target {
    fn from(id: ObjectId) -> Self {
        Target::Object(id)
    }
}

impl TryFrom<Target> for ObjectId {
    type Error = GitError;

    fn try_from(value: Target) -> Result<Self, GitError> {
        match value {
            Target::Object(id) => Ok(id),
            Target::Symbolic(_) => Err(GitError::Gen),
        }
    }
}

impl From<FullName> for Target {
    fn from(name: FullName) -> Self {
        Target::Symbolic(name)
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct FullName(BString);

#[derive(Hash, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct FullNameRef(BStr);

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
#[repr(transparent)]
struct PartialNameRef(BStr);

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
struct PartialName(BString);

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
enum Category<'a> {
    Tag,
    LocalBranch,
    RemoteBranch,
    Note,
    PseudoRef,
    MainPseudoRef,
    MainRef,
    LinkedPseudoRef { name: &'a BStr },
    LinkedRef { name: &'a BStr },
    Bisect,
    Rewritten,
    WorktreePrivate,
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
enum Target {
    Object(ObjectId),
    Symbolic(FullName),
}

type MutableOnDemand<T> = parking_lot::RwLock<T>;

#[derive(Debug)]
struct FileSnapshot<T> {
    value: T,
    modified: std::time::SystemTime,
}

impl<T> FileSnapshot<T> {
    fn new(value: T) -> Self {
        FileSnapshot {
            value,
            modified: std::time::UNIX_EPOCH,
        }
    }
}

impl<T> From<T> for FileSnapshot<T> {
    fn from(value: T) -> Self {
        FileSnapshot::new(value)
    }
}

impl<T: Clone + std::fmt::Debug> Clone for FileSnapshot<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            modified: self.modified,
        }
    }
}

impl<T> Deref for FileSnapshot<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> std::ops::DerefMut for FileSnapshot<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

type SharedFileSnapshot<T> = std::sync::Arc<FileSnapshot<T>>;

// helix
#[derive(Debug, Default)]
struct SharedFileSnapshotMut<T>(MutableOnDemand<Option<SharedFileSnapshot<T>>>);

impl<T> Deref for SharedFileSnapshotMut<T> {
    type Target = MutableOnDemand<Option<SharedFileSnapshot<T>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> SharedFileSnapshotMut<T> {
    #[must_use]
    fn new() -> Self {
        SharedFileSnapshotMut(MutableOnDemand::new(None))
    }

    fn recent_snapshot(
        &self,
        mut current_modification_time: impl FnMut() -> Option<std::time::SystemTime>,
        open: impl FnOnce() -> Result<Option<T>, GitError>,
    ) -> Result<Option<SharedFileSnapshot<T>>, GitError> {
        let state = self.0.read();
        let recent_modification = current_modification_time();
        let buffer = match (&*state, recent_modification) {
            (None, None) => (*state).clone(),
            (Some(_), None) => {
                drop(state);
                let mut state = self.0.write();
                *state = None;
                (*state).clone()
            }
            (Some(snapshot), Some(modified_time)) => {
                if snapshot.modified < modified_time {
                    drop(state);
                    let mut state = self.0.write();

                    if let (Some(_snapshot), Some(modified_time)) =
                        (&*state, current_modification_time())
                    {
                        *state = open()?.map(|value| {
                            Arc::new(FileSnapshot {
                                value,
                                modified: modified_time,
                            })
                        });
                    }

                    (*state).clone()
                } else {
                    Some(snapshot.clone())
                }
            }
            (None, Some(_modified_time)) => {
                drop(state);
                let mut state = self.0.write();
                if let (None, Some(modified_time)) = (&*state, current_modification_time()) {
                    *state = open()?.map(|value| {
                        Arc::new(FileSnapshot {
                            value,
                            modified: modified_time,
                        })
                    });
                }
                (*state).clone()
            }
        };
        Ok(buffer)
    }
}

impl<'a> AssignmentRef<'a> {
    fn new(name: NameRef<'a>, state: StateRef<'a>) -> AssignmentRef<'a> {
        AssignmentRef { name, state }
    }

    #[must_use]
    fn to_owned(self) -> Assignment {
        self.into()
    }
}

impl<'a> From<AssignmentRef<'a>> for Assignment {
    fn from(a: AssignmentRef<'a>) -> Self {
        Assignment {
            name: a.name.to_owned(),
            state: a.state.to_owned(),
        }
    }
}

impl<'a> Assignment {
    #[must_use]
    fn as_ref(&'a self) -> AssignmentRef<'a> {
        AssignmentRef::new(self.name.as_ref(), self.state.as_ref())
    }
}

impl std::fmt::Display for AssignmentRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        match self.state {
            StateRef::Set => f.write_str(self.name.as_str()),
            StateRef::Unset => {
                f.write_char('-')?;
                f.write_str(self.name.as_str())
            }
            StateRef::Value(v) => {
                f.write_str(self.name.as_str())?;
                f.write_char('=')?;
                f.write_str(v.as_bstr().to_str_lossy().as_ref())
            }
            StateRef::Unspecified => {
                f.write_char('!')?;
                f.write_str(self.name.as_str())
            }
        }
    }
}

pub(crate) mod name {
    use bstr::{BStr, ByteSlice};
    use kstring::KStringRef;

    use crate::git::GitError;
    use crate::git::{Name, NameRef};

    impl NameRef<'_> {
        #[must_use]
        pub(crate) fn to_owned(self) -> Name {
            Name(self.0.into())
        }

        #[must_use]
        pub(crate) fn as_str(&self) -> &str {
            self.0.as_str()
        }
    }

    impl AsRef<str> for NameRef<'_> {
        fn as_ref(&self) -> &str {
            self.0.as_ref()
        }
    }

    impl<'a> TryFrom<&'a BStr> for NameRef<'a> {
        type Error = GitError;

        fn try_from(attr: &'a BStr) -> Result<Self, GitError> {
            fn attr_valid(attr: &BStr) -> bool {
                if attr.first() == Some(&b'-') {
                    return false;
                }

                attr.bytes().all(
                    |b| matches!(b, b'-' | b'.' | b'_' | b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'),
                )
            }

            attr_valid(attr)
                .then(|| {
                    NameRef(KStringRef::from_ref(
                        attr.to_str().expect("no illformed utf8"),
                    ))
                })
                .ok_or(GitError::Gen)
        }
    }

    impl<'a> Name {
        #[must_use]
        pub(crate) fn as_ref(&'a self) -> NameRef<'a> {
            NameRef(self.0.as_ref())
        }

        #[must_use]
        pub(crate) fn as_str(&self) -> &str {
            self.0.as_str()
        }
    }

    impl AsRef<str> for Name {
        fn as_ref(&self) -> &str {
            self.0.as_str()
        }
    }
}

pub(crate) mod state {
    use bstr::{BStr, BString, ByteSlice};

    use crate::git::{StackState, StateRef};

    #[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
    pub(crate) struct Value(BString);

    #[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
    pub(crate) struct ValueRef<'a>(&'a [u8]);

    impl<'a> ValueRef<'a> {
        #[must_use]
        pub(crate) fn from_bytes(input: &'a [u8]) -> Self {
            Self(input)
        }
    }

    impl<'a> ValueRef<'a> {
        #[must_use]
        pub(crate) fn as_bstr(&self) -> &'a BStr {
            self.0.as_bytes().as_bstr()
        }
    }

    impl<'a> From<&'a str> for ValueRef<'a> {
        fn from(v: &'a str) -> Self {
            ValueRef(v.as_bytes())
        }
    }

    impl<'a> From<ValueRef<'a>> for Value {
        fn from(v: ValueRef<'a>) -> Self {
            Value(v.0.into())
        }
    }

    impl From<&str> for Value {
        fn from(v: &str) -> Self {
            Value(v.as_bytes().into())
        }
    }

    impl Value {
        #[must_use]
        pub(crate) fn as_ref(&self) -> ValueRef<'_> {
            ValueRef(self.0.as_ref())
        }
    }

    impl StateRef<'_> {
        #[must_use]
        pub(crate) fn is_set(&self) -> bool {
            matches!(self, StateRef::Set | StateRef::Value(_))
        }
    }

    impl<'a> StateRef<'a> {
        #[must_use]
        pub(crate) fn from_bytes(input: &'a [u8]) -> Self {
            Self::Value(ValueRef::from_bytes(input))
        }
    }

    impl StateRef<'_> {
        #[must_use]
        pub(crate) fn to_owned(self) -> StackState {
            self.into()
        }
    }

    impl<'a> StackState {
        #[must_use]
        pub(crate) fn as_ref(&'a self) -> StateRef<'a> {
            match self {
                StackState::Value(v) => StateRef::Value(v.as_ref()),
                StackState::Set => StateRef::Set,
                StackState::Unset => StateRef::Unset,
                StackState::Unspecified => StateRef::Unspecified,
            }
        }
    }

    impl<'a> From<StateRef<'a>> for StackState {
        fn from(s: StateRef<'a>) -> Self {
            match s {
                StateRef::Value(v) => StackState::Value(v.into()),
                StateRef::Set => StackState::Set,
                StateRef::Unset => StackState::Unset,
                StateRef::Unspecified => StackState::Unspecified,
            }
        }
    }
}

pub(crate) mod search {
    use kstring::KString;
    use smallvec::SmallVec;
    use std::collections::HashMap;

    mod attributes {
        use super::Attributes;
        use crate::git::GitError;
        use crate::git::Mapping;
        use crate::git::search::{
            Assignments, MetadataCollection, Outcome, TrackedAssignment, Value,
        };
        use crate::git::{Search, TraitPattern};
        use bstr::{BStr, ByteSlice};
        use std::path::{Path, PathBuf};

        impl Search {
            pub(crate) fn new_globals(
                files: impl IntoIterator<Item = impl Into<PathBuf>>,
                buf: &mut Vec<u8>,
                collection: &mut MetadataCollection,
            ) -> std::io::Result<Self> {
                let mut group = Self::default();
                group.add_patterns_buffer(
                    b"[attr]binary -diff -merge -text",
                    "[builtin]".into(),
                    None,
                    collection,
                    true, /* allow macros */
                );

                for path in files {
                    group.add_patterns_file(
                        path.into(),
                        true,
                        None,
                        buf,
                        collection,
                        true, /* allow macros */
                    )?;
                }
                Ok(group)
            }
        }

        impl Search {
            pub(crate) fn add_patterns_file(
                &mut self,
                source: PathBuf,
                follow_symlinks: bool,
                root: Option<&Path>,
                buf: &mut Vec<u8>,
                collection: &mut MetadataCollection,
                allow_macros: bool,
            ) -> std::io::Result<bool> {
                let was_added = crate::git::add_patterns_file(
                    &mut self.patterns,
                    source,
                    follow_symlinks,
                    root,
                    buf,
                    Attributes,
                )?;
                if was_added {
                    let last = self.patterns.last_mut().expect("just added");
                    if !allow_macros {
                        last.patterns
                            .retain(|p| !matches!(p.value, Value::MacroAssignments { .. }));
                    }
                    collection.update_from_list(last);
                }
                Ok(was_added)
            }
            pub(crate) fn add_patterns_buffer(
                &mut self,
                bytes: &[u8],
                source: PathBuf,
                root: Option<&Path>,
                collection: &mut MetadataCollection,
                allow_macros: bool,
            ) {
                self.patterns.push(crate::git::List::from_bytes(
                    bytes, source, root, Attributes,
                ));
                let last = self.patterns.last_mut().expect("just added");
                if !allow_macros {
                    last.patterns
                        .retain(|p| !matches!(p.value, Value::MacroAssignments { .. }));
                }
                collection.update_from_list(last);
            }
        }

        impl Search {
            pub(crate) fn pattern_matching_relative_path(
                &self,
                relative_path: &BStr,
                case: crate::git::Case,
                is_dir: Option<bool>,
                out: &mut Outcome,
            ) -> bool {
                let basename_pos = relative_path.rfind(b"/").map(|p| p + 1);
                let mut has_match = false;
                self.patterns.iter().rev().any(|pl| {
                    has_match |= pattern_matching_relative_path(
                        pl,
                        relative_path,
                        basename_pos,
                        case,
                        is_dir,
                        out,
                    );
                    out.is_done()
                });
                has_match
            }
        }

        impl TraitPattern for Attributes {
            type Value = Value;

            fn bytes_to_patterns(
                &self,
                bytes: &[u8],
                _source: &std::path::Path,
            ) -> Vec<Mapping<Self::Value>> {
                fn into_owned_assignments<'a>(
                    attrs: impl Iterator<Item = Result<crate::git::AssignmentRef<'a>, GitError>>,
                ) -> Option<Assignments> {
                    let res = attrs
                        .map(|res| {
                            res.map(|a| TrackedAssignment {
                                id: super::super::search::AttributeId(usize::MAX),
                                inner: a.to_owned(),
                            })
                        })
                        .collect::<Result<Assignments, _>>();
                    res.ok()
                }

                crate::git::parse_lines(bytes)
                    .filter_map(std::result::Result::ok)
                    .filter_map(|(pattern_kind, assignments, line_number)| {
                        let (pattern, value) = match pattern_kind {
                            crate::git::Kind::Macro(macro_name) => (
                                crate::git::Pattern {
                                    text: macro_name.as_str().into(),
                                    mode: macro_mode(),
                                    first_wildcard_pos: None,
                                },
                                Value::MacroAssignments {
                                    id: super::super::search::AttributeId(usize::MAX),
                                    assignments: into_owned_assignments(assignments)?,
                                },
                            ),
                            crate::git::Kind::Pattern(p) => (
                                (!p.is_negative()).then_some(p)?,
                                Value::Assignments(into_owned_assignments(assignments)?),
                            ),
                        };
                        Mapping {
                            pattern,
                            value,
                            sequence_number: line_number,
                        }
                        .into()
                    })
                    .collect()
            }
        }

        impl Attributes {
            fn may_use_glob_pattern(pattern: &crate::git::Pattern) -> bool {
                pattern.mode != macro_mode()
            }
        }

        fn macro_mode() -> crate::git::PatternMode {
            crate::git::PatternMode::all()
        }

        #[allow(unused_variables)]
        fn pattern_matching_relative_path(
            list: &crate::git::List<Attributes>,
            relative_path: &BStr,
            basename_pos: Option<usize>,
            case: crate::git::Case,
            is_dir: Option<bool>,
            out: &mut Outcome,
        ) -> bool {
            let (relative_path, basename_start_pos) = match list
                .strip_base_handle_recompute_basename_pos(relative_path, basename_pos, case)
            {
                Some(r) => r,
                None => return false,
            };
            let cur_len = out.remaining();
            'outer: for Mapping {
                pattern,
                value,
                sequence_number,
            } in list
                .patterns
                .iter()
                .rev()
                .filter(|pm| Attributes::may_use_glob_pattern(&pm.pattern))
            {
                let value: &Value = value;
                let attrs = match value {
                    Value::MacroAssignments { .. } => {
                        unreachable!("we can't match on macros as they have no pattern")
                    }
                    Value::Assignments(attrs) => attrs,
                };
                if out.has_unspecified_attributes(attrs.iter().map(|attr| attr.id))
                    && pattern.matches_repo_relative_path(
                        relative_path,
                        basename_start_pos,
                        is_dir,
                        case,
                        crate::git::WildmatchMode::NO_MATCH_SLASH_LITERAL,
                    )
                {
                    let all_filled = out.fill_attributes(
                        attrs.iter(),
                        pattern,
                        list.source.as_ref(),
                        *sequence_number,
                    );
                    if all_filled {
                        break 'outer;
                    }
                }
            }
            cur_len != out.remaining()
        }
    }
    mod outcome {
        use crate::git::Pattern;
        use crate::git::search::{
            Assignments, AttributeId, Attributes, MatchKind, Metadata, MetadataCollection, Outcome,
            RefMapKey, TrackedAssignment, Value,
        };
        use crate::git::{AssignmentRef, NameRef, StateRef};
        use bstr::{BString, ByteSlice};
        use kstring::{KString, KStringRef};

        impl Outcome {
            pub(crate) fn initialize(&mut self, collection: &MetadataCollection) {
                if self.matches_by_id.len() != collection.name_to_meta.len() {
                    let global_num_attrs = collection.name_to_meta.len();

                    self.matches_by_id
                        .resize(global_num_attrs, Default::default());

                    for (order, macro_attributes) in collection.iter().filter_map(|(_, meta)| {
                        (!meta.macro_attributes.is_empty())
                            .then_some((meta.id.0, &meta.macro_attributes))
                    }) {
                        self.matches_by_id[order]
                            .macro_attributes
                            .clone_from(macro_attributes);
                    }

                    for (name, id) in self.selected.iter_mut().filter(|(_, id)| id.is_none()) {
                        *id = collection
                            .name_to_meta
                            .get(name.as_str())
                            .map(|meta| meta.id);
                    }
                }
                self.reset();
            }

            pub(crate) fn initialize_with_selection<'a>(
                &mut self,
                collection: &MetadataCollection,
                attribute_names: impl IntoIterator<Item = impl Into<KStringRef<'a>>>,
            ) {
                self.initialize_with_selection_inner(
                    collection,
                    &mut attribute_names.into_iter().map(Into::into),
                );
            }

            fn initialize_with_selection_inner(
                &mut self,
                collection: &MetadataCollection,
                attribute_names: &mut dyn Iterator<Item = KStringRef<'_>>,
            ) {
                self.selected.clear();
                self.selected.extend(attribute_names.map(|name| {
                    (
                        name.to_owned(),
                        collection
                            .name_to_meta
                            .get(name.as_str())
                            .map(|meta| meta.id),
                    )
                }));

                self.initialize(collection);
                self.reset_remaining();
            }

            pub(crate) fn reset(&mut self) {
                self.matches_by_id
                    .iter_mut()
                    .for_each(|item| item.r#match = None);
                self.attrs_stack.clear();
                self.reset_remaining();
            }

            fn reset_remaining(&mut self) {
                self.remaining = Some(if self.selected.is_empty() {
                    self.matches_by_id.len()
                } else {
                    self.selected
                        .iter()
                        .filter(|(_name, id)| id.is_some())
                        .count()
                });
            }
        }

        impl Outcome {
            pub(crate) fn iter_selected(
                &self,
            ) -> impl Iterator<Item = crate::git::search::Match<'_>> {
                static DUMMY: Pattern = Pattern {
                    text: BString::new(Vec::new()),
                    mode: crate::git::PatternMode::empty(),
                    first_wildcard_pos: None,
                };
                self.selected.iter().map(|(name, id)| {
                    id.and_then(|id| {
                        self.matches_by_id[id.0]
                            .r#match
                            .as_ref()
                            .map(|m| m.to_outer(self))
                    })
                    .unwrap_or_else(|| crate::git::search::Match {
                        pattern: &DUMMY,
                        assignment: AssignmentRef {
                            name: NameRef::try_from(name.as_bytes().as_bstr())
                                .unwrap_or_else(|_| NameRef("invalid".into())),
                            state: StateRef::Unspecified,
                        },
                        kind: MatchKind::Attribute { macro_id: None },
                        location: crate::git::search::MatchLocation {
                            source: None,
                            sequence_number: 0,
                        },
                    })
                })
            }
            #[must_use]
            pub(crate) fn is_done(&self) -> bool {
                self.remaining() == 0
            }
        }

        impl Outcome {
            pub(crate) fn fill_attributes<'a>(
                &mut self,
                attrs: impl Iterator<Item = &'a TrackedAssignment>,
                pattern: &crate::git::Pattern,
                source: Option<&std::path::PathBuf>,
                sequence_number: usize,
            ) -> bool {
                self.attrs_stack.extend(
                    attrs
                        .filter(|attr| self.matches_by_id[attr.id.0].r#match.is_none())
                        .map(|attr| (attr.id, attr.inner.clone(), None)),
                );
                while let Some((id, assignment, parent_order)) = self.attrs_stack.pop() {
                    let slot = &mut self.matches_by_id[id.0];
                    if slot.r#match.is_some() {
                        continue;
                    }
                    let is_macro = !slot.macro_attributes.is_empty();
                    let expand_macro =
                        is_macro && matches!(assignment.state, crate::git::StackState::Set);
                    slot.r#match = Some(Match {
                        pattern: self.patterns.insert(pattern),
                        assignment: self.assignments.insert_owned(assignment),
                        kind: if is_macro {
                            MatchKind::Macro {
                                parent_macro_id: parent_order,
                            }
                        } else {
                            MatchKind::Attribute {
                                macro_id: parent_order,
                            }
                        },
                        location: MatchLocation {
                            source: source.map(|path| self.source_paths.insert(path)),
                            sequence_number,
                        },
                    });
                    if self.reduce_and_check_if_done(id) {
                        return true;
                    }

                    if expand_macro {
                        let slot = &self.matches_by_id[id.0];
                        self.attrs_stack.extend(
                            slot.macro_attributes
                                .iter()
                                .filter(|attr| self.matches_by_id[attr.id.0].r#match.is_none())
                                .map(|attr| (attr.id, attr.inner.clone(), Some(id))),
                        );
                    }
                }
                false
            }
        }

        impl Outcome {
            pub(crate) fn has_unspecified_attributes(
                &self,
                mut attrs: impl Iterator<Item = AttributeId>,
            ) -> bool {
                attrs.any(|order| self.matches_by_id[order.0].r#match.is_none())
            }
            pub(crate) fn remaining(&self) -> usize {
                self.remaining
                    .expect("BUG: instance must be initialized for each search set")
            }

            fn reduce_and_check_if_done(&mut self, attr: AttributeId) -> bool {
                if self.selected.is_empty()
                    || self.selected.iter().any(|(_name, id)| *id == Some(attr))
                {
                    *self.remaining.as_mut().expect("initialized") -= 1;
                }
                self.is_done()
            }
        }

        impl MetadataCollection {
            pub(crate) fn update_from_list(&mut self, list: &mut crate::git::List<Attributes>) {
                for pattern in &mut list.patterns {
                    match &mut pattern.value {
                        Value::MacroAssignments {
                            id: order,
                            assignments,
                        } => {
                            *order = self.id_for_macro(
                                pattern.pattern.text.to_str().expect(
                                    "valid macro names are always UTF8 and this was verified",
                                ),
                                assignments,
                            );
                        }
                        Value::Assignments(assignments) => {
                            self.assign_order_to_attributes(assignments);
                        }
                    }
                }
            }
        }

        impl MetadataCollection {
            pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &Metadata)> {
                self.name_to_meta.iter().map(|(k, v)| (k.as_str(), v))
            }
        }

        impl MetadataCollection {
            pub(crate) fn id_for_macro(
                &mut self,
                name: &str,
                attrs: &mut Assignments,
            ) -> AttributeId {
                let order = if let Some(meta) = self.name_to_meta.get_mut(name) {
                    meta.id
                } else {
                    let order = AttributeId(self.name_to_meta.len());
                    self.name_to_meta.insert(
                        KString::from_ref(name),
                        Metadata {
                            id: order,
                            macro_attributes: Default::default(),
                        },
                    );
                    order
                };

                self.assign_order_to_attributes(attrs);
                self.name_to_meta
                    .get_mut(name)
                    .expect("just added")
                    .macro_attributes
                    .clone_from(attrs);

                order
            }
            pub(crate) fn id_for_attribute(&mut self, name: &str) -> AttributeId {
                if let Some(meta) = self.name_to_meta.get(name) {
                    meta.id
                } else {
                    let order = AttributeId(self.name_to_meta.len());
                    self.name_to_meta
                        .insert(KString::from_ref(name), order.into());
                    order
                }
            }
            pub(crate) fn assign_order_to_attributes(
                &mut self,
                attributes: &mut [TrackedAssignment],
            ) {
                for TrackedAssignment {
                    id: order,
                    inner: crate::git::Assignment { name, .. },
                } in attributes
                {
                    *order = self.id_for_attribute(&name.0);
                }
            }
        }

        impl From<AttributeId> for Metadata {
            fn from(order: AttributeId) -> Self {
                Metadata {
                    id: order,
                    macro_attributes: Default::default(),
                }
            }
        }

        #[derive(Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
        pub(crate) struct Match {
            pub pattern: RefMapKey,
            pub assignment: RefMapKey,
            pub kind: MatchKind,
            pub location: MatchLocation,
        }

        impl Match {
            fn to_outer<'a>(&self, out: &'a Outcome) -> crate::git::search::Match<'a> {
                crate::git::search::Match {
                    pattern: out
                        .patterns
                        .resolve(self.pattern)
                        .expect("pattern still present"),
                    assignment: out
                        .assignments
                        .resolve(self.assignment)
                        .expect("assignment present")
                        .as_ref(),
                    kind: self.kind,
                    location: self.location.to_outer(out),
                }
            }
        }

        #[derive(Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
        pub(crate) struct MatchLocation {
            pub source: Option<RefMapKey>,
            pub sequence_number: usize,
        }

        impl MatchLocation {
            fn to_outer<'a>(&self, out: &'a Outcome) -> crate::git::search::MatchLocation<'a> {
                crate::git::search::MatchLocation {
                    source: self
                        .source
                        .and_then(|source| out.source_paths.resolve(source).map(AsRef::as_ref)),
                    sequence_number: self.sequence_number,
                }
            }
        }
    }

    use crate::git::{Assignment, AssignmentRef};
    use std::{
        collections::{BTreeMap, btree_map::Entry, hash_map::DefaultHasher},
        hash::{Hash, Hasher},
    };

    pub(crate) type RefMapKey = u64;
    #[derive(Clone)]
    pub(crate) struct RefMap<T>(BTreeMap<RefMapKey, T>);

    impl<T> Default for RefMap<T> {
        fn default() -> Self {
            RefMap(Default::default())
        }
    }

    impl<T> RefMap<T>
    where
        T: Hash + Clone,
    {
        pub(crate) fn insert(&mut self, value: &T) -> RefMapKey {
            let mut s = DefaultHasher::new();
            value.hash(&mut s);
            let key = s.finish();
            match self.0.entry(key) {
                Entry::Vacant(e) => {
                    e.insert(value.clone());
                    key
                }
                Entry::Occupied(_) => key,
            }
        }

        pub(crate) fn insert_owned(&mut self, value: T) -> RefMapKey {
            let mut s = DefaultHasher::new();
            value.hash(&mut s);
            let key = s.finish();
            match self.0.entry(key) {
                Entry::Vacant(e) => {
                    e.insert(value);
                    key
                }
                Entry::Occupied(_) => key,
            }
        }

        pub(crate) fn resolve(&self, key: RefMapKey) -> Option<&T> {
            self.0.get(&key)
        }
    }

    pub(crate) type Assignments = SmallVec<[TrackedAssignment; AVERAGE_NUM_ATTRS]>;

    #[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
    pub(crate) enum Value {
        MacroAssignments {
            id: AttributeId,
            assignments: Assignments,
        },
        Assignments(Assignments),
    }

    #[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
    pub(crate) struct TrackedAssignment {
        pub id: AttributeId,
        pub inner: Assignment,
    }

    #[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Default)]
    pub(crate) struct Attributes;

    #[derive(Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
    pub(crate) struct Match<'a> {
        pub pattern: &'a crate::git::Pattern,
        pub assignment: AssignmentRef<'a>,
        pub kind: MatchKind,
        pub location: MatchLocation<'a>,
    }

    #[derive(Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
    pub(crate) struct MatchLocation<'a> {
        pub source: Option<&'a std::path::Path>,
        pub sequence_number: usize,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
    pub(crate) enum MatchKind {
        Attribute {
            macro_id: Option<AttributeId>,
        },
        Macro {
            parent_macro_id: Option<AttributeId>,
        },
    }

    #[derive(Default, Clone)]
    pub(crate) struct Outcome {
        matches_by_id: Vec<Slot>,
        attrs_stack: SmallVec<[(AttributeId, Assignment, Option<AttributeId>); 8]>,
        selected: SmallVec<[(KString, Option<AttributeId>); AVERAGE_NUM_ATTRS]>,
        patterns: RefMap<crate::git::Pattern>,
        assignments: RefMap<Assignment>,
        source_paths: RefMap<std::path::PathBuf>,
        remaining: Option<usize>,
    }

    #[derive(Default, Clone)]
    struct Slot {
        r#match: Option<outcome::Match>,
        macro_attributes: Assignments,
    }

    #[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
    pub(crate) struct AttributeId(pub usize);

    #[derive(Clone, Debug, Default)]
    pub(crate) struct MetadataCollection {
        name_to_meta: HashMap<KString, Metadata>,
    }

    #[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
    pub(crate) struct Metadata {
        pub id: AttributeId,
        pub macro_attributes: Assignments,
    }

    const AVERAGE_NUM_ATTRS: usize = 3;
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]

enum Kind {
    Pattern(Pattern),
    Macro(Name),
}

struct Lines<'a> {
    lines: bstr::Lines<'a>,
    line_no: usize,
}

struct FieldIter<'a> {
    attrs: bstr::Fields<'a>,
}

impl<'a> FieldIter<'a> {
    #[must_use]
    fn new(input: &'a BStr) -> Self {
        FieldIter {
            attrs: input.fields(),
        }
    }

    fn parse_attr(&self, attr: &'a [u8]) -> Result<AssignmentRef<'a>, GitError> {
        let mut tokens = attr.splitn(2, |b| *b == b'=');
        let attr = tokens.next().expect("attr itself").as_bstr();
        let possibly_value = tokens.next();
        let (attr, state) = if attr.first() == Some(&b'-') {
            (&attr[1..], StateRef::Unset)
        } else if attr.first() == Some(&b'!') {
            (&attr[1..], StateRef::Unspecified)
        } else {
            (
                attr,
                possibly_value.map_or(StateRef::Set, StateRef::from_bytes),
            )
        };
        Ok(AssignmentRef::new(check_attr(attr)?, state))
    }
}

fn check_attr(attr: &BStr) -> Result<NameRef<'_>, GitError> {
    NameRef::try_from(attr)
}

impl<'a> Iterator for FieldIter<'a> {
    type Item = Result<AssignmentRef<'a>, GitError>;

    fn next(&mut self) -> Option<Self::Item> {
        let attr = self.attrs.next().filter(|a| !a.is_empty())?;
        self.parse_attr(attr).into()
    }
}

impl<'a> Lines<'a> {
    #[must_use]
    fn new(bytes: &'a [u8]) -> Self {
        let bom = unicode_bom::Bom::from(bytes);
        Lines {
            lines: bytes[bom.len()..].lines(),
            line_no: 0,
        }
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = Result<(Kind, FieldIter<'a>, usize), GitError>;

    fn next(&mut self) -> Option<Self::Item> {
        fn skip_blanks(line: &BStr) -> &BStr {
            line.find_not_byteset(BLANKS)
                .map_or(line, |pos| &line[pos..])
        }
        for line in self.lines.by_ref() {
            self.line_no += 1;
            let line = skip_blanks(line.into());
            if line.first() == Some(&b'#') {
                continue;
            }
            match parse_line(line, self.line_no) {
                None => continue,
                Some(res) => return Some(res),
            }
        }
        None
    }
}

fn parse_line(
    line: &BStr,
    line_number: usize,
) -> Option<Result<(Kind, FieldIter<'_>, usize), GitError>> {
    if line.is_empty() {
        return None;
    }

    let (line, attrs): (Cow<'_, _>, _) = if line.starts_with(b"\"") {
        let (unquoted, consumed) = match undo(line) {
            Ok(res) => res,
            Err(_) => return Some(Err(GitError::Gen)),
        };
        (unquoted, &line[consumed..])
    } else {
        line.find_byteset(BLANKS)
            .map(|pos| (line[..pos].as_bstr().into(), line[pos..].as_bstr()))
            .unwrap_or((line.into(), [].as_bstr()))
    };

    let kind_res = if let Some(macro_name) = line.strip_prefix(b"[attr]") {
        check_attr(macro_name.into())
            .map_err(|_| GitError::Gen)
            .map(|name| Kind::Macro(name.to_owned()))
    } else {
        let pattern = Pattern::from_bytes(line.as_ref())?;
        if pattern.mode.contains(PatternMode::NEGATIVE) {
            Err(GitError::Gen)
        } else {
            Ok(Kind::Pattern(pattern))
        }
    };
    let kind = match kind_res {
        Ok(kind) => kind,
        Err(err) => return Some(Err(err)),
    };
    Ok((kind, FieldIter::new(attrs), line_number)).into()
}

const BLANKS: &[u8] = b" \t\r";

#[must_use]
fn parse_lines(bytes: &[u8]) -> Lines<'_> {
    Lines::new(bytes)
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
enum StateRef<'a> {
    Set,
    Unset,
    Value(state::ValueRef<'a>),
    Unspecified,
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
enum StackState {
    Set,
    Unset,
    Value(state::Value),
    Unspecified,
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
struct Name(KString);

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
struct NameRef<'a>(KStringRef<'a>);

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
struct Assignment {
    name: Name,
    state: StackState,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
struct AssignmentRef<'a> {
    name: NameRef<'a>,
    state: StateRef<'a>,
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Default)]
struct Search {
    patterns: Vec<List<search::Attributes>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
enum AttributesSource {
    System,
}

impl AttributesSource {
    fn storage_location(self) -> Option<Cow<'static, Path>> {
        Some(system_prefix()?.join("etc/gitattributes").into())
    }
}

mod cache {
    pub(crate) struct Debug;

    impl Debug {
        #[inline]
        #[must_use]
        pub(crate) fn new(_owner: String) -> Self {
            Debug
        }
        pub(crate) fn put(&mut self) {}
        pub(crate) fn hit(&mut self) {}
        pub(crate) fn miss(&mut self) {}
    }
}

pub(crate) mod fs {
    #[must_use]
    pub(crate) fn open_options_no_follow() -> std::fs::OpenOptions {
        let mut options = std::fs::OpenOptions::new();
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
        options
    }
}

pub(crate) mod hash {
    #[must_use]
    pub(crate) fn crc32_update(previous_value: u32, bytes: &[u8]) -> u32 {
        let mut h = crc32fast::Hasher::new_with_initial(previous_value);
        h.update(bytes);
        h.finalize()
    }

    #[must_use]
    pub(crate) fn crc32(bytes: &[u8]) -> u32 {
        let mut h = crc32fast::Hasher::new();
        h.update(bytes);
        h.finalize()
    }
}

pub(crate) mod zlib {
    use crate::git::GitError;

    pub(crate) struct Decompress(zlib_rs::Inflate);

    impl Default for Decompress {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Decompress {
        #[must_use]
        pub(crate) fn total_in(&self) -> u64 {
            self.0.total_in()
        }

        #[must_use]
        pub(crate) fn total_out(&self) -> u64 {
            self.0.total_out()
        }

        #[must_use]
        pub(crate) fn new() -> Self {
            let config = zlib_rs::InflateConfig::default();
            let header = true;
            let inner = zlib_rs::Inflate::new(header, config.window_bits as u8);
            Self(inner)
        }

        pub(crate) fn reset(&mut self) {
            self.0.reset(true);
        }

        pub(crate) fn decompress(
            &mut self,
            input: &[u8],
            output: &mut [u8],
            flush: FlushDecompress,
        ) -> Result<Status, GitError> {
            let inflate_flush = match flush {
                FlushDecompress::None => zlib_rs::InflateFlush::NoFlush,

                FlushDecompress::Finish => zlib_rs::InflateFlush::Finish,
            };

            let status = self
                .0
                .decompress(input, output, inflate_flush)
                .map_err(|_| GitError::Gen)?;
            match status {
                zlib_rs::Status::Ok => Ok(Status::Ok),
                zlib_rs::Status::BufError => Ok(Status::BufError),
                zlib_rs::Status::StreamEnd => Ok(Status::StreamEnd),
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Status {
        Ok,
        BufError,
        StreamEnd,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    #[non_exhaustive]
    #[allow(clippy::unnecessary_cast)]
    pub(crate) enum FlushDecompress {
        None = 0,
        // Sync = 2,
        Finish = 4,
    }

    #[derive(Default)]
    pub(crate) struct Inflate {
        pub state: Decompress,
    }

    impl Inflate {
        pub(crate) fn once(
            &mut self,
            input: &[u8],
            out: &mut [u8],
        ) -> Result<(Status, usize, usize), GitError> {
            let before_in = self.state.total_in();
            let before_out = self.state.total_out();
            let status = self.state.decompress(input, out, FlushDecompress::None)?;
            Ok((
                status,
                (self.state.total_in() - before_in) as usize,
                (self.state.total_out() - before_out) as usize,
            ))
        }

        pub(crate) fn reset(&mut self) {
            self.state.reset();
        }
    }

    pub(crate) mod stream {
        pub(crate) mod inflate {
            use std::{io, io::BufRead};

            use crate::git::zlib::{Decompress, FlushDecompress, Status};

            pub(crate) fn read(
                rd: &mut impl BufRead,
                state: &mut Decompress,
                mut dst: &mut [u8],
            ) -> io::Result<usize> {
                let mut total_written = 0;
                loop {
                    let (written, consumed, ret, eof);
                    {
                        let input = rd.fill_buf()?;
                        eof = input.is_empty();
                        let before_out = state.total_out();
                        let before_in = state.total_in();
                        let flush = if eof {
                            FlushDecompress::Finish
                        } else {
                            FlushDecompress::None
                        };
                        ret = state.decompress(input, dst, flush);
                        written = (state.total_out() - before_out) as usize;
                        total_written += written;
                        dst = &mut dst[written..];
                        consumed = (state.total_in() - before_in) as usize;
                    }
                    rd.consume(consumed);

                    match ret {
                        Ok(Status::StreamEnd) => return Ok(total_written),
                        Ok(Status::Ok | Status::BufError) if eof || dst.is_empty() => {
                            return Ok(total_written);
                        }
                        Ok(Status::Ok | Status::BufError) if consumed != 0 || written != 0 => {
                            continue;
                        }
                        Ok(Status::Ok | Status::BufError) => {
                            unreachable!("Definitely a bug somewhere")
                        }
                        Err(..) => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "corrupt deflate stream",
                            ));
                        }
                    }
                }
            }
        }
    }
}

#[derive(Default, Clone)]
struct Buffers {
    src: Vec<u8>,
    dest: Vec<u8>,
}

impl Buffers {
    fn use_foreign_src<'a, 'src>(&'a mut self, src: &'src [u8]) -> WithForeignSource<'src, 'a> {
        self.clear();
        WithForeignSource {
            ro_src: Some(src),
            src: &mut self.src,
            dest: &mut self.dest,
        }
    }
}

impl Buffers {
    fn clear(&mut self) {
        self.src.clear();
        self.dest.clear();
    }
}

struct WithForeignSource<'src, 'bufs> {
    ro_src: Option<&'src [u8]>,
    src: &'bufs mut Vec<u8>,
    dest: &'bufs mut Vec<u8>,
}

impl WithForeignSource<'_, '_> {
    fn swap(&mut self) {
        self.ro_src.take();
        std::mem::swap(&mut self.src, &mut self.dest);
        self.dest.clear();
    }
    fn src_and_dest(&mut self) -> (&[u8], &mut Vec<u8>) {
        match self.ro_src {
            Some(src) => (src, &mut self.dest),
            None => (self.src, &mut self.dest),
        }
    }
}

#[must_use]
fn str_precompose(s: Cow<'_, str>) -> Cow<'_, str> {
    use unicode_normalization::{UnicodeNormalization, is_nfc};
    if is_nfc(s.as_ref()) {
        s
    } else {
        Cow::Owned(s.as_ref().nfc().collect())
    }
}

#[must_use]
fn str_decompose(s: Cow<'_, str>) -> Cow<'_, str> {
    use unicode_normalization::{UnicodeNormalization, is_nfd};
    if is_nfd(s.as_ref()) {
        s
    } else {
        Cow::Owned(s.as_ref().nfd().collect())
    }
}

fn to_unsigned_with_radix<I: MinNumTraits>(bytes: &[u8], radix: u32) -> Result<I, GitError> {
    let base = I::from_u32(radix).expect("radix can be represented as integer");

    if bytes.is_empty() {
        return Err(GitError::Gen);
    }

    let mut result = I::ZERO;

    for &digit in bytes {
        let x = match char::from(digit).to_digit(radix).and_then(I::from_u32) {
            Some(x) => x,
            None => {
                return Err(GitError::Gen);
            }
        };
        result = match result.checked_mul(base) {
            Some(result) => result,
            None => {
                return Err(GitError::Gen);
            }
        };
        result = match result.checked_add(x) {
            Some(result) => result,
            None => {
                return Err(GitError::Gen);
            }
        };
    }

    Ok(result)
}

fn to_signed<I: MinNumTraits>(bytes: &[u8]) -> Result<I, GitError> {
    to_signed_with_radix(bytes, 10)
}

fn to_signed_with_radix<I: MinNumTraits>(bytes: &[u8], radix: u32) -> Result<I, GitError> {
    let base = I::from_u32(radix).expect("radix can be represented as integer");

    if bytes.is_empty() {
        return Err(GitError::Gen);
    }

    let digits = match bytes[0] {
        b'+' => return to_unsigned_with_radix(&bytes[1..], radix),
        b'-' => &bytes[1..],
        _ => return to_unsigned_with_radix(bytes, radix),
    };

    if digits.is_empty() {
        return Err(GitError::Gen);
    }

    let mut result = I::ZERO;

    for &digit in digits {
        let Some(x) = char::from(digit).to_digit(radix).and_then(I::from_u32) else {
            return Err(GitError::Gen);
        };

        result = match result.checked_mul(base) {
            Some(result) => result,
            None => {
                return Err(GitError::Gen);
            }
        };
        result = match result.checked_sub(x) {
            Some(result) => result,
            None => {
                return Err(GitError::Gen);
            }
        };
    }

    Ok(result)
}

trait MinNumTraits: Sized + Copy + TryFrom<u32> {
    const ZERO: Self;
    #[must_use]
    fn from_u32(n: u32) -> Option<Self> {
        Self::try_from(n).ok()
    }
    fn checked_mul(self, rhs: Self) -> Option<Self>;
    fn checked_add(self, rhs: Self) -> Option<Self>;
    fn checked_sub(self, v: Self) -> Option<Self>;
}

impl MinNumTraits for i32 {
    const ZERO: Self = 0;

    fn checked_add(self, rhs: Self) -> Option<Self> {
        Self::checked_add(self, rhs)
    }

    fn checked_mul(self, rhs: Self) -> Option<Self> {
        Self::checked_mul(self, rhs)
    }

    fn checked_sub(self, rhs: Self) -> Option<Self> {
        Self::checked_sub(self, rhs)
    }
}

impl MinNumTraits for i64 {
    const ZERO: Self = 0;

    fn checked_add(self, rhs: Self) -> Option<Self> {
        Self::checked_add(self, rhs)
    }

    fn checked_mul(self, rhs: Self) -> Option<Self> {
        Self::checked_mul(self, rhs)
    }

    fn checked_sub(self, rhs: Self) -> Option<Self> {
        Self::checked_sub(self, rhs)
    }
}

impl MinNumTraits for u64 {
    const ZERO: Self = 0;

    fn checked_add(self, rhs: Self) -> Option<Self> {
        Self::checked_add(self, rhs)
    }

    fn checked_mul(self, rhs: Self) -> Option<Self> {
        Self::checked_mul(self, rhs)
    }

    fn checked_sub(self, rhs: Self) -> Option<Self> {
        Self::checked_sub(self, rhs)
    }
}

impl MinNumTraits for u8 {
    const ZERO: Self = 0;

    fn checked_add(self, rhs: Self) -> Option<Self> {
        Self::checked_add(self, rhs)
    }

    fn checked_mul(self, rhs: Self) -> Option<Self> {
        Self::checked_mul(self, rhs)
    }

    fn checked_sub(self, rhs: Self) -> Option<Self> {
        Self::checked_sub(self, rhs)
    }
}

impl MinNumTraits for usize {
    const ZERO: Self = 0;

    fn checked_add(self, rhs: Self) -> Option<Self> {
        Self::checked_add(self, rhs)
    }

    fn checked_mul(self, rhs: Self) -> Option<Self> {
        Self::checked_mul(self, rhs)
    }

    fn checked_sub(self, rhs: Self) -> Option<Self> {
        Self::checked_sub(self, rhs)
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]

pub(crate) struct Pattern {
    text: BString,
    mode: PatternMode,
    first_wildcard_pos: Option<usize>,
}

#[derive(Default, Debug, PartialOrd, PartialEq, Copy, Clone, Hash, Ord, Eq)]
enum Case {
    #[default]
    Sensitive,
}

impl Pattern {
    fn from_bytes(text: &[u8]) -> Option<Self> {
        parse_pattern(text, true).map(|(text, mode, first_wildcard_pos)| Pattern {
            text: text.into(),
            mode,
            first_wildcard_pos,
        })
    }
}

impl Pattern {
    fn is_negative(&self) -> bool {
        self.mode.contains(PatternMode::NEGATIVE)
    }

    fn matches_repo_relative_path(
        &self,
        path: &BStr,
        basename_start_pos: Option<usize>,
        is_dir: Option<bool>,
        case: Case,
        mode: WildmatchMode,
    ) -> bool {
        let is_dir = is_dir.unwrap_or(false);
        if !is_dir && self.mode.contains(PatternMode::MUST_BE_DIR) {
            return false;
        }

        let flags = mode
            | match case {
                // Case::Fold => WildmatchMode::IGNORE_CASE,
                Case::Sensitive => WildmatchMode::empty(),
            };
        if self.mode.contains(PatternMode::NO_SUB_DIR) && !self.mode.contains(PatternMode::ABSOLUTE)
        {
            let basename = &path[basename_start_pos.unwrap_or_default()..];
            self.matches(basename, flags)
        } else {
            self.matches(path, flags)
        }
    }

    fn matches(&self, value: &BStr, mode: WildmatchMode) -> bool {
        match self.first_wildcard_pos {
            Some(pos)
                if self.mode.contains(PatternMode::ENDS_WITH)
                    && (!mode.contains(WildmatchMode::NO_MATCH_SLASH_LITERAL)
                        || !value.contains(&b'/')) =>
            {
                let text = &self.text[pos + 1..];
                if mode.contains(WildmatchMode::IGNORE_CASE) {
                    value
                        .len()
                        .checked_sub(text.len())
                        .is_some_and(|start| text.eq_ignore_ascii_case(&value[start..]))
                } else {
                    value.ends_with(text.as_ref())
                }
            }
            Some(pos) => {
                if mode.contains(WildmatchMode::IGNORE_CASE) {
                    if !value
                        .get(..pos)
                        .is_some_and(|value| value.eq_ignore_ascii_case(&self.text[..pos]))
                    {
                        return false;
                    }
                } else if !value.starts_with(&self.text[..pos]) {
                    return false;
                }
                wildmatch(self.text.as_bstr(), value, mode)
            }
            None => {
                if mode.contains(WildmatchMode::IGNORE_CASE) {
                    self.text.eq_ignore_ascii_case(value)
                } else {
                    self.text == value
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Default)]
struct List<T: TraitPattern> {
    patterns: Vec<Mapping<T::Value>>,

    source: Option<PathBuf>,

    base: Option<BString>,
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
struct Mapping<T> {
    pattern: Pattern,
    value: T,
    sequence_number: usize,
}

fn read_in_full_ignore_missing(
    path: &Path,
    follow_symlinks: bool,
    buf: &mut Vec<u8>,
) -> std::io::Result<bool> {
    buf.clear();
    let file = if follow_symlinks {
        std::fs::File::open(path)
    } else {
        fs::open_options_no_follow().read(true).open(path)
    };
    Ok(match file {
        Ok(mut file) => {
            if let Err(err) = file.read_to_end(buf) {
                if io_err_is_dir(&err) {
                    false
                } else {
                    return Err(err);
                }
            } else {
                true
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound || io_err_is_dir(&err) => false,
        Err(err) => return Err(err),
    })
}

fn io_err_is_dir(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::IsADirectory | std::io::ErrorKind::NotADirectory
    ) || { false }
}

impl<T> List<T>
where
    T: TraitPattern,
{
    fn from_bytes(bytes: &[u8], source_file: PathBuf, root: Option<&Path>, parse: T) -> Self {
        let patterns = parse.bytes_to_patterns(bytes, source_file.as_path());
        let base = root
            .and_then(|root| source_file.parent().expect("file").strip_prefix(root).ok())
            .and_then(|base| {
                (!base.as_os_str().is_empty()).then(|| {
                    let mut base: BString =
                        hx_to_unix_separators_on_windows(into_bstr(base)).into_owned();

                    base.push_byte(b'/');
                    base
                })
            });
        List {
            patterns,
            source: Some(source_file),
            base,
        }
    }

    fn from_file(
        source: impl Into<PathBuf>,
        root: Option<&Path>,
        follow_symlinks: bool,
        buf: &mut Vec<u8>,
        parse: T,
    ) -> std::io::Result<Option<Self>> {
        let source = source.into();
        Ok(read_in_full_ignore_missing(&source, follow_symlinks, buf)?
            .then(|| Self::from_bytes(buf, source, root, parse)))
    }
}

impl<T> List<T>
where
    T: TraitPattern,
{
    fn strip_base_handle_recompute_basename_pos<'a>(
        &self,
        relative_path: &'a BStr,
        basename_pos: Option<usize>,
        case: Case,
    ) -> Option<(&'a BStr, Option<usize>)> {
        match self.base.as_deref() {
            Some(base) => strip_base_handle_recompute_basename_pos(
                base.as_bstr(),
                relative_path,
                basename_pos,
                case,
            )?,
            None => (relative_path, basename_pos),
        }
        .into()
    }
}

fn strip_base_handle_recompute_basename_pos<'a>(
    base: &BStr,
    relative_path: &'a BStr,
    basename_pos: Option<usize>,
    case: Case,
) -> Option<(&'a BStr, Option<usize>)> {
    Some((
        match case {
            Case::Sensitive => relative_path.strip_prefix(base.as_bytes())?.as_bstr(),
        },
        basename_pos.and_then(|pos| {
            let pos = pos - base.len();
            (pos != 0).then_some(pos)
        }),
    ))
}

trait TraitPattern:
    Clone + PartialEq + Eq + std::fmt::Debug + std::hash::Hash + Ord + PartialOrd + Default
{
    type Value: PartialEq + Eq + std::fmt::Debug + std::hash::Hash + Ord + PartialOrd + Clone;

    fn bytes_to_patterns(&self, bytes: &[u8], source: &Path) -> Vec<Mapping<Self::Value>>;
}

fn add_patterns_file<T: TraitPattern>(
    patterns: &mut Vec<List<T>>,
    source: PathBuf,
    follow_symlinks: bool,
    root: Option<&Path>,
    buf: &mut Vec<u8>,
    parse: T,
) -> std::io::Result<bool> {
    let previous_len = patterns.len();
    patterns.extend(List::<T>::from_file(
        source,
        root,
        follow_symlinks,
        buf,
        parse,
    )?);
    Ok(patterns.len() != previous_len)
}

#[derive(Eq, PartialEq)]
enum WildmatchResult {
    Match,
    NoMatch,
    AbortAll,
    AbortToStarStar,
    RecursionLimitReached,
}

fn match_recursive(
    pattern: &BStr,
    text: &BStr,
    mode: WildmatchMode,
    depth: usize,
) -> WildmatchResult {
    if depth == RECURSION_LIMIT {
        return RecursionLimitReached;
    }
    use crate::git::WildmatchResult::*;
    use crate::git::{
        BACKSLASH, BRACKET_CLOSE, BRACKET_OPEN, COLON, NEGATE_CLASS, RECURSION_LIMIT, SLASH, STAR,
    };
    let possibly_lowercase = |c: &u8| {
        if mode.contains(WildmatchMode::IGNORE_CASE) {
            c.to_ascii_lowercase()
        } else {
            *c
        }
    };
    let mut p = pattern
        .iter()
        .map(possibly_lowercase)
        .enumerate()
        .peekable();
    let mut t = text.iter().map(possibly_lowercase).enumerate();

    while let Some((mut p_idx, mut p_ch)) = p.next() {
        let (mut t_idx, mut t_ch) = match t.next() {
            Some(c) => c,
            None if p_ch != STAR => return AbortAll,
            None => (text.len(), 0),
        };

        if p_ch == BACKSLASH {
            match p.next() {
                Some((_p_idx, p_ch)) => {
                    if p_ch != t_ch {
                        return NoMatch;
                    } else {
                        continue;
                    }
                }
                None => return NoMatch,
            };
        }
        match p_ch {
            b'?' => {
                if mode.contains(WildmatchMode::NO_MATCH_SLASH_LITERAL) && t_ch == SLASH {
                    return NoMatch;
                } else {
                    continue;
                }
            }
            STAR => {
                let mut match_slash = !mode.contains(WildmatchMode::NO_MATCH_SLASH_LITERAL);
                match p.next() {
                    Some((next_p_idx, next_p_ch)) => {
                        let next;
                        if next_p_ch == STAR {
                            let leading_slash_idx = p_idx.checked_sub(1);
                            while p.next_if(|(_, c)| *c == STAR).is_some() {}
                            next = p.next();
                            if !mode.contains(WildmatchMode::NO_MATCH_SLASH_LITERAL) {
                                match_slash = true;
                            } else if leading_slash_idx.is_none_or(|idx| pattern[idx] == SLASH)
                                && next.is_none_or(|(_, c)| {
                                    c == SLASH
                                        || (c == BACKSLASH && p.peek().map(|t| t.1) == Some(SLASH))
                                })
                            {
                                if next.map_or(NoMatch, |(idx, _)| {
                                    match_recursive(
                                        pattern[idx + 1..].as_bstr(),
                                        text[t_idx..].as_bstr(),
                                        mode,
                                        depth + 1,
                                    )
                                }) == Match
                                {
                                    return Match;
                                }
                                match_slash = true;
                            } else {
                                match_slash = false;
                            }
                        } else {
                            next = Some((next_p_idx, next_p_ch));
                        }

                        match next {
                            None => {
                                return if !match_slash && text[t_idx..].contains(&SLASH) {
                                    AbortToStarStar
                                } else {
                                    Match
                                };
                            }
                            Some((next_p_idx, next_p_ch)) => {
                                p_idx = next_p_idx;
                                p_ch = next_p_ch;
                                if !match_slash && p_ch == SLASH {
                                    match text[t_idx..].find_byte(SLASH) {
                                        Some(distance_to_slash) => {
                                            for _ in t.by_ref().take(distance_to_slash) {}
                                            continue;
                                        }
                                        None => {
                                            return AbortAll;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        return if !match_slash && text[t_idx..].contains(&SLASH) {
                            AbortToStarStar
                        } else {
                            Match
                        };
                    }
                }

                return loop {
                    if !GLOB_CHARACTERS.contains(&p_ch) {
                        loop {
                            if (!match_slash && t_ch == SLASH) || t_ch == p_ch {
                                break;
                            }
                            match t.next() {
                                Some(t) => {
                                    t_idx = t.0;
                                    t_ch = t.1;
                                }
                                None => break,
                            }
                        }
                        if t_ch != p_ch {
                            return if match_slash {
                                AbortAll
                            } else {
                                AbortToStarStar
                            };
                        }
                    }
                    let res = match_recursive(
                        pattern[p_idx..].as_bstr(),
                        text[t_idx..].as_bstr(),
                        mode,
                        depth + 1,
                    );
                    if res != NoMatch {
                        if !match_slash || res != AbortToStarStar {
                            return res;
                        }
                    } else if !match_slash && t_ch == SLASH {
                        return AbortToStarStar;
                    }
                    match t.next() {
                        Some(t) => {
                            t_idx = t.0;
                            t_ch = t.1;
                        }
                        None => break AbortAll,
                    }
                };
            }
            BRACKET_OPEN => {
                match p.next() {
                    Some(t) => {
                        p_idx = t.0;
                        p_ch = t.1;
                    }
                    None => return AbortAll,
                }

                if p_ch == b'^' {
                    p_ch = NEGATE_CLASS;
                }
                let negated = p_ch == NEGATE_CLASS;
                let mut next = if negated {
                    p.next()
                } else {
                    Some((p_idx, p_ch))
                };
                let mut prev_p_ch = 0;
                let mut matched = false;
                let mut p_idx_ofs = 0;
                loop {
                    let Some((mut p_idx, mut p_ch)) = next else {
                        return AbortAll;
                    };
                    p_idx += p_idx_ofs;
                    match p_ch {
                        BACKSLASH => match p.next() {
                            Some((_, p_ch)) => {
                                if p_ch == t_ch {
                                    matched = true;
                                } else {
                                    prev_p_ch = p_ch;
                                }
                            }
                            None => return AbortAll,
                        },
                        b'-' if prev_p_ch != 0
                            && p.peek().is_some()
                            && p.peek().map(|t| t.1) != Some(BRACKET_CLOSE) =>
                        {
                            p_ch = p.next().expect("peeked").1;
                            if p_ch == BACKSLASH {
                                p_ch = match p.next() {
                                    Some(t) => t.1,
                                    None => return AbortAll,
                                };
                            }
                            if t_ch <= p_ch && t_ch >= prev_p_ch {
                                matched = true;
                            } else if mode.contains(WildmatchMode::IGNORE_CASE)
                                && t_ch.is_ascii_lowercase()
                            {
                                let t_ch_upper = t_ch.to_ascii_uppercase();
                                if (t_ch_upper <= p_ch.to_ascii_uppercase()
                                    && t_ch_upper >= prev_p_ch.to_ascii_uppercase())
                                    || (t_ch_upper <= prev_p_ch.to_ascii_uppercase()
                                        && t_ch_upper >= p_ch.to_ascii_uppercase())
                                {
                                    matched = true;
                                }
                            }
                            prev_p_ch = 0;
                        }
                        BRACKET_OPEN if matches!(p.peek(), Some((_, COLON))) => {
                            p.next();
                            while p.peek().is_some_and(|t| t.1 != BRACKET_CLOSE) {
                                p.next();
                            }
                            let closing_bracket_idx = match p.next() {
                                Some((idx, _)) => idx,
                                None => return AbortAll,
                            };
                            const BRACKET__COLON__BRACKET: usize = 3;
                            if closing_bracket_idx.saturating_sub(p_idx) < BRACKET__COLON__BRACKET
                                || pattern[closing_bracket_idx - 1] != COLON
                            {
                                if t_ch == BRACKET_OPEN {
                                    matched = true;
                                }
                                if p_idx > pattern.len() {
                                    return AbortAll;
                                }
                                p = pattern[p_idx..]
                                    .iter()
                                    .map(possibly_lowercase)
                                    .enumerate()
                                    .peekable();
                                p_idx_ofs += p_idx;
                            } else {
                                let class = &pattern.as_bytes()[p_idx + 2..closing_bracket_idx - 1];
                                match class {
                                    b"alnum" => {
                                        if t_ch.is_ascii_alphanumeric() {
                                            matched = true;
                                        }
                                    }
                                    b"alpha" => {
                                        if t_ch.is_ascii_alphabetic() {
                                            matched = true;
                                        }
                                    }
                                    b"blank" => {
                                        if t_ch.is_ascii_whitespace() {
                                            matched = true;
                                        }
                                    }
                                    b"cntrl" => {
                                        if t_ch.is_ascii_control() {
                                            matched = true;
                                        }
                                    }
                                    b"digit" => {
                                        if t_ch.is_ascii_digit() {
                                            matched = true;
                                        }
                                    }

                                    b"graph" => {
                                        if t_ch.is_ascii_graphic() {
                                            matched = true;
                                        }
                                    }
                                    b"lower" => {
                                        if t_ch.is_ascii_lowercase() {
                                            matched = true;
                                        }
                                    }
                                    b"print" => {
                                        if (0x20u8..=0x7e).contains(&t_ch) {
                                            matched = true;
                                        }
                                    }
                                    b"punct" => {
                                        if t_ch.is_ascii_punctuation() {
                                            matched = true;
                                        }
                                    }
                                    b"space" => {
                                        if t_ch == b' ' {
                                            matched = true;
                                        }
                                    }
                                    b"upper" => {
                                        if t_ch.is_ascii_uppercase()
                                            || mode.contains(WildmatchMode::IGNORE_CASE)
                                                && t_ch.is_ascii_lowercase()
                                        {
                                            matched = true;
                                        }
                                    }
                                    b"xdigit" => {
                                        if t_ch.is_ascii_hexdigit() {
                                            matched = true;
                                        }
                                    }
                                    _ => return AbortAll,
                                }
                                prev_p_ch = 0;
                            }
                        }
                        _ => {
                            prev_p_ch = p_ch;
                            if p_ch == t_ch {
                                matched = true;
                            }
                        }
                    }
                    next = p.next();
                    if let Some((_, BRACKET_CLOSE)) = next {
                        break;
                    }
                }
                if matched == negated
                    || mode.contains(WildmatchMode::NO_MATCH_SLASH_LITERAL) && t_ch == SLASH
                {
                    return NoMatch;
                }
                continue;
            }
            non_glob_ch => {
                if non_glob_ch != t_ch {
                    return NoMatch;
                } else {
                    continue;
                }
            }
        }
    }
    t.next().map_or(Match, |_| NoMatch)
}

fn wildmatch(pattern: &BStr, value: &BStr, mode: WildmatchMode) -> bool {
    let res = match_recursive(pattern, value, mode, 0);

    res == WildmatchResult::Match
}

#[inline]
fn parse_pattern(mut pat: &[u8], may_alter: bool) -> Option<(&[u8], PatternMode, Option<usize>)> {
    let mut mode = PatternMode::empty();
    if pat.is_empty() {
        return None;
    }
    if may_alter {
        if pat.first() == Some(&b'!') {
            mode |= PatternMode::NEGATIVE;
            pat = &pat[1..];
        } else if pat.first() == Some(&b'\\') {
            let second = pat.get(1);
            if second == Some(&b'!') || second == Some(&b'#') {
                pat = &pat[1..];
            }
        }
    }
    if pat.iter().all(u8::is_ascii_whitespace) {
        return None;
    }
    if pat.first() == Some(&b'/') {
        mode |= PatternMode::ABSOLUTE;
        pat = &pat[1..];
    }
    if pat.last() == Some(&b'/') {
        mode |= PatternMode::MUST_BE_DIR;
        pat = &pat[..pat.len() - 1];
    }

    if !pat.contains(&b'/') {
        mode |= PatternMode::NO_SUB_DIR;
    }
    if pat.first() == Some(&b'*') && first_wildcard_pos(&pat[1..]).is_none() {
        mode |= PatternMode::ENDS_WITH;
    }

    let pos_of_first_wildcard = first_wildcard_pos(pat);
    Some((pat, mode, pos_of_first_wildcard))
}

fn first_wildcard_pos(pat: &[u8]) -> Option<usize> {
    pat.find_byteset(GLOB_CHARACTERS)
}

const GLOB_CHARACTERS: &[u8] = br"*?[\";

fn validate_reference_name(path: &BStr) -> Result<&BStr, GitError> {
    match validate(path, ValidationMode::Complete)? {
        None => Ok(path),
        Some(_) => {
            unreachable!(
                "Without sanitization, there is no chance a sanitized version is returned."
            )
        }
    }
}

fn validate_reference_name_partial(path: &BStr) -> Result<&BStr, GitError> {
    match validate(path, ValidationMode::Partial)? {
        None => Ok(path),
        Some(_) => {
            unreachable!(
                "Without sanitization, there is no chance a sanitized version is returned."
            )
        }
    }
}

enum ValidationMode {
    Complete,
    Partial,
}

fn validate(path: &BStr, mode: ValidationMode) -> Result<Option<BString>, GitError> {
    let out = validate_tag_name_inner(path)?;
    if let ValidationMode::Complete = mode {
        let input = out.as_ref().map_or(path, |b| b.as_bstr());
        let saw_slash = input.find_byte(b'/').is_some();
        if !saw_slash && !input.iter().all(|c| c.is_ascii_uppercase() || *c == b'_') {
            return Err(GitError::Gen);
        }
    }
    Ok(out)
}

fn validate_tag_name_inner(input: &BStr) -> Result<Option<BString>, GitError> {
    if input.is_empty() {
        return Err(GitError::Gen);
    }

    if input.last() == Some(&b'/') {
        return Err(GitError::Gen);
    }

    if input.first() == Some(&b'/') {
        return Err(GitError::Gen);
    }

    let mut previous = 0;
    let mut component_start = 0;

    for (byte_pos, byte) in input.iter().enumerate() {
        match byte {
            b'\\' | b'^' | b':' | b'[' | b'?' | b' ' | b'~' | b'\0'..=b'\x1f' | b'\x7f' | b'*' => {
                return Err(GitError::Gen);
            }
            b'.' if previous == b'.' => {
                return Err(GitError::Gen);
            }
            b'.' if previous == b'/' => {
                return Err(GitError::Gen);
            }
            b'{' if previous == b'@' => {
                return Err(GitError::Gen);
            }
            b'/' if previous == b'/' => {
                return Err(GitError::Gen);
            }
            b'/' => {
                if input[component_start..byte_pos].ends_with_str(".lock") {
                    return Err(GitError::Gen);
                }

                component_start = byte_pos + 1;
            }
            _ => {}
        }

        previous = *byte;
    }

    if input[component_start..].ends_with_str(".lock") {
        return Err(GitError::Gen);
    }

    Ok(None)
}

bitflags! {

    #[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Ord, PartialOrd)]
    struct PatternMode: u32 {
        const NO_SUB_DIR = 1 << 0;
        const ENDS_WITH = 1 << 1;
        const MUST_BE_DIR = 1 << 2;
        const NEGATIVE = 1 << 3;
        const ABSOLUTE = 1 << 4;
    }
}

bitflags! {

    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
    struct WildmatchMode: u8 {
        const NO_MATCH_SLASH_LITERAL = 1 << 0;
        const IGNORE_CASE = 1 << 1;
    }
}
