use std::fmt::Write;
use std::path::is_separator;
use std::{
  borrow::Cow,
  panic::{RefUnwindSafe, UnwindSafe},
  path::Path,
  sync::Arc,
};
use {
  aho_corasick::AhoCorasick,
  bstr::{B, ByteSlice, ByteVec},
  regex_automata::{
    PatternSet,
    meta::Regex,
    util::pool::{Pool, PoolGuard},
  },
};

/// A convenience alias for creating a hash map with an FNV hasher.
type HashMap<K, V> = std::collections::HashMap<K, V, std::hash::BuildHasherDefault<Hasher>>;

/// A builder for a pattern.
///
/// This builder enables configuring the match semantics of a pattern. For
/// example, one can make matching case insensitive.
///
/// The lifetime `'a` refers to the lifetime of the pattern string.
#[derive(Clone, Debug)]
pub struct GlobBuilder<'a> {
  /// The glob pattern to compile.
  glob: &'a str,
  /// Options for the pattern.
  opts: GlobOptions,
}
impl<'a> GlobBuilder<'a> {
  /// Create a new builder for the pattern given.
  ///
  /// The pattern is not compiled until `build` is called.
  pub fn new(glob: &'a str) -> GlobBuilder<'a> {
    GlobBuilder {
      glob,
      opts: GlobOptions::default(),
    }
  }

  /// Parses and builds the pattern.
  pub fn build(&self) -> Result<Glob, Error> {
    let mut p = Parser {
      glob: &self.glob,
      alternates_stack: Vec::new(),
      branches: vec![Tokens::default()],
      chars: self.glob.chars().peekable(),
      prev: None,
      cur: None,
      found_unclosed_class: false,
      opts: &self.opts,
    };
    p.parse()?;
    if p.branches.is_empty() {
      // OK because of how the the branches/alternate_stack are managed.
      // If we end up here, then there *must* be a bug in the parser
      // somewhere.
      unreachable!()
    } else if p.branches.len() > 1 {
      Err(Error {
        glob: Some(self.glob.to_string()),
        kind: ErrorKind::UnclosedAlternates,
      })
    } else {
      let tokens = p.branches.pop().unwrap();
      Ok(Glob {
        glob: self.glob.to_string(),
        re: tokens.to_regex_with(&self.opts),
        opts: self.opts,
        tokens,
      })
    }
  }

  /// Toggle whether the pattern matches case insensitively or not.
  ///
  /// This is disabled by default.
  pub fn case_insensitive(&mut self, yes: bool) -> &mut GlobBuilder<'a> {
    self.opts.case_insensitive = yes;
    self
  }

  /// Toggle whether a literal `/` is required to match a path separator.
  ///
  /// By default this is false: `*` and `?` will match `/`.
  pub fn literal_separator(&mut self, yes: bool) -> &mut GlobBuilder<'a> {
    self.opts.literal_separator = yes;
    self
  }

  /// When enabled, a back slash (`\`) may be used to escape
  /// special characters in a glob pattern. Additionally, this will
  /// prevent `\` from being interpreted as a path separator on all
  /// platforms.
  ///
  /// This is enabled by default on platforms where `\` is not a
  /// path separator and disabled by default on platforms where `\`
  /// is a path separator.
  pub fn backslash_escape(&mut self, yes: bool) -> &mut GlobBuilder<'a> {
    self.opts.backslash_escape = yes;
    self
  }

  /// Toggle whether an empty pattern in a list of alternates is accepted.
  ///
  /// For example, if this is set then the glob `foo{,.txt}` will match both
  /// `foo` and `foo.txt`.
  ///
  /// By default this is false.
  pub fn empty_alternates(&mut self, yes: bool) -> &mut GlobBuilder<'a> {
    self.opts.empty_alternates = yes;
    self
  }

  /// Toggle whether unclosed character classes are allowed. When allowed,
  /// a `[` without a matching `]` is treated literally instead of resulting
  /// in a parse error.
  ///
  /// For example, if this is set then the glob `[abc` will be treated as the
  /// literal string `[abc` instead of returning an error.
  ///
  /// By default, this is false. Generally speaking, enabling this leads to
  /// worse failure modes since the glob parser becomes more permissive. You
  /// might want to enable this when compatibility (e.g., with POSIX glob
  /// implementations) is more important than good error messages.
  pub(crate) fn allow_unclosed_class(&mut self, yes: bool) -> &mut GlobBuilder<'a> {
    self.opts.allow_unclosed_class = yes;
    self
  }
}

/// GlobSet represents a group of globs that can be matched together in a
/// single pass.
#[derive(Clone, Debug)]
pub struct GlobSet {
  len: usize,
  strats: Vec<GlobSetMatchStrategy>,
}

impl GlobSet {
  /// Create a new [`GlobSetBuilder`]. A `GlobSetBuilder` can be used to add
  /// new patterns. Once all patterns have been added, `build` should be
  /// called to produce a `GlobSet`, which can then be used for matching.
  #[inline]
  pub fn builder() -> GlobSetBuilder {
    GlobSetBuilder::new()
  }

  /// Create an empty `GlobSet`. An empty set matches nothing.
  #[inline]
  pub const fn empty() -> GlobSet {
    GlobSet {
      len: 0,
      strats: vec![],
    }
  }

  /// Returns true if this set is empty, and therefore matches nothing.
  #[inline]
  pub(crate) fn is_empty(&self) -> bool {
    self.len == 0
  }

  /// Returns true if any glob in this set matches the path given.
  pub fn is_match<P: AsRef<Path>>(&self, path: P) -> bool {
    self.is_match_candidate(&Candidate::new(path.as_ref()))
  }

  /// Returns true if any glob in this set matches the path given.
  ///
  /// This takes a Candidate as input, which can be used to amortize the
  /// cost of preparing a path for matching.
  fn is_match_candidate(&self, path: &Candidate<'_>) -> bool {
    if self.is_empty() {
      return false;
    }
    for strat in &self.strats {
      if strat.is_match(path) {
        return true;
      }
    }
    false
  }

  /// Returns the sequence number of every glob pattern that matches the
  /// given path.
  pub fn matches<P: AsRef<Path>>(&self, path: P) -> Vec<usize> {
    self.matches_candidate(&Candidate::new(path.as_ref()))
  }

  /// Returns the sequence number of every glob pattern that matches the
  /// given path.
  ///
  /// This takes a Candidate as input, which can be used to amortize the
  /// cost of preparing a path for matching.
  fn matches_candidate(&self, path: &Candidate<'_>) -> Vec<usize> {
    let mut into = vec![];
    if self.is_empty() {
      return into;
    }
    self.matches_candidate_into(path, &mut into);
    into
  }

  /// Adds the sequence number of every glob pattern that matches the given
  /// path to the vec given.
  ///
  /// `into` is cleared before matching begins, and contains the set of
  /// sequence numbers (in ascending order) after matching ends. If no globs
  /// were matched, then `into` will be empty.
  pub(crate) fn matches_into<P: AsRef<Path>>(&self, path: P, into: &mut Vec<usize>) {
    self.matches_candidate_into(&Candidate::new(path.as_ref()), into);
  }

