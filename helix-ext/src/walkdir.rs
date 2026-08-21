use std::{
  cmp::{Ordering, min},
  fmt,
  fs::{self, ReadDir},
  io, iter,
  path::{Path, PathBuf},
  result, vec,
};
mod same_file {
  use std::fs::{File, OpenOptions};
  use std::hash::{Hash, Hasher};
  use std::io;
  use std::os::unix::fs::MetadataExt;
  use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};
  use std::path::Path;

  #[derive(Debug)]
  pub struct Handle {
    file: Option<File>,
    is_std: bool,
    dev: u64,
    ino: u64,
  }

  impl Handle {
    pub fn from_path<P: AsRef<Path>>(p: P) -> io::Result<Handle> {
      Handle::from_file(OpenOptions::new().read(true).open(p)?)
    }

    pub fn from_file(file: File) -> io::Result<Handle> {
      let md = file.metadata()?;
      Ok(Handle {
        file: Some(file),
        is_std: false,
        dev: md.dev(),
        ino: md.ino(),
      })
    }
  }

  impl Drop for Handle {
    fn drop(&mut self) {
      if self.is_std {
        // Leak the fd so the std stream stays open.
        let _ = self.file.take().unwrap().into_raw_fd();
      }
    }
  }

  impl Eq for Handle {}

  impl PartialEq for Handle {
    fn eq(&self, other: &Handle) -> bool {
      (self.dev, self.ino) == (other.dev, other.ino)
    }
  }

  impl Hash for Handle {
    fn hash<H: Hasher>(&self, state: &mut H) {
      self.dev.hash(state);
      self.ino.hash(state);
    }
  }

  impl AsRawFd for Handle {
    fn as_raw_fd(&self) -> RawFd {
      self.file.as_ref().unwrap().as_raw_fd()
    }
  }

  impl IntoRawFd for Handle {
    fn into_raw_fd(mut self) -> RawFd {
      self.file.take().unwrap().into_raw_fd()
    }
  }
}
use same_file::Handle;

pub use crate::walkdir::dent::DirEntry;
pub use crate::walkdir::dent::DirEntryExt;
pub use crate::walkdir::error::Error;

pub mod dent {
  use std::ffi::OsStr;
  use std::fmt;
  use std::fs::{self, FileType};
  use std::path::{Path, PathBuf};

  use crate::walkdir::Result;
  use crate::walkdir::error::Error;

  pub struct DirEntry {
    path: PathBuf,
    ty: FileType,
    follow_link: bool,
    depth: usize,

    ino: u64,
  }

  impl DirEntry {
    pub fn path(&self) -> &Path {
      &self.path
    }

    pub fn into_path(self) -> PathBuf {
      self.path
    }

    pub fn path_is_symlink(&self) -> bool {
      self.ty.is_symlink() || self.follow_link
    }

    pub fn metadata(&self) -> Result<fs::Metadata> {
      self.metadata_internal()
    }

    fn metadata_internal(&self) -> Result<fs::Metadata> {
      if self.follow_link {
        fs::metadata(&self.path)
      } else {
        fs::symlink_metadata(&self.path)
      }
      .map_err(|err| Error::from_entry(self, err))
    }

    pub fn file_type(&self) -> fs::FileType {
      self.ty
    }

    pub fn file_name(&self) -> &OsStr {
      self
        .path
        .file_name()
        .unwrap_or_else(|| self.path.as_os_str())
    }

    pub fn depth(&self) -> usize {
      self.depth
    }

    pub(crate) fn is_dir(&self) -> bool {
      self.ty.is_dir()
    }

    pub(crate) fn from_entry(depth: usize, ent: &fs::DirEntry) -> Result<DirEntry> {
      use std::os::unix::fs::DirEntryExt;

      let ty = ent
        .file_type()
        .map_err(|err| Error::from_path(depth, ent.path(), err))?;
      Ok(DirEntry {
        path: ent.path(),
        ty,
        follow_link: false,
        depth,
        ino: ent.ino(),
      })
    }

    pub(crate) fn from_path(depth: usize, pb: PathBuf, follow: bool) -> Result<DirEntry> {
      use std::os::unix::fs::MetadataExt;

      let md = if follow {
        fs::metadata(&pb).map_err(|err| Error::from_path(depth, pb.clone(), err))?
      } else {
        fs::symlink_metadata(&pb).map_err(|err| Error::from_path(depth, pb.clone(), err))?
      };
      Ok(DirEntry {
        path: pb,
        ty: md.file_type(),
        follow_link: follow,
        depth,
        ino: md.ino(),
      })
    }
  }

