use std::path::{Path, PathBuf};

pub use crate::ignore::walk::{
  DirEntry, ParallelVisitor, ParallelVisitorBuilder, Walk, WalkBuilder, WalkParallel, WalkState,
};

mod default_types {
  #[rustfmt::skip]
  pub(crate) const DEFAULT_TYPES: &[(&[&str], &[&str])] = &[
      (&["ada"], &["*.adb", "*.ads"]),
      (&["agda"], &["*.agda", "*.lagda"]),
      (&["aidl"], &["*.aidl"]),
      (&["alire"], &["alire.toml"]),
      (&["amake"], &["*.mk", "*.bp"]),
      (&["asciidoc"], &["*.adoc", "*.asc", "*.asciidoc"]),
      (&["asm"], &["*.asm", "*.s", "*.S"]),
      (&["asp"], &[
          "*.aspx", "*.aspx.cs", "*.aspx.vb", "*.ascx", "*.ascx.cs",
          "*.ascx.vb", "*.asp"
      ]),
      (&["ats"], &["*.ats", "*.dats", "*.sats", "*.hats"]),
      (&["avro"], &["*.avdl", "*.avpr", "*.avsc"]),
      (&["awk"], &["*.awk"]),
      (&["bat", "batch"], &["*.bat"]),
      (&["bazel"], &[
          "*.bazel", "*.bzl", "*.BUILD", "*.bazelrc", "BUILD", "MODULE.bazel",
          "WORKSPACE", "WORKSPACE.bazel", "WORKSPACE.bzlmod",
      ]),
      (&["bitbake"], &["*.bb", "*.bbappend", "*.bbclass", "*.conf", "*.inc"]),
      (&["boxlang"], &["*.bx", "*.bxm", "*.bxs"]),
      (&["brotli"], &["*.br"]),
      (&["buildstream"], &["*.bst"]),
      (&["bzip2"], &["*.bz2", "*.tbz2"]),
      (&["c"], &["*.[chH]", "*.[chH].in", "*.cats"]),
      (&["cabal"], &["*.cabal"]),
      (&["candid"], &["*.did"]),
      (&["carp"], &["*.carp"]),
      (&["cbor"], &["*.cbor"]),
      (&["ceylon"], &["*.ceylon"]),
      (&["cfml"], &["*.cfc", "*.cfm"]),
      (&["clojure"], &["*.clj", "*.cljc", "*.cljs", "*.cljx"]),
      (&["cmake"], &["*.cmake", "CMakeLists.txt"]),
      (&["cmd"], &["*.bat", "*.cmd"]),
      (&["cml"], &["*.cml"]),
      (&["coffeescript"], &["*.coffee"]),
      (&["config"], &["*.cfg", "*.conf", "*.config", "*.ini"]),
      (&["container"], &["*Containerfile*", "*Dockerfile*"]),
      (&["coq"], &["*.v"]),
      (&["cpp"], &[
          "*.[ChH]", "*.cc", "*.[ch]pp", "*.[ch]xx", "*.hh",  "*.inl",
          "*.[ChH].in", "*.cc.in", "*.[ch]pp.in", "*.[ch]xx.in", "*.hh.in",
      ]),
      (&["creole"], &["*.creole"]),
      (&["crystal"], &["Projectfile", "*.cr", "*.ecr", "shard.yml"]),
      (&["cs"], &["*.cs"]),
      (&["csharp"], &["*.cs"]),
      (&["cshtml"], &["*.cshtml"]),
      (&["csproj"], &["*.csproj"]),
      (&["css"], &["*.css", "*.scss"]),
      (&["csv"], &["*.csv"]),
      (&["cuda"], &["*.cu", "*.cuh"]),
      (&["cython"], &["*.pyx", "*.pxi", "*.pxd"]),
      (&["d"], &["*.d"]),
      (&["dart"], &["*.dart"]),
      (&["devicetree"], &["*.dts", "*.dtsi", "*.dtso"]),
      (&["dhall"], &["*.dhall"]),
      (&["diff"], &["*.patch", "*.diff"]),
      (&["dita"], &["*.dita", "*.ditamap", "*.ditaval"]),
      (&["docker"], &["*Dockerfile*"]),
      (&["dockercompose"], &["docker-compose.yml", "docker-compose.*.yml"]),
      (&["dts"], &["*.dts", "*.dtsi"]),
      (&["dvc"], &["Dvcfile", "*.dvc"]),
      (&["ebuild"], &["*.ebuild", "*.eclass"]),
      (&["edn"], &["*.edn"]),
      (&["elisp"], &["*.el"]),
      (&["elixir"], &["*.ex", "*.eex", "*.exs", "*.heex", "*.leex", "*.livemd"]),
      (&["elm"], &["*.elm"]),
      (&["erb"], &["*.erb"]),
      (&["erlang"], &["*.erl", "*.hrl"]),
      (&["fennel"], &["*.fnl"]),
      (&["fidl"], &["*.fidl"]),
      (&["fish"], &["*.fish"]),
      (&["flatbuffers"], &["*.fbs"]),
      (&["fortran"], &[
          "*.f", "*.F", "*.f77", "*.F77", "*.pfo",
          "*.f90", "*.F90", "*.f95", "*.F95",
      ]),
      (&["fsharp"], &["*.fs", "*.fsx", "*.fsi"]),
      (&["fut"], &["*.fut"]),
      (&["gap"], &["*.g", "*.gap", "*.gi", "*.gd", "*.tst"]),
      (&["gdscript"], &["*.gd"]),
      (&["gleam"], &["*.gleam"]),
      (&["gn"], &["*.gn", "*.gni"]),
      (&["go"], &["*.go"]),
      (&["gprbuild"], &["*.gpr"]),
      (&["gradle"], &[
          "*.gradle", "*.gradle.kts", "gradle.properties", "gradle-wrapper.*",
          "gradlew", "gradlew.bat",
      ]),
      (&["graphql"], &["*.graphql", "*.graphqls"]),
      (&["groovy"], &["*.groovy", "*.gradle"]),
      (&["gzip"], &["*.gz", "*.tgz"]),
      (&["h"], &["*.h", "*.hh", "*.hpp"]),
      (&["haml"], &["*.haml"]),
      (&["hare"], &["*.ha"]),
      (&["haskell"], &["*.hs", "*.lhs", "*.cpphs", "*.c2hs", "*.hsc"]),
      (&["hbs"], &["*.hbs"]),
      (&["hs"], &["*.hs", "*.lhs"]),
      (&["html"], &["*.htm", "*.html", "*.ejs"]),
      (&["hurl"], &["*.hurl"]),
      (&["hy"], &["*.hy"]),
      (&["idris"], &["*.idr", "*.lidr"]),
      (&["janet"], &["*.janet"]),
      (&["java"], &["*.java", "*.jsp", "*.jspx", "*.properties"]),
      (&["jinja"], &["*.j2", "*.jinja", "*.jinja2"]),
      (&["jl"], &["*.jl"]),
      (&["js"], &["*.js", "*.jsx", "*.vue", "*.cjs", "*.mjs"]),
      (&["json"], &["*.json", "composer.lock", "*.sarif"]),
      (&["jsonl"], &["*.jsonl"]),
      (&["julia"], &["*.jl"]),
      (&["jupyter"], &["*.ipynb", "*.jpynb"]),
      (&["k"], &["*.k"]),
      (&["kconfig"], &["Kconfig", "Kconfig.*"]),
      (&["kotlin"], &["*.kt", "*.kts"]),
      (&["lean"], &["*.lean"]),
      (&["less"], &["*.less"]),
      (&["license"], &[
          // General
          "COPYING", "COPYING[.-]*",
          "COPYRIGHT", "COPYRIGHT[.-]*",
          "EULA", "EULA[.-]*",
          "licen[cs]e", "licen[cs]e.*",
          "LICEN[CS]E", "LICEN[CS]E[.-]*", "*[.-]LICEN[CS]E*",
          "NOTICE", "NOTICE[.-]*",
          "PATENTS", "PATENTS[.-]*",
          "UNLICEN[CS]E", "UNLICEN[CS]E[.-]*",
          // GPL (gpl.txt, etc.)
          "agpl[.-]*",
          "gpl[.-]*",
          "lgpl[.-]*",
          // Other license-specific (APACHE-2.0.txt, etc.)
          "AGPL-*[0-9]*",
          "APACHE-*[0-9]*",
          "BSD-*[0-9]*",
          "CC-BY-*",
          "GFDL-*[0-9]*",
          "GNU-*[0-9]*",
          "GPL-*[0-9]*",
          "LGPL-*[0-9]*",
          "MIT-*[0-9]*",
          "MPL-*[0-9]*",
          "OFL-*[0-9]*",
      ]),
      (&["lilypond"], &["*.ly", "*.ily"]),
      (&["lisp"], &["*.el", "*.jl", "*.lisp", "*.lsp", "*.sc", "*.scm"]),
      (&["llvm"], &["*.ll"]),
      (&["lock"], &["*.lock", "package-lock.json"]),
      (&["log"], &["*.log"]),
      (&["lua"], &["*.lua"]),
      (&["lz4"], &["*.lz4"]),
      (&["lzma"], &["*.lzma"]),
      (&["m4"], &["*.ac", "*.m4"]),
      (&["make"], &[
          "[Gg][Nn][Uu]makefile", "[Mm]akefile",
          "[Gg][Nn][Uu]makefile.am", "[Mm]akefile.am",
          "[Gg][Nn][Uu]makefile.in", "[Mm]akefile.in",
          "Makefile.*",
          "*.mk", "*.mak"
      ]),
      (&["mako"], &["*.mako", "*.mao"]),
      (&["man"], &["*.[0-9lnpx]", "*.[0-9][cEFMmpSx]"]),
      (&["markdown", "md"], &[
          "*.markdown",
          "*.md",
          "*.mdown",
          "*.mdwn",
          "*.mkd",
          "*.mkdn",
          "*.mdx",
      ]),
      (&["matlab"], &["*.m"]),
      (&["meson"], &["meson.build", "meson_options.txt", "meson.options"]),
      (&["minified"], &["*.min.html", "*.min.css", "*.min.js"]),
      (&["mint"], &["*.mint"]),
      (&["mk"], &["mkfile"]),
      (&["ml"], &["*.ml"]),
      (&["mojo"], &["*.mojo"]),
      (&["motoko"], &["*.mo"]),
      (&["msbuild"], &[
          "*.csproj", "*.fsproj", "*.vcxproj", "*.proj", "*.props", "*.targets",
          "*.sln", "*.slnf"
      ]),
      (&["nim"], &["*.nim", "*.nimf", "*.nimble", "*.nims"]),
      (&["nix"], &["*.nix"]),
      (&["objc"], &["*.h", "*.m"]),
      (&["objcpp"], &["*.h", "*.mm"]),
      (&["ocaml"], &["*.ml", "*.mli", "*.mll", "*.mly"]),
      (&["org"], &["*.org", "*.org_archive"]),
      (&["pants"], &["BUILD"]),
      (&["pascal"], &["*.pas", "*.dpr", "*.lpr", "*.pp", "*.inc"]),
      (&["pdf"], &["*.pdf"]),
      (&["perl"], &["*.perl", "*.pl", "*.PL", "*.plh", "*.plx", "*.pm", "*.t"]),
      (&["php"], &[
          // note that PHP 6 doesn't exist
          // See: https://wiki.php.net/rfc/php6
          "*.php", "*.php3", "*.php4", "*.php5", "*.php7", "*.php8",
          "*.pht", "*.phtml"
      ]),
      (&["pkgbuild"], &["PKGBUILD"]),
      (&["po"], &["*.po"]),
      (&["pod"], &["*.pod"]),
      (&["postscript"], &["*.eps", "*.ps"]),
      (&["prolog"], &["*.pl", "*.pro", "*.prolog", "*.P"]),
      (&["proto", "protobuf"], &["*.proto"]),
      (&["ps"], &["*.cdxml", "*.ps1", "*.ps1xml", "*.psd1", "*.psm1"]),
      (&["puppet"], &["*.epp", "*.erb", "*.pp", "*.rb"]),
      (&["purs"], &["*.purs"]),
      (&["py", "python"], &["*.py", "*.pyi"]),
      (&["qmake"], &["*.pro", "*.pri", "*.prf"]),
      (&["qml"], &["*.qml"]),
      (&["qrc"], &["*.qrc"]),
      (&["qui"], &["*.ui"]),
      (&["r"], &["*.R", "*.r", "*.Rmd", "*.rmd", "*.Rnw", "*.rnw"]),
      (&["racket"], &["*.rkt"]),
      (&["raku"], &[
          "*.raku", "*.rakumod", "*.rakudoc", "*.rakutest",
          "*.p6", "*.pl6", "*.pm6"
      ]),
      (&["rdoc"], &["*.rdoc"]),
      (&["readme"], &["README*", "*README"]),
      (&["reasonml"], &["*.re", "*.rei"]),
      (&["red"], &["*.r", "*.red", "*.reds"]),
      (&["rescript"], &["*.res", "*.resi"]),
      (&["robot"], &["*.robot"]),
      (&["rocq"], &["*.v"]),
      (&["rst"], &["*.rst"]),
      (&["ruby"], &[
          // Idiomatic files
          "config.ru", "Gemfile", ".irbrc", "Rakefile",
          // Extensions
          "*.gemspec", "*.rb", "*.rbw", "*.rake"
      ]),
      (&["rust"], &["*.rs"]),
      (&["sass"], &["*.sass", "*.scss"]),
      (&["scala"], &["*.scala", "*.sbt"]),
      (&["scdoc"], &["*.scd", "*.scdoc"]),
      (&["seed7"], &["*.sd7", "*.s7i"]),
      (&["sh"], &[
          // Portable/misc. init files
          ".env", ".login", ".logout", ".profile", "profile",
          // bash-specific init files
          ".bash_login", "bash_login",
          ".bash_logout", "bash_logout",
          ".bash_profile", "bash_profile",
          ".bashrc", "bashrc", "*.bashrc",
          // csh-specific init files
          ".cshrc", "*.cshrc",
          // ksh-specific init files
          ".kshrc", "*.kshrc",
          // tcsh-specific init files
          ".tcshrc",
          // zsh-specific init files
          ".zshenv", "zshenv",
          ".zlogin", "zlogin",
          ".zlogout", "zlogout",
          ".zprofile", "zprofile",
          ".zshrc", "zshrc",
          // Extensions
          "*.bash", "*.csh", "*.env", "*.ksh", "*.sh", "*.tcsh", "*.zsh",
      ]),
      (&["slim"], &["*.skim", "*.slim", "*.slime"]),
      (&["smarty"], &["*.tpl"]),
      (&["sml"], &["*.sml", "*.sig"]),
      (&["solidity"], &["*.sol"]),
      (&["soy"], &["*.soy"]),
      (&["spark"], &["*.spark"]),
      (&["spec"], &["*.spec"]),
      (&["sql"], &["*.sql", "*.psql"]),
      (&["ssa"], &["*.ssa"]),
      (&["stylus"], &["*.styl"]),
      (&["sv"], &["*.v", "*.vg", "*.sv", "*.svh", "*.h"]),
      (&["svelte"], &["*.svelte", "*.svelte.ts"]),
      (&["svg"], &["*.svg"]),
      (&["swift"], &["*.swift"]),
      (&["swig"], &["*.def", "*.i"]),
      (&["systemd"], &[
          "*.automount", "*.conf", "*.device", "*.link", "*.mount", "*.path",
          "*.scope", "*.service", "*.slice", "*.socket", "*.swap", "*.target",
          "*.timer",
      ]),
      (&["taskpaper"], &["*.taskpaper"]),
      (&["tcl"], &["*.tcl"]),
      (&["tex"], &["*.tex", "*.ltx", "*.cls", "*.sty", "*.bib", "*.dtx", "*.ins"]),
      (&["texinfo"], &["*.texi"]),
      (&["textile"], &["*.textile"]),
      (&["tf"], &[
          "*.tf", "*.tf.json", "*.tfvars", "*.tfvars.json",
          "*.terraformrc", "terraform.rc", "*.tfrc", "*.terraform.lock.hcl",
      ]),
      (&["thrift"], &["*.thrift"]),
      (&["toml"], &["*.toml", "Cargo.lock"]),
      (&["ts", "typescript"], &["*.ts", "*.tsx", "*.cts", "*.mts"]),
      (&["twig"], &["*.twig"]),
      (&["txt"], &["*.txt"]),
      (&["typoscript"], &["*.typoscript", "*.ts"]),
      (&["typst"], &["*.typ"]),
      (&["usd"], &["*.usd", "*.usda", "*.usdc"]),
      (&["v"], &["*.v", "*.vsh"]),
      (&["vala"], &["*.vala"]),
      (&["vb"], &["*.vb"]),
      (&["vcl"], &["*.vcl"]),
      (&["verilog"], &["*.v", "*.vh", "*.sv", "*.svh"]),
      (&["vhdl"], &["*.vhd", "*.vhdl"]),
      (&["vim"], &[
          "*.vim", ".vimrc", ".gvimrc", "vimrc", "gvimrc", "_vimrc", "_gvimrc",
      ]),
      (&["vimscript"], &[
          "*.vim", ".vimrc", ".gvimrc", "vimrc", "gvimrc", "_vimrc", "_gvimrc",
      ]),
      (&["vue"], &["*.vue"]),
      (&["webidl"], &["*.idl", "*.webidl", "*.widl"]),
      (&["wgsl"], &["*.wgsl"]),
      (&["wiki"], &["*.mediawiki", "*.wiki"]),
      (&["xml"], &[
          "*.xml", "*.xml.dist", "*.dtd", "*.xsl", "*.xslt", "*.xsd", "*.xjb",
          "*.rng", "*.sch", "*.xhtml",
      ]),
      (&["xz"], &["*.xz", "*.txz"]),
      (&["yacc"], &["*.y"]),
      (&["yaml"], &["*.yaml", "*.yml"]),
      (&["yang"], &["*.yang"]),
      (&["z"], &["*.Z"]),
      (&["zig"], &["*.zig"]),
      (&["zsh"], &[
          ".zshenv", "zshenv",
          ".zlogin", "zlogin",
          ".zlogout", "zlogout",
          ".zprofile", "zprofile",
          ".zshrc", "zshrc",
          "*.zsh",
      ]),
      (&["zstd"], &["*.zst", "*.zstd"]),
  ];
}

mod dir {