  /// Adds the sequence number of every glob pattern that matches the given
  /// path to the vec given.
  ///
  /// `into` is cleared before matching begins, and contains the set of
  /// sequence numbers (in ascending order) after matching ends. If no globs
  /// were matched, then `into` will be empty.
  ///
  /// This takes a Candidate as input, which can be used to amortize the
  /// cost of preparing a path for matching.
  pub(crate) fn matches_candidate_into(&self, path: &Candidate<'_>, into: &mut Vec<usize>) {
    into.clear();
    if self.is_empty() {
      return;
    }
    for strat in &self.strats {
      strat.matches_into(path, into);
    }
    into.sort();
    into.dedup();
  }

  /// Builds a new matcher from a collection of Glob patterns.
  ///
  /// Once a matcher is built, no new patterns can be added to it.
  fn new<I, G>(globs: I) -> Result<GlobSet, Error>
  where
    I: IntoIterator<Item = G>,
    G: AsRef<Glob>,
  {
    let mut it = globs.into_iter().peekable();
    if it.peek().is_none() {
      return Ok(GlobSet::empty());
    }

    let mut len = 0;
    let mut lits = LiteralStrategy::new();
    let mut base_lits = BasenameLiteralStrategy::new();
    let mut exts = ExtensionStrategy::new();
    let mut prefixes = MultiStrategyBuilder::new();
    let mut suffixes = MultiStrategyBuilder::new();
    let mut required_exts = RequiredExtensionStrategyBuilder::new();
    let mut regexes = MultiStrategyBuilder::new();
    for (i, p) in it.enumerate() {
      len += 1;

      let p = p.as_ref();
      match MatchStrategy::new(p) {
        MatchStrategy::Literal(lit) => {
          lits.add(i, lit);
        }
        MatchStrategy::BasenameLiteral(lit) => {
          base_lits.add(i, lit);
        }
        MatchStrategy::Extension(ext) => {
          exts.add(i, ext);
        }
        MatchStrategy::Prefix(prefix) => {
          prefixes.add(i, prefix);
        }
        MatchStrategy::Suffix { suffix, component } => {
          if component {
            lits.add(i, suffix[1..].to_string());
          }
          suffixes.add(i, suffix);
        }
        MatchStrategy::RequiredExtension(ext) => {
          required_exts.add(i, ext, p.regex().to_owned());
        }
        MatchStrategy::Regex => {
          regexes.add(i, p.regex().to_owned());
        }
      }
    }

    let mut strats = Vec::with_capacity(7);
    // Only add strategies that are populated
    if !exts.0.is_empty() {
      strats.push(GlobSetMatchStrategy::Extension(exts));
    }
    if !base_lits.0.is_empty() {
      strats.push(GlobSetMatchStrategy::BasenameLiteral(base_lits));
    }
    if !lits.0.is_empty() {
      strats.push(GlobSetMatchStrategy::Literal(lits));
    }
    if !suffixes.is_empty() {
      strats.push(GlobSetMatchStrategy::Suffix(suffixes.suffix()));
    }
    if !prefixes.is_empty() {
      strats.push(GlobSetMatchStrategy::Prefix(prefixes.prefix()));
    }
    if !required_exts.0.is_empty() {
      strats.push(GlobSetMatchStrategy::RequiredExtension(
        required_exts.build()?,
      ));
    }
    if !regexes.is_empty() {
      strats.push(GlobSetMatchStrategy::Regex(regexes.regex_set()?));
    }

    Ok(GlobSet { len, strats })
  }
}

impl Default for GlobSet {
  /// Create a default empty GlobSet.
  fn default() -> Self {
    GlobSet::empty()
  }
}

/// A hasher that implements the Fowler–Noll–Vo (FNV) hash.
struct Hasher(u64);

impl Hasher {
  const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
  const PRIME: u64 = 0x100000001b3;
}

impl Default for Hasher {
  fn default() -> Hasher {
    Hasher(Hasher::OFFSET_BASIS)
  }
}

impl std::hash::Hasher for Hasher {
  fn finish(&self) -> u64 {
    self.0
  }

  fn write(&mut self, bytes: &[u8]) {
    for &byte in bytes.iter() {
      self.0 = self.0 ^ u64::from(byte);
      self.0 = self.0.wrapping_mul(Hasher::PRIME);
    }
  }
}

/// Describes a matching strategy for a particular pattern.
///
/// This provides a way to more quickly determine whether a pattern matches
/// a particular file path in a way that scales with a large number of
/// patterns. For example, if many patterns are of the form `*.ext`, then it's
/// possible to test whether any of those patterns matches by looking up a
/// file path's extension in a hash table.
#[derive(Clone, Debug, Eq, PartialEq)]
enum MatchStrategy {
  /// A pattern matches if and only if the entire file path matches this
  /// literal string.
  Literal(String),
  /// A pattern matches if and only if the file path's basename matches this
  /// literal string.
  BasenameLiteral(String),
  /// A pattern matches if and only if the file path's extension matches this
  /// literal string.
  Extension(String),
  /// A pattern matches if and only if this prefix literal is a prefix of the
  /// candidate file path.
  Prefix(String),
  /// A pattern matches if and only if this prefix literal is a prefix of the
  /// candidate file path.
  ///
  /// An exception: if `component` is true, then `suffix` must appear at the
  /// beginning of a file path or immediately following a `/`.
  Suffix {
    /// The actual suffix.
    suffix: String,
    /// Whether this must start at the beginning of a path component.
    component: bool,
  },
  /// A pattern matches only if the given extension matches the file path's
  /// extension. Note that this is a necessary but NOT sufficient criterion.
  /// Namely, if the extension matches, then a full regex search is still
  /// required.
  RequiredExtension(String),
  /// A regex needs to be used for matching.
  Regex,
}

impl MatchStrategy {
  /// Returns a matching strategy for the given pattern.
  fn new(pat: &Glob) -> MatchStrategy {
    if let Some(lit) = pat.basename_literal() {
      MatchStrategy::BasenameLiteral(lit)
    } else if let Some(lit) = pat.literal() {
      MatchStrategy::Literal(lit)
    } else if let Some(ext) = pat.ext() {
      MatchStrategy::Extension(ext)
    } else if let Some(prefix) = pat.prefix() {
      MatchStrategy::Prefix(prefix)
    } else if let Some((suffix, component)) = pat.suffix() {
      MatchStrategy::Suffix { suffix, component }
    } else if let Some(ext) = pat.required_ext() {
      MatchStrategy::RequiredExtension(ext)
    } else {
      MatchStrategy::Regex
    }
  }
}

/// Glob represents a successfully parsed shell glob pattern.
///
/// It cannot be used directly to match file paths, but it can be converted
/// to a regular expression string or a matcher.
#[derive(Clone, Eq)]
pub struct Glob {
  glob: String,
  re: String,
  opts: GlobOptions,
  tokens: Tokens,
}

