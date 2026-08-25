const NUM_RETRIES: u32 = 65536;
const NUM_RAND_CHARS: usize = 6;

use std::ffi::{OsStr, OsString};
use std::fs::{File, Permissions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::iter::repeat_with;
use std::ops::Deref;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};
use std::path::{Path, PathBuf};
use std::{fmt, fs, io, mem};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Builder<'a, 'b> {
  random_len: usize,
  prefix: &'a OsStr,
  suffix: &'b OsStr,
  append: bool,
  permissions: Option<Permissions>,
  disable_cleanup: bool,
}

impl Default for Builder<'_, '_> {
  fn default() -> Self {
    Builder {
      random_len: NUM_RAND_CHARS,
      prefix: OsStr::new(".tmp"),
      suffix: OsStr::new(""),
      append: false,
      permissions: None,
      disable_cleanup: false,
    }
  }
}

impl<'a, 'b> Builder<'a, 'b> {
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  pub fn prefix<S: AsRef<OsStr> + ?Sized>(&mut self, prefix: &'a S) -> &mut Self {
    self.prefix = prefix.as_ref();
    self
  }

  pub fn suffix<S: AsRef<OsStr> + ?Sized>(&mut self, suffix: &'b S) -> &mut Self {
    self.suffix = suffix.as_ref();
    self
  }

  pub fn make_in<F, R, P>(&self, dir: P, mut f: F) -> io::Result<NamedTempFile<R>>
  where
    F: FnMut(&Path) -> io::Result<R>,
    P: AsRef<Path>,
  {
    create_helper(
      dir.as_ref(),
      self.prefix,
      self.suffix,
      self.random_len,
      move |path| {
        Ok(NamedTempFile::from_parts(
          f(&path)?,
          TempPath::new(path, self.disable_cleanup),
        ))
      },
    )
  }
}

pub fn keep(_: &Path) -> io::Result<()> {
  Ok(())
}

#[derive(Debug)]
pub struct PathPersistError {
  error: io::Error,
  path: TempPath,
}

impl From<PathPersistError> for io::Error {
  #[inline]
  fn from(error: PathPersistError) -> io::Error {
    error.error
  }
}

impl From<PathPersistError> for TempPath {
  #[inline]
  fn from(error: PathPersistError) -> TempPath {
    error.path
  }
}

impl fmt::Display for PathPersistError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "failed to persist temporary file path: {}", self.error)
  }
}

impl std::error::Error for PathPersistError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    Some(&self.error)
  }
}

pub struct TempPath {
  path: Box<Path>,
  disable_cleanup: bool,
}

impl TempPath {
  pub fn keep(mut self) -> Result<PathBuf, PathPersistError> {
    match keep(&self.path) {
      Ok(_) => {
        self.disable_cleanup(true);
        Ok(mem::replace(&mut self.path, PathBuf::new().into_boxed_path()).into_path_buf())
      }
      Err(e) => Err(PathPersistError {
        error: e,
        path: self,
      }),
    }
  }

  fn disable_cleanup(&mut self, disable_cleanup: bool) {
    self.disable_cleanup = disable_cleanup
  }

  pub fn try_from_path(path: impl Into<PathBuf>) -> io::Result<Self> {
    let mut path = path.into();
    if !path.is_absolute() {
      if path == Path::new("") {
        return Err(io::Error::new(
          io::ErrorKind::InvalidInput,
          "cannot construct a TempPath from an empty path",
        ));
      }
      let mut cwd = std::env::current_dir()?;
      cwd.push(path);
      path = cwd;
    };

    Ok(Self {
      path: path.into_boxed_path(),
      disable_cleanup: false,
    })
  }

  fn new(path: PathBuf, disable_cleanup: bool) -> Self {
    Self {
      path: path.into_boxed_path(),
      disable_cleanup,
    }
  }
}

impl fmt::Debug for TempPath {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.path.fmt(f)
  }
}

impl Drop for TempPath {
  fn drop(&mut self) {
    if !self.disable_cleanup {
      let _ = fs::remove_file(&self.path);
    }
  }
}

impl Deref for TempPath {
  type Target = Path;

  fn deref(&self) -> &Path {
    &self.path
  }
}

impl AsRef<Path> for TempPath {
  fn as_ref(&self) -> &Path {
    &self.path
  }
}

impl AsRef<OsStr> for TempPath {
  fn as_ref(&self) -> &OsStr {
    self.path.as_os_str()
  }
}

pub struct NamedTempFile<F = File> {
  path: TempPath,
  file: F,
}

impl<F> fmt::Debug for NamedTempFile<F> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "NamedTempFile({:?})", self.path)
  }
}

impl<F> AsRef<Path> for NamedTempFile<F> {
  #[inline]
  fn as_ref(&self) -> &Path {
    self.path()
  }
}

impl<F> NamedTempFile<F> {
  #[inline]
  fn path(&self) -> &Path {
    &self.path
  }

  fn as_file(&self) -> &F {
    &self.file
  }

  fn as_file_mut(&mut self) -> &mut F {
    &mut self.file
  }

  pub fn into_temp_path(self) -> TempPath {
    self.path
  }

  fn from_parts(file: F, path: TempPath) -> Self {
    Self { file, path }
  }
}