  use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs::{self, File, FileType},
    io::{self, BufRead},
    path::{Path, PathBuf},
    sync::{Arc, RwLock, Weak},
  };

  use crate::{
    ignore::gitignore::{self, Gitignore, GitignoreBuilder},
    ignore::overrides::{self, Override},
    ignore::pathutil::{is_hidden, strip_prefix},
    ignore::types::{self, Types},
    ignore::walk::DirEntry,
    ignore::{Error, Match, PartialErrorBuilder},
  };

  /// IgnoreMatch represents information about where a match came from when using
  /// the `Ignore` matcher.
  #[derive(Clone, Debug)]
  #[allow(dead_code)]
  pub(crate) struct IgnoreMatch<'a>(IgnoreMatchInner<'a>);

  /// IgnoreMatchInner describes precisely where the match information came from.
  /// This is private to allow expansion to more matchers in the future.
  #[derive(Clone, Debug)]
  #[allow(dead_code)]
  enum IgnoreMatchInner<'a> {
    Override(overrides::Glob<'a>),
    Gitignore(&'a gitignore::Glob),
    Types(types::Glob<'a>),
    Hidden,
  }

  impl<'a> IgnoreMatch<'a> {
    fn overrides(x: overrides::Glob<'a>) -> IgnoreMatch<'a> {
      IgnoreMatch(IgnoreMatchInner::Override(x))
    }

    fn gitignore(x: &'a gitignore::Glob) -> IgnoreMatch<'a> {
      IgnoreMatch(IgnoreMatchInner::Gitignore(x))
    }

    fn types(x: types::Glob<'a>) -> IgnoreMatch<'a> {
      IgnoreMatch(IgnoreMatchInner::Types(x))
    }

    fn hidden() -> IgnoreMatch<'static> {
      IgnoreMatch(IgnoreMatchInner::Hidden)
    }
  }

  /// Options for the ignore matcher, shared between the matcher itself and the
  /// builder.
  #[derive(Clone, Copy, Debug)]
  struct IgnoreOptions {
    /// Whether to ignore hidden file paths or not.
    hidden: bool,
    /// Whether to read .ignore files.
    ignore: bool,
    /// Whether to respect any ignore files in parent directories.
    parents: bool,
    /// Whether to read git's global gitignore file.
    git_global: bool,
    /// Whether to read .gitignore files.
    git_ignore: bool,
    /// Whether to read .git/info/exclude files.
    git_exclude: bool,
    /// Whether to ignore files case insensitively
    ignore_case_insensitive: bool,
    /// Whether a git repository must be present in order to apply any
    /// git-related ignore rules.
    require_git: bool,
  }

  /// Ignore is a matcher useful for recursively walking one or more directories.
  #[derive(Clone, Debug)]
  pub(crate) struct Ignore {
    inner: Arc<IgnoreInner>,
    // Parent matchers are cached independently of the path being walked, but
    // matching them still needs the canonicalized path originally passed to
    // `add_parents`. For example, when walking `/tmp/project/src`, parent
    // matchers use `/tmp/project/src` to rewrite `/tmp/project/src/foo.py`
    // before matching it against ignore files from `/tmp/project` and its
    // ancestors.
    //
    // For ripgrep itself, this means that `rg pat src tests` must rewrite
    // `src/foo` relative to `.../src`, and not whatever root was prepared
    // first.
    //
    // See: https://github.com/BurntSushi/ripgrep/pull/3420
    // See: https://github.com/BurntSushi/ripgrep/issues/3376
    // See: https://github.com/BurntSushi/ripgrep/issues/3419
    // See: https://github.com/BurntSushi/ripgrep/issues/3320
    absolute_base: Option<Arc<PathBuf>>,
  }

  #[derive(Clone, Debug)]
  struct IgnoreInner {
    /// A map of all existing directories that have already been
    /// compiled into matchers.
    ///
    /// Note that this is never used during matching, only when adding new
    /// parent directory matchers. This avoids needing to rebuild glob sets for
    /// parent directories if many paths are being searched.
    compiled: Arc<RwLock<HashMap<OsString, Weak<IgnoreInner>>>>,
    /// The path to the directory that this matcher was built from.
    dir: PathBuf,
    /// An override matcher (default is empty).
    overrides: Arc<Override>,
    /// A file type matcher.
    types: Arc<Types>,
    /// The parent directory to match next.
    ///
    /// If this is the root directory or there are otherwise no more
    /// directories to match, then `parent` is `None`.
    parent: Option<Arc<IgnoreInner>>,
    /// Whether this is an absolute parent matcher, as added by add_parent.
    is_absolute_parent: bool,
    /// The directory that gitignores should be interpreted relative to.
    ///
    /// Usually this is the directory containing the gitignore file. But in
    /// some cases, like for global gitignores or for gitignores specified
    /// explicitly, this should generally be set to the current working
    /// directory. This is only used for global gitignores or "explicit"
    /// gitignores.
    ///
    /// When `None`, this means the CWD could not be determined or is unknown.
    /// In this case, global gitignore files are ignored because they otherwise
    /// cannot be matched correctly.
    global_gitignores_relative_to: Option<PathBuf>,
    /// Explicit global ignore matchers specified by the caller.
    explicit_ignores: Arc<Vec<Gitignore>>,
    /// Ignore files used in addition to `.ignore`
    custom_ignore_filenames: Arc<Vec<OsString>>,
    /// The matcher for custom ignore files
    custom_ignore_matcher: Gitignore,
    /// The matcher for .ignore files.
    ignore_matcher: Gitignore,
    /// A global gitignore matcher, usually from $XDG_CONFIG_HOME/git/ignore.
    git_global_matcher: Arc<Gitignore>,
    /// The matcher for .gitignore files.
    git_ignore_matcher: Gitignore,
    /// Special matcher for `.git/info/exclude` files.
    git_exclude_matcher: Gitignore,
    /// Whether this directory contains a .git sub-directory.
    has_git: bool,
    /// Ignore config.
    opts: IgnoreOptions,
  }

  impl Ignore {
    /// Return true if this matcher has no parent.
    pub(crate) fn is_root(&self) -> bool {
      self.inner.parent.is_none()
    }

    /// Return this matcher's parent, if one exists.
    pub(crate) fn parent(&self) -> Option<Ignore> {
      self.inner.parent.as_ref().map(|parent| Ignore {
        inner: parent.clone(),
        absolute_base: self.absolute_base.clone(),
      })
    }

    /// Create a new `Ignore` matcher with the parent directories of `dir`.
    ///
    /// Note that this can only be called on an `Ignore` matcher with no
    /// parents (i.e., `is_root` returns `true`). This will panic otherwise.
    pub(crate) fn add_parents<P: AsRef<Path>>(&self, path: P) -> (Ignore, Option<Error>) {
      if !self.inner.opts.parents
        && !self.inner.opts.git_ignore
        && !self.inner.opts.git_exclude
        && !self.inner.opts.git_global
      {
        // If we never need info from parent directories, then don't do
        // anything.
        return (self.clone(), None);
      }
      if !self.is_root() {
        panic!("Ignore::add_parents called on non-root matcher");
      }
      let absolute_base = match path.as_ref().canonicalize() {
        Ok(path) => Arc::new(path),
        Err(_) => {
          // There's not much we can do here, so just return our
          // existing matcher. We drop the error to be consistent
          // with our general pattern of ignoring I/O errors when
          // processing ignore files.
          return (self.clone(), None);
        }
      };
      // List of parents, from child to root.
      let mut parents = vec![];
      let mut path = &**absolute_base;
      while let Some(parent) = path.parent() {
        parents.push(parent);
        path = parent;
      }
      let mut errs = PartialErrorBuilder::default();
      let mut ig = self.clone();
      for parent in parents.into_iter().rev() {
        let mut compiled = self.inner.compiled.write().unwrap();
        if let Some(weak) = compiled.get(parent.as_os_str())
          && let Some(prebuilt) = weak.upgrade()
        {
          ig = Ignore {
            inner: prebuilt,
            absolute_base: Some(absolute_base.clone()),
          };
          continue;
        }
        let (mut igtmp, err) = ig.add_child_path(parent);
        errs.maybe_push(err);
        igtmp.is_absolute_parent = true;
        igtmp.has_git = if self.inner.opts.require_git && self.inner.opts.git_ignore {
          parent.join(".git").exists() || parent.join(".jj").exists()
        } else {
          false
        };
        let ig_arc = Arc::new(igtmp);
        ig = Ignore {
          inner: ig_arc.clone(),
          absolute_base: Some(absolute_base.clone()),
        };
        compiled.insert(parent.as_os_str().to_os_string(), Arc::downgrade(&ig_arc));
      }
      (ig, errs.into_error_option())
    }

    /// Create a new `Ignore` matcher for the given child directory.
    ///
    /// Since building the matcher may require reading from multiple
    /// files, it's possible that this method partially succeeds. Therefore,
    /// a matcher is always returned (which may match nothing) and an error is
    /// returned if it exists.
    ///
    /// Note that all I/O errors are completely ignored.
    pub(crate) fn add_child<P: AsRef<Path>>(&self, dir: P) -> (Ignore, Option<Error>) {
      let (ig, err) = self.add_child_path(dir.as_ref());
      (
        Ignore {
          inner: Arc::new(ig),
          absolute_base: self.absolute_base.clone(),
        },
        err,
      )
    }

    /// Like add_child, but uses successful read_dir entries to reduce
    /// probing when discovering ignore files.
    pub(crate) fn add_child_with_entries<P: AsRef<Path>>(
      &self,
      dir: P,
      entries: &[fs::DirEntry],
    ) -> (Ignore, Option<Error>) {
      let files = self.collect_ignore_files(entries);
      let (ig, err) = self.add_child_path_with_found_ignore_files(dir.as_ref(), Some(&files));
      (
        Ignore {
          inner: Arc::new(ig),
          absolute_base: self.absolute_base.clone(),
        },
        err,
      )
    }

    /// Like add_child, but takes a full path and returns an IgnoreInner.
    fn add_child_path(&self, dir: &Path) -> (IgnoreInner, Option<Error>) {
      self.add_child_path_with_found_ignore_files(dir, None)
    }

    fn collect_ignore_files(&self, entries: &[fs::DirEntry]) -> IgnoreFilesFound {
      let custom_ignore_filenames = &self.inner.custom_ignore_filenames;
      let mut files = IgnoreFilesFound {
        has_ignore: false,
        has_git_ignore: false,
        has_git_dir: false,
        has_jj_dir: false,
        custom_ignore_files: vec![false; custom_ignore_filenames.len()],
      };
      for entry in entries {
        let file_name = entry.file_name();
        if file_name == OsStr::new(".ignore") {
          files.has_ignore = true;
        } else if file_name == OsStr::new(".gitignore") {
          files.has_git_ignore = true;
        } else if file_name == OsStr::new(".git") {
          files.has_git_dir = true;
        } else if file_name == OsStr::new(".jj") {
          files.has_jj_dir = true;
        }
        for (i, name) in custom_ignore_filenames.iter().enumerate() {
          if file_name == name.as_os_str() {
            files.custom_ignore_files[i] = true;
          }
        }
      }
      files
    }

    fn add_child_path_with_found_ignore_files(
      &self,
      dir: &Path,
      ignore_files_list: Option<&IgnoreFilesFound>,
    ) -> (IgnoreInner, Option<Error>) {
      let check_vcs_dir =
        self.inner.opts.require_git && (self.inner.opts.git_ignore || self.inner.opts.git_exclude);
      let git_type = if check_vcs_dir && ignore_files_list.is_none_or(|i| i.has_git_dir) {
        dir.join(".git").metadata().ok().map(|md| md.file_type())
      } else {
        None
      };
      let has_jj =
        check_vcs_dir && ignore_files_list.is_none_or(|i| i.has_jj_dir) && dir.join(".jj").exists();
      let has_git = check_vcs_dir && (git_type.is_some() || has_jj);

      let mut errs = PartialErrorBuilder::default();
      let custom_ig_matcher = if self.inner.custom_ignore_filenames.is_empty() {
        Gitignore::empty()
      } else {
        let custom_ignore_names: Vec<&OsString> = match ignore_files_list {
          None => self.inner.custom_ignore_filenames.iter().collect(),
          Some(m) => self
            .inner
            .custom_ignore_filenames
            .iter()
            .zip(m.custom_ignore_files.iter())
            .filter(|&(_, &matched)| matched)
            .map(|(name, _)| name)
            .collect(),
        };
        if custom_ignore_names.is_empty() {
          Gitignore::empty()
        } else {
          let (m, err) = create_gitignore(
            dir,
            dir,
            &custom_ignore_names,
            self.inner.opts.ignore_case_insensitive,
          );
          errs.maybe_push(err);
          m
        }
      };
      let ig_matcher = if !self.inner.opts.ignore || !ignore_files_list.is_none_or(|i| i.has_ignore)
      {
        Gitignore::empty()
      } else {
        let (m, err) = create_gitignore(
          dir,
          dir,
          &[".ignore"],
          self.inner.opts.ignore_case_insensitive,
        );
        errs.maybe_push(err);
        m
      };
      let gi_matcher =
        if !self.inner.opts.git_ignore || !ignore_files_list.is_none_or(|i| i.has_git_ignore) {
          Gitignore::empty()
        } else {
          let (m, err) = create_gitignore(
            dir,
            dir,
            &[".gitignore"],
            self.inner.opts.ignore_case_insensitive,
          );
          errs.maybe_push(err);
          m
        };

      let gi_exclude_matcher =
        if !self.inner.opts.git_exclude || !ignore_files_list.is_none_or(|i| i.has_git_dir) {
          Gitignore::empty()
        } else {
          match resolve_git_commondir(dir, git_type) {
            Ok(git_dir) => {
              let (m, err) = create_gitignore(
                dir,
                &git_dir,
                &["info/exclude"],
                self.inner.opts.ignore_case_insensitive,
              );
              errs.maybe_push(err);
              m
            }
            Err(err) => {
              errs.maybe_push(err);
              Gitignore::empty()
            }
          }
        };
      let ig = IgnoreInner {
        compiled: self.inner.compiled.clone(),
        dir: dir.to_path_buf(),
        overrides: self.inner.overrides.clone(),
        types: self.inner.types.clone(),
        parent: Some(self.inner.clone()),
        is_absolute_parent: false,
        global_gitignores_relative_to: self.inner.global_gitignores_relative_to.clone(),
        explicit_ignores: self.inner.explicit_ignores.clone(),
        custom_ignore_filenames: self.inner.custom_ignore_filenames.clone(),
        custom_ignore_matcher: custom_ig_matcher,
        ignore_matcher: ig_matcher,
        git_global_matcher: self.inner.git_global_matcher.clone(),
        git_ignore_matcher: gi_matcher,
        git_exclude_matcher: gi_exclude_matcher,
        has_git,
        opts: self.inner.opts,
      };
      (ig, errs.into_error_option())
    }

    /// Returns true if at least one type of ignore rule should be matched.
    fn has_any_ignore_rules(&self) -> bool {
      let opts = self.inner.opts;
      let has_custom_ignore_files = !self.inner.custom_ignore_filenames.is_empty();
      let has_explicit_ignores = !self.inner.explicit_ignores.is_empty();

      opts.ignore
        || opts.git_global
        || opts.git_ignore
        || opts.git_exclude
        || has_custom_ignore_files
        || has_explicit_ignores
    }

    /// Like `matched`, but works with a directory entry instead.
    pub(crate) fn matched_dir_entry<'a>(&'a self, dent: &DirEntry) -> Match<IgnoreMatch<'a>> {
      let m = self.matched(dent.path(), dent.is_dir());
      if m.is_none() && self.inner.opts.hidden && is_hidden(dent) {
        return Match::Ignore(IgnoreMatch::hidden());
      }
      m
    }

    /// Returns a match indicating whether the given file path should be
    /// ignored or not.
    ///
    /// The match contains information about its origin.
    fn matched<'a, P: AsRef<Path>>(&'a self, path: P, is_dir: bool) -> Match<IgnoreMatch<'a>> {
      // We need to be careful with our path. If it has a leading ./, then
      // strip it because it causes nothing but trouble.
      let mut path = path.as_ref();
      if let Some(p) = strip_prefix("./", path) {
        path = p;
      }
      // Match against the override patterns. If an override matches
      // regardless of whether it's whitelist/ignore, then we quit and
      // return that result immediately. Overrides have the highest
      // precedence.
      if !self.inner.overrides.is_empty() {
        let mat = self
          .inner
          .overrides
          .matched(path, is_dir)
          .map(IgnoreMatch::overrides);
        if !mat.is_none() {
          return mat;
        }
      }
      let mut whitelisted = Match::None;
      if self.has_any_ignore_rules() {
        let mat = self.matched_ignore(path, is_dir);
        if mat.is_ignore() {
          return mat;
        } else if mat.is_whitelist() {
          whitelisted = mat;
        }
      }
      if !self.inner.types.is_empty() {
        let mat = self
          .inner
          .types
          .matched(path, is_dir)
          .map(IgnoreMatch::types);
        if mat.is_ignore() {
          return mat;
        } else if mat.is_whitelist() {
          whitelisted = mat;
        }
      }
      whitelisted
    }

    /// Performs matching only on the ignore files for this directory and
    /// all parent directories.
    fn matched_ignore<'a>(&'a self, path: &Path, is_dir: bool) -> Match<IgnoreMatch<'a>> {
      let (mut m_custom_ignore, mut m_ignore, mut m_gi, mut m_gi_exclude, mut m_explicit) = (
        Match::None,
        Match::None,
        Match::None,
        Match::None,
        Match::None,
      );
      let any_git = !self.inner.opts.require_git || self.parents().any(|ig| ig.inner.has_git);
      let mut saw_git = false;
      for ig in self.parents().take_while(|ig| !ig.inner.is_absolute_parent) {
        if m_custom_ignore.is_none() {
          m_custom_ignore = ig
            .inner
            .custom_ignore_matcher
            .matched(path, is_dir)
            .map(IgnoreMatch::gitignore);
        }
        if m_ignore.is_none() {
          m_ignore = ig
            .inner
            .ignore_matcher
            .matched(path, is_dir)
            .map(IgnoreMatch::gitignore);
        }
        if any_git && !saw_git && m_gi.is_none() {
          m_gi = ig
            .inner
            .git_ignore_matcher
            .matched(path, is_dir)
            .map(IgnoreMatch::gitignore);
        }
        if any_git && !saw_git && m_gi_exclude.is_none() {
          m_gi_exclude = ig
            .inner
            .git_exclude_matcher
            .matched(path, is_dir)
            .map(IgnoreMatch::gitignore);
        }
        saw_git = saw_git || ig.inner.has_git;
      }
      if self.inner.opts.parents
        && let Some(abs_parent_path) = self.absolute_base()
      {
        // What we want to do here is take the absolute base path of
        // this directory and join it with the path we're searching.
        // The main issue we want to avoid is accidentally duplicating
        // directory components, so we try to strip any common prefix
        // off of `path`. Overall, this seems a little ham-fisted, but
        // it does fix a nasty bug. It should do fine until we overhaul
        // this crate.
        let path = abs_parent_path.join(
          self
            .parents()
            .take_while(|ig| !ig.inner.is_absolute_parent)
            .last()
            .map_or(path, |ig| {
              // This is a weird special case when ripgrep users
              // search with just a `.`, as some tools do
              // automatically (like consult). In this case, if
              // we don't bail out now, the code below will strip
              // a leading `.` from `path`, which might mangle
              // a hidden file name!
              if ig.inner.dir.as_path() == Path::new(".") {
                return path;
              }
              let without_dot_slash = strip_if_is_prefix("./", ig.inner.dir.as_path());
              let relative_base = strip_if_is_prefix(without_dot_slash, path);
              strip_if_is_prefix("/", relative_base)
            }),
        );

        for ig in self.parents().skip_while(|ig| !ig.inner.is_absolute_parent) {
          if m_custom_ignore.is_none() {
            m_custom_ignore = ig
              .inner
              .custom_ignore_matcher
              .matched(&path, is_dir)
              .map(IgnoreMatch::gitignore);
          }
          if m_ignore.is_none() {
            m_ignore = ig
              .inner
              .ignore_matcher
              .matched(&path, is_dir)
              .map(IgnoreMatch::gitignore);
          }
          if any_git && !saw_git && m_gi.is_none() {
            m_gi = ig
              .inner
              .git_ignore_matcher
              .matched(&path, is_dir)
              .map(IgnoreMatch::gitignore);
          }
          if any_git && !saw_git && m_gi_exclude.is_none() {
            m_gi_exclude = ig
              .inner
              .git_exclude_matcher
              .matched(&path, is_dir)
              .map(IgnoreMatch::gitignore);
          }
          saw_git = saw_git || ig.inner.has_git;
        }
      }
      for gi in self.inner.explicit_ignores.iter().rev() {
        if !m_explicit.is_none() {
          break;
        }
        m_explicit = gi.matched(path, is_dir).map(IgnoreMatch::gitignore);
      }
      let m_global = if any_git {
        self
          .inner
          .git_global_matcher
          .matched(path, is_dir)
          .map(IgnoreMatch::gitignore)
      } else {
        Match::None
      };

      m_custom_ignore
        .or(m_ignore)
        .or(m_gi)
        .or(m_gi_exclude)
        .or(m_global)
        .or(m_explicit)
    }

    /// Returns an iterator over parent ignore matchers, including this one.
    pub(crate) fn parents(&self) -> Parents<'_> {
      Parents(Some(IgnoreRef { inner: &self.inner }))
    }

    /// Returns the first absolute path of the first absolute parent, if
    /// one exists.
    fn absolute_base(&self) -> Option<&Path> {
      self.absolute_base.as_ref().map(|p| &***p)
    }
  }

  /// State for tracking what kinds of files ripgrep is interested in for a
  /// given directory.
  ///
  /// This is computed over the entire set of files in a directory instead of
  /// trying to stat each file individually. If a file is present, it's only then
  /// that we stat it for more information, instead of relying on the stat to
  /// determine its existence.
  #[derive(Debug)]
  struct IgnoreFilesFound {
    has_ignore: bool,
    has_git_ignore: bool,
    has_git_dir: bool,
    has_jj_dir: bool,
    custom_ignore_files: Vec<bool>,
  }

  #[derive(Clone, Copy)]
  pub(crate) struct IgnoreRef<'a> {
    inner: &'a IgnoreInner,
  }

  impl IgnoreRef<'_> {
    pub(crate) fn path(&self) -> &Path {
      &self.inner.dir
    }

    pub(crate) fn is_absolute_parent(&self) -> bool {
      self.inner.is_absolute_parent
    }
  }

  /// An iterator over all parents of an ignore matcher, including itself.
  pub(crate) struct Parents<'a>(Option<IgnoreRef<'a>>);

  impl<'a> Iterator for Parents<'a> {
    type Item = IgnoreRef<'a>;

    fn next(&mut self) -> Option<IgnoreRef<'a>> {
      match self.0.take() {
        None => None,
        Some(ig) => {
          self.0 = ig.inner.parent.as_deref().map(|inner| IgnoreRef { inner });
          Some(ig)
        }
      }
    }
  }

  /// A builder for creating an Ignore matcher.
  #[derive(Clone, Debug)]
  pub(crate) struct IgnoreBuilder {
    /// The root directory path for this ignore matcher.
    dir: PathBuf,
    /// An override matcher (default is empty).
    overrides: Arc<Override>,
    /// A type matcher (default is empty).
    types: Arc<Types>,
    /// Explicit global ignore matchers.
    explicit_ignores: Vec<Gitignore>,
    /// Ignore files in addition to .ignore.
    custom_ignore_filenames: Vec<OsString>,
    /// The directory that gitignores should be interpreted relative to.
    ///
    /// Usually this is the directory containing the gitignore file. But in
    /// some cases, like for global gitignores or for gitignores specified
    /// explicitly, this should generally be set to the current working
    /// directory. This is only used for global gitignores or "explicit"
    /// gitignores.
    ///
    /// When `None`, global gitignores are ignored.
    global_gitignores_relative_to: Option<PathBuf>,
    /// Ignore config.
    opts: IgnoreOptions,
  }

  impl IgnoreBuilder {
    /// Create a new builder for an `Ignore` matcher.
    ///
    /// It is likely a bug to use this without also calling `current_dir()`
    /// outside of tests. This isn't made mandatory because this is an internal
    /// abstraction and it's annoying to update tests.
    pub(crate) fn new() -> IgnoreBuilder {
      IgnoreBuilder {
        dir: Path::new("").to_path_buf(),
        overrides: Arc::new(Override::empty()),
        types: Arc::new(Types::empty()),
        explicit_ignores: vec![],
        custom_ignore_filenames: vec![],
        global_gitignores_relative_to: None,
        opts: IgnoreOptions {
          hidden: true,
          ignore: true,
          parents: true,
          git_global: true,
          git_ignore: true,
          git_exclude: true,
          ignore_case_insensitive: false,
          require_git: true,
        },
      }
    }

    /// Builds a new `Ignore` matcher.
    ///
    /// The matcher returned won't match anything until ignore rules from
    /// directories are added to it.
    pub(crate) fn build(&self) -> Ignore {
      self.build_with_cwd(None)
    }

    /// Builds a new `Ignore` matcher using the given CWD directory.
    ///
    /// The matcher returned won't match anything until ignore rules from
    /// directories are added to it.
    pub(crate) fn build_with_cwd(&self, cwd: Option<PathBuf>) -> Ignore {
      let global_gitignores_relative_to =
        cwd.or_else(|| self.global_gitignores_relative_to.clone());
      let git_global_matcher = if !self.opts.git_global {
        Gitignore::empty()
      } else if let Some(ref cwd) = global_gitignores_relative_to {
        let mut builder = GitignoreBuilder::new(cwd);
        builder
          .case_insensitive(self.opts.ignore_case_insensitive)
          .unwrap();
        let (gi, err) = builder.build_global();
        if let Some(err) = err {
          log::debug!("{}", err);
        }
        gi
      } else {
        log::debug!("ignoring global gitignore file because CWD is not known");
        Gitignore::empty()
      };

      Ignore {
        inner: Arc::new(IgnoreInner {
          compiled: Arc::new(RwLock::new(HashMap::new())),
          dir: self.dir.clone(),
          overrides: self.overrides.clone(),
          types: self.types.clone(),
          parent: None,
          is_absolute_parent: true,
          global_gitignores_relative_to,
          explicit_ignores: Arc::new(self.explicit_ignores.clone()),
          custom_ignore_filenames: Arc::new(self.custom_ignore_filenames.clone()),
          custom_ignore_matcher: Gitignore::empty(),
          ignore_matcher: Gitignore::empty(),
          git_global_matcher: Arc::new(git_global_matcher),
          git_ignore_matcher: Gitignore::empty(),
          git_exclude_matcher: Gitignore::empty(),
          has_git: false,
          opts: self.opts,
        }),
        absolute_base: None,
      }
    }

    /// Set the current directory used for matching global gitignores.
    pub(crate) fn current_dir(&mut self, cwd: impl Into<PathBuf>) -> &mut IgnoreBuilder {
      self.global_gitignores_relative_to = Some(cwd.into());
      self
    }

    /// Add an override matcher.
    ///
    /// By default, no override matcher is used.
    ///
    /// This overrides any previous setting.
    pub(crate) fn overrides(&mut self, overrides: Override) -> &mut IgnoreBuilder {
      self.overrides = Arc::new(overrides);
      self
    }

    /// Add a file type matcher.
    ///
    /// By default, no file type matcher is used.
    ///
    /// This overrides any previous setting.
    pub(crate) fn types(&mut self, types: Types) -> &mut IgnoreBuilder {
      self.types = Arc::new(types);
      self
    }

    /// Adds a new global ignore matcher from the ignore file path given.
    pub(crate) fn add_ignore(&mut self, ig: Gitignore) -> &mut IgnoreBuilder {
      self.explicit_ignores.push(ig);
      self
    }

    /// Add a custom ignore file name
    ///
    /// These ignore files have higher precedence than all other ignore files.
    ///
    /// When specifying multiple names, earlier names have lower precedence than
    /// later names.
    pub(crate) fn add_custom_ignore_filename<S: AsRef<OsStr>>(
      &mut self,
      file_name: S,
    ) -> &mut IgnoreBuilder {
      self
        .custom_ignore_filenames
        .push(file_name.as_ref().to_os_string());
      self
    }

    /// Enables ignoring hidden files.
    ///
    /// This is enabled by default.
    pub(crate) fn hidden(&mut self, yes: bool) -> &mut IgnoreBuilder {
      self.opts.hidden = yes;
      self
    }

    /// Enables reading `.ignore` files.
    ///
    /// `.ignore` files have the same semantics as `gitignore` files and are
    /// supported by search tools such as ripgrep and The Silver Searcher.
    ///
    /// This is enabled by default.
    pub(crate) fn ignore(&mut self, yes: bool) -> &mut IgnoreBuilder {
      self.opts.ignore = yes;
      self
    }

    /// Enables reading ignore files from parent directories.
    ///
    /// If this is enabled, then .gitignore files in parent directories of each
    /// file path given are respected. Otherwise, they are ignored.
    ///
    /// This is enabled by default.
    pub(crate) fn parents(&mut self, yes: bool) -> &mut IgnoreBuilder {
      self.opts.parents = yes;
      self
    }

    /// Add a global gitignore matcher.
    ///
    /// Its precedence is lower than both normal `.gitignore` files and
    /// `.git/info/exclude` files.
    ///
    /// This overwrites any previous global gitignore setting.
    ///
    /// This is enabled by default.
    pub(crate) fn git_global(&mut self, yes: bool) -> &mut IgnoreBuilder {
      self.opts.git_global = yes;
      self
    }

    /// Enables reading `.gitignore` files.
    ///
    /// `.gitignore` files have match semantics as described in the `gitignore`
    /// man page.
    ///
    /// This is enabled by default.
    pub(crate) fn git_ignore(&mut self, yes: bool) -> &mut IgnoreBuilder {
      self.opts.git_ignore = yes;
      self
    }

    /// Enables reading `.git/info/exclude` files.
    ///
    /// `.git/info/exclude` files have match semantics as described in the
    /// `gitignore` man page.
    ///
    /// This is enabled by default.
    pub(crate) fn git_exclude(&mut self, yes: bool) -> &mut IgnoreBuilder {
      self.opts.git_exclude = yes;
      self
    }

    /// Whether a git repository is required to apply git-related ignore
    /// rules (global rules, .gitignore and local exclude rules).
    ///
    /// When disabled, git-related ignore rules are applied even when searching
    /// outside a git repository.
    pub(crate) fn require_git(&mut self, yes: bool) -> &mut IgnoreBuilder {
      self.opts.require_git = yes;
      self
    }

    /// Process ignore files case insensitively
    ///
    /// This is disabled by default.
    pub(crate) fn ignore_case_insensitive(&mut self, yes: bool) -> &mut IgnoreBuilder {
      self.opts.ignore_case_insensitive = yes;
      self
    }
  }

  /// Creates a new gitignore matcher for the directory given.
  ///
  /// The matcher is meant to match files below `dir`.
  /// Ignore globs are extracted from each of the file names relative to
  /// `dir_for_ignorefile` in the order given (earlier names have lower
  /// precedence than later names).
  ///
  /// I/O errors are ignored.
  pub(crate) fn create_gitignore<T: AsRef<OsStr>>(
    dir: &Path,
    dir_for_ignorefile: &Path,
    names: &[T],
    case_insensitive: bool,
  ) -> (Gitignore, Option<Error>) {
    let mut builder = GitignoreBuilder::new(dir);
    let mut errs = PartialErrorBuilder::default();
    builder.case_insensitive(case_insensitive).unwrap();
    for name in names {
      let gipath = dir_for_ignorefile.join(name.as_ref());
      // This check is not necessary, but is added for performance. Namely,
      // a simple stat call checking for existence can often be just a bit
      // quicker than actually trying to open a file. Since the number of
      // directories without ignore files likely greatly exceeds the number
      // with ignore files, this check generally makes sense.
      //
      // However, until demonstrated otherwise, we speculatively do not do
      // this on Windows since Windows is notorious for having slow file
      // system operations. Namely, it's not clear whether this analysis
      // makes sense on Windows.
      //
      // For more details: https://github.com/BurntSushi/ripgrep/pull/1381
      if cfg!(windows) || gipath.exists() {
        errs.maybe_push_ignore_io(builder.add(gipath));
      }
    }
    let gi = match builder.build() {
      Ok(gi) => gi,
      Err(err) => {
        errs.push(err);
        GitignoreBuilder::new(dir).build().unwrap()
      }
    };
    (gi, errs.into_error_option())
  }

  fn resolve_git_commondir(
    dir: &Path,
    git_type: Option<FileType>,
  ) -> Result<PathBuf, Option<Error>> {
    let git_dir_path = || dir.join(".git");
    let git_dir = git_dir_path();
    if !git_type.is_some_and(|ft| ft.is_file()) {
      return Ok(git_dir);
    }
    let file = match File::open(git_dir) {
      Ok(file) => io::BufReader::new(file),
      Err(err) => {
        return Err(Some(Error::Io(err).with_path(git_dir_path())));
      }
    };
    let dot_git_line = match file.lines().next() {
      Some(Ok(line)) => line,
      Some(Err(err)) => {
        return Err(Some(Error::Io(err).with_path(git_dir_path())));
      }
      None => return Err(None),
    };
    if !dot_git_line.starts_with("gitdir: ") {
      return Err(None);
    }
    let real_git_dir = PathBuf::from(&dot_git_line["gitdir: ".len()..]);
    let git_commondir_file = || real_git_dir.join("commondir");
    let file = match File::open(git_commondir_file()) {
      Ok(file) => io::BufReader::new(file),
      Err(_) => return Err(None),
    };
    let commondir_line = match file.lines().next() {
      Some(Ok(line)) => line,
      Some(Err(err)) => {
        return Err(Some(Error::Io(err).with_path(git_commondir_file())));
      }
      None => return Err(None),
    };
    let commondir_abs = if commondir_line.starts_with(".") {
      real_git_dir.join(commondir_line) // relative commondir
    } else {
      PathBuf::from(commondir_line)
    };
    Ok(commondir_abs)
  }

  /// Strips `prefix` from `path` if it's a prefix, otherwise returns `path`
  /// unchanged.
  fn strip_if_is_prefix<'a, P: AsRef<Path> + ?Sized>(prefix: &'a P, path: &'a Path) -> &'a Path {
    strip_prefix(prefix, path).map_or(path, |p| p)
  }
}
pub mod gitignore {