impl AsRef<Glob> for Glob {
  fn as_ref(&self) -> &Glob {
    self
  }
}

impl PartialEq for Glob {
  fn eq(&self, other: &Glob) -> bool {
    self.glob == other.glob && self.opts == other.opts
  }
}

impl std::hash::Hash for Glob {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.glob.hash(state);
    self.opts.hash(state);
  }
}

impl std::fmt::Debug for Glob {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if f.alternate() {
      f.debug_struct("Glob")
        .field("glob", &self.glob)
        .field("re", &self.re)
        .field("opts", &self.opts)
        .field("tokens", &self.tokens)
        .finish()
    } else {
      f.debug_tuple("Glob").field(&self.glob).finish()
    }
  }
}

impl std::fmt::Display for Glob {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.glob.fmt(f)
  }
}

impl std::str::FromStr for Glob {
  type Err = Error;

  fn from_str(glob: &str) -> Result<Self, Self::Err> {
    Self::new(glob)
  }
}

/// A matcher for a single pattern.
#[derive(Clone, Debug)]
pub struct GlobMatcher {
  /// The underlying pattern.
  pat: Glob,
  /// The pattern, as a compiled regex.
  re: Regex,
}

impl GlobMatcher {
  /// Tests whether the given path matches this pattern or not.
  pub fn is_match<P: AsRef<Path>>(&self, path: P) -> bool {
    self.is_match_candidate(&Candidate::new(path.as_ref()))
  }

  /// Tests whether the given path matches this pattern or not.
  fn is_match_candidate(&self, path: &Candidate<'_>) -> bool {
    self.re.is_match(&path.path)
  }