  impl Clone for DirEntry {
    fn clone(&self) -> DirEntry {
      DirEntry {
        path: self.path.clone(),
        ty: self.ty,
        follow_link: self.follow_link,
        depth: self.depth,
        ino: self.ino,
      }
    }
  }

  impl fmt::Debug for DirEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "DirEntry({:?})", self.path)
    }
  }

  pub trait DirEntryExt {
    fn ino(&self) -> u64;
  }

  impl DirEntryExt for DirEntry {
    fn ino(&self) -> u64 {
      self.ino
    }
  }
}
mod error {
  use std::error;
  use std::fmt;
  use std::io;
  use std::path::{Path, PathBuf};

  #[derive(Debug)]
  pub struct Error {
    depth: usize,
    inner: ErrorInner,
  }

  #[derive(Debug)]
  enum ErrorInner {
    Io {
      path: Option<PathBuf>,
      err: io::Error,
    },
    Loop {
      ancestor: PathBuf,
      child: PathBuf,
    },
  }

  impl Error {
    pub fn path(&self) -> Option<&Path> {
      match self.inner {
        ErrorInner::Io { path: None, .. } => None,
        ErrorInner::Io {
          path: Some(ref path),
          ..
        } => Some(path),
        ErrorInner::Loop { ref child, .. } => Some(child),
      }
    }

    pub fn loop_ancestor(&self) -> Option<&Path> {
      match self.inner {
        ErrorInner::Loop { ref ancestor, .. } => Some(ancestor),
        _ => None,
      }
    }

    pub fn depth(&self) -> usize {
      self.depth
    }

    pub fn io_error(&self) -> Option<&io::Error> {
      match self.inner {
        ErrorInner::Io { ref err, .. } => Some(err),
        ErrorInner::Loop { .. } => None,
      }
    }

    pub fn into_io_error(self) -> Option<io::Error> {
      match self.inner {
        ErrorInner::Io { err, .. } => Some(err),
        ErrorInner::Loop { .. } => None,
      }
    }

    pub(crate) fn from_path(depth: usize, pb: PathBuf, err: io::Error) -> Self {
      Error {
        depth,
        inner: ErrorInner::Io {
          path: Some(pb),
          err,
        },
      }
    }

    pub(crate) fn from_entry(dent: &crate::walkdir::DirEntry, err: io::Error) -> Self {
      Error {
        depth: dent.depth(),
        inner: ErrorInner::Io {
          path: Some(dent.path().to_path_buf()),
          err,
        },
      }
    }

    pub(crate) fn from_io(depth: usize, err: io::Error) -> Self {
      Error {
        depth,
        inner: ErrorInner::Io { path: None, err },
      }
    }

    pub(crate) fn from_loop(depth: usize, ancestor: &Path, child: &Path) -> Self {
      Error {
        depth,
        inner: ErrorInner::Loop {
          ancestor: ancestor.to_path_buf(),
          child: child.to_path_buf(),
        },
      }
    }
  }

  impl error::Error for Error {
    #[allow(deprecated)]
    fn description(&self) -> &str {
      match self.inner {
        ErrorInner::Io { ref err, .. } => err.description(),
        ErrorInner::Loop { .. } => "file system loop found",
      }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
      self.source()
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
      match self.inner {
        ErrorInner::Io { ref err, .. } => Some(err),
        ErrorInner::Loop { .. } => None,
      }
    }
  }

  impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      match self.inner {
        ErrorInner::Io {
          path: None,
          ref err,
        } => err.fmt(f),
        ErrorInner::Io {
          path: Some(ref path),
          ref err,
        } => write!(f, "IO error for operation on {}: {}", path.display(), err),
        ErrorInner::Loop {
          ref ancestor,
          ref child,
        } => write!(
          f,
          "File system loop found: \
                 {} points to an ancestor {}",
          child.display(),
          ancestor.display()
        ),
      }
    }
  }

  impl From<Error> for io::Error {
    fn from(walk_err: Error) -> io::Error {
      let kind = match walk_err {
        Error {
          inner: ErrorInner::Io { ref err, .. },
          ..
        } => err.kind(),
        Error {
          inner: ErrorInner::Loop { .. },
          ..
        } => io::ErrorKind::Other,
      };
      io::Error::new(kind, walk_err)
    }
  }
}
mod util {
  use std::io;
  use std::path::Path;