  use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    sync::Arc,
  };

  use {
    globset::{Candidate, GlobBuilder, GlobSet, GlobSetBuilder},
    regex_automata::util::pool::Pool,
  };

  use crate::{
    ignore::Error,
    ignore::Match,
    ignore::PartialErrorBuilder,
    ignore::pathutil::{is_file_name, strip_prefix},
  };

  #[derive(Clone, Debug)]
  pub struct Glob {
    /// The file path that this glob was extracted from.
    from: Option<PathBuf>,
    /// The original glob string.
    original: String,
    /// The actual glob string used to convert to a regex.
    actual: String,
    /// Whether this is a whitelisted glob or not.
    is_whitelist: bool,
    /// Whether this glob should only match directories or not.
    is_only_dir: bool,
  }

  impl Glob {
    /// Returns the file path that defined this glob.
    pub fn from(&self) -> Option<&Path> {
      self.from.as_deref()
    }

    /// The original glob as it was defined in a gitignore file.
    pub fn original(&self) -> &str {
      &self.original
    }

    /// The actual glob that was compiled to respect gitignore
    /// semantics.
    pub fn actual(&self) -> &str {
      &self.actual
    }

    /// Whether this was a whitelisted glob or not.
    pub fn is_whitelist(&self) -> bool {
      self.is_whitelist
    }

    /// Whether this glob must match a directory or not.
    pub fn is_only_dir(&self) -> bool {
      self.is_only_dir
    }

    /// Returns true if and only if this glob has a `**/` prefix.
    fn has_doublestar_prefix(&self) -> bool {
      self.actual.starts_with("**/") || self.actual == "**"
    }
  }

  /// Gitignore is a matcher for the globs in one or more gitignore files
  /// in the same directory.
  #[derive(Clone, Debug)]
  pub struct Gitignore {
    set: GlobSet,
    root: PathBuf,
    globs: Vec<Glob>,
    num_ignores: u64,
    num_whitelists: u64,
    matches: Option<Arc<Pool<Vec<usize>>>>,
  }

  impl Gitignore {
    /// Creates a new gitignore matcher from the gitignore file path given.
    ///
    /// If it's desirable to include multiple gitignore files in a single
    /// matcher, or read gitignore globs from a different source, then
    /// use `GitignoreBuilder`.
    ///
    /// This always returns a valid matcher, even if it's empty. In particular,
    /// a Gitignore file can be partially valid, e.g., when one glob is invalid
    /// but the rest aren't.
    ///
    /// Note that I/O errors are ignored. For more granular control over
    /// errors, use `GitignoreBuilder`.
    pub fn new<P: AsRef<Path>>(gitignore_path: P) -> (Gitignore, Option<Error>) {
      let path = gitignore_path.as_ref();
      let parent = path.parent().unwrap_or(Path::new("/"));
      let mut builder = GitignoreBuilder::new(parent);
      let mut errs = PartialErrorBuilder::default();
      errs.maybe_push_ignore_io(builder.add(path));
      match builder.build() {
        Ok(gi) => (gi, errs.into_error_option()),
        Err(err) => {
          errs.push(err);
          (Gitignore::empty(), errs.into_error_option())
        }
      }
    }

    /// Creates a new gitignore matcher from the global ignore file, if one
    /// exists.
    ///
    /// The global config file path is specified by git's `core.excludesFile`
    /// config option.
    ///
    /// # Behavior
    ///
    /// This routine does its best to discover any global git exclude files.
    /// This will try to parse out the `excludesFile` config option in your
    /// global git configuration, if necessary.
    ///
    /// The specific things this routine tries (which are subject to change
    /// based on how git behaves) are:
    ///
    ///
    ///
    /// Git's config file location is `$HOME/.gitconfig`. If `$HOME/.gitconfig`
    /// does not exist or does not specify `core.excludesFile`, then
    /// `$XDG_CONFIG_HOME/git/ignore` is read. If `$XDG_CONFIG_HOME` is not
    /// set or is empty, then `$HOME/.config/git/ignore` is used instead.
    pub fn global() -> (Gitignore, Option<Error>) {
      match std::env::current_dir() {
        Ok(cwd) => GitignoreBuilder::new(cwd).build_global(),
        Err(err) => (Gitignore::empty(), Some(err.into())),
      }
    }

    /// Creates a new empty gitignore matcher that never matches anything.
    ///
    /// Its path is empty.
    pub fn empty() -> Gitignore {
      Gitignore {
        set: GlobSet::empty(),
        root: PathBuf::from(""),
        globs: vec![],
        num_ignores: 0,
        num_whitelists: 0,
        matches: None,
      }
    }

    /// Returns the directory containing this gitignore matcher.
    ///
    /// All matches are done relative to this path.
    pub fn path(&self) -> &Path {
      &self.root
    }

    /// Returns true if and only if this gitignore has zero globs, and
    /// therefore never matches any file path.
    pub fn is_empty(&self) -> bool {
      self.set.is_empty()
    }

    /// Returns the total number of globs, which should be equivalent to
    /// `num_ignores + num_whitelists`.
    pub fn len(&self) -> usize {
      self.set.len()
    }

    /// Returns the total number of ignore globs.
    pub fn num_ignores(&self) -> u64 {
      self.num_ignores
    }

    /// Returns the total number of whitelisted globs.
    pub fn num_whitelists(&self) -> u64 {
      self.num_whitelists
    }

    /// Returns whether the given path (file or directory) matched a pattern in
    /// this gitignore matcher.
    ///
    /// `is_dir` should be true if the path refers to a directory and false
    /// otherwise.
    ///
    /// The given path is matched relative to the path given when building
    /// the matcher. Specifically, before matching `path`, its prefix (as
    /// determined by a common suffix of the directory containing this
    /// gitignore) is stripped. If there is no common suffix/prefix overlap,
    /// then `path` is assumed to be relative to this matcher.
    pub fn matched<P: AsRef<Path>>(&self, path: P, is_dir: bool) -> Match<&Glob> {
      if self.is_empty() {
        return Match::None;
      }
      self.matched_stripped(self.strip(path.as_ref()), is_dir)
    }

    /// Returns whether the given path (file or directory, and expected to be
    /// under the root) or any of its parent directories (up to the root)
    /// matched a pattern in this gitignore matcher.
    ///
    /// NOTE: This method is more expensive than walking the directory hierarchy
    /// top-to-bottom and matching the entries. But, is easier to use in cases
    /// when a list of paths are available without a hierarchy.
    ///
    /// `is_dir` should be true if the path refers to a directory and false
    /// otherwise.
    ///
    /// The given path is matched relative to the path given when building
    /// the matcher. Specifically, before matching `path`, its prefix (as
    /// determined by a common suffix of the directory containing this
    /// gitignore) is stripped. If there is no common suffix/prefix overlap,
    /// then `path` is assumed to be relative to this matcher.
    ///
    /// # Panics
    ///
    /// This method panics if the given file path is not under the root path
    /// of this matcher.
    pub fn matched_path_or_any_parents<P: AsRef<Path>>(
      &self,
      path: P,
      is_dir: bool,
    ) -> Match<&Glob> {
      if self.is_empty() {
        return Match::None;
      }
      let mut path = self.strip(path.as_ref());
      assert!(!path.has_root(), "path is expected to be under the root");

      match self.matched_stripped(path, is_dir) {
        Match::None => (), // walk up
        a_match => return a_match,
      }
      while let Some(parent) = path.parent() {
        match self.matched_stripped(parent, /* is_dir */ true) {
          Match::None => path = parent, // walk up
          a_match => return a_match,
        }
      }
      Match::None
    }

    /// Like matched, but takes a path that has already been stripped.
    fn matched_stripped<P: AsRef<Path>>(&self, path: P, is_dir: bool) -> Match<&Glob> {
      if self.is_empty() {
        return Match::None;
      }
      let path = path.as_ref();
      let mut matches = self.matches.as_ref().unwrap().get();
      let candidate = Candidate::new(path);
      self.set.matches_candidate_into(&candidate, &mut matches);
      for &i in matches.iter().rev() {
        let glob = &self.globs[i];
        if !glob.is_only_dir() || is_dir {
          return if glob.is_whitelist() {
            Match::Whitelist(glob)
          } else {
            Match::Ignore(glob)
          };
        }
      }
      Match::None
    }

    /// Strips the given path such that it's suitable for matching with this
    /// gitignore matcher.
    fn strip<'a, P: 'a + AsRef<Path> + ?Sized>(&'a self, path: &'a P) -> &'a Path {
      let mut path = path.as_ref();
      // A leading ./ is completely superfluous. We also strip it from
      // our gitignore root path, so we need to strip it from our candidate
      // path too.
      if let Some(p) = strip_prefix("./", path) {
        path = p;
      }
      // Strip any common prefix between the candidate path and the root
      // of the gitignore, to make sure we get relative matching right.
      // BUT, a file name might not have any directory components to it,
      // in which case, we don't want to accidentally strip any part of the
      // file name.
      //
      // As an additional special case, if the root is just `.`, then we
      // shouldn't try to strip anything, e.g., when path begins with a `.`.
      if self.root != Path::new(".")
        && !is_file_name(path)
        && let Some(p) = strip_prefix(&self.root, path)
      {
        path = p;
        // If we're left with a leading slash, get rid of it.
        if let Some(p) = strip_prefix("/", path) {
          path = p;
        }
      }
      path
    }
  }

  /// Builds a matcher for a single set of globs from a .gitignore file.
  #[derive(Clone, Debug)]
  pub struct GitignoreBuilder {
    builder: GlobSetBuilder,
    root: PathBuf,
    globs: Vec<Glob>,
    case_insensitive: bool,
    allow_unclosed_class: bool,
  }

  impl GitignoreBuilder {
    /// Create a new builder for a gitignore file.
    ///
    /// The path given should be the path at which the globs for this gitignore
    /// file should be matched. Note that paths are always matched relative
    /// to the root path given here. Generally, the root path should correspond
    /// to the *directory* containing a `.gitignore` file.
    pub fn new<P: AsRef<Path>>(root: P) -> GitignoreBuilder {
      let root = root.as_ref();
      GitignoreBuilder {
        builder: GlobSetBuilder::new(),
        root: strip_prefix("./", root).unwrap_or(root).to_path_buf(),
        globs: vec![],
        case_insensitive: false,
        allow_unclosed_class: true,
      }
    }

    /// Builds a new matcher from the globs added so far.
    ///
    /// Once a matcher is built, no new globs can be added to it.
    pub fn build(&self) -> Result<Gitignore, Error> {
      let nignore = self.globs.iter().filter(|g| !g.is_whitelist()).count();
      let nwhite = self.globs.iter().filter(|g| g.is_whitelist()).count();
      let set = self.builder.build().map_err(|err| Error::Glob {
        glob: None,
        err: err.to_string(),
      })?;
      Ok(Gitignore {
        set,
        root: self.root.clone(),
        globs: self.globs.clone(),
        num_ignores: nignore as u64,
        num_whitelists: nwhite as u64,
        matches: Some(Arc::new(Pool::new(std::vec::Vec::new))),
      })
    }

    /// Build a global gitignore matcher using the configuration in this
    /// builder.
    ///
    /// This consumes ownership of the builder unlike `build` because it
    /// must mutate the builder to add the global gitignore globs.
    ///
    /// Note that this ignores the path given to this builder's constructor
    /// and instead derives the path automatically from git's global
    /// configuration.
    pub fn build_global(mut self) -> (Gitignore, Option<Error>) {
      match gitconfig_excludes_path() {
        None => (Gitignore::empty(), None),
        Some(path) => {
          if !path.is_file() {
            (Gitignore::empty(), None)
          } else {
            let mut errs = PartialErrorBuilder::default();
            errs.maybe_push_ignore_io(self.add(path));
            match self.build() {
              Ok(gi) => (gi, errs.into_error_option()),
              Err(err) => {
                errs.push(err);
                (Gitignore::empty(), errs.into_error_option())
              }
            }
          }
        }
      }
    }

    /// Add each glob from the file path given.
    ///
    /// The file given should be formatted as a `gitignore` file.
    ///
    /// Note that partial errors can be returned. For example, if there was
    /// a problem adding one glob, an error for that will be returned, but
    /// all other valid globs will still be added.
    pub fn add<P: AsRef<Path>>(&mut self, path: P) -> Option<Error> {
      let path = path.as_ref();
      let file = match File::open(path) {
        Err(err) => return Some(Error::Io(err).with_path(path)),
        Ok(file) => file,
      };
      log::debug!("opened gitignore file: {}", path.display());
      let rdr = BufReader::new(file);
      let mut errs = PartialErrorBuilder::default();
      for (i, line) in rdr.lines().enumerate() {
        let lineno = (i + 1) as u64;
        let line = match line {
          Ok(line) => line,
          Err(err) => {
            errs.push(Error::Io(err).tagged(path, lineno));
            break;
          }
        };

        // Match Git's handling of .gitignore files that begin with the Unicode BOM
        const UTF8_BOM: &str = "\u{feff}";
        let line = if i == 0 {
          line.trim_start_matches(UTF8_BOM)
        } else {
          &line
        };

        if let Err(err) = self.add_line(Some(path.to_path_buf()), line) {
          errs.push(err.tagged(path, lineno));
        }
      }
      errs.into_error_option()
    }

    /// Add a line from a gitignore file to this builder.
    ///
    /// If this line came from a particular `gitignore` file, then its path
    /// should be provided here.
    ///
    /// If the line could not be parsed as a glob, then an error is returned.
    pub fn add_line(
      &mut self,
      from: Option<PathBuf>,
      mut line: &str,
    ) -> Result<&mut GitignoreBuilder, Error> {
      #![allow(deprecated)]

      if line.starts_with("#") {
        return Ok(self);
      }
      if !line.ends_with("\\ ") {
        line = line.trim_right();
      }
      if line.is_empty() {
        return Ok(self);
      }
      let mut glob = Glob {
        from,
        original: line.to_string(),
        actual: String::new(),
        is_whitelist: false,
        is_only_dir: false,
      };
      let mut is_absolute = false;
      if line.starts_with("\\!") || line.starts_with("\\#") {
        line = &line[1..];
        is_absolute = line.chars().nth(0) == Some('/');
      } else {
        if line.starts_with("!") {
          glob.is_whitelist = true;
          line = &line[1..];
        }
        if line.starts_with("/") {
          // `man gitignore` says that if a glob starts with a slash,
          // then the glob can only match the beginning of a path
          // (relative to the location of gitignore). We achieve this by
          // simply banning wildcards from matching /.
          line = &line[1..];
          is_absolute = true;
        }
      }
      // If it ends with a slash, then this should only match directories,
      // but the slash should otherwise not be used while globbing.
      if line.as_bytes().last() == Some(&b'/') {
        glob.is_only_dir = true;
        line = &line[..line.len() - 1];
        // If the slash was escaped, then remove the escape.
        // See: https://github.com/BurntSushi/ripgrep/issues/2236
        if line.as_bytes().last() == Some(&b'\\') {
          line = &line[..line.len() - 1];
        }
      }
      glob.actual = line.to_string();
      // If there is a literal slash, then this is a glob that must match the
      // entire path name. Otherwise, we should let it match anywhere, so use
      // a **/ prefix.
      if !is_absolute && !line.chars().any(|c| c == '/') {
        // ... but only if we don't already have a **/ prefix.
        if !glob.has_doublestar_prefix() {
          glob.actual = format!("**/{}", glob.actual);
        }
      }
      // If the glob ends with `/**`, then we should only match everything
      // inside a directory, but not the directory itself. Standard globs
      // will match the directory. So we add `/*` to force the issue.
      if glob.actual.ends_with("/**") {
        glob.actual = format!("{}/*", glob.actual);
      }
      let parsed = GlobBuilder::new(&glob.actual)
        .literal_separator(true)
        .case_insensitive(self.case_insensitive)
        .backslash_escape(true)
        .allow_unclosed_class(self.allow_unclosed_class)
        .build()
        .map_err(|err| Error::Glob {
          glob: Some(glob.original.clone()),
          err: err.kind().to_string(),
        })?;
      self.builder.add(parsed);
      self.globs.push(glob);
      Ok(self)
    }

    /// Toggle whether the globs should be matched case insensitively or not.
    ///
    /// When this option is changed, only globs added after the change will be
    /// affected.
    ///
    /// This is disabled by default.
    pub fn case_insensitive(&mut self, yes: bool) -> Result<&mut GitignoreBuilder, Error> {
      // TODO: This should not return a `Result`. Fix this in the next semver
      // release.
      self.case_insensitive = yes;
      Ok(self)
    }

    /// Toggle whether unclosed character classes are allowed. When allowed,
    /// a `[` without a matching `]` is treated literally instead of resulting
    /// in a parse error.
    ///
    /// For example, if this is set then the glob `[abc` will be treated as the
    /// literal string `[abc` instead of returning an error.
    ///
    /// By default, this is true in order to match established `gitignore`
    /// semantics. Generally speaking, enabling this leads to worse failure
    /// modes since the glob parser becomes more permissive. You might want to
    /// enable this when compatibility (e.g., with POSIX glob implementations)
    /// is more important than good error messages.
    pub fn allow_unclosed_class(&mut self, yes: bool) -> &mut GitignoreBuilder {
      self.allow_unclosed_class = yes;
      self
    }
  }

  /// Return the file path of the current environment's global gitignore file.
  ///
  /// Note that the file path returned may not exist.
  pub fn gitconfig_excludes_path() -> Option<PathBuf> {
    // When GIT_CONFIG_GLOBAL is set, it replaces both $HOME/.gitconfig and
    // $XDG_CONFIG_HOME/git/config (per git 2.32+). Otherwise, git supports
    // $HOME/.gitconfig and $XDG_CONFIG_HOME/git/config simultaneously, where
    // $HOME/.gitconfig takes precedent.
    gitconfig_global_env_contents()
      .and_then(|x| parse_excludes_file(&x))
      .or_else(|| gitconfig_home_contents().and_then(|x| parse_excludes_file(&x)))
      .or_else(|| gitconfig_xdg_contents().and_then(|x| parse_excludes_file(&x)))
      // System-level config has the lowest priority for core.excludesFile.
      // GIT_CONFIG_SYSTEM overrides the default /etc/gitconfig path.
      .or_else(|| gitconfig_system_contents().and_then(|x| parse_excludes_file(&x)))
      .or_else(excludes_file_default)
  }

  /// Returns the file contents of git's global config file from the path
  /// specified by the `GIT_CONFIG_GLOBAL` environment variable.
  fn gitconfig_global_env_contents() -> Option<Vec<u8>> {
    let path = std::env::var_os("GIT_CONFIG_GLOBAL").map(PathBuf::from)?;
    if path.as_os_str().is_empty() {
      return None;
    }
    let mut file = BufReader::new(File::open(path).ok()?);
    let mut contents = vec![];
    file.read_to_end(&mut contents).ok().map(|_| contents)
  }

  /// Returns the file contents of git's system-level config file.
  ///
  /// Checks `GIT_CONFIG_SYSTEM` first, then falls back to `/etc/gitconfig`.
  fn gitconfig_system_contents() -> Option<Vec<u8>> {
    let path = std::env::var_os("GIT_CONFIG_SYSTEM")
      .map(PathBuf::from)
      .filter(|x| !x.as_os_str().is_empty())
      .unwrap_or_else(|| PathBuf::from("/etc/gitconfig"));
    let mut file = BufReader::new(File::open(path).ok()?);
    let mut contents = vec![];
    file.read_to_end(&mut contents).ok().map(|_| contents)
  }

  /// Returns the file contents of git's global config file, if one exists, in
  /// the user's home directory.
  fn gitconfig_home_contents() -> Option<Vec<u8>> {
    let home = home_dir()?;
    let mut file = BufReader::new(File::open(home.join(".gitconfig")).ok()?);
    let mut contents = vec![];
    file.read_to_end(&mut contents).ok().map(|_| contents)
  }

  /// Returns the file contents of git's global config file, if one exists, in
  /// the user's XDG_CONFIG_HOME directory.
  fn gitconfig_xdg_contents() -> Option<Vec<u8>> {
    let path = std::env::var_os("XDG_CONFIG_HOME")
      .map(PathBuf::from)
      .filter(|x| !x.as_os_str().is_empty())
      .or_else(|| home_dir().map(|p| p.join(".config")))
      .map(|x| x.join("git/config"))?;
    let mut file = BufReader::new(File::open(path).ok()?);
    let mut contents = vec![];
    file.read_to_end(&mut contents).ok().map(|_| contents)
  }

  /// Returns the default file path for a global .gitignore file.
  ///
  /// Specifically, this respects XDG_CONFIG_HOME.
  fn excludes_file_default() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
      .map(PathBuf::from)
      .filter(|x| !x.as_os_str().is_empty())
      .or_else(|| home_dir().map(|p| p.join(".config")))
      .map(|x| x.join("git/ignore"))
  }

  /// Extract git's `core.excludesfile` config setting from the raw file contents
  /// given.
  fn parse_excludes_file(data: &[u8]) -> Option<PathBuf> {
    use std::sync::OnceLock;

    use regex_automata::{meta::Regex, util::syntax};

    // N.B. This is the lazy approach, and isn't technically correct, but
    // probably works in more circumstances. I guess we would ideally have
    // a full INI parser. Yuck.
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
      Regex::builder()
        .configure(Regex::config().utf8_empty(false))
        .syntax(syntax::Config::new().utf8(false))
        .build(r#"(?im-u)^\s*excludesfile\s*=\s*"?\s*(\S+?)\s*"?\s*$"#)
        .unwrap()
    });
    // We don't care about amortizing allocs here I think. This should only
    // be called ~once per traversal or so? (Although it's not guaranteed...)
    let mut caps = re.create_captures();
    re.captures(data, &mut caps);
    let span = caps.get_group(1)?;
    let candidate = &data[span];
    std::str::from_utf8(candidate)
      .ok()
      .map(|s| PathBuf::from(expand_tilde(s)))
  }

  /// Expands ~ in file paths to the value of $HOME.
  fn expand_tilde(path: &str) -> String {
    let home = match home_dir() {
      None => return path.to_string(),
      Some(home) => home.to_string_lossy().into_owned(),
    };
    path.replace("~", &home)
  }

  /// Returns the location of the user's home directory.
  fn home_dir() -> Option<PathBuf> {
    // We're fine with using std::env::home_dir for now. Its bugs are, IMO,
    // pretty minor corner cases.
    #![allow(deprecated)]
    std::env::home_dir()
  }
}
pub mod overrides {