  /// Returns the `Glob` used to compile this matcher.
  pub fn glob(&self) -> &Glob {
    &self.pat
  }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct GlobOptions {
  /// Whether to match case insensitively.
  case_insensitive: bool,
  /// Whether to require a literal separator to match a separator in a file
  /// path. e.g., when enabled, `*` won't match `/`.
  literal_separator: bool,
  /// Whether or not to use `\` to escape special characters.
  /// e.g., when enabled, `\*` will match a literal `*`.
  backslash_escape: bool,
  /// Whether or not an empty case in an alternate will be removed.
  /// e.g., when enabled, `{,a}` will match "" and "a".
  empty_alternates: bool,
  /// Whether or not an unclosed character class is allowed. When an unclosed
  /// character class is found, the opening `[` is treated as a literal `[`.
  /// When this isn't enabled, an opening `[` without a corresponding `]` is
  /// treated as an error.
  allow_unclosed_class: bool,
}

impl GlobOptions {
  fn default() -> GlobOptions {
    GlobOptions {
      case_insensitive: false,
      literal_separator: false,
      backslash_escape: !is_separator('\\'),
      empty_alternates: false,
      allow_unclosed_class: false,
    }
  }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Tokens(Vec<Token>);

impl std::ops::Deref for Tokens {
  type Target = Vec<Token>;
  fn deref(&self) -> &Vec<Token> {
    &self.0
  }
}

impl std::ops::DerefMut for Tokens {
  fn deref_mut(&mut self) -> &mut Vec<Token> {
    &mut self.0
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Token {
  Literal(char),
  Any,
  ZeroOrMore,
  RecursivePrefix,
  RecursiveSuffix,
  RecursiveZeroOrMore,
  Class {
    negated: bool,
    ranges: Vec<(char, char)>,
  },
  Alternates(Vec<Tokens>),
}

impl Glob {
  /// Builds a new pattern with default options.
  pub fn new(glob: &str) -> Result<Glob, Error> {
    GlobBuilder::new(glob).build()
  }

  /// Returns a matcher for this pattern.
  pub fn compile_matcher(&self) -> GlobMatcher {
    let re = new_regex(&self.re).expect("regex compilation shouldn't fail");
    GlobMatcher {
      pat: self.clone(),
      re,
    }
  }

  /// Returns the original glob pattern used to build this pattern.
  pub fn glob(&self) -> &str {
    &self.glob
  }

  /// Returns the regular expression string for this glob.
  ///
  /// Note that regular expressions for globs are intended to be matched on
  /// arbitrary bytes (`&[u8]`) instead of Unicode strings (`&str`). In
  /// particular, globs are frequently used on file paths, where there is no
  /// general guarantee that file paths are themselves valid UTF-8. As a
  /// result, callers will need to ensure that they are using a regex API
  /// that can match on arbitrary bytes. For example, the
  /// [`regex`](https://crates.io/regex)
  /// crate's
  /// [`Regex`](https://docs.rs/regex/*/regex/struct.Regex.html)
  /// API is not suitable for this since it matches on `&str`, but its
  /// [`bytes::Regex`](https://docs.rs/regex/*/regex/bytes/struct.Regex.html)
  /// API is suitable for this.
  fn regex(&self) -> &str {
    &self.re
  }

  /// Returns the pattern as a literal if and only if the pattern must match
  /// an entire path exactly.
  ///
  /// The basic format of these patterns is `{literal}`.
  fn literal(&self) -> Option<String> {
    if self.opts.case_insensitive {
      return None;
    }
    let mut lit = String::new();
    for t in &*self.tokens {
      let Token::Literal(c) = *t else { return None };
      lit.push(c);
    }
    if lit.is_empty() { None } else { Some(lit) }
  }

  /// Returns an extension if this pattern matches a file path if and only
  /// if the file path has the extension returned.
  ///
  /// Note that this extension returned differs from the extension that
  /// std::path::Path::extension returns. Namely, this extension includes
  /// the '.'. Also, paths like `.rs` are considered to have an extension
  /// of `.rs`.
  fn ext(&self) -> Option<String> {
    if self.opts.case_insensitive {
      return None;
    }
    let start = match *self.tokens.get(0)? {
      Token::RecursivePrefix => 1,
      _ => 0,
    };
    match *self.tokens.get(start)? {
      Token::ZeroOrMore => {
        // If there was no recursive prefix, then we only permit
        // `*` if `*` can match a `/`. For example, if `*` can't
        // match `/`, then `*.c` doesn't match `foo/bar.c`.
        if start == 0 && self.opts.literal_separator {
          return None;
        }
      }
      _ => return None,
    }
    match *self.tokens.get(start + 1)? {
      Token::Literal('.') => {}
      _ => return None,
    }
    let mut lit = ".".to_string();
    for t in self.tokens[start + 2..].iter() {
      match *t {
        Token::Literal('.') | Token::Literal('/') => return None,
        Token::Literal(c) => lit.push(c),
        _ => return None,
      }
    }
    if lit.is_empty() { None } else { Some(lit) }
  }

  /// This is like `ext`, but returns an extension even if it isn't sufficient
  /// to imply a match. Namely, if an extension is returned, then it is
  /// necessary but not sufficient for a match.
  fn required_ext(&self) -> Option<String> {
    if self.opts.case_insensitive {
      return None;
    }
    // We don't care at all about the beginning of this pattern. All we
    // need to check for is if it ends with a literal of the form `.ext`.
    let mut ext: Vec<char> = vec![]; // built in reverse
    for t in self.tokens.iter().rev() {
      match *t {
        Token::Literal('/') => return None,
        Token::Literal(c) => {
          ext.push(c);
          if c == '.' {
            break;
          }
        }
        _ => return None,
      }
    }
    if ext.last() != Some(&'.') {
      None
    } else {
      ext.reverse();
      Some(ext.into_iter().collect())
    }
  }

  /// Returns a literal prefix of this pattern if the entire pattern matches
  /// if the literal prefix matches.
  fn prefix(&self) -> Option<String> {
    if self.opts.case_insensitive {
      return None;
    }
    let (end, need_sep) = match *self.tokens.last()? {
      Token::ZeroOrMore => {
        if self.opts.literal_separator {
          // If a trailing `*` can't match a `/`, then we can't
          // assume a match of the prefix corresponds to a match
          // of the overall pattern. e.g., `foo/*` with
          // `literal_separator` enabled matches `foo/bar` but not
          // `foo/bar/baz`, even though `foo/bar/baz` has a `foo/`
          // literal prefix.
          return None;
        }
        (self.tokens.len() - 1, false)
      }
      Token::RecursiveSuffix => (self.tokens.len() - 1, true),
      _ => (self.tokens.len(), false),
    };
    let mut lit = String::new();
    for t in &self.tokens[0..end] {
      let Token::Literal(c) = *t else { return None };
      lit.push(c);
    }
    if need_sep {
      lit.push('/');
    }
    if lit.is_empty() { None } else { Some(lit) }
  }

  /// Returns a literal suffix of this pattern if the entire pattern matches
  /// if the literal suffix matches.
  ///
  /// If a literal suffix is returned and it must match either the entire
  /// file path or be preceded by a `/`, then also return true. This happens
  /// with a pattern like `**/foo/bar`. Namely, this pattern matches
  /// `foo/bar` and `baz/foo/bar`, but not `foofoo/bar`. In this case, the
  /// suffix returned is `/foo/bar` (but should match the entire path
  /// `foo/bar`).
  ///
  /// When this returns true, the suffix literal is guaranteed to start with
  /// a `/`.
  fn suffix(&self) -> Option<(String, bool)> {
    if self.opts.case_insensitive {
      return None;
    }
    let mut lit = String::new();
    let (start, entire) = match *self.tokens.get(0)? {
      Token::RecursivePrefix => {
        // We only care if this follows a path component if the next
        // token is a literal.
        if let Some(&Token::Literal(_)) = self.tokens.get(1) {
          lit.push('/');
          (1, true)
        } else {
          (1, false)
        }
      }
      _ => (0, false),
    };
    let start = match *self.tokens.get(start)? {
      Token::ZeroOrMore => {
        // If literal_separator is enabled, then a `*` can't
        // necessarily match everything, so reporting a suffix match
        // as a match of the pattern would be a false positive.
        if self.opts.literal_separator {
          return None;
        }
        start + 1
      }
      _ => start,
    };
    for t in &self.tokens[start..] {
      let Token::Literal(c) = *t else { return None };
      lit.push(c);
    }
    if lit.is_empty() || lit == "/" {
      None
    } else {
      Some((lit, entire))
    }
  }

  /// If this pattern only needs to inspect the basename of a file path,
  /// then the tokens corresponding to only the basename match are returned.
  ///
  /// For example, given a pattern of `**/*.foo`, only the tokens
  /// corresponding to `*.foo` are returned.
  ///
  /// Note that this will return None if any match of the basename tokens
  /// doesn't correspond to a match of the entire pattern. For example, the
  /// glob `foo` only matches when a file path has a basename of `foo`, but
  /// doesn't *always* match when a file path has a basename of `foo`. e.g.,
  /// `foo` doesn't match `abc/foo`.
  fn basename_tokens(&self) -> Option<&[Token]> {
    if self.opts.case_insensitive {
      return None;
    }
    let start = match *self.tokens.get(0)? {
      Token::RecursivePrefix => 1,
      _ => {
        // With nothing to gobble up the parent portion of a path,
        // we can't assume that matching on only the basename is
        // correct.
        return None;
      }
    };
    if self.tokens[start..].is_empty() {
      return None;
    }
    for t in self.tokens[start..].iter() {
      match *t {
        Token::Literal('/') => return None,
        Token::Literal(_) => {} // OK
        Token::Any | Token::ZeroOrMore => {
          if !self.opts.literal_separator {
            // In this case, `*` and `?` can match a path
            // separator, which means this could reach outside
            // the basename.
            return None;
          }
        }
        Token::RecursivePrefix | Token::RecursiveSuffix | Token::RecursiveZeroOrMore => {
          return None;
        }
        Token::Class { .. } | Token::Alternates(..) => {
          // We *could* be a little smarter here, but either one
          // of these is going to prevent our literal optimizations
          // anyway, so give up.
          return None;
        }
      }
    }
    Some(&self.tokens[start..])
  }

  /// Returns the pattern as a literal if and only if the pattern exclusively
  /// matches the basename of a file path *and* is a literal.
  ///
  /// The basic format of these patterns is `**/{literal}`, where `{literal}`
  /// does not contain a path separator.
  fn basename_literal(&self) -> Option<String> {
    let tokens = self.basename_tokens()?;
    let mut lit = String::new();
    for t in tokens {
      let Token::Literal(c) = *t else { return None };
      lit.push(c);
    }
    Some(lit)
  }
}

impl Tokens {
  /// Convert this pattern to a string that is guaranteed to be a valid
  /// regular expression and will represent the matching semantics of this
  /// glob pattern and the options given.
  fn to_regex_with(&self, options: &GlobOptions) -> String {
    let mut re = String::new();
    re.push_str("(?-u)");
    if options.case_insensitive {
      re.push_str("(?i)");
    }
    re.push('^');
    // Special case. If the entire glob is just `**`, then it should match
    // everything.
    if self.len() == 1 && self[0] == Token::RecursivePrefix {
      re.push_str(".*");
      re.push('$');
      return re;
    }
    self.tokens_to_regex(options, &self, &mut re);
    re.push('$');
    re
  }

  fn tokens_to_regex(&self, options: &GlobOptions, tokens: &[Token], re: &mut String) {
    for tok in tokens.iter() {
      match *tok {
        Token::Literal(c) => {
          re.push_str(&char_to_escaped_literal(c));
        }
        Token::Any => {
          if options.literal_separator {
            re.push_str("[^/]");
          } else {
            re.push_str(".");
          }
        }
        Token::ZeroOrMore => {
          if options.literal_separator {
            re.push_str("[^/]*");
          } else {
            re.push_str(".*");
          }
        }
        Token::RecursivePrefix => {
          re.push_str("(?:/?|.*/)");
        }
        Token::RecursiveSuffix => {
          re.push_str("/.*");
        }
        Token::RecursiveZeroOrMore => {
          re.push_str("(?:/|/.*/)");
        }
        Token::Class {
          negated,
          ref ranges,
        } => {
          re.push('[');
          if negated {
            re.push('^');
          }
          for r in ranges {
            if r.0 == r.1 {
              // Not strictly necessary, but nicer to look at.
              re.push_str(&char_to_escaped_literal(r.0));
            } else {
              re.push_str(&char_to_escaped_literal(r.0));
              re.push('-');
              re.push_str(&char_to_escaped_literal(r.1));
            }
          }
          re.push(']');
        }
        Token::Alternates(ref patterns) => {
          let mut parts = vec![];
          for pat in patterns {
            let mut altre = String::new();
            self.tokens_to_regex(options, &pat, &mut altre);
            if !altre.is_empty() || options.empty_alternates {
              parts.push(altre);
            }
          }

          // It is possible to have an empty set in which case the
          // resulting alternation '()' would be an error.
          if !parts.is_empty() {
            re.push_str("(?:");
            re.push_str(&parts.join("|"));
            re.push(')');
          }
        }
      }
    }
  }
}

/// Convert a Unicode scalar value to an escaped string suitable for use as
/// a literal in a non-Unicode regex.
fn char_to_escaped_literal(c: char) -> String {
  let mut buf = [0; 4];
  let bytes = c.encode_utf8(&mut buf).as_bytes();
  bytes_to_escaped_literal(bytes)
}

/// Converts an arbitrary sequence of bytes to a UTF-8 string. All non-ASCII
/// code units are converted to their escaped form.
fn bytes_to_escaped_literal(bs: &[u8]) -> String {
  let mut s = String::with_capacity(bs.len());
  for &b in bs {
    if b <= 0x7F {
      regex_syntax::escape_into(char::from(b).encode_utf8(&mut [0; 4]), &mut s);
    } else {
      write!(&mut s, "\\x{:02x}", b).unwrap();
    }
  }
  s
}

struct Parser<'a> {
  /// The glob to parse.
  glob: &'a str,
  /// Marks the index in `stack` where the alternation started.
  alternates_stack: Vec<usize>,
  /// The set of active alternation branches being parsed.
  /// Tokens are added to the end of the last one.
  branches: Vec<Tokens>,
  /// A character iterator over the glob pattern to parse.
  chars: std::iter::Peekable<std::str::Chars<'a>>,
  /// The previous character seen.
  prev: Option<char>,
  /// The current character.
  cur: Option<char>,
  /// Whether we failed to find a closing `]` for a character
  /// class. This can only be true when `GlobOptions::allow_unclosed_class`
  /// is enabled. When enabled, it is impossible to ever parse another
  /// character class with this glob. That's because classes cannot be
  /// nested *and* the only way this happens is when there is never a `]`.
  ///
  /// We track this state so that we don't end up spending quadratic time
  /// trying to parse something like `[[[[[[[[[[[[[[[[[[[[[[[...`.
  found_unclosed_class: bool,
  /// Glob options, which may influence parsing.
  opts: &'a GlobOptions,
}

impl<'a> Parser<'a> {
  fn error(&self, kind: ErrorKind) -> Error {
    Error {
      glob: Some(self.glob.to_string()),
      kind,
    }
  }