impl<F: Read> Read for NamedTempFile<F> {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    self.as_file_mut().read(buf).with_err_path(|| self.path())
  }

  fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
    self
      .as_file_mut()
      .read_vectored(bufs)
      .with_err_path(|| self.path())
  }

  fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
    self
      .as_file_mut()
      .read_to_end(buf)
      .with_err_path(|| self.path())
  }

  fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
    self
      .as_file_mut()
      .read_to_string(buf)
      .with_err_path(|| self.path())
  }

  fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
    self
      .as_file_mut()
      .read_exact(buf)
      .with_err_path(|| self.path())
  }
}

impl Read for &NamedTempFile<File> {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    self.as_file().read(buf).with_err_path(|| self.path())
  }

  fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
    self
      .as_file()
      .read_vectored(bufs)
      .with_err_path(|| self.path())
  }

  fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
    self
      .as_file()
      .read_to_end(buf)
      .with_err_path(|| self.path())
  }

  fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
    self
      .as_file()
      .read_to_string(buf)
      .with_err_path(|| self.path())
  }

  fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
    self.as_file().read_exact(buf).with_err_path(|| self.path())
  }
}

impl<F: Write> Write for NamedTempFile<F> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.as_file_mut().write(buf).with_err_path(|| self.path())
  }
  #[inline]
  fn flush(&mut self) -> io::Result<()> {
    self.as_file_mut().flush().with_err_path(|| self.path())
  }

  fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
    self
      .as_file_mut()
      .write_vectored(bufs)
      .with_err_path(|| self.path())
  }

  fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
    self
      .as_file_mut()
      .write_all(buf)
      .with_err_path(|| self.path())
  }

  fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
    self
      .as_file_mut()
      .write_fmt(fmt)
      .with_err_path(|| self.path())
  }
}

impl Write for &NamedTempFile<File> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.as_file().write(buf).with_err_path(|| self.path())
  }
  #[inline]
  fn flush(&mut self) -> io::Result<()> {
    self.as_file().flush().with_err_path(|| self.path())
  }

  fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
    self
      .as_file()
      .write_vectored(bufs)
      .with_err_path(|| self.path())
  }

  fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
    self.as_file().write_all(buf).with_err_path(|| self.path())
  }

  fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
    self.as_file().write_fmt(fmt).with_err_path(|| self.path())
  }
}

impl<F: Seek> Seek for NamedTempFile<F> {
  fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
    self.as_file_mut().seek(pos).with_err_path(|| self.path())
  }
}

impl Seek for &NamedTempFile<File> {
  fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
    self.as_file().seek(pos).with_err_path(|| self.path())
  }
}

impl<F: AsFd> AsFd for NamedTempFile<F> {
  fn as_fd(&self) -> BorrowedFd<'_> {
    self.as_file().as_fd()
  }
}

impl<F: AsRawFd> AsRawFd for NamedTempFile<F> {
  #[inline]
  fn as_raw_fd(&self) -> RawFd {
    self.as_file().as_raw_fd()
  }
}

fn tmpname(rng: &mut fastrand::Rng, prefix: &OsStr, suffix: &OsStr, rand_len: usize) -> OsString {
  let capacity = prefix
    .len()
    .saturating_add(suffix.len())
    .saturating_add(rand_len);
  let mut buf = OsString::with_capacity(capacity);
  buf.push(prefix);
  let mut char_buf = [0u8; 4];
  for c in repeat_with(|| rng.alphanumeric()).take(rand_len) {
    buf.push(c.encode_utf8(&mut char_buf));
  }
  buf.push(suffix);
  buf
}

fn create_helper<R>(
  base: &Path,
  prefix: &OsStr,
  suffix: &OsStr,
  random_len: usize,
  mut f: impl FnMut(PathBuf) -> io::Result<R>,
) -> io::Result<R> {
  let mut base = base; // re-borrow to shrink lifetime
  let base_path_storage; // slot to store the absolute path, if necessary.
  if !base.is_absolute() {
    let cur_dir = std::env::current_dir()?;
    base_path_storage = cur_dir.join(base);
    base = &base_path_storage;
  }

  let num_retries = if random_len != 0 { NUM_RETRIES } else { 1 };

  let mut rng = fastrand::Rng::new();
  for i in 0..num_retries {
    if i == 3 {
      if let Ok(seed) = getrandom::u64() {
        rng.seed(seed);
      }
    }
    let _ = i; // avoid unused variable warning for the above.

    let path = base.join(tmpname(&mut rng, prefix, suffix, random_len));
    return match f(path) {
      Err(ref e) if e.kind() == io::ErrorKind::AlreadyExists && num_retries > 1 => continue,
      Err(ref e) if e.kind() == io::ErrorKind::AddrInUse && num_retries > 1 => continue,
      res => res,
    };
  }

  Err(io::Error::new(
    io::ErrorKind::AlreadyExists,
    "too many temporary files exist",
  ))
  .with_err_path(|| base)
}

#[derive(Debug)]
struct PathError {
  path: PathBuf,
  err: io::Error,
}

impl fmt::Display for PathError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{} at path {:?}", self.err, self.path)
  }
}

impl std::error::Error for PathError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    self.err.source()
  }
}

trait IoResultExt<T> {
  fn with_err_path<F, P>(self, path: F) -> Self
  where
    F: FnOnce() -> P,
    P: Into<PathBuf>;
}

impl<T> IoResultExt<T> for Result<T, io::Error> {
  fn with_err_path<F, P>(self, path: F) -> Self
  where
    F: FnOnce() -> P,
    P: Into<PathBuf>,
  {
    self.map_err(|e| {
      io::Error::new(
        e.kind(),
        PathError {
          path: path().into(),
          err: e,
        },
      )
    })
  }
}