  use std::path::Path;

  use crate::{
    ignore::Error,
    ignore::Match,
    ignore::gitignore::{self, Gitignore, GitignoreBuilder},
  };

  #[derive(Clone, Debug)]
  #[allow(dead_code)]
  pub struct Glob<'a>(GlobInner<'a>);

  #[derive(Clone, Debug)]
  #[allow(dead_code)]
  enum GlobInner<'a> {
    /// No glob matched, but the file path should still be ignored.
    UnmatchedIgnore,
    /// A glob matched.
    Matched(&'a gitignore::Glob),
  }

  impl<'a> Glob<'a> {
    fn unmatched() -> Glob<'a> {
      Glob(GlobInner::UnmatchedIgnore)
    }
  }

  /// Manages a set of overrides provided explicitly by the end user.
  #[derive(Clone, Debug)]
  pub struct Override(Gitignore);

  impl Override {
    /// Returns an empty matcher that never matches any file path.
    pub fn empty() -> Override {
      Override(Gitignore::empty())
    }

    /// Returns the directory of this override set.
    ///
    /// All matches are done relative to this path.
    pub fn path(&self) -> &Path {
      self.0.path()
    }

    /// Returns true if and only if this matcher is empty.
    ///
    /// When a matcher is empty, it will never match any file path.
    pub fn is_empty(&self) -> bool {
      self.0.is_empty()
    }

    /// Returns the total number of ignore globs.
    pub fn num_ignores(&self) -> u64 {
      self.0.num_whitelists()
    }

    /// Returns the total number of whitelisted globs.
    pub fn num_whitelists(&self) -> u64 {
      self.0.num_ignores()
    }

    /// Returns whether the given file path matched a pattern in this override
    /// matcher.
    ///
    /// `is_dir` should be true if the path refers to a directory and false
    /// otherwise.
    ///
    /// If there are no overrides, then this always returns `Match::None`.
    ///
    /// If there is at least one whitelist override and `is_dir` is false, then
    /// this never returns `Match::None`, since non-matches are interpreted as
    /// ignored.
    ///
    /// The given path is matched to the globs relative to the path given
    /// when building the override matcher. Specifically, before matching
    /// `path`, its prefix (as determined by a common suffix of the directory
    /// given) is stripped. If there is no common suffix/prefix overlap, then
    /// `path` is assumed to reside in the same directory as the root path for
    /// this set of overrides.
    pub fn matched<'a, P: AsRef<Path>>(&'a self, path: P, is_dir: bool) -> Match<Glob<'a>> {
      if self.is_empty() {
        return Match::None;
      }
      let mat = self.0.matched(path, is_dir).invert();
      if mat.is_none() && self.num_whitelists() > 0 && !is_dir {
        return Match::Ignore(Glob::unmatched());
      }
      mat.map(move |giglob| Glob(GlobInner::Matched(giglob)))
    }
  }

  /// Builds a matcher for a set of glob overrides.
  #[derive(Clone, Debug)]
  pub struct OverrideBuilder {
    builder: GitignoreBuilder,
  }

  impl OverrideBuilder {
    /// Create a new override builder.
    ///
    /// Matching is done relative to the directory path provided.
    pub fn new<P: AsRef<Path>>(path: P) -> OverrideBuilder {
      let mut builder = GitignoreBuilder::new(path);
      builder.allow_unclosed_class(false);
      OverrideBuilder { builder }
    }

    /// Builds a new override matcher from the globs added so far.
    ///
    /// Once a matcher is built, no new globs can be added to it.
    pub fn build(&self) -> Result<Override, Error> {
      Ok(Override(self.builder.build()?))
    }

    /// Add a glob to the set of overrides.
    ///
    /// Globs provided here have precisely the same semantics as a single
    /// line in a `gitignore` file, where the meaning of `!` is inverted:
    /// namely, `!` at the beginning of a glob will ignore a file. Without `!`,
    /// all matches of the glob provided are treated as whitelist matches.
    pub fn add(&mut self, glob: &str) -> Result<&mut OverrideBuilder, Error> {
      self.builder.add_line(None, glob)?;
      Ok(self)
    }

    /// Toggle whether the globs should be matched case insensitively or not.
    ///
    /// When this option is changed, only globs added after the change will be
    /// affected.
    ///
    /// This is disabled by default.
    pub fn case_insensitive(&mut self, yes: bool) -> Result<&mut OverrideBuilder, Error> {
      // TODO: This should not return a `Result`. Fix this in the next semver
      // release.
      self.builder.case_insensitive(yes)?;
      Ok(self)
    }

    pub fn allow_unclosed_class(&mut self, yes: bool) -> &mut OverrideBuilder {
      self.builder.allow_unclosed_class(yes);
      self
    }
  }
}
mod pathutil {
  use std::{ffi::OsStr, path::Path};

  use crate::ignore::walk::DirEntry;

  pub(crate) fn is_hidden(dent: &DirEntry) -> bool {
    use std::os::unix::ffi::OsStrExt;

    if let Some(name) = file_name(dent.path()) {
      name.as_bytes().first() == Some(&b'.')
    } else {
      false
    }
  }

  pub(crate) fn strip_prefix<'a, P: AsRef<Path> + ?Sized>(
    prefix: &'a P,
    path: &'a Path,
  ) -> Option<&'a Path> {
    use std::os::unix::ffi::OsStrExt;

    let prefix = prefix.as_ref().as_os_str().as_bytes();
    let path = path.as_os_str().as_bytes();
    if prefix.len() > path.len() || prefix != &path[0..prefix.len()] {
      None
    } else {
      Some(Path::new(OsStr::from_bytes(&path[prefix.len()..])))
    }
  }

  pub(crate) fn is_file_name<P: AsRef<Path>>(path: P) -> bool {
    use std::os::unix::ffi::OsStrExt;

    use memchr::memchr;

    let path = path.as_ref().as_os_str().as_bytes();
    memchr(b'/', path).is_none()
  }

  pub(crate) fn file_name<P: AsRef<Path> + ?Sized>(path: &P) -> Option<&OsStr> {
    use memchr::memrchr;
    use std::os::unix::ffi::OsStrExt;

    let path = path.as_ref().as_os_str().as_bytes();
    if path.is_empty()
      || (path.len() == 1 && path[0] == b'.')
      || path.last() == Some(&b'.')
      || (path.len() >= 2 && path[path.len() - 2..] == b".."[..])
    {
      return None;
    }
    let last_slash = memrchr(b'/', path).map(|i| i + 1).unwrap_or(0);
    Some(OsStr::from_bytes(&path[last_slash..]))
  }
}
pub mod types {

  use std::{collections::HashMap, path::Path, sync::Arc};

  use {
    globset::{GlobBuilder, GlobSet, GlobSetBuilder},
    regex_automata::util::pool::Pool,
  };

  use crate::{
    ignore::Error, ignore::Match, ignore::default_types::DEFAULT_TYPES, ignore::pathutil::file_name,
  };