  fn parse(&mut self) -> Result<(), Error> {
    while let Some(c) = self.bump() {
      match c {
        '?' => self.push_token(Token::Any)?,
        '*' => self.parse_star()?,
        '[' if !self.found_unclosed_class => self.parse_class()?,
        '{' => self.push_alternate()?,
        '}' => self.pop_alternate()?,
        ',' => self.parse_comma()?,
        '\\' => self.parse_backslash()?,
        c => self.push_token(Token::Literal(c))?,
      }
    }
    Ok(())
  }

  fn push_alternate(&mut self) -> Result<(), Error> {
    self.alternates_stack.push(self.branches.len());
    self.branches.push(Tokens::default());
    Ok(())
  }

  fn pop_alternate(&mut self) -> Result<(), Error> {
    let Some(start) = self.alternates_stack.pop() else {
      return Err(self.error(ErrorKind::UnopenedAlternates));
    };
    assert!(start <= self.branches.len());
    let alts = Token::Alternates(self.branches.drain(start..).collect());
    self.push_token(alts)?;
    Ok(())
  }

  fn push_token(&mut self, tok: Token) -> Result<(), Error> {
    if let Some(ref mut pat) = self.branches.last_mut() {
      return Ok(pat.push(tok));
    }
    Err(self.error(ErrorKind::UnopenedAlternates))
  }

  fn pop_token(&mut self) -> Result<Token, Error> {
    if let Some(ref mut pat) = self.branches.last_mut() {
      return Ok(pat.pop().unwrap());
    }
    Err(self.error(ErrorKind::UnopenedAlternates))
  }

  fn have_tokens(&self) -> Result<bool, Error> {
    match self.branches.last() {
      None => Err(self.error(ErrorKind::UnopenedAlternates)),
      Some(ref pat) => Ok(!pat.is_empty()),
    }
  }

  fn parse_comma(&mut self) -> Result<(), Error> {
    // If we aren't inside a group alternation, then don't
    // treat commas specially. Otherwise, we need to start
    // a new alternate branch.
    if self.alternates_stack.is_empty() {
      self.push_token(Token::Literal(','))
    } else {
      Ok(self.branches.push(Tokens::default()))
    }
  }

  fn parse_backslash(&mut self) -> Result<(), Error> {
    if self.opts.backslash_escape {
      match self.bump() {
        None => Err(self.error(ErrorKind::DanglingEscape)),
        Some(c) => self.push_token(Token::Literal(c)),
      }
    } else if is_separator('\\') {
      // Normalize all patterns to use / as a separator.
      self.push_token(Token::Literal('/'))
    } else {
      self.push_token(Token::Literal('\\'))
    }
  }

  fn parse_star(&mut self) -> Result<(), Error> {
    let prev = self.prev;
    if self.peek() != Some('*') {
      self.push_token(Token::ZeroOrMore)?;
      return Ok(());
    }
    assert!(self.bump() == Some('*'));
    if !self.have_tokens()? {
      if !self.peek().map_or(true, is_separator) {
        self.push_token(Token::ZeroOrMore)?;
        self.push_token(Token::ZeroOrMore)?;
      } else {
        self.push_token(Token::RecursivePrefix)?;
        assert!(self.bump().map_or(true, is_separator));
      }
      return Ok(());
    }

    if !prev.map(is_separator).unwrap_or(false) {
      if self.branches.len() <= 1 || (prev != Some(',') && prev != Some('{')) {
        self.push_token(Token::ZeroOrMore)?;
        self.push_token(Token::ZeroOrMore)?;
        return Ok(());
      }
    }
    let is_suffix = match self.peek() {
      None => {
        assert!(self.bump().is_none());
        true
      }
      Some(',') | Some('}') if self.branches.len() >= 2 => true,
      Some(c) if is_separator(c) => {
        assert!(self.bump().map(is_separator).unwrap_or(false));
        false
      }
      _ => {
        self.push_token(Token::ZeroOrMore)?;
        self.push_token(Token::ZeroOrMore)?;
        return Ok(());
      }
    };
    match self.pop_token()? {
      Token::RecursivePrefix => {
        self.push_token(Token::RecursivePrefix)?;
      }
      Token::RecursiveSuffix => {
        self.push_token(Token::RecursiveSuffix)?;
      }
      _ => {
        if is_suffix {
          self.push_token(Token::RecursiveSuffix)?;
        } else {
          self.push_token(Token::RecursiveZeroOrMore)?;
        }
      }
    }
    Ok(())
  }