  pub fn device_num<P: AsRef<Path>>(path: P) -> io::Result<u64> {
    use std::os::unix::fs::MetadataExt;

    path.as_ref().metadata().map(|md| md.dev())
  }
}

macro_rules! itry {
  ($e:expr) => {
    match $e {
      Ok(v) => v,
      Err(err) => return Some(Err(From::from(err))),
    }
  };
}

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub struct WalkDir {
  opts: WalkDirOptions,
  root: PathBuf,
}

struct WalkDirOptions {
  follow_links: bool,
  follow_root_links: bool,
  max_open: usize,
  min_depth: usize,
  max_depth: usize,
  sorter: Option<Box<dyn FnMut(&DirEntry, &DirEntry) -> Ordering + Send + Sync + 'static>>,
  contents_first: bool,
  same_file_system: bool,
}

impl fmt::Debug for WalkDirOptions {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> result::Result<(), fmt::Error> {
    let sorter_str = if self.sorter.is_some() {
      "Some(...)"
    } else {
      "None"
    };
    f.debug_struct("WalkDirOptions")
      .field("follow_links", &self.follow_links)
      .field("follow_root_link", &self.follow_root_links)
      .field("max_open", &self.max_open)
      .field("min_depth", &self.min_depth)
      .field("max_depth", &self.max_depth)
      .field("sorter", &sorter_str)
      .field("contents_first", &self.contents_first)
      .field("same_file_system", &self.same_file_system)
      .finish()
  }
}

impl WalkDir {
  pub fn new<P: AsRef<Path>>(root: P) -> Self {
    WalkDir {
      opts: WalkDirOptions {
        follow_links: false,
        follow_root_links: true,
        max_open: 10,
        min_depth: 0,
        max_depth: ::std::usize::MAX,
        sorter: None,
        contents_first: false,
        same_file_system: false,
      },
      root: root.as_ref().to_path_buf(),
    }
  }

  pub fn min_depth(mut self, depth: usize) -> Self {
    self.opts.min_depth = depth;
    if self.opts.min_depth > self.opts.max_depth {
      self.opts.min_depth = self.opts.max_depth;
    }
    self
  }

  pub fn max_depth(mut self, depth: usize) -> Self {
    self.opts.max_depth = depth;
    if self.opts.max_depth < self.opts.min_depth {
      self.opts.max_depth = self.opts.min_depth;
    }
    self
  }

  pub fn follow_links(mut self, yes: bool) -> Self {
    self.opts.follow_links = yes;
    self
  }
  pub fn follow_root_links(mut self, yes: bool) -> Self {
    self.opts.follow_root_links = yes;
    self
  }

  pub fn max_open(mut self, mut n: usize) -> Self {
    if n == 0 {
      n = 1;
    }
    self.opts.max_open = n;
    self
  }

  pub fn sort_by<F>(mut self, cmp: F) -> Self
  where
    F: FnMut(&DirEntry, &DirEntry) -> Ordering + Send + Sync + 'static,
  {
    self.opts.sorter = Some(Box::new(cmp));
    self
  }

  pub fn sort_by_key<K, F>(self, mut cmp: F) -> Self
  where
    F: FnMut(&DirEntry) -> K + Send + Sync + 'static,
    K: Ord,
  {
    self.sort_by(move |a, b| cmp(a).cmp(&cmp(b)))
  }

  pub fn sort_by_file_name(self) -> Self {
    self.sort_by(|a, b| a.file_name().cmp(b.file_name()))
  }

  pub fn contents_first(mut self, yes: bool) -> Self {
    self.opts.contents_first = yes;
    self
  }

  pub fn same_file_system(mut self, yes: bool) -> Self {
    self.opts.same_file_system = yes;
    self
  }
}

impl IntoIterator for WalkDir {
  type Item = Result<DirEntry>;
  type IntoIter = IntoIter;

  fn into_iter(self) -> IntoIter {
    IntoIter {
      opts: self.opts,
      start: Some(self.root),
      stack_list: vec![],
      stack_path: vec![],
      oldest_opened: 0,
      depth: 0,
      deferred_dirs: vec![],
      root_device: None,
    }
  }
}

#[derive(Debug)]
pub struct IntoIter {
  opts: WalkDirOptions,
  start: Option<PathBuf>,
  stack_list: Vec<DirList>,
  stack_path: Vec<Ancestor>,
  oldest_opened: usize,
  depth: usize,
  deferred_dirs: Vec<DirEntry>,
  root_device: Option<u64>,
}