  #[derive(Clone, Debug)]
  pub struct Glob<'a>(GlobInner<'a>);

  #[derive(Clone, Debug)]
  enum GlobInner<'a> {
    /// No glob matched, but the file path should still be ignored.
    UnmatchedIgnore,
    /// A glob matched.
    Matched {
      /// The file type definition which provided the glob.
      def: &'a FileTypeDef,
    },
  }

  impl<'a> Glob<'a> {
    fn unmatched() -> Glob<'a> {
      Glob(GlobInner::UnmatchedIgnore)
    }

    /// Return the file type definition that matched, if one exists. A file type
    /// definition always exists when a specific definition matches a file
    /// path.
    pub fn file_type_def(&self) -> Option<&FileTypeDef> {
      match self {
        Glob(GlobInner::UnmatchedIgnore) => None,
        Glob(GlobInner::Matched { def, .. }) => Some(def),
      }
    }
  }

  #[derive(Clone, Debug, Eq, PartialEq)]
  pub struct FileTypeDef {
    name: String,
    globs: Vec<String>,
  }

  impl FileTypeDef {
    /// Return the name of this file type.
    pub fn name(&self) -> &str {
      &self.name
    }

    /// Return the globs used to recognize this file type.
    pub fn globs(&self) -> &[String] {
      &self.globs
    }
  }

  /// Types is a file type matcher.
  #[derive(Clone, Debug)]
  pub struct Types {
    /// All of the file type definitions, sorted lexicographically by name.
    defs: Vec<FileTypeDef>,
    /// All of the selections made by the user.
    selections: Vec<Selection<FileTypeDef>>,
    /// Whether there is at least one Selection::Select in our selections.
    /// When this is true, a Match::None is converted to Match::Ignore.
    has_selected: bool,
    /// A mapping from glob index in the set to two indices. The first is an
    /// index into `selections` and the second is an index into the
    /// corresponding file type definition's list of globs.
    glob_to_selection: Vec<(usize, usize)>,
    /// The set of all glob selections, used for actual matching.
    set: GlobSet,
    /// Temporary storage for globs that match.
    matches: Arc<Pool<Vec<usize>>>,
  }

  /// Indicates the type of a selection for a particular file type.
  #[derive(Clone, Debug)]
  enum Selection<T> {
    Select(String, T),
    Negate(String, T),
  }

  impl<T> Selection<T> {
    fn is_negated(&self) -> bool {
      match *self {
        Selection::Select(..) => false,
        Selection::Negate(..) => true,
      }
    }

    fn name(&self) -> &str {
      match *self {
        Selection::Select(ref name, _) => name,
        Selection::Negate(ref name, _) => name,
      }
    }

    fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Selection<U> {
      match self {
        Selection::Select(name, inner) => Selection::Select(name, f(inner)),
        Selection::Negate(name, inner) => Selection::Negate(name, f(inner)),
      }
    }

    fn inner(&self) -> &T {
      match *self {
        Selection::Select(_, ref inner) => inner,
        Selection::Negate(_, ref inner) => inner,
      }
    }
  }

  impl Types {
    /// Creates a new file type matcher that never matches any path and
    /// contains no file type definitions.
    pub fn empty() -> Types {
      Types {
        defs: vec![],
        selections: vec![],
        has_selected: false,
        glob_to_selection: vec![],
        set: GlobSetBuilder::new().build().unwrap(),
        matches: Arc::new(Pool::new(std::vec::Vec::new)),
      }
    }

    /// Returns true if and only if this matcher has zero selections.
    pub fn is_empty(&self) -> bool {
      self.selections.is_empty()
    }

    /// Returns the number of selections used in this matcher.
    pub fn len(&self) -> usize {
      self.selections.len()
    }

    /// Return the set of current file type definitions.
    ///
    /// Definitions and globs are sorted.
    pub fn definitions(&self) -> &[FileTypeDef] {
      &self.defs
    }

    pub fn matched<'a, P: AsRef<Path>>(&'a self, path: P, is_dir: bool) -> Match<Glob<'a>> {
      // File types don't apply to directories, and we can't do anything
      // if our glob set is empty.
      if is_dir || self.set.is_empty() {
        return Match::None;
      }
      // We only want to match against the file name, so extract it.
      // If one doesn't exist, then we can't match it.
      let name = match file_name(path.as_ref()) {
        Some(name) => name,
        None if self.has_selected => {
          return Match::Ignore(Glob::unmatched());
        }
        None => {
          return Match::None;
        }
      };
      let mut matches = self.matches.get();
      self.set.matches_into(name, &mut matches);
      // The highest precedent match is the last one.
      if let Some(&i) = matches.last() {
        let (isel, _) = self.glob_to_selection[i];
        let sel = &self.selections[isel];
        let glob = Glob(GlobInner::Matched { def: sel.inner() });
        return if sel.is_negated() {
          Match::Ignore(glob)
        } else {
          Match::Whitelist(glob)
        };
      }
      if self.has_selected {
        Match::Ignore(Glob::unmatched())
      } else {
        Match::None
      }
    }
  }

  /// TypesBuilder builds a type matcher from a set of file type definitions and
  /// a set of file type selections.
  pub struct TypesBuilder {
    types: HashMap<String, FileTypeDef>,
    selections: Vec<Selection<()>>,
  }

  impl TypesBuilder {
    pub fn new() -> TypesBuilder {
      TypesBuilder {
        types: HashMap::new(),
        selections: vec![],
      }
    }

    /// Build the current set of file type definitions *and* selections into
    /// a file type matcher.
    pub fn build(&self) -> Result<Types, Error> {
      let defs = self.definitions();
      let has_selected = self.selections.iter().any(|s| !s.is_negated());

      let mut selections = vec![];
      let mut glob_to_selection = vec![];
      let mut build_set = GlobSetBuilder::new();
      for (isel, selection) in self.selections.iter().enumerate() {
        let def = match self.types.get(selection.name()) {
          Some(def) => def.clone(),
          None => {
            let name = selection.name().to_string();
            return Err(Error::UnrecognizedFileType(name));
          }
        };
        for (iglob, glob) in def.globs.iter().enumerate() {
          build_set.add(
            GlobBuilder::new(glob)
              .literal_separator(true)
              .build()
              .map_err(|err| Error::Glob {
                glob: Some(glob.to_string()),
                err: err.kind().to_string(),
              })?,
          );
          glob_to_selection.push((isel, iglob));
        }
        selections.push(selection.clone().map(move |_| def));
      }
      let set = build_set.build().map_err(|err| Error::Glob {
        glob: None,
        err: err.to_string(),
      })?;
      Ok(Types {
        defs,
        selections,
        has_selected,
        glob_to_selection,
        set,
        matches: Arc::new(Pool::new(std::vec::Vec::new)),
      })
    }

    /// Return the set of current file type definitions.
    ///
    /// Definitions and globs are sorted.
    pub fn definitions(&self) -> Vec<FileTypeDef> {
      let mut defs = vec![];
      for def in self.types.values() {
        let mut def = def.clone();
        def.globs.sort();
        defs.push(def);
      }
      defs.sort_by(|def1, def2| def1.name().cmp(def2.name()));
      defs
    }

    /// Select the file type given by `name`.
    ///
    /// If `name` is `all`, then all file types currently defined are selected.
    pub fn select(&mut self, name: &str) -> &mut TypesBuilder {
      if name == "all" {
        for name in self.types.keys() {
          self
            .selections
            .push(Selection::Select(name.to_string(), ()));
        }
      } else {
        self
          .selections
          .push(Selection::Select(name.to_string(), ()));
      }
      self
    }

    /// Ignore the file type given by `name`.
    ///
    /// If `name` is `all`, then all file types currently defined are negated.
    pub fn negate(&mut self, name: &str) -> &mut TypesBuilder {
      if name == "all" {
        for name in self.types.keys() {
          self
            .selections
            .push(Selection::Negate(name.to_string(), ()));
        }
      } else {
        self
          .selections
          .push(Selection::Negate(name.to_string(), ()));
      }
      self
    }

    /// Clear any file type definitions for the type name given.
    pub fn clear(&mut self, name: &str) -> &mut TypesBuilder {
      self.types.remove(name);
      self
    }

    /// Add a new file type definition. `name` can be arbitrary and `pat`
    /// should be a glob recognizing file paths belonging to the `name` type.
    ///
    /// If `name` is `all` or otherwise contains any character that is not a
    /// Unicode letter or number, then an error is returned.
    pub fn add(&mut self, name: &str, glob: &str) -> Result<(), Error> {
      if name == "all" || !name.chars().all(|c| c.is_alphanumeric()) {
        return Err(Error::InvalidDefinition);
      }
      let (key, glob) = (name.to_string(), glob.to_string());
      self
        .types
        .entry(key)
        .or_insert_with(|| FileTypeDef {
          name: name.to_string(),
          globs: vec![],
        })
        .globs
        .push(glob);
      Ok(())
    }

    /// Add a new file type definition specified in string form. There are two
    /// valid formats:
    /// 1. `{name}:{glob}`.  This defines a 'root' definition that associates the
    ///     given name with the given glob.
    /// 2. `{name}:include:{comma-separated list of already defined names}.
    ///     This defines an 'include' definition that associates the given name
    ///     with the definitions of the given existing types.
    /// Names may not include any characters that are not
    /// Unicode letters or numbers.
    pub fn add_def(&mut self, def: &str) -> Result<(), Error> {
      let parts: Vec<&str> = def.split(':').collect();
      match parts.len() {
        2 => {
          let name = parts[0];
          let glob = parts[1];
          if name.is_empty() || glob.is_empty() {
            return Err(Error::InvalidDefinition);
          }
          self.add(name, glob)
        }
        3 => {
          let name = parts[0];
          let types_string = parts[2];
          if name.is_empty() || parts[1] != "include" || types_string.is_empty() {
            return Err(Error::InvalidDefinition);
          }
          let types = types_string.split(',');
          // Check ahead of time to ensure that all types specified are
          // present and fail fast if not.
          if types.clone().any(|t| !self.types.contains_key(t)) {
            return Err(Error::InvalidDefinition);
          }
          for type_name in types {
            let globs = self.types.get(type_name).unwrap().globs.clone();
            for glob in globs {
              self.add(name, &glob)?;
            }
          }
          Ok(())
        }
        _ => Err(Error::InvalidDefinition),
      }
    }

    /// Add a set of default file type definitions.
    pub fn add_defaults(&mut self) -> &mut TypesBuilder {
      static MSG: &'static str = "adding a default type should never fail";
      for &(names, exts) in DEFAULT_TYPES {
        for name in names {
          for ext in exts {
            self.add(name, ext).expect(MSG);
          }
        }
      }
      self
    }
  }
}

