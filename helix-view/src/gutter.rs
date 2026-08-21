use std::fmt::Write;

use helix_core::syntax::config::LanguageServerFeature;

use crate::{Document, Editor, Theme, View, editor::GutterType, graphics::Style};

fn count_digits(n: usize) -> usize {
  (usize::checked_ilog10(n).unwrap_or(0) + 1) as usize
}

pub type GutterFn<'doc> = Box<dyn FnMut(usize, bool, bool, &mut String) -> Option<Style> + 'doc>;
pub type Gutter =
  for<'doc> fn(&'doc Editor, &'doc Document, &View, &Theme, bool, usize) -> GutterFn<'doc>;

impl GutterType {
  pub fn style<'doc>(
    self,
    editor: &'doc Editor,
    doc: &'doc Document,
    view: &View,
    theme: &Theme,
    is_focused: bool,
  ) -> GutterFn<'doc> {
    match self {
      GutterType::Diagnostics => diagnostics(editor, doc, view, theme, is_focused),
      GutterType::LineNumbers => line_numbers(editor, doc, view, theme, is_focused),
      GutterType::Spacer => padding(editor, doc, view, theme, is_focused),
      GutterType::CodeActionHint => code_action_hint(editor, doc, view, theme, is_focused),
    }
  }

  pub fn width(self, view: &View, doc: &Document) -> usize {
    match self {
      GutterType::LineNumbers => line_numbers_width(view, doc),
      GutterType::Spacer | GutterType::CodeActionHint | GutterType::Diagnostics => 1,
    }
  }
}

pub fn diagnostic<'doc>(
  _editor: &'doc Editor,
  doc: &'doc Document,
  _view: &View,
  theme: &Theme,
  _is_focused: bool,
) -> GutterFn<'doc> {
  use helix_core::diagnostic::Severity;
  let warning = theme.get("warning");
  let error = theme.get("error");
  let info = theme.get("info");
  let hint = theme.get("hint");
  let diagnostics = &doc.diagnostics;

  Box::new(
    move |line: usize, _selected: bool, first_visual_line: bool, out: &mut String| {
      if !first_visual_line {
        return None;
      }
      let first_diag_idx_maybe_on_line = diagnostics.partition_point(|d| d.line < line);
      let diagnostics_on_line = diagnostics[first_diag_idx_maybe_on_line..]
        .iter()
        .take_while(|d| {
          d.line == line
            && d.provider.language_server_id().is_none_or(|id| {
              doc
                .language_servers_with_feature(LanguageServerFeature::Diagnostics)
                .any(|ls| ls.id() == id)
            })
        });
      diagnostics_on_line.max_by_key(|d| d.severity).map(|d| {
        write!(out, "●").ok();
        match d.severity {
          Some(Severity::Error) => error,
          Some(Severity::Warning) | None => warning,
          Some(Severity::Info) => info,
          Some(Severity::Hint) => hint,
        }
      })
    },
  )
}

pub fn line_numbers<'doc>(
  editor: &'doc Editor,
  doc: &'doc Document,
  view: &View,
  theme: &Theme,
  is_focused: bool,
) -> GutterFn<'doc> {
  let text = doc.text().slice(..);
  let width = line_numbers_width(view, doc);

  let last_line_in_view = view.estimate_last_doc_line(doc);

  // Whether to draw the line number for the last line of the
  // document or not.  We only draw it if it's not an empty line.
  let draw_last = text.line_to_byte(last_line_in_view) < text.len_bytes();

  let linenr = theme.get("ui.linenr");
  let linenr_select = theme.get("ui.linenr.selected");

  let current_line = doc
    .text()
    .char_to_line(doc.selection(view.id).primary().cursor(text));

  let line_number = editor.config().line_number;
  let mode = editor.mode;

  Box::new(
    move |line: usize, selected: bool, first_visual_line: bool, out: &mut String| {
      if line == last_line_in_view && !draw_last {
        write!(out, "{:>1$}", '~', width).unwrap();
        Some(linenr)
      } else {
        use crate::{document::Mode, editor::LineNumber};

        let relative = line_number == LineNumber::Relative
          && mode != Mode::Insert
          && is_focused
          && current_line != line;

        let display_num = if relative {
          current_line.abs_diff(line)
        } else {
          line + 1
        };

        let style = if selected && is_focused {
          linenr_select
        } else {
          linenr
        };

        if first_visual_line {
          write!(out, "{display_num:>width$}").unwrap();
        } else {
          write!(out, "{:>1$}", " ", width).unwrap();
        }

        first_visual_line.then_some(style)
      }
    },
  )
}

/// The width of a "line-numbers" gutter
///
/// The width of the gutter depends on the number of lines in the document,
/// whether there is content on the last line (the `~` line), and the
/// `editor.gutters.line-numbers.min-width` settings.
fn line_numbers_width(view: &View, doc: &Document) -> usize {
  let text = doc.text();
  let last_line = text.len_lines().saturating_sub(1);
  let draw_last = text.line_to_byte(last_line) < text.len_bytes();
  let last_drawn = if draw_last { last_line + 1 } else { last_line };
  let digits = count_digits(last_drawn);
  let n_min = view.gutters.line_numbers.min_width;
  digits.max(n_min)
}