#[derive(Debug)]
struct Ancestor {
  path: PathBuf,
}

impl Ancestor {
  fn new(dent: &DirEntry) -> io::Result<Ancestor> {
    Ok(Ancestor {
      path: dent.path().to_path_buf(),
    })
  }

  fn is_same(&self, child: &Handle) -> io::Result<bool> {
    Ok(child == &Handle::from_path(&self.path)?)
  }
}

#[derive(Debug)]
enum DirList {
  Opened {
    depth: usize,
    it: result::Result<ReadDir, Option<Error>>,
  },
  Closed(vec::IntoIter<Result<DirEntry>>),
}

impl Iterator for IntoIter {
  type Item = Result<DirEntry>;
  fn next(&mut self) -> Option<Result<DirEntry>> {
    if let Some(start) = self.start.take() {
      if self.opts.same_file_system {
        let result = util::device_num(&start).map_err(|e| Error::from_path(0, start.clone(), e));
        self.root_device = Some(itry!(result));
      }
      let dent = itry!(DirEntry::from_path(0, start, false));
      if let Some(result) = self.handle_entry(dent) {
        return Some(result);
      }
    }
    while !self.stack_list.is_empty() {
      self.depth = self.stack_list.len();
      if let Some(dentry) = self.get_deferred_dir() {
        return Some(Ok(dentry));
      }
      if self.depth > self.opts.max_depth {
        self.pop();
        continue;
      }
      let next = self
        .stack_list
        .last_mut()
        .expect("BUG: stack should be non-empty")
        .next();
      match next {
        None => self.pop(),
        Some(Err(err)) => return Some(Err(err)),
        Some(Ok(dent)) => {
          if let Some(result) = self.handle_entry(dent) {
            return Some(result);
          }
        }
      }
    }
    if self.opts.contents_first {
      self.depth = self.stack_list.len();
      if let Some(dentry) = self.get_deferred_dir() {
        return Some(Ok(dentry));
      }
    }
    None
  }
}

impl IntoIter {
  pub fn skip_current_dir(&mut self) {
    if !self.stack_list.is_empty() {
      self.pop();
    }
  }

  pub fn filter_entry<P>(self, predicate: P) -> FilterEntry<Self, P>
  where
    P: FnMut(&DirEntry) -> bool,
  {
    FilterEntry {
      it: self,
      predicate,
    }
  }

  fn handle_entry(&mut self, mut dent: DirEntry) -> Option<Result<DirEntry>> {
    if self.opts.follow_links && dent.file_type().is_symlink() {
      dent = itry!(self.follow(dent));
    }
    let is_normal_dir = !dent.file_type().is_symlink() && dent.is_dir();
    if is_normal_dir {
      if self.opts.same_file_system && dent.depth() > 0 {
        if itry!(self.is_same_file_system(&dent)) {
          itry!(self.push(&dent));
        }
      } else {
        itry!(self.push(&dent));
      }
    } else if dent.depth() == 0 && dent.file_type().is_symlink() && self.opts.follow_root_links {
      let md = itry!(
        fs::metadata(dent.path())
          .map_err(|err| { Error::from_path(dent.depth(), dent.path().to_path_buf(), err) })
      );
      if md.file_type().is_dir() {
        itry!(self.push(&dent));
      }
    }
    if is_normal_dir && self.opts.contents_first {
      self.deferred_dirs.push(dent);
      None
    } else if self.skippable() {
      None
    } else {
      Some(Ok(dent))
    }
  }

  fn get_deferred_dir(&mut self) -> Option<DirEntry> {
    if self.opts.contents_first {
      if self.depth < self.deferred_dirs.len() {
        let deferred: DirEntry = self
          .deferred_dirs
          .pop()
          .expect("BUG: deferred_dirs should be non-empty");
        if !self.skippable() {
          return Some(deferred);
        }
      }
    }
    None
  }