mod walk {
  use std::{
    cmp::Ordering,
    ffi::OsStr,
    fs::{self, FileType, Metadata},
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering},
    sync::{Arc, OnceLock},
  };

  use {
    crate::walkdir::WalkDir,
    crossbeam_deque::{Stealer, Worker as Deque},
    same_file::Handle,
  };

  use crate::{
    ignore::Error,
    ignore::PartialErrorBuilder,
    ignore::dir::{Ignore, IgnoreBuilder},
    ignore::gitignore::GitignoreBuilder,
    ignore::overrides::Override,
    ignore::types::Types,
  };

  /// A directory entry with a possible error attached.
  ///
  /// The error typically refers to a problem parsing ignore files in a
  /// particular directory.
  #[derive(Clone, Debug)]
  pub struct DirEntry {
    dent: DirEntryInner,
    err: Option<Error>,
  }

  impl DirEntry {
    /// The full path that this entry represents.
    pub fn path(&self) -> &Path {
      self.dent.path()
    }

    /// The full path that this entry represents.
    /// Analogous to [`DirEntry::path`], but moves ownership of the path.
    pub fn into_path(self) -> PathBuf {
      self.dent.into_path()
    }

    /// Whether this entry corresponds to a symbolic link or not.
    pub fn path_is_symlink(&self) -> bool {
      self.dent.path_is_symlink()
    }

    /// Returns true if and only if this entry corresponds to stdin.
    ///
    /// i.e., The entry has depth 0 and its file name is `-`.
    pub fn is_stdin(&self) -> bool {
      self.dent.is_stdin()
    }

    /// Return the metadata for the file that this entry points to.
    pub fn metadata(&self) -> Result<Metadata, Error> {
      self.dent.metadata()
    }

    /// Return the file type for the file that this entry points to.
    ///
    /// This entry doesn't have a file type if it corresponds to stdin.
    pub fn file_type(&self) -> Option<FileType> {
      self.dent.file_type()
    }

    /// Return the file name of this entry.
    ///
    /// If this entry has no file name (e.g., `/`), then the full path is
    /// returned.
    pub fn file_name(&self) -> &OsStr {
      self.dent.file_name()
    }

    /// Returns the depth at which this entry was created relative to the root.
    pub fn depth(&self) -> usize {
      self.dent.depth()
    }

    /// Returns the underlying inode number if one exists.
    ///
    /// If this entry doesn't have an inode number, then `None` is returned.
    #[cfg(unix)]
    pub fn ino(&self) -> Option<u64> {
      self.dent.ino()
    }

    /// Returns an error, if one exists, associated with processing this entry.
    ///
    /// An example of an error is one that occurred while parsing an ignore
    /// file. Errors related to traversing a directory tree itself are reported
    /// as part of yielding the directory entry, and not with this method.
    pub fn error(&self) -> Option<&Error> {
      self.err.as_ref()
    }

    /// Returns true if and only if this entry points to a directory.
    pub(crate) fn is_dir(&self) -> bool {
      self.dent.is_dir()
    }

    fn new_stdin() -> DirEntry {
      DirEntry {
        dent: DirEntryInner::Stdin,
        err: None,
      }
    }

    fn new_walkdir(dent: crate::walkdir::DirEntry, err: Option<Error>) -> DirEntry {
      DirEntry {
        dent: DirEntryInner::Walkdir(dent),
        err,
      }
    }

    fn new_raw(dent: DirEntryRaw, err: Option<Error>) -> DirEntry {
      DirEntry {
        dent: DirEntryInner::Raw(dent),
        err,
      }
    }
  }

  /// DirEntryInner is the implementation of DirEntry.
  ///
  /// It specifically represents three distinct sources of directory entries:
  ///
  /// 1. From the walkdir crate.
  /// 2. Special entries that represent things like stdin.
  /// 3. From a path.
  ///
  /// Specifically, (3) has to essentially re-create the DirEntry implementation
  /// from WalkDir.
  #[derive(Clone, Debug)]
  enum DirEntryInner {
    Stdin,
    Walkdir(crate::walkdir::DirEntry),
    Raw(DirEntryRaw),
  }

  impl DirEntryInner {
    fn path(&self) -> &Path {
      use self::DirEntryInner::*;
      match *self {
        Stdin => Path::new("<stdin>"),
        Walkdir(ref x) => x.path(),
        Raw(ref x) => x.path(),
      }
    }

    fn into_path(self) -> PathBuf {
      use self::DirEntryInner::*;
      match self {
        Stdin => PathBuf::from("<stdin>"),
        Walkdir(x) => x.into_path(),
        Raw(x) => x.into_path(),
      }
    }

    fn path_is_symlink(&self) -> bool {
      use self::DirEntryInner::*;
      match *self {
        Stdin => false,
        Walkdir(ref x) => x.path_is_symlink(),
        Raw(ref x) => x.path_is_symlink(),
      }
    }

    fn is_stdin(&self) -> bool {
      match *self {
        DirEntryInner::Stdin => true,
        _ => false,
      }
    }

    fn metadata(&self) -> Result<Metadata, Error> {
      use self::DirEntryInner::*;
      match *self {
        Stdin => {
          let err = Error::Io(io::Error::new(
            io::ErrorKind::Other,
            "<stdin> has no metadata",
          ));
          Err(err.with_path("<stdin>"))
        }
        Walkdir(ref x) => x.metadata().map_err(|err| {
          Error::Io(io::Error::from(err))
            .with_depth(x.depth())
            .with_path(x.path())
        }),
        Raw(ref x) => x.metadata(),
      }
    }

    fn file_type(&self) -> Option<FileType> {
      use self::DirEntryInner::*;
      match *self {
        Stdin => None,
        Walkdir(ref x) => Some(x.file_type()),
        Raw(ref x) => Some(x.file_type()),
      }
    }

    fn file_name(&self) -> &OsStr {
      use self::DirEntryInner::*;
      match *self {
        Stdin => OsStr::new("<stdin>"),
        Walkdir(ref x) => x.file_name(),
        Raw(ref x) => x.file_name(),
      }
    }

    fn depth(&self) -> usize {
      use self::DirEntryInner::*;
      match *self {
        Stdin => 0,
        Walkdir(ref x) => x.depth(),
        Raw(ref x) => x.depth(),
      }
    }

    fn ino(&self) -> Option<u64> {
      use self::DirEntryInner::*;
      use crate::walkdir::DirEntryExt;
      match *self {
        Stdin => None,
        Walkdir(ref x) => Some(x.ino()),
        Raw(ref x) => Some(x.ino()),
      }
    }

    /// Returns true if and only if this entry points to a directory.
    fn is_dir(&self) -> bool {
      self.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
    }
  }

  /// DirEntryRaw is essentially copied from the walkdir crate so that we can
  /// build `DirEntry`s from whole cloth in the parallel iterator.
  #[derive(Clone)]
  struct DirEntryRaw {
    /// The path as reported by the `fs::ReadDir` iterator (even if it's a
    /// symbolic link).
    path: PathBuf,
    /// The file type. Necessary for recursive iteration, so store it.
    ty: FileType,
    /// Is set when this entry was created from a symbolic link and the user
    /// expects the iterator to follow symbolic links.
    follow_link: bool,
    /// The depth at which this entry was generated relative to the root.
    depth: usize,
    /// The underlying inode number (Unix only).
    #[cfg(unix)]
    ino: u64,
    /// The underlying metadata (Windows only). We store this on Windows
    /// because this comes for free while reading a directory.
    #[cfg(windows)]
    metadata: fs::Metadata,
  }

  impl std::fmt::Debug for DirEntryRaw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      // Leaving out FileType because it doesn't have a debug impl
      // in Rust 1.9. We could add it if we really wanted to by manually
      // querying each possibly file type. Meh. ---AG
      f.debug_struct("DirEntryRaw")
        .field("path", &self.path)
        .field("follow_link", &self.follow_link)
        .field("depth", &self.depth)
        .finish()
    }
  }

  impl DirEntryRaw {
    fn path(&self) -> &Path {
      &self.path
    }

    fn into_path(self) -> PathBuf {
      self.path
    }

    fn path_is_symlink(&self) -> bool {
      self.ty.is_symlink() || self.follow_link
    }

    fn metadata(&self) -> Result<Metadata, Error> {
      self.metadata_internal()
    }

    #[cfg(windows)]
    fn metadata_internal(&self) -> Result<fs::Metadata, Error> {
      if self.follow_link {
        fs::metadata(&self.path)
      } else {
        Ok(self.metadata.clone())
      }
      .map_err(|err| Error::Io(io::Error::from(err)).with_path(&self.path))
    }

    #[cfg(not(windows))]
    fn metadata_internal(&self) -> Result<fs::Metadata, Error> {
      if self.follow_link {
        fs::metadata(&self.path)
      } else {
        fs::symlink_metadata(&self.path)
      }
      .map_err(|err| Error::Io(err).with_depth(self.depth).with_path(&self.path))
    }

    fn file_type(&self) -> FileType {
      self.ty
    }

    fn file_name(&self) -> &OsStr {
      self
        .path
        .file_name()
        .unwrap_or_else(|| self.path.as_os_str())
    }

    fn depth(&self) -> usize {
      self.depth
    }

    #[cfg(unix)]
    fn ino(&self) -> u64 {
      self.ino
    }

    fn from_entry(depth: usize, ent: &fs::DirEntry) -> Result<DirEntryRaw, Error> {
      let ty = ent.file_type().map_err(|err| {
        let err = Error::Io(err).with_depth(depth).with_path(ent.path());
        Error::WithDepth {
          depth,
          err: Box::new(err),
        }
      })?;
      DirEntryRaw::from_entry_os(depth, ent, ty)
    }

    #[cfg(windows)]
    fn from_entry_os(
      depth: usize,
      ent: &fs::DirEntry,
      ty: fs::FileType,
    ) -> Result<DirEntryRaw, Error> {
      let md = ent.metadata().map_err(|err| {
        let err = Error::Io(err).with_depth(depth).with_path(ent.path());
        Error::WithDepth {
          depth,
          err: Box::new(err),
        }
      })?;
      Ok(DirEntryRaw {
        path: ent.path(),
        ty,
        follow_link: false,
        depth,
        metadata: md,
      })
    }

    #[cfg(unix)]
    fn from_entry_os(
      depth: usize,
      ent: &fs::DirEntry,
      ty: fs::FileType,
    ) -> Result<DirEntryRaw, Error> {
      use std::os::unix::fs::DirEntryExt;

      Ok(DirEntryRaw {
        path: ent.path(),
        ty,
        follow_link: false,
        depth,
        ino: ent.ino(),
      })
    }

    // Placeholder implementation to allow compiling on non-standard platforms
    // (e.g. wasm32).
    #[cfg(not(any(windows, unix)))]
    fn from_entry_os(
      depth: usize,
      ent: &fs::DirEntry,
      ty: fs::FileType,
    ) -> Result<DirEntryRaw, Error> {
      Err(Error::Io(io::Error::new(
        io::ErrorKind::Other,
        "unsupported platform",
      )))
    }

    #[cfg(windows)]
    fn from_path(depth: usize, pb: PathBuf, link: bool) -> Result<DirEntryRaw, Error> {
      let md = fs::metadata(&pb).map_err(|err| Error::Io(err).with_depth(depth).with_path(&pb))?;
      Ok(DirEntryRaw {
        path: pb,
        ty: md.file_type(),
        follow_link: link,
        depth,
        metadata: md,
      })
    }

    #[cfg(unix)]
    fn from_path(depth: usize, pb: PathBuf, link: bool) -> Result<DirEntryRaw, Error> {
      use std::os::unix::fs::MetadataExt;

      let md = fs::metadata(&pb).map_err(|err| Error::Io(err).with_depth(depth).with_path(&pb))?;
      Ok(DirEntryRaw {
        path: pb,
        ty: md.file_type(),
        follow_link: link,
        depth,
        ino: md.ino(),
      })
    }

    // Placeholder implementation to allow compiling on non-standard platforms
    // (e.g. wasm32).
    #[cfg(not(any(windows, unix)))]
    fn from_path(depth: usize, pb: PathBuf, link: bool) -> Result<DirEntryRaw, Error> {
      Err(Error::Io(io::Error::new(
        io::ErrorKind::Other,
        "unsupported platform",
      )))
    }
  }

  /// WalkBuilder builds a recursive directory iterator.
  ///
  /// The builder supports a large number of configurable options. This includes
  /// specific glob overrides, file type matching, toggling whether hidden
  /// files are ignored or not, and of course, support for respecting gitignore
  /// files.
  ///
  /// By default, all ignore files found are respected. This includes `.ignore`,
  /// `.gitignore`, `.git/info/exclude` and even your global gitignore
  /// globs, usually found in `$XDG_CONFIG_HOME/git/ignore`.
  ///
  /// Some standard recursive directory options are also supported, such as
  /// limiting the recursive depth or whether to follow symbolic links (disabled
  /// by default).
  ///
  /// # Ignore rules
  ///
  /// There are many rules that influence whether a particular file or directory
  /// is skipped by this iterator. Those rules are documented here. Note that
  /// the rules assume a default configuration.
  ///
  /// * First, glob overrides are checked. If a path matches a glob override,
  /// then matching stops. The path is then only skipped if the glob that matched
  /// the path is an ignore glob. (An override glob is a whitelist glob unless it
  /// starts with a `!`, in which case it is an ignore glob.)
  /// * Second, ignore files are checked. Ignore files currently only come from
  /// git ignore files (`.gitignore`, `.git/info/exclude` and the configured
  /// global gitignore file), plain `.ignore` files, which have the same format
  /// as gitignore files, or explicitly added ignore files. The precedence order
  /// is: `.ignore`, `.gitignore`, `.git/info/exclude`, global gitignore and
  /// finally explicitly added ignore files. Note that precedence between
  /// different types of ignore files is not impacted by the directory hierarchy;
  /// any `.ignore` file overrides all `.gitignore` files. Within each precedence
  /// level, more nested ignore files have a higher precedence than less nested
  /// ignore files.
  /// * Third, if the previous step yields an ignore match, then all matching
  /// is stopped and the path is skipped. If it yields a whitelist match, then
  /// matching continues. A whitelist match can be overridden by a later matcher.
  /// * Fourth, unless the path is a directory, the file type matcher is run on
  /// the path. As above, if it yields an ignore match, then all matching is
  /// stopped and the path is skipped. If it yields a whitelist match, then
  /// matching continues.
  /// * Fifth, if the path hasn't been whitelisted and it is hidden, then the
  /// path is skipped.
  /// * Sixth, unless the path is a directory, the size of the file is compared
  /// against the max filesize limit. If it exceeds the limit, it is skipped.
  /// * Seventh, if the path has made it this far then it is yielded in the
  /// iterator.
  #[derive(Clone)]
  pub struct WalkBuilder {
    paths: Vec<PathBuf>,
    ig_builder: IgnoreBuilder,
    max_depth: Option<usize>,
    min_depth: Option<usize>,
    max_filesize: Option<u64>,
    follow_links: bool,
    same_file_system: bool,
    sorter: Option<Sorter>,
    threads: usize,
    skip: Option<Arc<Handle>>,
    filter: Option<Filter>,
    /// The directory that gitignores should be interpreted relative to.
    ///
    /// Usually this is the directory containing the gitignore file. But in
    /// some cases, like for global gitignores or for gitignores specified
    /// explicitly, this should generally be set to the current working
    /// directory. This is only used for global gitignores or "explicit"
    /// gitignores.
    ///
    /// When `None`, the CWD is fetched from `std::env::current_dir()`. If
    /// that fails, then global gitignores are ignored (an error is logged).
    global_gitignores_relative_to: OnceLock<Result<PathBuf, Arc<std::io::Error>>>,
  }

  #[derive(Clone)]
  enum Sorter {
    ByName(Arc<dyn Fn(&OsStr, &OsStr) -> Ordering + Send + Sync + 'static>),
    ByPath(Arc<dyn Fn(&Path, &Path) -> Ordering + Send + Sync + 'static>),
  }

  #[derive(Clone)]
  struct Filter(Arc<dyn Fn(&DirEntry) -> bool + Send + Sync + 'static>);

  impl std::fmt::Debug for WalkBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.debug_struct("WalkBuilder")
        .field("paths", &self.paths)
        .field("ig_builder", &self.ig_builder)
        .field("max_depth", &self.max_depth)
        .field("min_depth", &self.min_depth)
        .field("max_filesize", &self.max_filesize)
        .field("follow_links", &self.follow_links)
        .field("same_file_system", &self.same_file_system)
        .field("sorter", &"<...>")
        .field("threads", &self.threads)
        .field("skip", &self.skip)
        .field("filter", &"<...>")
        .field(
          "global_gitignores_relative_to",
          &self.global_gitignores_relative_to,
        )
        .finish()
    }
  }

  impl WalkBuilder {
    /// Create a new builder for a recursive directory iterator for the
    /// directory given.
    ///
    /// Note that if you want to traverse multiple different directories, it
    /// is better to call `add` on this builder than to create multiple
    /// `Walk` values.
    pub fn new<P: AsRef<Path>>(path: P) -> WalkBuilder {
      WalkBuilder::from_iter([path])
    }

    /// Create an empty builder to which paths can be added.
    ///
    /// Note that if you call `build` on this instance before calling `add`
    /// on it, it will return exactly zero items during iteration.
    pub fn empty() -> WalkBuilder {
      WalkBuilder {
        paths: vec![],
        ig_builder: IgnoreBuilder::new(),
        max_depth: None,
        min_depth: None,
        max_filesize: None,
        follow_links: false,
        same_file_system: false,
        sorter: None,
        threads: 0,
        skip: None,
        filter: None,
        global_gitignores_relative_to: OnceLock::new(),
      }
    }

    /// Create a new builder for a recursive directory iterator from the
    /// sequence of paths.
    ///
    /// Note that if the iterator is empty, this is the same as
    /// `WalkBuilder::empty`.
    pub fn from_iter<P: AsRef<Path>, I: IntoIterator<Item = P>>(paths: I) -> WalkBuilder {
      let mut builder = WalkBuilder::empty();
      for path in paths.into_iter() {
        builder.add(path);
      }
      builder
    }

    /// Build a new `Walk` iterator.
    pub fn build(&self) -> Walk {
      let follow_links = self.follow_links;
      let max_depth = self.max_depth;
      let min_depth = self.min_depth;
      let sorter = self.sorter.clone();
      let its = self
        .paths
        .iter()
        .map(move |p| {
          if p == Path::new("-") {
            (p.to_path_buf(), None)
          } else {
            let mut wd = WalkDir::new(p);
            wd = wd.follow_links(follow_links || p.is_file());
            wd = wd.same_file_system(self.same_file_system);
            if let Some(max_depth) = max_depth {
              wd = wd.max_depth(max_depth);
            }
            if let Some(min_depth) = min_depth {
              wd = wd.min_depth(min_depth);
            }
            if let Some(ref sorter) = sorter {
              match sorter.clone() {
                Sorter::ByName(cmp) => {
                  wd = wd.sort_by(move |a, b| cmp(a.file_name(), b.file_name()));
                }
                Sorter::ByPath(cmp) => {
                  wd = wd.sort_by(move |a, b| cmp(a.path(), b.path()));
                }
              }
            }
            (p.to_path_buf(), Some(WalkEventIter::from(wd)))
          }
        })
        .collect::<Vec<_>>()
        .into_iter();
      let ig_root = self
        .get_or_set_current_dir()
        .map(|cwd| self.ig_builder.build_with_cwd(Some(cwd.to_path_buf())))
        .unwrap_or_else(|| self.ig_builder.build());
      Walk {
        its,
        it: None,
        ig_root: ig_root.clone(),
        ig: ig_root.clone(),
        max_filesize: self.max_filesize,
        skip: self.skip.clone(),
        filter: self.filter.clone(),
      }
    }

    /// Build a new `WalkParallel` iterator.
    ///
    /// Note that this *doesn't* return something that implements `Iterator`.
    /// Instead, the returned value must be run with a closure. e.g.,
    /// `builder.build_parallel().run(|| |path| { println!("{path:?}"); WalkState::Continue })`.
    pub fn build_parallel(&self) -> WalkParallel {
      let ig_root = self
        .get_or_set_current_dir()
        .map(|cwd| self.ig_builder.build_with_cwd(Some(cwd.to_path_buf())))
        .unwrap_or_else(|| self.ig_builder.build());
      WalkParallel {
        paths: self.paths.clone().into_iter(),
        ig_root,
        max_depth: self.max_depth,
        min_depth: self.min_depth,
        max_filesize: self.max_filesize,
        follow_links: self.follow_links,
        same_file_system: self.same_file_system,
        threads: self.threads,
        skip: self.skip.clone(),
        filter: self.filter.clone(),
      }
    }

    /// Add a file path to the iterator.
    ///
    /// Each additional file path added is traversed recursively. This should
    /// be preferred over building multiple `Walk` iterators since this
    /// enables reusing resources across iteration.
    pub fn add<P: AsRef<Path>>(&mut self, path: P) -> &mut WalkBuilder {
      self.paths.push(path.as_ref().to_path_buf());
      self
    }

    /// The maximum depth to recurse.
    ///
    /// The default, `None`, imposes no depth restriction.
    pub fn max_depth(&mut self, depth: Option<usize>) -> &mut WalkBuilder {
      self.max_depth = depth;
      if self.min_depth.is_some() && self.max_depth.is_some() && self.max_depth < self.min_depth {
        self.max_depth = self.min_depth;
      }
      self
    }

    /// The minimum depth to recurse.
    ///
    /// The default, `None`, imposes no minimum depth restriction.
    pub fn min_depth(&mut self, depth: Option<usize>) -> &mut WalkBuilder {
      self.min_depth = depth;
      if self.max_depth.is_some() && self.min_depth.is_some() && self.min_depth > self.max_depth {
        self.min_depth = self.max_depth;
      }
      self
    }

    /// Whether to follow symbolic links or not.
    pub fn follow_links(&mut self, yes: bool) -> &mut WalkBuilder {
      self.follow_links = yes;
      self
    }

    /// Whether to ignore files above the specified limit.
    pub fn max_filesize(&mut self, filesize: Option<u64>) -> &mut WalkBuilder {
      self.max_filesize = filesize;
      self
    }

    /// The number of threads to use for traversal.
    ///
    /// Note that this only has an effect when using `build_parallel`.
    ///
    /// The default setting is `0`, which chooses the number of threads
    /// automatically using heuristics.
    pub fn threads(&mut self, n: usize) -> &mut WalkBuilder {
      self.threads = n;
      self
    }

    /// Add a global ignore file to the matcher.
    ///
    /// This has lower precedence than all other sources of ignore rules.
    ///
    /// # Errors
    ///
    /// If there was a problem adding the ignore file, then an error is
    /// returned. Note that the error may indicate *partial* failure. For
    /// example, if an ignore file contains an invalid glob, all other globs
    /// are still applied.
    ///
    /// An error will also occur if this walker could not get the current
    /// working directory (and `WalkBuilder::current_dir` isn't set).
    pub fn add_ignore<P: AsRef<Path>>(&mut self, path: P) -> Option<Error> {
      let path = path.as_ref();
      let Some(cwd) = self.get_or_set_current_dir() else {
        let err = std::io::Error::other(format!(
          "CWD is not known, ignoring global gitignore {}",
          path.display()
        ));
        return Some(err.into());
      };
      let mut builder = GitignoreBuilder::new(cwd);
      let mut errs = PartialErrorBuilder::default();
      errs.maybe_push(builder.add(path));
      match builder.build() {
        Ok(gi) => {
          self.ig_builder.add_ignore(gi);
        }
        Err(err) => {
          errs.push(err);
        }
      }
      errs.into_error_option()
    }

    /// Add a custom ignore file name
    ///
    /// These ignore files have higher precedence than all other ignore files.
    ///
    /// When specifying multiple names, earlier names have lower precedence than
    /// later names.
    pub fn add_custom_ignore_filename<S: AsRef<OsStr>>(
      &mut self,
      file_name: S,
    ) -> &mut WalkBuilder {
      self.ig_builder.add_custom_ignore_filename(file_name);
      self
    }

    /// Add an override matcher.
    ///
    /// By default, no override matcher is used.
    ///
    /// This overrides any previous setting.
    pub fn overrides(&mut self, overrides: Override) -> &mut WalkBuilder {
      self.ig_builder.overrides(overrides);
      self
    }

    /// Add a file type matcher.
    ///
    /// By default, no file type matcher is used.
    ///
    /// This overrides any previous setting.
    pub fn types(&mut self, types: Types) -> &mut WalkBuilder {
      self.ig_builder.types(types);
      self
    }

    /// Enables all the standard ignore filters.
    ///
    /// This toggles, as a group, all the filters that are enabled by default:
    ///
    /// - [hidden()](#method.hidden)
    /// - [parents()](#method.parents)
    /// - [ignore()](#method.ignore)
    /// - [git_ignore()](#method.git_ignore)
    /// - [git_global()](#method.git_global)
    /// - [git_exclude()](#method.git_exclude)
    ///
    /// They may still be toggled individually after calling this function.
    ///
    /// This is (by definition) enabled by default.
    pub fn standard_filters(&mut self, yes: bool) -> &mut WalkBuilder {
      self
        .hidden(yes)
        .parents(yes)
        .ignore(yes)
        .git_ignore(yes)
        .git_global(yes)
        .git_exclude(yes)
    }

    /// Enables ignoring hidden files.
    ///
    /// This is enabled by default.
    pub fn hidden(&mut self, yes: bool) -> &mut WalkBuilder {
      self.ig_builder.hidden(yes);
      self
    }

    /// Enables reading ignore files from parent directories.
    ///
    /// If this is enabled, then .gitignore files in parent directories of each
    /// file path given are respected. Otherwise, they are ignored.
    ///
    /// This is enabled by default.
    pub fn parents(&mut self, yes: bool) -> &mut WalkBuilder {
      self.ig_builder.parents(yes);
      self
    }

    /// Enables reading `.ignore` files.
    ///
    /// `.ignore` files have the same semantics as `gitignore` files and are
    /// supported by search tools such as ripgrep and The Silver Searcher.
    ///
    /// This is enabled by default.
    pub fn ignore(&mut self, yes: bool) -> &mut WalkBuilder {
      self.ig_builder.ignore(yes);
      self
    }

    /// Enables reading a global gitignore file, whose path is specified in
    /// git's `core.excludesFile` config option.
    ///
    /// Git's config file location is `$HOME/.gitconfig`. If `$HOME/.gitconfig`
    /// does not exist or does not specify `core.excludesFile`, then
    /// `$XDG_CONFIG_HOME/git/ignore` is read. If `$XDG_CONFIG_HOME` is not
    /// set or is empty, then `$HOME/.config/git/ignore` is used instead.
    ///
    /// This is enabled by default.
    pub fn git_global(&mut self, yes: bool) -> &mut WalkBuilder {
      self.ig_builder.git_global(yes);
      self
    }

    /// Enables reading `.gitignore` files.
    ///
    /// `.gitignore` files have match semantics as described in the `gitignore`
    /// man page.
    ///
    /// This is enabled by default.
    pub fn git_ignore(&mut self, yes: bool) -> &mut WalkBuilder {
      self.ig_builder.git_ignore(yes);
      self
    }

    /// Enables reading `.git/info/exclude` files.
    ///
    /// `.git/info/exclude` files have match semantics as described in the
    /// `gitignore` man page.
    ///
    /// This is enabled by default.
    pub fn git_exclude(&mut self, yes: bool) -> &mut WalkBuilder {
      self.ig_builder.git_exclude(yes);
      self
    }

    /// Whether a git repository is required to apply git-related ignore
    /// rules (global rules, .gitignore and local exclude rules).
    ///
    /// When disabled, git-related ignore rules are applied even when searching
    /// outside a git repository.
    ///
    /// In particular, if this is `false` then `.gitignore` files will be read
    /// from parent directories above the git root directory containing `.git`,
    /// which is different from the git behavior.
    pub fn require_git(&mut self, yes: bool) -> &mut WalkBuilder {
      self.ig_builder.require_git(yes);
      self
    }

    /// Process ignore files case insensitively
    ///
    /// This is disabled by default.
    pub fn ignore_case_insensitive(&mut self, yes: bool) -> &mut WalkBuilder {
      self.ig_builder.ignore_case_insensitive(yes);
      self
    }

    /// Set a function for sorting directory entries by their path.
    ///
    /// If a compare function is set, the resulting iterator will return all
    /// paths in sorted order. The compare function will be called to compare
    /// entries from the same directory.
    ///
    /// This is like `sort_by_file_name`, except the comparator accepts
    /// a `&Path` instead of the base file name, which permits it to sort by
    /// more criteria.
    ///
    /// This method will override any previous sorter set by this method or
    /// by `sort_by_file_name`.
    ///
    /// Note that this is not used in the parallel iterator.
    pub fn sort_by_file_path<F>(&mut self, cmp: F) -> &mut WalkBuilder
    where
      F: Fn(&Path, &Path) -> Ordering + Send + Sync + 'static,
    {
      self.sorter = Some(Sorter::ByPath(Arc::new(cmp)));
      self
    }

    /// Set a function for sorting directory entries by file name.
    ///
    /// If a compare function is set, the resulting iterator will return all
    /// paths in sorted order. The compare function will be called to compare
    /// names from entries from the same directory using only the name of the
    /// entry.
    ///
    /// This method will override any previous sorter set by this method or
    /// by `sort_by_file_path`.
    ///
    /// Note that this is not used in the parallel iterator.
    pub fn sort_by_file_name<F>(&mut self, cmp: F) -> &mut WalkBuilder
    where
      F: Fn(&OsStr, &OsStr) -> Ordering + Send + Sync + 'static,
    {
      self.sorter = Some(Sorter::ByName(Arc::new(cmp)));
      self
    }

    /// Do not cross file system boundaries.
    ///
    /// When this option is enabled, directory traversal will not descend into
    /// directories that are on a different file system from the root path.
    ///
    /// Currently, this option is only supported on Unix and Windows. If this
    /// option is used on an unsupported platform, then directory traversal
    /// will immediately return an error and will not yield any entries.
    pub fn same_file_system(&mut self, yes: bool) -> &mut WalkBuilder {
      self.same_file_system = yes;
      self
    }

    /// Do not yield directory entries that are believed to correspond to
    /// stdout.
    ///
    /// This is useful when a command is invoked via shell redirection to a
    /// file that is also being read. For example, `grep -r foo ./ > results`
    /// might end up trying to search `results` even though it is also writing
    /// to it, which could cause an unbounded feedback loop. Setting this
    /// option prevents this from happening by skipping over the `results`
    /// file.
    ///
    /// This is disabled by default.
    pub fn skip_stdout(&mut self, yes: bool) -> &mut WalkBuilder {
      if yes {
        self.skip = stdout_handle().map(Arc::new);
      } else {
        self.skip = None;
      }
      self
    }

    /// Yields only entries which satisfy the given predicate and skips
    /// descending into directories that do not satisfy the given predicate.
    ///
    /// The predicate is applied to all entries. If the predicate is
    /// true, iteration carries on as normal. If the predicate is false, the
    /// entry is ignored and if it is a directory, it is not descended into.
    ///
    /// Note that the errors for reading entries that may not satisfy the
    /// predicate will still be yielded.
    ///
    /// Note also that only one filter predicate can be applied to a
    /// `WalkBuilder`. Calling this subsequent times overrides previous filter
    /// predicates.
    pub fn filter_entry<P>(&mut self, filter: P) -> &mut WalkBuilder
    where
      P: Fn(&DirEntry) -> bool + Send + Sync + 'static,
    {
      self.filter = Some(Filter(Arc::new(filter)));
      self
    }

    /// Set the current working directory used for matching global gitignores.
    ///
    /// If this is not set, then this walker will attempt to discover the
    /// correct path from the environment's current working directory. If
    /// that fails, then global gitignore files will be ignored.
    ///
    /// Global gitignore files come from things like a user's git configuration
    /// or from gitignore files added via [`WalkBuilder::add_ignore`].
    pub fn current_dir(&mut self, cwd: impl Into<PathBuf>) -> &mut WalkBuilder {
      let cwd = cwd.into();
      self.ig_builder.current_dir(cwd.clone());
      if let Err(cwd) = self.global_gitignores_relative_to.set(Ok(cwd)) {
        // OK because `Err` from `set` implies a value exists.
        *self.global_gitignores_relative_to.get_mut().unwrap() = cwd;
      }
      self
    }

    /// Gets the currently configured CWD on this walk builder.
    ///
    /// This is "lazy." That is, we only ask for the CWD from the environment
    /// if `WalkBuilder::current_dir` hasn't been called yet. And we ensure
    /// that we only do it once.
    fn get_or_set_current_dir(&self) -> Option<&Path> {
      let result = self.global_gitignores_relative_to.get_or_init(|| {
        let result = std::env::current_dir().map_err(Arc::new);
        match result {
          Ok(ref path) => {
            log::trace!("automatically discovered CWD: {}", path.display());
          }
          Err(ref err) => {
            log::debug!(
              "failed to find CWD \
                         (global gitignores will be ignored): \
                         {err}"
            );
          }
        }
        result
      });
      result.as_ref().ok().map(|path| &**path)
    }
  }

  /// Walk is a recursive directory iterator over file paths in one or more
  /// directories.
  ///
  /// Only file and directory paths matching the rules are returned. By default,
  /// ignore files like `.gitignore` are respected. The precise matching rules
  /// and precedence is explained in the documentation for `WalkBuilder`.
  pub struct Walk {
    its: std::vec::IntoIter<(PathBuf, Option<WalkEventIter>)>,
    it: Option<WalkEventIter>,
    ig_root: Ignore,
    ig: Ignore,
    max_filesize: Option<u64>,
    skip: Option<Arc<Handle>>,
    filter: Option<Filter>,
  }

  impl Walk {
    /// Creates a new recursive directory iterator for the file path given.
    ///
    /// Note that this uses default settings, which include respecting
    /// `.gitignore` files. To configure the iterator, use `WalkBuilder`
    /// instead.
    pub fn new<P: AsRef<Path>>(path: P) -> Walk {
      WalkBuilder::new(path).build()
    }

    /// Create a new recursive directory iterator from the sequence of paths
    /// given.
    ///
    /// Note that if the provided iterator is empty, then `Walk` is guaranteed
    /// to yield zero entries.
    pub fn from_iter<P: AsRef<Path>, I: IntoIterator<Item = P>>(paths: I) -> Walk {
      WalkBuilder::from_iter(paths).build()
    }

    fn skip_entry(&self, ent: &DirEntry) -> Result<bool, Error> {
      if ent.depth() == 0 {
        return Ok(false);
      }
      // We ensure that trivial skipping is done before any other potentially
      // expensive operations (stat, filesystem other) are done. This seems
      // like an obvious optimization but becomes critical when filesystem
      // operations even as simple as stat can result in significant
      // overheads; an example of this was a bespoke filesystem layer in
      // Windows that hosted files remotely and would download them on-demand
      // when particular filesystem operations occurred. Users of this system
      // who ensured correct file-type filters were being used could still
      // get unnecessary file access resulting in large downloads.
      if should_skip_entry(&self.ig, ent) {
        return Ok(true);
      }
      if let Some(ref stdout) = self.skip {
        if path_equals(ent, stdout)? {
          return Ok(true);
        }
      }
      if self.max_filesize.is_some() && !ent.is_dir() {
        return Ok(skip_filesize(
          self.max_filesize.unwrap(),
          ent.path(),
          &ent.metadata().ok(),
        ));
      }
      if let Some(Filter(filter)) = &self.filter {
        if !filter(ent) {
          return Ok(true);
        }
      }
      Ok(false)
    }
  }

  impl Iterator for Walk {
    type Item = Result<DirEntry, Error>;

    #[inline(always)]
    fn next(&mut self) -> Option<Result<DirEntry, Error>> {
      loop {
        let ev = match self.it.as_mut().and_then(|it| it.next()) {
          Some(ev) => ev,
          None => {
            match self.its.next() {
              None => return None,
              Some((_, None)) => {
                return Some(Ok(DirEntry::new_stdin()));
              }
              Some((path, Some(it))) => {
                self.it = Some(it);
                if path.is_dir() {
                  let (ig, err) = self.ig_root.add_parents(path);
                  self.ig = ig;
                  if let Some(err) = err {
                    return Some(Err(err));
                  }
                } else {
                  self.ig = self.ig_root.clone();
                }
              }
            }
            continue;
          }
        };
        match ev {
          Err(err) => {
            return Some(Err(Error::from_walkdir(err)));
          }
          Ok(WalkEvent::Exit) => {
            self.ig = self.ig.parent().unwrap();
          }
          Ok(WalkEvent::Dir(ent)) => {
            let mut ent = DirEntry::new_walkdir(ent, None);
            let should_skip = match self.skip_entry(&ent) {
              Err(err) => return Some(Err(err)),
              Ok(should_skip) => should_skip,
            };
            if should_skip {
              self.it.as_mut().unwrap().it.skip_current_dir();
              // Still need to push this on the stack because
              // we'll get a WalkEvent::Exit event for this dir.
              // We don't care if it errors though.
              let (igtmp, _) = self.ig.add_child(ent.path());
              self.ig = igtmp;
              continue;
            }
            let (igtmp, err) = self.ig.add_child(ent.path());
            self.ig = igtmp;
            ent.err = err;
            return Some(Ok(ent));
          }
          Ok(WalkEvent::File(ent)) => {
            let ent = DirEntry::new_walkdir(ent, None);
            let should_skip = match self.skip_entry(&ent) {
              Err(err) => return Some(Err(err)),
              Ok(should_skip) => should_skip,
            };
            if should_skip {
              continue;
            }
            return Some(Ok(ent));
          }
        }
      }
    }
  }

  impl std::iter::FusedIterator for Walk {}

  /// WalkEventIter transforms a WalkDir iterator into an iterator that more
  /// accurately describes the directory tree. Namely, it emits events that are
  /// one of three types: directory, file or "exit." An "exit" event means that
  /// the entire contents of a directory have been enumerated.
  struct WalkEventIter {
    depth: usize,
    it: crate::walkdir::IntoIter,
    next: Option<Result<crate::walkdir::DirEntry, crate::walkdir::Error>>,
  }

  #[derive(Debug)]
  enum WalkEvent {
    Dir(crate::walkdir::DirEntry),
    File(crate::walkdir::DirEntry),
    Exit,
  }

  impl From<WalkDir> for WalkEventIter {
    fn from(it: WalkDir) -> WalkEventIter {
      WalkEventIter {
        depth: 0,
        it: it.into_iter(),
        next: None,
      }
    }
  }

  impl Iterator for WalkEventIter {
    type Item = crate::walkdir::Result<WalkEvent>;

    #[inline(always)]
    fn next(&mut self) -> Option<crate::walkdir::Result<WalkEvent>> {
      let dent = self.next.take().or_else(|| self.it.next());
      let depth = match dent {
        None => 0,
        Some(Ok(ref dent)) => dent.depth(),
        Some(Err(ref err)) => err.depth(),
      };
      if depth < self.depth {
        self.depth -= 1;
        self.next = dent;
        return Some(Ok(WalkEvent::Exit));
      }
      self.depth = depth;
      match dent {
        None => None,
        Some(Err(err)) => Some(Err(err)),
        Some(Ok(dent)) => {
          if walkdir_is_dir(&dent) {
            self.depth += 1;
            Some(Ok(WalkEvent::Dir(dent)))
          } else {
            Some(Ok(WalkEvent::File(dent)))
          }
        }
      }
    }
  }

  /// WalkState is used in the parallel recursive directory iterator to indicate
  /// whether walking should continue as normal, skip descending into a
  /// particular directory or quit the walk entirely.
  #[derive(Clone, Copy, Debug, Eq, PartialEq)]
  pub enum WalkState {
    /// Continue walking as normal.
    Continue,
    /// If the directory entry given is a directory, don't descend into it.
    /// In all other cases, this has no effect.
    Skip,
    /// Quit the entire iterator as soon as possible.
    ///
    /// Note that this is an inherently asynchronous action. It is possible
    /// for more entries to be yielded even after instructing the iterator
    /// to quit.
    Quit,
  }

  impl WalkState {
    fn is_continue(&self) -> bool {
      *self == WalkState::Continue
    }

    fn is_quit(&self) -> bool {
      *self == WalkState::Quit
    }
  }

  /// A builder for constructing a visitor when using [`WalkParallel::visit`].
  /// The builder will be called for each thread started by `WalkParallel`. The
  /// visitor returned from each builder is then called for every directory
  /// entry.
  pub trait ParallelVisitorBuilder<'s> {
    /// Create per-thread `ParallelVisitor`s for `WalkParallel`.
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's>;
  }

  impl<'a, 's, P: ParallelVisitorBuilder<'s>> ParallelVisitorBuilder<'s> for &'a mut P {
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's> {
      (**self).build()
    }
  }

  /// Receives files and directories for the current thread.
  ///
  /// Setup for the traversal can be implemented as part of
  /// [`ParallelVisitorBuilder::build`]. Teardown when traversal finishes can be
  /// implemented by implementing the `Drop` trait on your traversal type.
  pub trait ParallelVisitor: Send {
    /// Receives files and directories for the current thread. This is called
    /// once for every directory entry visited by traversal.
    fn visit(&mut self, entry: Result<DirEntry, Error>) -> WalkState;
  }

  struct FnBuilder<F> {
    builder: F,
  }

  impl<'s, F: FnMut() -> FnVisitor<'s>> ParallelVisitorBuilder<'s> for FnBuilder<F> {
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's> {
      let visitor = (self.builder)();
      Box::new(FnVisitorImp { visitor })
    }
  }

  type FnVisitor<'s> = Box<dyn FnMut(Result<DirEntry, Error>) -> WalkState + Send + 's>;

  struct FnVisitorImp<'s> {
    visitor: FnVisitor<'s>,
  }

  impl<'s> ParallelVisitor for FnVisitorImp<'s> {
    fn visit(&mut self, entry: Result<DirEntry, Error>) -> WalkState {
      (self.visitor)(entry)
    }
  }

  /// WalkParallel is a parallel recursive directory iterator over files paths
  /// in one or more directories.
  ///
  /// Only file and directory paths matching the rules are returned. By default,
  /// ignore files like `.gitignore` are respected. The precise matching rules
  /// and precedence is explained in the documentation for `WalkBuilder`.
  ///
  /// Unlike `Walk`, this uses multiple threads for traversing a directory.
  pub struct WalkParallel {
    paths: std::vec::IntoIter<PathBuf>,
    ig_root: Ignore,
    max_filesize: Option<u64>,
    max_depth: Option<usize>,
    min_depth: Option<usize>,
    follow_links: bool,
    same_file_system: bool,
    threads: usize,
    skip: Option<Arc<Handle>>,
    filter: Option<Filter>,
  }

  impl WalkParallel {
    /// Execute the parallel recursive directory iterator. `mkf` is called
    /// for each thread used for iteration. The function produced by `mkf`
    /// is then in turn called for each visited file path.
    pub fn run<'s, F>(self, mkf: F)
    where
      F: FnMut() -> FnVisitor<'s>,
    {
      self.visit(&mut FnBuilder { builder: mkf })
    }

    /// Execute the parallel recursive directory iterator using a custom
    /// visitor.
    ///
    /// The builder given is used to construct a visitor for every thread
    /// used by this traversal. The visitor returned from each builder is then
    /// called for every directory entry seen by that thread.
    ///
    /// Typically, creating a custom visitor is useful if you need to perform
    /// some kind of cleanup once traversal is finished. This can be achieved
    /// by implementing `Drop` for your builder (or for your visitor, if you
    /// want to execute cleanup for every thread that is launched).
    ///
    /// For example, each visitor might build up a data structure of results
    /// corresponding to the directory entries seen for each thread. Since each
    /// visitor runs on only one thread, this build-up can be done without
    /// synchronization. Then, once traversal is complete, all of the results
    /// can be merged together into a single data structure.
    pub fn visit(mut self, builder: &mut dyn ParallelVisitorBuilder<'_>) {
      let threads = self.threads();
      let mut stack = vec![];
      {
        let mut visitor = builder.build();
        let mut paths = Vec::new().into_iter();
        std::mem::swap(&mut paths, &mut self.paths);
        // Send the initial set of root paths to the pool of workers. Note
        // that we only send directories. For files, we send to them the
        // callback directly.
        for path in paths {
          let (dent, root_device) = if path == Path::new("-") {
            (DirEntry::new_stdin(), None)
          } else {
            let root_device = if !self.same_file_system {
              None
            } else {
              match device_num(&path) {
                Ok(root_device) => Some(root_device),
                Err(err) => {
                  let err = Error::Io(err).with_path(path);
                  if visitor.visit(Err(err)).is_quit() {
                    return;
                  }
                  continue;
                }
              }
            };
            match DirEntryRaw::from_path(0, path, false) {
              Ok(dent) => (DirEntry::new_raw(dent, None), root_device),
              Err(err) => {
                if visitor.visit(Err(err)).is_quit() {
                  return;
                }
                continue;
              }
            }
          };
          stack.push(Message::Work(Work {
            dent,
            ignore: self.ig_root.clone(),
            root_device,
          }));
        }
        // ... but there's no need to start workers if we don't need them.
        if stack.is_empty() {
          return;
        }
      }
      // Create the workers and then wait for them to finish.
      let quit_now = Arc::new(AtomicBool::new(false));
      let active_workers = Arc::new(AtomicUsize::new(threads));
      let stacks = Stack::new_for_each_thread(threads, stack);
      std::thread::scope(|s| {
        let handles: Vec<_> = stacks
          .into_iter()
          .map(|stack| Worker {
            visitor: builder.build(),
            stack,
            quit_now: quit_now.clone(),
            active_workers: active_workers.clone(),
            max_depth: self.max_depth,
            min_depth: self.min_depth,
            max_filesize: self.max_filesize,
            follow_links: self.follow_links,
            skip: self.skip.clone(),
            filter: self.filter.clone(),
          })
          .map(|worker| s.spawn(|| worker.run()))
          .collect();
        for handle in handles {
          handle.join().unwrap();
        }
      });
    }

    fn threads(&self) -> usize {
      if self.threads == 0 {
        std::thread::available_parallelism()
          .map_or(1, |n| n.get())
          .min(12)
      } else {
        self.threads
      }
    }
  }

  /// Message is the set of instructions that a worker knows how to process.
  enum Message {
    /// A work item corresponds to a directory that should be descended into.
    /// Work items for entries that should be skipped or ignored should not
    /// be produced.
    Work(Work),
    /// This instruction indicates that the worker should quit.
    Quit,
  }

  /// A unit of work for each worker to process.
  ///
  /// Each unit of work corresponds to a directory that should be descended
  /// into.
  struct Work {
    /// The directory entry.
    dent: DirEntry,
    /// Any ignore matchers that have been built for this directory's parents.
    ignore: Ignore,
    /// The root device number. When present, only files with the same device
    /// number should be considered.
    root_device: Option<u64>,
  }

  #[derive(Default)]
  struct ReadDirResult {
    entries: Vec<fs::DirEntry>,
    errors: Vec<Error>,
  }

  impl Work {
    /// Returns true if and only if this work item is a directory.
    fn is_dir(&self) -> bool {
      self.dent.is_dir()
    }

    /// Returns true if and only if this work item is a symlink.
    fn is_symlink(&self) -> bool {
      self.dent.file_type().map_or(false, |ft| ft.is_symlink())
    }

    /// Adds ignore rules for parent directories.
    ///
    /// Note that this only applies to entries at depth 0. On all other
    /// entries, this is a no-op.
    fn add_parents(&mut self) -> Option<Error> {
      if self.dent.depth() > 0 {
        return None;
      }
      // At depth 0, the path of this entry is a root path, so we can
      // use it directly to add parent ignore rules.
      let (ig, err) = self.ignore.add_parents(self.dent.path());
      self.ignore = ig;
      err
    }

    /// Adds ignore rules for this directory without reading its contents.
    fn add_ignore(&mut self) {
      let (ig, err) = self.ignore.add_child(self.dent.path());
      self.ignore = ig;
      self.dent.err = err;
    }

    /// Reads the directory contents of this work item and adds ignore
    /// rules for this directory.
    ///
    /// If there was a problem with reading the directory contents, then
    /// an error is returned. If there was a problem reading the ignore
    /// rules for this directory, then the error is attached to this
    /// work item's directory entry.
    fn read_dir(&mut self) -> Result<ReadDirResult, Error> {
      let readdir = match fs::read_dir(self.dent.path()) {
        Ok(readdir) => readdir,
        Err(err) => {
          let err = Error::from(err)
            .with_path(self.dent.path())
            .with_depth(self.dent.depth());
          return Err(err);
        }
      };
      // Actually descend into the directory and read its contents
      let mut result = ReadDirResult::default();
      for entry in readdir {
        match entry {
          Ok(entry) => result.entries.push(entry),
          Err(err) => result.errors.push(
            Error::from(err)
              .with_path(self.dent.path())
              .with_depth(self.dent.depth() + 1),
          ),
        }
      }
      let (ig, err) = self
        .ignore
        .add_child_with_entries(self.dent.path(), &result.entries);
      self.ignore = ig;
      self.dent.err = err;
      Ok(result)
    }
  }

  /// A work-stealing stack.
  #[derive(Debug)]
  struct Stack {
    /// This thread's index.
    index: usize,
    /// The thread-local stack.
    deque: Deque<Message>,
    /// The work stealers.
    stealers: Arc<[Stealer<Message>]>,
  }

  impl Stack {
    /// Create a work-stealing stack for each thread. The given messages
    /// correspond to the initial paths to start the search at. They will
    /// be distributed automatically to each stack in a round-robin fashion.
    fn new_for_each_thread(threads: usize, init: Vec<Message>) -> Vec<Stack> {
      // Using new_lifo() ensures each worker operates depth-first, not
      // breadth-first. We do depth-first because a breadth first traversal
      // on wide directories with a lot of gitignores is disastrous (for
      // example, searching a directory tree containing all of crates.io).
      let deques: Vec<Deque<Message>> = std::iter::repeat_with(Deque::new_lifo)
        .take(threads)
        .collect();
      let stealers =
        Arc::<[Stealer<Message>]>::from(deques.iter().map(Deque::stealer).collect::<Vec<_>>());
      let stacks: Vec<Stack> = deques
        .into_iter()
        .enumerate()
        .map(|(index, deque)| Stack {
          index,
          deque,
          stealers: stealers.clone(),
        })
        .collect();
      // Distribute the initial messages, reverse the order to cancel out
      // the other reversal caused by the inherent LIFO processing of the
      // per-thread stacks which are filled here.
      init
        .into_iter()
        .rev()
        .zip(stacks.iter().cycle())
        .for_each(|(m, s)| s.push(m));
      stacks
    }

    /// Push a message.
    fn push(&self, msg: Message) {
      self.deque.push(msg);
    }

    /// Pop a message.
    fn pop(&self) -> Option<Message> {
      self.deque.pop().or_else(|| self.steal())
    }

    /// Steal a message from another queue.
    fn steal(&self) -> Option<Message> {
      // For fairness, try to steal from index + 1, index + 2, ... len - 1,
      // then wrap around to 0, 1, ... index - 1.
      let (left, right) = self.stealers.split_at(self.index);
      // Don't steal from ourselves
      let right = &right[1..];

      right
        .iter()
        .chain(left.iter())
        .map(|s| s.steal_batch_and_pop(&self.deque))
        .find_map(|s| s.success())
    }
  }

  /// A worker is responsible for descending into directories, updating the
  /// ignore matchers, producing new work and invoking the caller's callback.
  ///
  /// Note that a worker is *both* a producer and a consumer.
  struct Worker<'s> {
    /// The caller's callback.
    visitor: Box<dyn ParallelVisitor + 's>,
    /// A work-stealing stack of work to do.
    ///
    /// We use a stack instead of a channel because a stack lets us visit
    /// directories in depth first order. This can substantially reduce peak
    /// memory usage by keeping both the number of file paths and gitignore
    /// matchers in memory lower.
    stack: Stack,
    /// Whether all workers should terminate at the next opportunity. Note
    /// that we need this because we don't want other `Work` to be done after
    /// we quit. We wouldn't need this if have a priority channel.
    quit_now: Arc<AtomicBool>,
    /// The number of currently active workers.
    active_workers: Arc<AtomicUsize>,
    /// The maximum depth of directories to descend. A value of `0` means no
    /// descension at all.
    max_depth: Option<usize>,
    /// The minimum depth of directories to descend.
    min_depth: Option<usize>,
    /// The maximum size a searched file can be (in bytes). If a file exceeds
    /// this size it will be skipped.
    max_filesize: Option<u64>,
    /// Whether to follow symbolic links or not. When this is enabled, loop
    /// detection is performed.
    follow_links: bool,
    /// A file handle to skip, currently is either `None` or stdout, if it's
    /// a file and it has been requested to skip files identical to stdout.
    skip: Option<Arc<Handle>>,
    /// A predicate applied to dir entries. If true, the entry and all
    /// children will be skipped.
    filter: Option<Filter>,
  }

  impl<'s> Worker<'s> {
    /// Runs this worker until there is no more work left to do.
    ///
    /// The worker will call the caller's callback for all entries that aren't
    /// skipped by the ignore matcher.
    fn run(mut self) {
      while let Some(work) = self.get_work() {
        if let WalkState::Quit = self.run_one(work) {
          self.quit_now();
        }
      }
    }

    fn run_one(&mut self, mut work: Work) -> WalkState {
      let should_visit = self
        .min_depth
        .map(|min_depth| work.dent.depth() >= min_depth)
        .unwrap_or(true);

      // If the work is not a directory, then we can just execute the
      // caller's callback immediately and move on.
      if work.is_symlink() || !work.is_dir() {
        return if should_visit {
          self.visitor.visit(Ok(work.dent))
        } else {
          WalkState::Continue
        };
      }
      if let Some(err) = work.add_parents() {
        let state = self.visitor.visit(Err(err));
        if state.is_quit() {
          return state;
        }
      }

      let descend = if let Some(root_device) = work.root_device {
        match is_same_file_system(root_device, work.dent.path()) {
          Ok(true) => true,
          Ok(false) => false,
          Err(err) => {
            let state = self.visitor.visit(Err(err));
            if state.is_quit() {
              return state;
            }
            false
          }
        }
      } else {
        true
      };

      // Try to read the directory first before we transfer ownership
      // to the provided closure. Do not unwrap it immediately, though,
      // as we may receive an `Err` value e.g. in the case when we do not
      // have sufficient read permissions to list the directory.
      // In that case we still want to provide the closure with a valid
      // entry before passing the error value.
      let depth = work.dent.depth();
      let readdir = if descend && self.max_depth.is_none_or(|m| depth < m) {
        Some(work.read_dir())
      } else {
        work.add_ignore();
        None
      };
      if should_visit {
        let state = self.visitor.visit(Ok(work.dent));
        if !state.is_continue() {
          return state;
        }
      }
      if !descend {
        return WalkState::Skip;
      }

      let readdir = match readdir {
        Some(readdir) => readdir,
        None => return WalkState::Skip,
      };
      let readdir = match readdir {
        Ok(readdir) => readdir,
        Err(err) => {
          return self.visitor.visit(Err(err));
        }
      };

      for result in readdir.entries {
        let state = self.generate_work(&work.ignore, depth + 1, work.root_device, result);
        if state.is_quit() {
          return state;
        }
      }
      for err in readdir.errors {
        let state = self.visitor.visit(Err(err));
        if state.is_quit() {
          return state;
        }
      }
      WalkState::Continue
    }

    /// Decides whether to submit the given directory entry as a file to
    /// search.
    ///
    /// If the entry is a path that should be ignored, then this is a no-op.
    /// Otherwise, the entry is pushed on to the queue. (The actual execution
    /// of the callback happens in `run_one`.)
    ///
    /// If an error occurs while reading the entry, then it is sent to the
    /// caller's callback.
    ///
    /// `ig` is the `Ignore` matcher for the parent directory. `depth` should
    /// be the depth of this entry. `result` should be the item yielded by
    /// a directory iterator.
    fn generate_work(
      &mut self,
      ig: &Ignore,
      depth: usize,
      root_device: Option<u64>,
      fs_dent: fs::DirEntry,
    ) -> WalkState {
      let mut dent = match DirEntryRaw::from_entry(depth, &fs_dent) {
        Ok(dent) => DirEntry::new_raw(dent, None),
        Err(err) => {
          return self.visitor.visit(Err(err));
        }
      };
      let is_symlink = dent.file_type().map_or(false, |ft| ft.is_symlink());
      if self.follow_links && is_symlink {
        let path = dent.path().to_path_buf();
        dent = match DirEntryRaw::from_path(depth, path, true) {
          Ok(dent) => DirEntry::new_raw(dent, None),
          Err(err) => {
            return self.visitor.visit(Err(err));
          }
        };
        if dent.is_dir() {
          if let Err(err) = check_symlink_loop(ig, dent.path(), depth) {
            return self.visitor.visit(Err(err));
          }
        }
      }
      // N.B. See analogous call in the single-threaded implementation about
      // why it's important for this to come before the checks below.
      if should_skip_entry(ig, &dent) {
        return WalkState::Continue;
      }
      if let Some(ref stdout) = self.skip {
        let is_stdout = match path_equals(&dent, stdout) {
          Ok(is_stdout) => is_stdout,
          Err(err) => return self.visitor.visit(Err(err)),
        };
        if is_stdout {
          return WalkState::Continue;
        }
      }
      let should_skip_filesize = if self.max_filesize.is_some() && !dent.is_dir() {
        skip_filesize(
          self.max_filesize.unwrap(),
          dent.path(),
          &dent.metadata().ok(),
        )
      } else {
        false
      };
      let should_skip_filtered = if let Some(Filter(predicate)) = &self.filter {
        !predicate(&dent)
      } else {
        false
      };
      if !should_skip_filesize && !should_skip_filtered {
        self.send(Work {
          dent,
          ignore: ig.clone(),
          root_device,
        });
      }
      WalkState::Continue
    }

    /// Returns the next directory to descend into.
    ///
    /// If all work has been exhausted, then this returns None. The worker
    /// should then subsequently quit.
    fn get_work(&mut self) -> Option<Work> {
      let mut value = self.recv();
      loop {
        // Simulate a priority channel: If quit_now flag is set, we can
        // receive only quit messages.
        if self.is_quit_now() {
          value = Some(Message::Quit)
        }
        match value {
          Some(Message::Work(work)) => {
            return Some(work);
          }
          Some(Message::Quit) => {
            // Repeat quit message to wake up sleeping threads, if
            // any. The domino effect will ensure that every thread
            // will quit.
            self.send_quit();
            return None;
          }
          None => {
            if self.deactivate_worker() == 0 {
              // If deactivate_worker() returns 0, every worker thread
              // is currently within the critical section between the
              // acquire in deactivate_worker() and the release in
              // activate_worker() below.  For this to happen, every
              // worker's local deque must be simultaneously empty,
              // meaning there is no more work left at all.
              self.send_quit();
              return None;
            }
            // Wait for next `Work` or `Quit` message.
            loop {
              if let Some(v) = self.recv() {
                self.activate_worker();
                value = Some(v);
                break;
              }
              // Our stack isn't blocking. Instead of burning the
              // CPU waiting, we let the thread sleep for a bit. In
              // general, this tends to only occur once the search is
              // approaching termination.
              let dur = std::time::Duration::from_millis(1);
              std::thread::sleep(dur);
            }
          }
        }
      }
    }

    /// Indicates that all workers should quit immediately.
    fn quit_now(&self) {
      self.quit_now.store(true, AtomicOrdering::SeqCst);
    }

    /// Returns true if this worker should quit immediately.
    fn is_quit_now(&self) -> bool {
      self.quit_now.load(AtomicOrdering::SeqCst)
    }

    /// Send work.
    fn send(&self, work: Work) {
      self.stack.push(Message::Work(work));
    }

    /// Send a quit message.
    fn send_quit(&self) {
      self.stack.push(Message::Quit);
    }

    /// Receive work.
    fn recv(&self) -> Option<Message> {
      self.stack.pop()
    }

    /// Deactivates a worker and returns the number of currently active workers.
    fn deactivate_worker(&self) -> usize {
      self.active_workers.fetch_sub(1, AtomicOrdering::Acquire) - 1
    }

    /// Reactivates a worker.
    fn activate_worker(&self) {
      self.active_workers.fetch_add(1, AtomicOrdering::Release);
    }
  }

  fn check_symlink_loop(
    ig_parent: &Ignore,
    child_path: &Path,
    child_depth: usize,
  ) -> Result<(), Error> {
    let hchild = Handle::from_path(child_path).map_err(|err| {
      Error::from(err)
        .with_path(child_path)
        .with_depth(child_depth)
    })?;
    for ig in ig_parent
      .parents()
      .take_while(|ig| !ig.is_absolute_parent())
    {
      let h = Handle::from_path(ig.path()).map_err(|err| {
        Error::from(err)
          .with_path(child_path)
          .with_depth(child_depth)
      })?;
      if hchild == h {
        return Err(
          Error::Loop {
            ancestor: ig.path().to_path_buf(),
            child: child_path.to_path_buf(),
          }
          .with_depth(child_depth),
        );
      }
    }
    Ok(())
  }

  // Before calling this function, make sure that you ensure that is really
  // necessary as the arguments imply a file stat.
  fn skip_filesize(max_filesize: u64, path: &Path, ent: &Option<Metadata>) -> bool {
    let filesize = match *ent {
      Some(ref md) => Some(md.len()),
      None => None,
    };

    if let Some(fs) = filesize {
      if fs > max_filesize {
        log::debug!("ignoring {}: {} bytes", path.display(), fs);
        true
      } else {
        false
      }
    } else {
      false
    }
  }

  fn should_skip_entry(ig: &Ignore, dent: &DirEntry) -> bool {
    let m = ig.matched_dir_entry(dent);
    if m.is_ignore() {
      log::debug!("ignoring {}: {:?}", dent.path().display(), m);
      true
    } else if m.is_whitelist() {
      log::debug!("whitelisting {}: {:?}", dent.path().display(), m);
      false
    } else {
      false
    }
  }

  /// Returns a handle to stdout for filtering search.
  ///
  /// A handle is returned if and only if stdout is being redirected to a file.
  /// The handle returned corresponds to that file.
  ///
  /// This can be used to ensure that we do not attempt to search a file that we
  /// may also be writing to.
  fn stdout_handle() -> Option<Handle> {
    let h = match Handle::stdout() {
      Err(_) => return None,
      Ok(h) => h,
    };
    let md = match h.as_file().metadata() {
      Err(_) => return None,
      Ok(md) => md,
    };
    if !md.is_file() {
      return None;
    }
    Some(h)
  }

  /// Returns true if and only if the given directory entry is believed to be
  /// equivalent to the given handle. If there was a problem querying the path
  /// for information to determine equality, then that error is returned.
  fn path_equals(dent: &DirEntry, handle: &Handle) -> Result<bool, Error> {
    #[cfg(unix)]
    fn never_equal(dent: &DirEntry, handle: &Handle) -> bool {
      dent.ino() != Some(handle.ino())
    }

    // If we know for sure that these two things aren't equal, then avoid
    // the costly extra stat call to determine equality.
    if dent.is_stdin() || never_equal(dent, handle) {
      return Ok(false);
    }
    Handle::from_path(dent.path())
      .map(|h| &h == handle)
      .map_err(|err| {
        Error::Io(err)
          .with_depth(dent.depth())
          .with_path(dent.path())
      })
  }

  fn walkdir_is_dir(dent: &crate::walkdir::DirEntry) -> bool {
    if dent.file_type().is_dir() {
      return true;
    }
    if !dent.file_type().is_symlink() || dent.depth() > 0 {
      return false;
    }
    dent
      .path()
      .metadata()
      .ok()
      .map_or(false, |md| md.file_type().is_dir())
  }

  /// Returns true if and only if the given path is on the same device as the
  /// given root device.
  fn is_same_file_system(root_device: u64, path: &Path) -> Result<bool, Error> {
    let dent_device = device_num(path).map_err(|err| Error::Io(err).with_path(path))?;
    Ok(root_device == dent_device)
  }

  #[cfg(unix)]
  fn device_num<P: AsRef<Path>>(path: P) -> io::Result<u64> {
    use std::os::unix::fs::MetadataExt;

    path.as_ref().metadata().map(|md| md.dev())
  }
}