  fn parse_class(&mut self) -> Result<(), Error> {
    // Save parser state for potential rollback to literal '[' parsing.
    let saved_chars = self.chars.clone();
    let saved_prev = self.prev;
    let saved_cur = self.cur;

    fn add_to_last_range(glob: &str, r: &mut (char, char), add: char) -> Result<(), Error> {
      r.1 = add;
      if r.1 < r.0 {
        Err(Error {
          glob: Some(glob.to_string()),
          kind: ErrorKind::InvalidRange(r.0, r.1),
        })
      } else {
        Ok(())
      }
    }
    let mut ranges = vec![];
    let negated = match self.chars.peek() {
      Some(&'!') | Some(&'^') => {
        let bump = self.bump();
        assert!(bump == Some('!') || bump == Some('^'));
        true
      }
      _ => false,
    };
    let mut first = true;
    let mut in_range = false;
    loop {
      let Some(c) = self.bump() else {
        return if self.opts.allow_unclosed_class == true {
          self.chars = saved_chars;
          self.cur = saved_cur;
          self.prev = saved_prev;
          self.found_unclosed_class = true;

          self.push_token(Token::Literal('['))
        } else {
          Err(self.error(ErrorKind::UnclosedClass))
        };
      };
      match c {
        ']' => {
          if first {
            ranges.push((']', ']'));
          } else {
            break;
          }
        }
        '-' => {
          if first {
            ranges.push(('-', '-'));
          } else if in_range {
            // invariant: in_range is only set when there is
            // already at least one character seen.
            let r = ranges.last_mut().unwrap();
            add_to_last_range(&self.glob, r, '-')?;
            in_range = false;
          } else {
            assert!(!ranges.is_empty());
            in_range = true;
          }
        }
        c => {
          if in_range {
            // invariant: in_range is only set when there is
            // already at least one character seen.
            add_to_last_range(&self.glob, ranges.last_mut().unwrap(), c)?;
          } else {
            ranges.push((c, c));
          }
          in_range = false;
        }
      }
      first = false;
    }
    if in_range {
      // Means that the last character in the class was a '-', so add
      // it as a literal.
      ranges.push(('-', '-'));
    }
    self.push_token(Token::Class { negated, ranges })
  }

  fn bump(&mut self) -> Option<char> {
    self.prev = self.cur;
    self.cur = self.chars.next();
    self.cur
  }

  fn peek(&mut self) -> Option<char> {
    self.chars.peek().map(|&ch| ch)
  }
}

/// The final component of the path, if it is a normal file.
///
/// If the path terminates in `..`, or consists solely of a root of prefix,
/// file_name will return `None`.
fn file_name<'a>(path: &Cow<'a, [u8]>) -> Option<Cow<'a, [u8]>> {
  if path.is_empty() {
    return None;
  }
  let last_slash = path.rfind_byte(b'/').map(|i| i + 1).unwrap_or(0);
  let got = match *path {
    Cow::Borrowed(path) => Cow::Borrowed(&path[last_slash..]),
    Cow::Owned(ref path) => {
      let mut path = path.clone();
      path.drain_bytes(..last_slash);
      Cow::Owned(path)
    }
  };
  if got == &b".."[..] {
    return None;
  }
  Some(got)
}

fn file_name_ext<'a>(name: &Cow<'a, [u8]>) -> Option<Cow<'a, [u8]>> {
  if name.is_empty() {
    return None;
  }
  let last_dot_at = match name.rfind_byte(b'.') {
    None => return None,
    Some(i) => i,
  };
  Some(match *name {
    Cow::Borrowed(name) => Cow::Borrowed(&name[last_dot_at..]),
    Cow::Owned(ref name) => {
      let mut name = name.clone();
      name.drain_bytes(..last_dot_at);
      Cow::Owned(name)
    }
  })
}

fn normalize_path(path: Cow<'_, [u8]>) -> Cow<'_, [u8]> {
  // UNIX only uses /, so we're good.
  path
}

/// Represents an error that can occur when parsing a glob pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
  /// The original glob provided by the caller.
  glob: Option<String>,
  /// The kind of error.
  kind: ErrorKind,
}

/// The kind of error that can occur when parsing a glob pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) enum ErrorKind {
  /// Occurs when a character class (e.g., `[abc]`) is not closed.
  UnclosedClass,
  /// Occurs when a range in a character (e.g., `[a-z]`) is invalid. For
  /// example, if the range starts with a lexicographically larger character
  /// than it ends with.
  InvalidRange(char, char),
  /// Occurs when a `}` is found without a matching `{`.
  UnopenedAlternates,
  /// Occurs when a `{` is found without a matching `}`.
  UnclosedAlternates,

  /// Occurs when an unescaped '\' is found at the end of a glob.
  DanglingEscape,
  /// An error associated with parsing or compiling a regex.
  Regex(String),
}

impl std::error::Error for Error {
  fn description(&self) -> &str {
    self.kind.description()
  }
}

impl Error {
  /// Return the kind of this error.
  pub(crate) fn kind(&self) -> &ErrorKind {
    &self.kind
  }
}

impl ErrorKind {
  fn description(&self) -> &str {
    match *self {
      ErrorKind::UnclosedClass => "unclosed character class; missing ']'",
      ErrorKind::InvalidRange(_, _) => "invalid character range",
      ErrorKind::UnopenedAlternates => {
        "unopened alternate group; missing '{' \
                (maybe escape '}' with '[}]'?)"
      }
      ErrorKind::UnclosedAlternates => {
        "unclosed alternate group; missing '}' \
                (maybe escape '{' with '[{]'?)"
      }

      ErrorKind::DanglingEscape => "dangling '\\'",
      ErrorKind::Regex(ref err) => err,
    }
  }
}

impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self.glob {
      None => self.kind.fmt(f),
      Some(ref glob) => {
        write!(f, "error parsing glob '{}': {}", glob, self.kind)
      }
    }
  }
}

impl std::fmt::Display for ErrorKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match *self {
      ErrorKind::UnclosedClass
      | ErrorKind::UnopenedAlternates
      | ErrorKind::UnclosedAlternates
      | ErrorKind::DanglingEscape
      | ErrorKind::Regex(_) => write!(f, "{}", self.description()),
      ErrorKind::InvalidRange(s, e) => {
        write!(f, "invalid range; '{}' > '{}'", s, e)
      }
    }
  }
}