pub fn padding<'doc>(
  _editor: &'doc Editor,
  _doc: &'doc Document,
  _view: &View,
  _theme: &Theme,
  _is_focused: bool,
) -> GutterFn<'doc> {
  Box::new(|_line: usize, _selected: bool, _first_visual_line: bool, _out: &mut String| None)
}

pub fn diagnostics<'doc>(
  editor: &'doc Editor,
  doc: &'doc Document,
  view: &View,
  theme: &Theme,
  is_focused: bool,
) -> GutterFn<'doc> {
  let mut diagnostics = diagnostic(editor, doc, view, theme, is_focused);
  Box::new(move |line, selected, first_visual_line: bool, out| {
    diagnostics(line, selected, first_visual_line, out)
  })
}

pub fn code_action_hint<'doc>(
  _editor: &'doc Editor,
  doc: &'doc Document,
  view: &View,
  theme: &Theme,
  is_focused: bool,
) -> GutterFn<'doc> {
  let style = theme.get("ui.text");
  let text = doc.text().slice(..);
  let show_hint = doc.code_action_hints(view.id);
  let current_line = doc
    .text()
    .char_to_line(doc.selection(view.id).primary().cursor(text));

  Box::new(
    move |line: usize, _selected: bool, first_visual_line: bool, out: &mut String| {
      (is_focused && show_hint && current_line == line && first_visual_line).then(|| {
        write!(out, "⋮").unwrap();
        style
      })
    },
  )
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use super::*;
  use crate::DocumentId;
  use crate::document::Document;
  use crate::editor::{Config, GutterConfig, GutterLineNumbersConfig};
  use crate::graphics::Rect;
  use arc_swap::ArcSwap;
  use helix_core::{Rope, syntax};

  #[test]
  fn test_default_gutter_widths() {
    let mut view = View::new(DocumentId::default(), GutterConfig::default());
    view.area = Rect::new(40, 40, 40, 40);

    let rope = Rope::from_str("abc\n\tdef");
    let doc = Document::from(
      rope,
      None,
      Arc::new(ArcSwap::new(Arc::new(Config::default()))),
      Arc::new(ArcSwap::from_pointee(syntax::Loader::default())),
    );

    assert_eq!(view.gutters.layout.len(), 5);
    assert_eq!(view.gutters.layout[0].width(&view, &doc), 1);
    assert_eq!(view.gutters.layout[1].width(&view, &doc), 1);
    assert_eq!(view.gutters.layout[2].width(&view, &doc), 3);
    assert_eq!(view.gutters.layout[3].width(&view, &doc), 1);
    assert_eq!(view.gutters.layout[4].width(&view, &doc), 1);
  }

  #[test]
  fn test_configured_gutter_widths() {
    let gutters = GutterConfig {
      layout: vec![GutterType::Diagnostics],
      ..Default::default()
    };

    let mut view = View::new(DocumentId::default(), gutters);
    view.area = Rect::new(40, 40, 40, 40);

    let rope = Rope::from_str("abc\n\tdef");
    let doc = Document::from(
      rope,
      None,
      Arc::new(ArcSwap::new(Arc::new(Config::default()))),
      Arc::new(ArcSwap::from_pointee(syntax::Loader::default())),
    );

    assert_eq!(view.gutters.layout.len(), 1);
    assert_eq!(view.gutters.layout[0].width(&view, &doc), 1);

    let gutters = GutterConfig {
      layout: vec![GutterType::Diagnostics, GutterType::LineNumbers],
      line_numbers: GutterLineNumbersConfig { min_width: 10 },
    };

    let mut view = View::new(DocumentId::default(), gutters);
    view.area = Rect::new(40, 40, 40, 40);

    let rope = Rope::from_str("abc\n\tdef");
    let doc = Document::from(
      rope,
      None,
      Arc::new(ArcSwap::new(Arc::new(Config::default()))),
      Arc::new(ArcSwap::from_pointee(syntax::Loader::default())),
    );

    assert_eq!(view.gutters.layout.len(), 2);
    assert_eq!(view.gutters.layout[0].width(&view, &doc), 1);
    assert_eq!(view.gutters.layout[1].width(&view, &doc), 10);
  }

  #[test]
  fn test_line_numbers_gutter_width_resizes() {
    let gutters = GutterConfig {
      layout: vec![GutterType::Diagnostics, GutterType::LineNumbers],
      line_numbers: GutterLineNumbersConfig { min_width: 1 },
    };

    let mut view = View::new(DocumentId::default(), gutters);
    view.area = Rect::new(40, 40, 40, 40);

    let rope = Rope::from_str("a\nb");
    let doc_short = Document::from(
      rope,
      None,
      Arc::new(ArcSwap::new(Arc::new(Config::default()))),
      Arc::new(ArcSwap::from_pointee(syntax::Loader::default())),
    );

    let rope = Rope::from_str("a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn\no\np");
    let doc_long = Document::from(
      rope,
      None,
      Arc::new(ArcSwap::new(Arc::new(Config::default()))),
      Arc::new(ArcSwap::from_pointee(syntax::Loader::default())),
    );

    assert_eq!(view.gutters.layout.len(), 2);
    assert_eq!(view.gutters.layout[1].width(&view, &doc_short), 1);
    assert_eq!(view.gutters.layout[1].width(&view, &doc_long), 2);
  }
}