/// Represents an error that can occur when parsing a gitignore file.
#[derive(Debug)]
pub enum Error {
  /// A collection of "soft" errors. These occur when adding an ignore
  /// file partially succeeded.
  Partial(Vec<Error>),
  /// An error associated with a specific line number.
  WithLineNumber {
    /// The line number.
    line: u64,
    /// The underlying error.
    err: Box<Error>,
  },
  /// An error associated with a particular file path.
  WithPath {
    /// The file path.
    path: PathBuf,
    /// The underlying error.
    err: Box<Error>,
  },
  /// An error associated with a particular directory depth when recursively
  /// walking a directory.
  WithDepth {
    /// The directory depth.
    depth: usize,
    /// The underlying error.
    err: Box<Error>,
  },
  /// An error that occurs when a file loop is detected when traversing
  /// symbolic links.
  Loop {
    /// The ancestor file path in the loop.
    ancestor: PathBuf,
    /// The child file path in the loop.
    child: PathBuf,
  },
  /// An error that occurs when doing I/O, such as reading an ignore file.
  Io(std::io::Error),
  /// An error that occurs when trying to parse a glob.
  Glob {
    glob: Option<String>,
    /// The underlying glob error as a string.
    err: String,
  },
  /// A type selection for a file type that is not defined.
  UnrecognizedFileType(String),
  /// A user specified file type definition could not be parsed.
  InvalidDefinition,
}