fn new_regex(pat: &str) -> Result<Regex, Error> {
  let syntax = regex_automata::util::syntax::Config::new()
    .utf8(false)
    .dot_matches_new_line(true);
  let config = Regex::config()
    .utf8_empty(false)
    .nfa_size_limit(Some(10 * (1 << 20)))
    .hybrid_cache_capacity(10 * (1 << 20));
  Regex::builder()
    .syntax(syntax)
    .configure(config)
    .build(pat)
    .map_err(|err| Error {
      glob: Some(pat.to_string()),
      kind: ErrorKind::Regex(err.to_string()),
    })
}

fn new_regex_set(pats: Vec<String>) -> Result<Regex, Error> {
  let syntax = regex_automata::util::syntax::Config::new()
    .utf8(false)
    .dot_matches_new_line(true);
  let config = Regex::config()
    .match_kind(regex_automata::MatchKind::All)
    .utf8_empty(false)
    .nfa_size_limit(Some(10 * (1 << 20)))
    .hybrid_cache_capacity(10 * (1 << 20));
  Regex::builder()
    .syntax(syntax)
    .configure(config)
    .build_many(&pats)
    .map_err(|err| Error {
      glob: None,
      kind: ErrorKind::Regex(err.to_string()),
    })
}

/// GlobSetBuilder builds a group of patterns that can be used to
/// simultaneously match a file path.
#[derive(Clone, Debug)]
pub struct GlobSetBuilder {
  pats: Vec<Glob>,
}

impl GlobSetBuilder {
  /// Create a new `GlobSetBuilder`. A `GlobSetBuilder` can be used to add new
  /// patterns. Once all patterns have been added, `build` should be called
  /// to produce a [`GlobSet`], which can then be used for matching.
  pub fn new() -> GlobSetBuilder {
    GlobSetBuilder { pats: vec![] }
  }

  /// Builds a new matcher from all of the glob patterns added so far.
  ///
  /// Once a matcher is built, no new patterns can be added to it.
  pub fn build(&self) -> Result<GlobSet, Error> {
    GlobSet::new(self.pats.iter())
  }

  /// Add a new pattern to this set.
  pub fn add(&mut self, pat: Glob) -> &mut GlobSetBuilder {
    self.pats.push(pat);
    self
  }
}

/// A candidate path for matching.
///
/// All glob matching in this crate operates on `Candidate` values.
/// Constructing candidates has a very small cost associated with it, so
/// callers may find it beneficial to amortize that cost when matching a single
/// path against multiple globs or sets of globs.
#[derive(Clone)]
pub(crate) struct Candidate<'a> {
  path: Cow<'a, [u8]>,
  basename: Cow<'a, [u8]>,
  ext: Cow<'a, [u8]>,
}

impl<'a> std::fmt::Debug for Candidate<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    f.debug_struct("Candidate")
      .field("path", &self.path.as_bstr())
      .field("basename", &self.basename.as_bstr())
      .field("ext", &self.ext.as_bstr())
      .finish()
  }
}

