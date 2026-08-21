use helix_loader::grammar::{build_grammars, fetch_grammars};

const STRICT: bool = true;

fn main() {
  // NOTE(pk) temp deactivation for offline build
  if std::env::var("HELIX_DISABLE_AUTO_GRAMMAR_BUILD").is_err() {
    fetch_grammars(STRICT).expect("Failed to fetch tree-sitter grammars");
    build_grammars(Some(std::env::var("TARGET").unwrap()), STRICT)
      .expect("Failed to compile tree-sitter grammars");
  }
}