impl Clone for Error {
  fn clone(&self) -> Error {
    match *self {
      Error::Partial(ref errs) => Error::Partial(errs.clone()),
      Error::WithLineNumber { line, ref err } => Error::WithLineNumber {
        line,
        err: err.clone(),
      },
      Error::WithPath { ref path, ref err } => Error::WithPath {
        path: path.clone(),
        err: err.clone(),
      },
      Error::WithDepth { depth, ref err } => Error::WithDepth {
        depth,
        err: err.clone(),
      },
      Error::Loop {
        ref ancestor,
        ref child,
      } => Error::Loop {
        ancestor: ancestor.clone(),
        child: child.clone(),
      },
      Error::Io(ref err) => match err.raw_os_error() {
        Some(e) => Error::Io(std::io::Error::from_raw_os_error(e)),
        None => Error::Io(std::io::Error::new(err.kind(), err.to_string())),
      },
      Error::Glob { ref glob, ref err } => Error::Glob {
        glob: glob.clone(),
        err: err.clone(),
      },
      Error::UnrecognizedFileType(ref err) => Error::UnrecognizedFileType(err.clone()),
      Error::InvalidDefinition => Error::InvalidDefinition,
    }
  }
}

impl Error {
  pub fn is_partial(&self) -> bool {
    match *self {
      Error::Partial(_) => true,
      Error::WithLineNumber { ref err, .. } => err.is_partial(),
      Error::WithPath { ref err, .. } => err.is_partial(),
      Error::WithDepth { ref err, .. } => err.is_partial(),
      _ => false,
    }
  }

  /// Returns true if this error is exclusively an I/O error.
  pub fn is_io(&self) -> bool {
    match *self {
      Error::Partial(ref errs) => errs.len() == 1 && errs[0].is_io(),
      Error::WithLineNumber { ref err, .. } => err.is_io(),
      Error::WithPath { ref err, .. } => err.is_io(),
      Error::WithDepth { ref err, .. } => err.is_io(),
      Error::Loop { .. } => false,
      Error::Io(_) => true,
      Error::Glob { .. } => false,
      Error::UnrecognizedFileType(_) => false,
      Error::InvalidDefinition => false,
    }
  }

  pub fn io_error(&self) -> Option<&std::io::Error> {
    match *self {
      Error::Partial(ref errs) => {
        if errs.len() == 1 {
          errs[0].io_error()
        } else {
          None
        }
      }
      Error::WithLineNumber { ref err, .. } => err.io_error(),
      Error::WithPath { ref err, .. } => err.io_error(),
      Error::WithDepth { ref err, .. } => err.io_error(),
      Error::Loop { .. } => None,
      Error::Io(ref err) => Some(err),
      Error::Glob { .. } => None,
      Error::UnrecognizedFileType(_) => None,
      Error::InvalidDefinition => None,
    }
  }

  /// Similar to [`io_error`] except consumes self to convert to the original
  /// [`std::io::Error`] if one exists.
  ///
  /// [`io_error`]: struct.Error.html#method.io_error
  /// [`std::io::Error`]: https://doc.rust-lang.org/stable/std/io/struct.Error.html
  pub fn into_io_error(self) -> Option<std::io::Error> {
    match self {
      Error::Partial(mut errs) => {
        if errs.len() == 1 {
          errs.remove(0).into_io_error()
        } else {
          None
        }
      }
      Error::WithLineNumber { err, .. } => err.into_io_error(),
      Error::WithPath { err, .. } => err.into_io_error(),
      Error::WithDepth { err, .. } => err.into_io_error(),
      Error::Loop { .. } => None,
      Error::Io(err) => Some(err),
      Error::Glob { .. } => None,
      Error::UnrecognizedFileType(_) => None,
      Error::InvalidDefinition => None,
    }
  }

  /// Returns a depth associated with recursively walking a directory (if
  /// this error was generated from a recursive directory iterator).
  pub fn depth(&self) -> Option<usize> {
    match *self {
      Error::WithPath { ref err, .. } => err.depth(),
      Error::WithDepth { depth, .. } => Some(depth),
      _ => None,
    }
  }

  /// Turn an error into a tagged error with the given file path.
  fn with_path<P: AsRef<Path>>(self, path: P) -> Error {
    Error::WithPath {
      path: path.as_ref().to_path_buf(),
      err: Box::new(self),
    }
  }

  /// Turn an error into a tagged error with the given depth.
  fn with_depth(self, depth: usize) -> Error {
    Error::WithDepth {
      depth,
      err: Box::new(self),
    }
  }

  /// Turn an error into a tagged error with the given file path and line
  /// number. If path is empty, then it is omitted from the error.
  fn tagged<P: AsRef<Path>>(self, path: P, lineno: u64) -> Error {
    let errline = Error::WithLineNumber {
      line: lineno,
      err: Box::new(self),
    };
    if path.as_ref().as_os_str().is_empty() {
      return errline;
    }
    errline.with_path(path)
  }

  /// Build an error from a walkdir error.
  fn from_walkdir(err: crate::walkdir::Error) -> Error {
    let depth = err.depth();
    if let (Some(anc), Some(child)) = (err.loop_ancestor(), err.path()) {
      return Error::WithDepth {
        depth,
        err: Box::new(Error::Loop {
          ancestor: anc.to_path_buf(),
          child: child.to_path_buf(),
        }),
      };
    }
    let path = err.path().map(|p| p.to_path_buf());
    let mut ig_err = Error::WithDepth {
      depth,
      err: Box::new(Error::Io(std::io::Error::from(err))),
    };
    if let Some(path) = path {
      ig_err = Error::WithPath {
        path,
        err: Box::new(ig_err),
      };
    }
    ig_err
  }
}

impl std::error::Error for Error {
  #[allow(deprecated)]
  fn description(&self) -> &str {
    match *self {
      Error::Partial(_) => "partial error",
      Error::WithLineNumber { ref err, .. } => err.description(),
      Error::WithPath { ref err, .. } => err.description(),
      Error::WithDepth { ref err, .. } => err.description(),
      Error::Loop { .. } => "file system loop found",
      Error::Io(ref err) => err.description(),
      Error::Glob { ref err, .. } => err,
      Error::UnrecognizedFileType(_) => "unrecognized file type",
      Error::InvalidDefinition => "invalid definition",
    }
  }
}

impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match *self {
      Error::Partial(ref errs) => {
        let msgs: Vec<String> = errs.iter().map(|err| err.to_string()).collect();
        write!(f, "{}", msgs.join("\n"))
      }
      Error::WithLineNumber { line, ref err } => {
        write!(f, "line {}: {}", line, err)
      }
      Error::WithPath { ref path, ref err } => {
        write!(f, "{}: {}", path.display(), err)
      }
      Error::WithDepth { ref err, .. } => err.fmt(f),
      Error::Loop {
        ref ancestor,
        ref child,
      } => write!(
        f,
        "File system loop found: \
                           {} points to an ancestor {}",
        child.display(),
        ancestor.display()
      ),
      Error::Io(ref err) => err.fmt(f),
      Error::Glob {
        glob: None,
        ref err,
      } => write!(f, "{}", err),
      Error::Glob {
        glob: Some(ref glob),
        ref err,
      } => {
        write!(f, "error parsing glob '{}': {}", glob, err)
      }
      Error::UnrecognizedFileType(ref ty) => {
        write!(f, "unrecognized file type: {}", ty)
      }
      Error::InvalidDefinition => write!(
        f,
        "invalid definition (format is type:glob, e.g., \
                           html:*.html)"
      ),
    }
  }
}

impl From<std::io::Error> for Error {
  fn from(err: std::io::Error) -> Error {
    Error::Io(err)
  }
}

#[derive(Debug, Default)]
struct PartialErrorBuilder(Vec<Error>);

impl PartialErrorBuilder {
  fn push(&mut self, err: Error) {
    self.0.push(err);
  }

  fn push_ignore_io(&mut self, err: Error) {
    if !err.is_io() {
      self.push(err);
    }
  }

  fn maybe_push(&mut self, err: Option<Error>) {
    if let Some(err) = err {
      self.push(err);
    }
  }

  fn maybe_push_ignore_io(&mut self, err: Option<Error>) {
    if let Some(err) = err {
      self.push_ignore_io(err);
    }
  }

  fn into_error_option(mut self) -> Option<Error> {
    if self.0.is_empty() {
      None
    } else if self.0.len() == 1 {
      Some(self.0.pop().unwrap())
    } else {
      Some(Error::Partial(self.0))
    }
  }
}

/// The result of a glob match.
///
/// The type parameter `T` typically refers to a type that provides more
/// information about a particular match. For example, it might identify
/// the specific gitignore file and the specific glob pattern that caused
/// the match.
#[derive(Clone, Debug)]
pub enum Match<T> {
  /// The path didn't match any glob.
  None,
  /// The highest precedent glob matched indicates the path should be
  /// ignored.
  Ignore(T),
  /// The highest precedent glob matched indicates the path should be
  /// whitelisted.
  Whitelist(T),
}

impl<T> Match<T> {
  /// Returns true if the match result didn't match any globs.
  pub fn is_none(&self) -> bool {
    match *self {
      Match::None => true,
      Match::Ignore(_) | Match::Whitelist(_) => false,
    }
  }

  /// Returns true if the match result implies the path should be ignored.
  pub fn is_ignore(&self) -> bool {
    match *self {
      Match::Ignore(_) => true,
      Match::None | Match::Whitelist(_) => false,
    }
  }

  /// Returns true if the match result implies the path should be
  /// whitelisted.
  pub fn is_whitelist(&self) -> bool {
    match *self {
      Match::Whitelist(_) => true,
      Match::None | Match::Ignore(_) => false,
    }
  }

  /// Inverts the match so that `Ignore` becomes `Whitelist` and
  /// `Whitelist` becomes `Ignore`. A non-match remains the same.
  pub fn invert(self) -> Match<T> {
    match self {
      Match::None => Match::None,
      Match::Ignore(t) => Match::Whitelist(t),
      Match::Whitelist(t) => Match::Ignore(t),
    }
  }

  /// Return the value inside this match if it exists.
  pub fn inner(&self) -> Option<&T> {
    match *self {
      Match::None => None,
      Match::Ignore(ref t) => Some(t),
      Match::Whitelist(ref t) => Some(t),
    }
  }

  /// Apply the given function to the value inside this match.
  ///
  /// If the match has no value, then return the match unchanged.
  pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Match<U> {
    match self {
      Match::None => Match::None,
      Match::Ignore(t) => Match::Ignore(f(t)),
      Match::Whitelist(t) => Match::Whitelist(f(t)),
    }
  }

  /// Return the match if it is not none. Otherwise, return other.
  pub fn or(self, other: Self) -> Self {
    if self.is_none() { other } else { self }
  }
}