impl<'a> Candidate<'a> {
  /// Create a new candidate for matching from the given path.
  pub(crate) fn new<P: AsRef<Path> + ?Sized>(path: &'a P) -> Candidate<'a> {
    Self::from_cow(Vec::from_path_lossy(path.as_ref()))
  }

  fn from_cow(path: Cow<'a, [u8]>) -> Candidate<'a> {
    let path = normalize_path(path);
    let basename = file_name(&path).unwrap_or(Cow::Borrowed(B("")));
    let ext = file_name_ext(&basename).unwrap_or(Cow::Borrowed(B("")));
    Candidate {
      path,
      basename,
      ext,
    }
  }

  fn path_prefix(&self, max: usize) -> &[u8] {
    if self.path.len() <= max {
      &*self.path
    } else {
      &self.path[..max]
    }
  }

  fn path_suffix(&self, max: usize) -> &[u8] {
    if self.path.len() <= max {
      &*self.path
    } else {
      &self.path[self.path.len() - max..]
    }
  }
}

#[derive(Clone, Debug)]
enum GlobSetMatchStrategy {
  Literal(LiteralStrategy),
  BasenameLiteral(BasenameLiteralStrategy),
  Extension(ExtensionStrategy),
  Prefix(PrefixStrategy),
  Suffix(SuffixStrategy),
  RequiredExtension(RequiredExtensionStrategy),
  Regex(RegexSetStrategy),
}

impl GlobSetMatchStrategy {
  fn is_match(&self, candidate: &Candidate<'_>) -> bool {
    use self::GlobSetMatchStrategy::*;
    match *self {
      Literal(ref s) => s.is_match(candidate),
      BasenameLiteral(ref s) => s.is_match(candidate),
      Extension(ref s) => s.is_match(candidate),
      Prefix(ref s) => s.is_match(candidate),
      Suffix(ref s) => s.is_match(candidate),
      RequiredExtension(ref s) => s.is_match(candidate),
      Regex(ref s) => s.is_match(candidate),
    }
  }

  fn matches_into(&self, candidate: &Candidate<'_>, matches: &mut Vec<usize>) {
    use self::GlobSetMatchStrategy::*;
    match *self {
      Literal(ref s) => s.matches_into(candidate, matches),
      BasenameLiteral(ref s) => s.matches_into(candidate, matches),
      Extension(ref s) => s.matches_into(candidate, matches),
      Prefix(ref s) => s.matches_into(candidate, matches),
      Suffix(ref s) => s.matches_into(candidate, matches),
      RequiredExtension(ref s) => s.matches_into(candidate, matches),
      Regex(ref s) => s.matches_into(candidate, matches),
    }
  }
}

#[derive(Clone, Debug)]
struct LiteralStrategy(HashMap<Vec<u8>, Vec<usize>>);

impl LiteralStrategy {
  fn new() -> LiteralStrategy {
    LiteralStrategy(HashMap::default())
  }

  fn add(&mut self, global_index: usize, lit: String) {
    self
      .0
      .entry(lit.into_bytes())
      .or_insert(vec![])
      .push(global_index);
  }

  fn is_match(&self, candidate: &Candidate<'_>) -> bool {
    self.0.contains_key(candidate.path.as_bytes())
  }

  #[inline(never)]
  fn matches_into(&self, candidate: &Candidate<'_>, matches: &mut Vec<usize>) {
    if let Some(hits) = self.0.get(candidate.path.as_bytes()) {
      matches.extend(hits);
    }
  }
}

#[derive(Clone, Debug)]
struct BasenameLiteralStrategy(HashMap<Vec<u8>, Vec<usize>>);

impl BasenameLiteralStrategy {
  fn new() -> BasenameLiteralStrategy {
    BasenameLiteralStrategy(HashMap::default())
  }

  fn add(&mut self, global_index: usize, lit: String) {
    self
      .0
      .entry(lit.into_bytes())
      .or_insert(vec![])
      .push(global_index);
  }

  fn is_match(&self, candidate: &Candidate<'_>) -> bool {
    if candidate.basename.is_empty() {
      return false;
    }
    self.0.contains_key(candidate.basename.as_bytes())
  }

  #[inline(never)]
  fn matches_into(&self, candidate: &Candidate<'_>, matches: &mut Vec<usize>) {
    if candidate.basename.is_empty() {
      return;
    }
    if let Some(hits) = self.0.get(candidate.basename.as_bytes()) {
      matches.extend(hits);
    }
  }
}

#[derive(Clone, Debug)]
struct ExtensionStrategy(HashMap<Vec<u8>, Vec<usize>>);

impl ExtensionStrategy {
  fn new() -> ExtensionStrategy {
    ExtensionStrategy(HashMap::default())
  }

  fn add(&mut self, global_index: usize, ext: String) {
    self
      .0
      .entry(ext.into_bytes())
      .or_insert(vec![])
      .push(global_index);
  }

  fn is_match(&self, candidate: &Candidate<'_>) -> bool {
    if candidate.ext.is_empty() {
      return false;
    }
    self.0.contains_key(candidate.ext.as_bytes())
  }

  #[inline(never)]
  fn matches_into(&self, candidate: &Candidate<'_>, matches: &mut Vec<usize>) {
    if candidate.ext.is_empty() {
      return;
    }
    if let Some(hits) = self.0.get(candidate.ext.as_bytes()) {
      matches.extend(hits);
    }
  }
}

#[derive(Clone, Debug)]
struct PrefixStrategy {
  matcher: AhoCorasick,
  map: Vec<usize>,
  longest: usize,
}

impl PrefixStrategy {
  fn is_match(&self, candidate: &Candidate<'_>) -> bool {
    let path = candidate.path_prefix(self.longest);
    for m in self.matcher.find_overlapping_iter(path) {
      if m.start() == 0 {
        return true;
      }
    }
    false
  }

  fn matches_into(&self, candidate: &Candidate<'_>, matches: &mut Vec<usize>) {
    let path = candidate.path_prefix(self.longest);
    for m in self.matcher.find_overlapping_iter(path) {
      if m.start() == 0 {
        matches.push(self.map[m.pattern()]);
      }
    }
  }
}

#[derive(Clone, Debug)]
struct SuffixStrategy {
  matcher: AhoCorasick,
  map: Vec<usize>,
  longest: usize,
}

impl SuffixStrategy {
  fn is_match(&self, candidate: &Candidate<'_>) -> bool {
    let path = candidate.path_suffix(self.longest);
    for m in self.matcher.find_overlapping_iter(path) {
      if m.end() == path.len() {
        return true;
      }
    }
    false
  }

  fn matches_into(&self, candidate: &Candidate<'_>, matches: &mut Vec<usize>) {
    let path = candidate.path_suffix(self.longest);
    for m in self.matcher.find_overlapping_iter(path) {
      if m.end() == path.len() {
        matches.push(self.map[m.pattern()]);
      }
    }
  }
}

#[derive(Clone, Debug)]
struct RequiredExtensionStrategy(HashMap<Vec<u8>, Vec<(usize, Regex)>>);

impl RequiredExtensionStrategy {
  fn is_match(&self, candidate: &Candidate<'_>) -> bool {
    if candidate.ext.is_empty() {
      return false;
    }
    match self.0.get(candidate.ext.as_bytes()) {
      None => false,
      Some(regexes) => {
        for &(_, ref re) in regexes {
          if re.is_match(candidate.path.as_bytes()) {
            return true;
          }
        }
        false
      }
    }
  }

  #[inline(never)]
  fn matches_into(&self, candidate: &Candidate<'_>, matches: &mut Vec<usize>) {
    if candidate.ext.is_empty() {
      return;
    }
    if let Some(regexes) = self.0.get(candidate.ext.as_bytes()) {
      for &(global_index, ref re) in regexes {
        if re.is_match(candidate.path.as_bytes()) {
          matches.push(global_index);
        }
      }
    }
  }
}

#[derive(Clone, Debug)]
struct RegexSetStrategy {
  matcher: Regex,
  map: Vec<usize>,
  patset: Arc<Pool<PatternSet, PatternSetPoolFn>>,
}

type PatternSetPoolFn = Box<dyn Fn() -> PatternSet + Send + Sync + UnwindSafe + RefUnwindSafe>;

impl RegexSetStrategy {
  fn is_match(&self, candidate: &Candidate<'_>) -> bool {
    self.matcher.is_match(candidate.path.as_bytes())
  }

  fn matches_into(&self, candidate: &Candidate<'_>, matches: &mut Vec<usize>) {
    let input = regex_automata::Input::new(candidate.path.as_bytes());
    let mut patset = self.patset.get();
    patset.clear();
    self.matcher.which_overlapping_matches(&input, &mut patset);
    for i in patset.iter() {
      matches.push(self.map[i]);
    }
    PoolGuard::put(patset);
  }
}

#[derive(Clone, Debug)]
struct MultiStrategyBuilder {
  literals: Vec<String>,
  map: Vec<usize>,
  longest: usize,
}

impl MultiStrategyBuilder {
  fn new() -> MultiStrategyBuilder {
    MultiStrategyBuilder {
      literals: vec![],
      map: vec![],
      longest: 0,
    }
  }

  fn add(&mut self, global_index: usize, literal: String) {
    if literal.len() > self.longest {
      self.longest = literal.len();
    }
    self.map.push(global_index);
    self.literals.push(literal);
  }

  fn prefix(self) -> PrefixStrategy {
    PrefixStrategy {
      matcher: AhoCorasick::new(&self.literals).unwrap(),
      map: self.map,
      longest: self.longest,
    }
  }

  fn suffix(self) -> SuffixStrategy {
    SuffixStrategy {
      matcher: AhoCorasick::new(&self.literals).unwrap(),
      map: self.map,
      longest: self.longest,
    }
  }

  fn regex_set(self) -> Result<RegexSetStrategy, Error> {
    let matcher = new_regex_set(self.literals)?;
    let pattern_len = matcher.pattern_len();
    let create: PatternSetPoolFn = Box::new(move || PatternSet::new(pattern_len));
    Ok(RegexSetStrategy {
      matcher,
      map: self.map,
      patset: Arc::new(Pool::new(create)),
    })
  }

  fn is_empty(&self) -> bool {
    self.literals.is_empty()
  }
}

#[derive(Clone, Debug)]
struct RequiredExtensionStrategyBuilder(HashMap<Vec<u8>, Vec<(usize, String)>>);

impl RequiredExtensionStrategyBuilder {
  fn new() -> RequiredExtensionStrategyBuilder {
    RequiredExtensionStrategyBuilder(HashMap::default())
  }

  fn add(&mut self, global_index: usize, ext: String, regex: String) {
    self
      .0
      .entry(ext.into_bytes())
      .or_insert(vec![])
      .push((global_index, regex));
  }

  fn build(self) -> Result<RequiredExtensionStrategy, Error> {
    let mut exts = HashMap::default();
    for (ext, regexes) in self.0.into_iter() {
      exts.insert(ext.clone(), vec![]);
      for (global_index, regex) in regexes {
        let compiled = new_regex(&regex)?;
        exts.get_mut(&ext).unwrap().push((global_index, compiled));
      }
    }
    Ok(RequiredExtensionStrategy(exts))
  }
}