  fn push(&mut self, dent: &DirEntry) -> Result<()> {
    let free = self
      .stack_list
      .len()
      .checked_sub(self.oldest_opened)
      .unwrap();
    if free == self.opts.max_open {
      self.stack_list[self.oldest_opened].close();
    }
    let rd = fs::read_dir(dent.path())
      .map_err(|err| Some(Error::from_path(self.depth, dent.path().to_path_buf(), err)));
    let mut list = DirList::Opened {
      depth: self.depth,
      it: rd,
    };
    if let Some(ref mut cmp) = self.opts.sorter {
      let mut entries: Vec<_> = list.collect();
      entries.sort_by(|a, b| match (a, b) {
        (&Ok(ref a), &Ok(ref b)) => cmp(a, b),
        (&Err(_), &Err(_)) => Ordering::Equal,
        (&Ok(_), &Err(_)) => Ordering::Greater,
        (&Err(_), &Ok(_)) => Ordering::Less,
      });
      list = DirList::Closed(entries.into_iter());
    }
    if self.opts.follow_links {
      let ancestor = Ancestor::new(&dent).map_err(|err| Error::from_io(self.depth, err))?;
      self.stack_path.push(ancestor);
    }
    self.stack_list.push(list);
    if free == self.opts.max_open {
      self.oldest_opened = self.oldest_opened.checked_add(1).unwrap();
    }
    Ok(())
  }

  fn pop(&mut self) {
    self
      .stack_list
      .pop()
      .expect("BUG: cannot pop from empty stack");
    if self.opts.follow_links {
      self
        .stack_path
        .pop()
        .expect("BUG: list/path stacks out of sync");
    }
    self.oldest_opened = min(self.oldest_opened, self.stack_list.len());
  }

  fn follow(&self, mut dent: DirEntry) -> Result<DirEntry> {
    dent = DirEntry::from_path(self.depth, dent.path().to_path_buf(), true)?;
    if dent.is_dir() {
      self.check_loop(dent.path())?;
    }
    Ok(dent)
  }

  fn check_loop<P: AsRef<Path>>(&self, child: P) -> Result<()> {
    let hchild = Handle::from_path(&child).map_err(|err| Error::from_io(self.depth, err))?;
    for ancestor in self.stack_path.iter().rev() {
      let is_same = ancestor
        .is_same(&hchild)
        .map_err(|err| Error::from_io(self.depth, err))?;
      if is_same {
        return Err(Error::from_loop(self.depth, &ancestor.path, child.as_ref()));
      }
    }
    Ok(())
  }

  fn is_same_file_system(&mut self, dent: &DirEntry) -> Result<bool> {
    let dent_device = util::device_num(dent.path()).map_err(|err| Error::from_entry(dent, err))?;
    Ok(
      self
        .root_device
        .map(|d| d == dent_device)
        .expect("BUG: called is_same_file_system without root device"),
    )
  }

  fn skippable(&self) -> bool {
    self.depth < self.opts.min_depth || self.depth > self.opts.max_depth
  }
}

impl iter::FusedIterator for IntoIter {}

impl DirList {
  fn close(&mut self) {
    if let DirList::Opened { .. } = *self {
      *self = DirList::Closed(self.collect::<Vec<_>>().into_iter());
    }
  }
}

impl Iterator for DirList {
  type Item = Result<DirEntry>;

  #[inline(always)]
  fn next(&mut self) -> Option<Result<DirEntry>> {
    match *self {
      DirList::Closed(ref mut it) => it.next(),
      DirList::Opened { depth, ref mut it } => match *it {
        Err(ref mut err) => err.take().map(Err),
        Ok(ref mut rd) => rd.next().map(|r| match r {
          Ok(r) => DirEntry::from_entry(depth + 1, &r),
          Err(err) => Err(Error::from_io(depth + 1, err)),
        }),
      },
    }
  }
}

#[derive(Debug)]
pub struct FilterEntry<I, P> {
  it: I,
  predicate: P,
}

impl<P> Iterator for FilterEntry<IntoIter, P>
where
  P: FnMut(&DirEntry) -> bool,
{
  type Item = Result<DirEntry>;

  fn next(&mut self) -> Option<Result<DirEntry>> {
    loop {
      let dent = match self.it.next() {
        None => return None,
        Some(result) => itry!(result),
      };
      if !(self.predicate)(&dent) {
        if dent.is_dir() {
          self.it.skip_current_dir();
        }
        continue;
      }
      return Some(Ok(dent));
    }
  }
}

impl<P> iter::FusedIterator for FilterEntry<IntoIter, P> where P: FnMut(&DirEntry) -> bool {}

impl<P> FilterEntry<IntoIter, P>
where
  P: FnMut(&DirEntry) -> bool,
{
  pub fn filter_entry(self, predicate: P) -> FilterEntry<Self, P> {
    FilterEntry {
      it: self,
      predicate,
    }
  }

  pub fn skip_current_dir(&mut self) {
    self.it.skip_current_dir();
  }
}
