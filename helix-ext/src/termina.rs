pub(crate) mod base64 {

  use core::ops::{BitAnd, BitOr, Shl, Shr};

  const PAD_BYTE: u8 = b'=';
  const ENCODE_TABLE: &[u8] =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".as_bytes();
  const LOW_SIX_BITS: u32 = 0x3F;

  pub(crate) fn base64_encode(input: &[u8]) -> String {
    let rem = input.len() % 3;
    let complete_chunks = input.len() / 3;
    let remainder_chunk = usize::from(rem != 0);
    let encoded_size = (complete_chunks + remainder_chunk) * 4;

    let mut output = vec![0; encoded_size];

    let complete_chunk_len = input.len() - rem;

    let mut input_index = 0_usize;
    let mut output_index = 0_usize;
    while input_index < complete_chunk_len {
      let chunk = &input[input_index..input_index + 3];

      let chunk_int: u32 = (chunk[0] as u32).shl(16) | (chunk[1] as u32).shl(8) | (chunk[2] as u32);
      output[output_index] = ENCODE_TABLE[chunk_int.shr(18) as usize];
      output[output_index + 1] = ENCODE_TABLE[chunk_int.shr(12_u8).bitand(LOW_SIX_BITS) as usize];
      output[output_index + 2] = ENCODE_TABLE[chunk_int.shr(6_u8).bitand(LOW_SIX_BITS) as usize];
      output[output_index + 3] = ENCODE_TABLE[chunk_int.bitand(LOW_SIX_BITS) as usize];

      input_index += 3;
      output_index += 4;
    }

    if rem == 2 {
      let chunk = &input[input_index..input_index + 2];

      output[output_index] = ENCODE_TABLE[chunk[0].shr(2) as usize];
      output[output_index + 1] = ENCODE_TABLE
        [(chunk[0].shl(4_u8).bitor(chunk[1].shr(4_u8)) as u32).bitand(LOW_SIX_BITS) as usize];
      output[output_index + 2] =
        ENCODE_TABLE[(chunk[1].shl(2_u8) as u32).bitand(LOW_SIX_BITS) as usize];
      output[output_index + 3] = PAD_BYTE;
    } else if rem == 1 {
      let byte = input[input_index];
      output[output_index] = ENCODE_TABLE[byte.shr(2) as usize];
      output[output_index + 1] =
        ENCODE_TABLE[(byte.shl(4_u8) as u32).bitand(LOW_SIX_BITS) as usize];
      output[output_index + 2] = PAD_BYTE;
      output[output_index + 3] = PAD_BYTE;
    }
    String::from_utf8(output).expect("Invalid UTF8")
  }
}
pub mod escape {

  pub mod csi {
    use std::{
      fmt::{self, Display},
      num::NonZeroU16,
    };

    use crate::termina::{
      OneBased,
      event::Modifiers,
      style::{
        Blink, ColorSpec, CursorStyle, Font, Intensity, RgbaColor, Underline, VerticalAlign,
      },
    };

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Csi {
      Sgr(Sgr),

      Cursor(Cursor),

      Edit(Edit),

      Mode(Mode),

      Mouse(MouseReport),

      Keyboard(Keyboard),

      Device(Device),

      Window(Box<Window>),
    }

    impl Display for Csi {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(super::CSI)?;
        match self {
          Self::Sgr(sgr) => write!(f, "{sgr}m"),
          Self::Cursor(cursor) => cursor.fmt(f),
          Self::Edit(edit) => edit.fmt(f),
          Self::Mode(mode) => mode.fmt(f),
          Self::Mouse(report) => report.fmt(f),
          Self::Keyboard(keyboard) => keyboard.fmt(f),
          Self::Device(device) => device.fmt(f),
          Self::Window(window) => window.fmt(f),
        }
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Sgr {
      Reset,
      Intensity(Intensity),
      Underline(Underline),
      Blink(Blink),
      Italic(bool),
      Reverse(bool),
      Invisible(bool),
      StrikeThrough(bool),
      Overline(bool),
      Font(Font),
      VerticalAlign(VerticalAlign),
      Foreground(ColorSpec),
      Background(ColorSpec),
      UnderlineColor(ColorSpec),
      Attributes(SgrAttributes),
    }

    impl Display for Sgr {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn write_true_color(
          code: u8,
          RgbaColor {
            red,
            green,
            blue,
            alpha,
          }: RgbaColor,
          f: &mut fmt::Formatter,
        ) -> fmt::Result {
          if alpha == 255 {
            write!(f, "{code};2;{red};{green};{blue}")
          } else {
            write!(f, "{code}:6::{red}:{green}:{blue}:{alpha}")
          }
        }

        match self {
          Self::Reset => (),
          Self::Intensity(Intensity::Normal) => write!(f, "22")?,
          Self::Intensity(Intensity::Bold) => write!(f, "1")?,
          Self::Intensity(Intensity::Dim) => write!(f, "2")?,
          Self::Underline(Underline::None) => write!(f, "24")?,
          Self::Underline(Underline::Single) => write!(f, "4")?,
          Self::Underline(Underline::Double) => write!(f, "21")?,
          Self::Underline(Underline::Curly) => write!(f, "4:3")?,
          Self::Underline(Underline::Dotted) => write!(f, "4:4")?,
          Self::Underline(Underline::Dashed) => write!(f, "4:5")?,
          Self::Blink(Blink::None) => write!(f, "25")?,
          Self::Blink(Blink::Slow) => write!(f, "5")?,
          Self::Blink(Blink::Rapid) => write!(f, "6")?,
          Self::Italic(true) => write!(f, "3")?,
          Self::Italic(false) => write!(f, "23")?,
          Self::Reverse(true) => write!(f, "7")?,
          Self::Reverse(false) => write!(f, "27")?,
          Self::Invisible(true) => write!(f, "8")?,
          Self::Invisible(false) => write!(f, "28")?,
          Self::StrikeThrough(true) => write!(f, "9")?,
          Self::StrikeThrough(false) => write!(f, "29")?,
          Self::Overline(true) => write!(f, "53")?,
          Self::Overline(false) => write!(f, "55")?,
          Self::Font(Font::Default) => write!(f, "10")?,
          Self::Font(Font::Alternate(1)) => write!(f, "11")?,
          Self::Font(Font::Alternate(2)) => write!(f, "12")?,
          Self::Font(Font::Alternate(3)) => write!(f, "13")?,
          Self::Font(Font::Alternate(4)) => write!(f, "14")?,
          Self::Font(Font::Alternate(5)) => write!(f, "15")?,
          Self::Font(Font::Alternate(6)) => write!(f, "16")?,
          Self::Font(Font::Alternate(7)) => write!(f, "17")?,
          Self::Font(Font::Alternate(8)) => write!(f, "18")?,
          Self::Font(Font::Alternate(9)) => write!(f, "19")?,
          Self::Font(_) => (),
          Self::VerticalAlign(VerticalAlign::BaseLine) => write!(f, "75")?,
          Self::VerticalAlign(VerticalAlign::SuperScript) => write!(f, "73")?,
          Self::VerticalAlign(VerticalAlign::SubScript) => write!(f, "74")?,
          Self::Foreground(ColorSpec::Reset) => write!(f, "39")?,
          Self::Foreground(ColorSpec::BLACK) => write!(f, "30")?,
          Self::Foreground(ColorSpec::RED) => write!(f, "31")?,
          Self::Foreground(ColorSpec::GREEN) => write!(f, "32")?,
          Self::Foreground(ColorSpec::YELLOW) => write!(f, "33")?,
          Self::Foreground(ColorSpec::BLUE) => write!(f, "34")?,
          Self::Foreground(ColorSpec::MAGENTA) => write!(f, "35")?,
          Self::Foreground(ColorSpec::CYAN) => write!(f, "36")?,
          Self::Foreground(ColorSpec::WHITE) => write!(f, "37")?,
          Self::Foreground(ColorSpec::BRIGHT_BLACK) => write!(f, "90")?,
          Self::Foreground(ColorSpec::BRIGHT_RED) => write!(f, "91")?,
          Self::Foreground(ColorSpec::BRIGHT_GREEN) => write!(f, "92")?,
          Self::Foreground(ColorSpec::BRIGHT_YELLOW) => write!(f, "93")?,
          Self::Foreground(ColorSpec::BRIGHT_BLUE) => write!(f, "94")?,
          Self::Foreground(ColorSpec::BRIGHT_MAGENTA) => write!(f, "95")?,
          Self::Foreground(ColorSpec::BRIGHT_CYAN) => write!(f, "96")?,
          Self::Foreground(ColorSpec::BRIGHT_WHITE) => write!(f, "97")?,
          Self::Foreground(ColorSpec::PaletteIndex(idx)) => write!(f, "38;5;{idx}")?,
          Self::Foreground(ColorSpec::TrueColor(color)) => write_true_color(38, *color, f)?,
          Self::Background(ColorSpec::Reset) => write!(f, "49")?,
          Self::Background(ColorSpec::BLACK) => write!(f, "40")?,
          Self::Background(ColorSpec::RED) => write!(f, "41")?,
          Self::Background(ColorSpec::GREEN) => write!(f, "42")?,
          Self::Background(ColorSpec::YELLOW) => write!(f, "43")?,
          Self::Background(ColorSpec::BLUE) => write!(f, "44")?,
          Self::Background(ColorSpec::MAGENTA) => write!(f, "45")?,
          Self::Background(ColorSpec::CYAN) => write!(f, "46")?,
          Self::Background(ColorSpec::WHITE) => write!(f, "47")?,
          Self::Background(ColorSpec::BRIGHT_BLACK) => write!(f, "100")?,
          Self::Background(ColorSpec::BRIGHT_RED) => write!(f, "101")?,
          Self::Background(ColorSpec::BRIGHT_GREEN) => write!(f, "102")?,
          Self::Background(ColorSpec::BRIGHT_YELLOW) => write!(f, "103")?,
          Self::Background(ColorSpec::BRIGHT_BLUE) => write!(f, "104")?,
          Self::Background(ColorSpec::BRIGHT_MAGENTA) => write!(f, "105")?,
          Self::Background(ColorSpec::BRIGHT_CYAN) => write!(f, "106")?,
          Self::Background(ColorSpec::BRIGHT_WHITE) => write!(f, "107")?,
          Self::Background(ColorSpec::PaletteIndex(idx)) => write!(f, "48;5;{idx}")?,
          Self::Background(ColorSpec::TrueColor(color)) => write_true_color(48, *color, f)?,
          Self::UnderlineColor(ColorSpec::Reset) => write!(f, "59")?,
          Self::UnderlineColor(ColorSpec::PaletteIndex(idx)) => write!(f, "58:5:{idx}")?,
          Self::UnderlineColor(ColorSpec::TrueColor(RgbaColor {
            red,
            green,
            blue,
            alpha: 255,
          })) => {
            write!(f, "58:2::{red}:{green}:{blue}")?;
          }
          Self::UnderlineColor(ColorSpec::TrueColor(RgbaColor {
            red,
            green,
            blue,
            alpha,
          })) => {
            write!(f, "58:6::{red}:{green}:{blue}:{alpha}")?;
          }
          Self::Attributes(attributes) => {
            use SgrModifiers as Mod;

            let ps_budget = attributes.parameter_chunk_size.get();
            let mut ps_written = 0;
            let mut first = true;
            let mut write = |sgr: Self, n_ps: u16| {
              ps_written += n_ps;
              if ps_written > ps_budget {
                write!(f, "m{}", super::CSI)?;
                ps_written = n_ps;
              } else if !first {
                f.write_str(";")?;
              }
              first = false;
              write!(f, "{sgr}")
            };
            if attributes.modifiers.contains(Mod::RESET) {
              write(Self::Reset, 0)?;
            }
            if let Some(color) = attributes.foreground {
              write(
                Self::Foreground(color),
                match color {
                  ColorSpec::Reset => 1,
                  ColorSpec::PaletteIndex(_) => 3,
                  ColorSpec::TrueColor(RgbaColor { alpha: 255, .. }) => 5,
                  ColorSpec::TrueColor(_) => 6,
                },
              )?;
            }
            if let Some(color) = attributes.background {
              write(
                Self::Background(color),
                match color {
                  ColorSpec::Reset => 1,
                  ColorSpec::PaletteIndex(_) => 3,
                  ColorSpec::TrueColor(RgbaColor { alpha: 255, .. }) => 5,
                  ColorSpec::TrueColor(_) => 6,
                },
              )?;
            }
            if let Some(color) = attributes.underline_color {
              write(
                Self::UnderlineColor(color),
                match color {
                  ColorSpec::Reset => 1,
                  ColorSpec::PaletteIndex(_) => 3,
                  ColorSpec::TrueColor(_) => 6,
                },
              )?;
            }
            if attributes.modifiers.contains(Mod::INTENSITY_NORMAL) {
              write(Self::Intensity(Intensity::Normal), 1)?;
            }
            if attributes.modifiers.contains(Mod::INTENSITY_DIM) {
              write(Self::Intensity(Intensity::Dim), 1)?;
            }
            if attributes.modifiers.contains(Mod::INTENSITY_BOLD) {
              write(Self::Intensity(Intensity::Bold), 1)?;
            }
            if attributes.modifiers.contains(Mod::UNDERLINE_NONE) {
              write(Self::Underline(Underline::None), 1)?;
            }
            if attributes.modifiers.contains(Mod::UNDERLINE_SINGLE) {
              write(Self::Underline(Underline::Single), 1)?;
            }
            if attributes.modifiers.contains(Mod::UNDERLINE_DOUBLE) {
              write(Self::Underline(Underline::Double), 1)?;
            }
            if attributes.modifiers.contains(Mod::UNDERLINE_CURLY) {
              write(Self::Underline(Underline::Curly), 2)?;
            }
            if attributes.modifiers.contains(Mod::UNDERLINE_DOTTED) {
              write(Self::Underline(Underline::Dotted), 2)?;
            }
            if attributes.modifiers.contains(Mod::UNDERLINE_DASHED) {
              write(Self::Underline(Underline::Dashed), 2)?;
            }
            if attributes.modifiers.contains(Mod::BLINK_NONE) {
              write(Self::Blink(Blink::None), 1)?;
            }
            if attributes.modifiers.contains(Mod::BLINK_SLOW) {
              write(Self::Blink(Blink::Slow), 1)?;
            }
            if attributes.modifiers.contains(Mod::BLINK_RAPID) {
              write(Self::Blink(Blink::Rapid), 1)?;
            }
            if attributes.modifiers.contains(Mod::ITALIC) {
              write(Self::Italic(true), 1)?;
            }
            if attributes.modifiers.contains(Mod::NO_ITALIC) {
              write(Self::Italic(false), 1)?;
            }
            if attributes.modifiers.contains(Mod::REVERSE) {
              write(Self::Reverse(true), 1)?;
            }
            if attributes.modifiers.contains(Mod::NO_REVERSE) {
              write(Self::Reverse(false), 1)?;
            }
            if attributes.modifiers.contains(Mod::INVISIBLE) {
              write(Self::Invisible(true), 1)?;
            }
            if attributes.modifiers.contains(Mod::NO_INVISIBLE) {
              write(Self::Invisible(false), 1)?;
            }
            if attributes.modifiers.contains(Mod::STRIKE_THROUGH) {
              write(Self::StrikeThrough(true), 1)?;
            }
            if attributes.modifiers.contains(Mod::NO_STRIKE_THROUGH) {
              write(Self::StrikeThrough(false), 1)?;
            }
          }
        }
        Ok(())
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SgrAttributes {
      pub foreground: Option<ColorSpec>,

      pub background: Option<ColorSpec>,

      underline_color: Option<ColorSpec>,

      pub modifiers: SgrModifiers,

      parameter_chunk_size: NonZeroU16,
    }

    impl Default for SgrAttributes {
      fn default() -> Self {
        Self {
          foreground: Default::default(),
          background: Default::default(),
          underline_color: Default::default(),
          modifiers: Default::default(),
          parameter_chunk_size: unsafe { NonZeroU16::new_unchecked(10) },
        }
      }
    }

    impl SgrAttributes {
      #[inline]
      pub fn is_empty(&self) -> bool {
        self.foreground.is_none()
          && self.background.is_none()
          && self.underline_color.is_none()
          && self.modifiers.is_empty()
      }
    }

    bitflags::bitflags! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct SgrModifiers: u32 {
            const NONE = 0;

            const RESET = 1 << 1;

            const INTENSITY_NORMAL = 1 << 2;

            const INTENSITY_DIM = 1 << 3;

            const INTENSITY_BOLD = 1 << 4;

            const UNDERLINE_NONE = 1 << 5;

            const UNDERLINE_SINGLE = 1 << 6;

            const UNDERLINE_DOUBLE = 1 << 7;

            const UNDERLINE_CURLY = 1 << 8;

            const UNDERLINE_DOTTED = 1 << 9;

            const UNDERLINE_DASHED = 1 << 10;

            const BLINK_NONE = 1 << 11;

            const BLINK_SLOW = 1 << 12;

            const BLINK_RAPID = 1 << 13;

            const ITALIC = 1 << 14;

            const NO_ITALIC = 1 << 15;

            const REVERSE = 1 << 16;

            const NO_REVERSE = 1 << 17;

            const INVISIBLE = 1 << 18;

            const NO_INVISIBLE = 1 << 19;

            const STRIKE_THROUGH = 1 << 20;

            const NO_STRIKE_THROUGH = 1 << 21;
        }
    }

    impl Default for SgrModifiers {
      fn default() -> Self {
        Self::NONE
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MultiCursorShape {
      Style(CursorStyle),

      FollowMainCursor,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MultiCursorCapability {
      BlockShape = 1,
      BeamShape = 2,
      UnderlineShape = 3,
      FollowMainCursorShape = 29,
      TextColor = 30,
      CursorColor = 40,
      QueryCurrentCursors = 100,
      QueryColors = 101,
    }

    impl TryFrom<u8> for MultiCursorCapability {
      type Error = u8;

      fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
          1 => Ok(Self::BlockShape),
          2 => Ok(Self::BeamShape),
          3 => Ok(Self::UnderlineShape),
          29 => Ok(Self::FollowMainCursorShape),
          30 => Ok(Self::TextColor),
          40 => Ok(Self::CursorColor),
          100 => Ok(Self::QueryCurrentCursors),
          101 => Ok(Self::QueryColors),
          _ => Err(value),
        }
      }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Cursor {
      BackwardTabulation(u32),

      TabulationClear(TabulationClear),

      CharacterAbsolute(OneBased),

      CharacterPositionAbsolute(OneBased),

      CharacterPositionBackward(u32),

      CharacterPositionForward(u32),

      CharacterAndLinePosition {
        line: OneBased,
        col: OneBased,
      },

      LinePositionAbsolute(u32),

      LinePositionBackward(u32),

      LinePositionForward(u32),

      ForwardTabulation(u32),

      NextLine(u32),

      PrecedingLine(u32),

      ActivePositionReport {
        line: OneBased,
        col: OneBased,
      },

      RequestActivePositionReport,

      SaveCursor,

      RestoreCursor,

      TabulationControl(CursorTabulationControl),

      Left(u32),

      Down(u32),

      Right(u32),

      Up(u32),

      Position {
        line: OneBased,
        col: OneBased,
      },

      LineTabulation(u32),

      SetTopAndBottomMargins {
        top: OneBased,
        bottom: OneBased,
      },

      SetLeftAndRightMargins {
        left: OneBased,
        right: OneBased,
      },

      CursorStyle(CursorStyle),

      QueryCursorShape,

      CursorShapeQueryResponse(Vec<MultiCursorCapability>),

      SetMultipleCursors {
        shape: MultiCursorShape,

        positions: Vec<(OneBased, OneBased)>,
      },

      ClearSecondaryCursors,
    }

    impl Display for Cursor {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn write_csi<T: Default + Eq + Display>(
          value: T,
          f: &mut fmt::Formatter<'_>,
          control: &str,
        ) -> fmt::Result {
          if value == T::default() {
            write!(f, "{control}")
          } else {
            write!(f, "{value}{control}")
          }
        }

        match self {
          Cursor::BackwardTabulation(n) => write_csi(*n, f, "Z"),
          Cursor::TabulationClear(n) => write_csi(*n, f, "g"),
          Cursor::CharacterAbsolute(n) => write_csi(*n, f, "G"),
          Cursor::CharacterPositionAbsolute(n) => write_csi(*n, f, "``"),
          Cursor::CharacterPositionBackward(n) => write_csi(*n, f, "j"),
          Cursor::CharacterPositionForward(n) => write_csi(*n, f, "a"),
          Cursor::CharacterAndLinePosition { line, col } => write!(f, "{line};{col}f"),
          Cursor::LinePositionAbsolute(n) => write_csi(*n, f, "d"),
          Cursor::LinePositionBackward(n) => write_csi(*n, f, "k"),
          Cursor::LinePositionForward(n) => write_csi(*n, f, "e"),
          Cursor::ForwardTabulation(n) => write_csi(*n, f, "I"),
          Cursor::NextLine(n) => write_csi(*n, f, "E"),
          Cursor::PrecedingLine(n) => write_csi(*n, f, "F"),
          Cursor::ActivePositionReport { line, col } => write!(f, "{line};{col}R"),
          Cursor::RequestActivePositionReport => write!(f, "6n"),
          Cursor::SaveCursor => write!(f, "s"),
          Cursor::RestoreCursor => write!(f, "u"),
          Cursor::TabulationControl(n) => write_csi(*n, f, "W"),
          Cursor::Left(n) => write_csi(*n, f, "D"),
          Cursor::Down(n) => write_csi(*n, f, "B"),
          Cursor::Right(n) => write_csi(*n, f, "C"),
          Cursor::Up(n) => write_csi(*n, f, "A"),
          Cursor::Position { line, col } => write!(f, "{line};{col}H"),
          Cursor::LineTabulation(n) => write_csi(*n, f, "Y"),
          Cursor::SetTopAndBottomMargins { top, bottom } => {
            if top.get() == 1 && bottom.get() == u16::MAX {
              write!(f, "r")
            } else {
              write!(f, "{top};{bottom}r")
            }
          }
          Cursor::SetLeftAndRightMargins { left, right } => {
            if left.get() == 1 && right.get() == u16::MAX {
              write!(f, "s")
            } else {
              write!(f, "{left};{right}s")
            }
          }
          Cursor::CursorStyle(style) => write!(f, "{} q", *style as u8),
          Cursor::QueryCursorShape => write!(f, "> q"),
          Cursor::CursorShapeQueryResponse(caps) => {
            write!(f, ">")?;
            for (i, cap) in caps.iter().enumerate() {
              if i > 0 {
                write!(f, ";")?;
              }
              write!(f, "{}", *cap as u8)?;
            }
            write!(f, " q")
          }
          Cursor::SetMultipleCursors { shape, positions } => {
            write!(
              f,
              ">{}",
              match shape {
                MultiCursorShape::Style(style) => *style as u8,
                MultiCursorShape::FollowMainCursor => 29,
              }
            )?;
            for (line, col) in positions {
              write!(f, ";2:{}:{}", line, col)?;
            }
            write!(f, " q")
          }
          Cursor::ClearSecondaryCursors => write!(f, ">0;4 q"),
        }
      }
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub enum CursorTabulationControl {
      #[default]
      SetCharacterTabStopAtActivePosition = 0,

      SetLineTabStopAtActiveLine = 1,

      ClearCharacterTabStopAtActivePosition = 2,

      ClearLineTabstopAtActiveLine = 3,

      ClearAllCharacterTabStopsAtActiveLine = 4,

      ClearAllCharacterTabStops = 5,

      ClearAllLineTabStops = 6,
    }

    impl Display for CursorTabulationControl {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8)
      }
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub enum TabulationClear {
      #[default]
      ClearCharacterTabStopAtActivePosition = 0,

      ClearLineTabStopAtActiveLine = 1,

      ClearCharacterTabStopsAtActiveLine = 2,

      ClearAllCharacterTabStops = 3,

      ClearAllLineTabStops = 4,

      ClearAllTabStops = 5,
    }

    impl Display for TabulationClear {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8)
      }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Edit {
      DeleteCharacter(u32),

      DeleteLine(u32),

      EraseCharacter(u32),

      EraseInLine(EraseInLine),

      InsertCharacter(u32),

      InsertLine(u32),

      ScrollDown(u32),

      ScrollUp(u32),

      EraseInDisplay(EraseInDisplay),

      Repeat(u32),
    }

    impl Display for Edit {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn write_csi(param: u32, f: &mut fmt::Formatter<'_>, control: &str) -> fmt::Result {
          if param == 1 {
            write!(f, "{control}")
          } else {
            write!(f, "{param}{control}")
          }
        }

        match self {
          Self::DeleteCharacter(n) => write_csi(*n, f, "P"),
          Self::DeleteLine(n) => write_csi(*n, f, "M"),
          Self::EraseCharacter(n) => write_csi(*n, f, "X"),
          Self::EraseInLine(n) => write_csi(*n as u32, f, "K"),
          Self::InsertCharacter(n) => write_csi(*n, f, "@"),
          Self::InsertLine(n) => write_csi(*n, f, "L"),
          Self::ScrollDown(n) => write_csi(*n, f, "T"),
          Self::ScrollUp(n) => write_csi(*n, f, "S"),
          Self::EraseInDisplay(n) => write_csi(*n as u32, f, "J"),
          Self::Repeat(n) => write_csi(*n, f, "b"),
        }
      }
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub enum EraseInLine {
      #[default]
      EraseToEndOfLine = 0,

      EraseToStartOfLine = 1,

      EraseLine = 2,
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub enum EraseInDisplay {
      #[default]
      EraseToEndOfDisplay = 0,
      EraseToStartOfDisplay = 1,
      EraseDisplay = 2,
      EraseScrollback = 3,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Mode {
      SetDecPrivateMode(DecPrivateMode),

      ResetDecPrivateMode(DecPrivateMode),

      SaveDecPrivateMode(DecPrivateMode),

      RestoreDecPrivateMode(DecPrivateMode),

      QueryDecPrivateMode(DecPrivateMode),

      ReportDecPrivateMode {
        mode: DecPrivateMode,

        setting: DecModeSetting,
      },

      SetMode(TerminalMode),

      ResetMode(TerminalMode),

      QueryMode(TerminalMode),

      XtermKeyMode {
        resource: XtermKeyModifierResource,

        value: Option<i64>,
      },

      QueryTheme,

      ReportTheme(ThemeMode),
    }

    impl Display for Mode {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
          Self::SetDecPrivateMode(mode) => write!(f, "?{mode}h"),
          Self::ResetDecPrivateMode(mode) => write!(f, "?{mode}l"),
          Self::SaveDecPrivateMode(mode) => write!(f, "?{mode}s"),
          Self::RestoreDecPrivateMode(mode) => write!(f, "?{mode}r"),
          Self::QueryDecPrivateMode(mode) => write!(f, "?{mode}$p"),
          Self::ReportDecPrivateMode { mode, setting } => {
            write!(f, "?{mode};{}$y", *setting as u8)
          }
          Self::SetMode(mode) => write!(f, "{mode}h"),
          Self::ResetMode(mode) => write!(f, "{mode}l"),
          Self::QueryMode(mode) => write!(f, "?{mode}$p"),
          Self::XtermKeyMode { resource, value } => {
            write!(f, ">{}", *resource as u8)?;
            if let Some(value) = value {
              write!(f, ";{}", value)?;
            } else {
              write!(f, ";")?;
            }
            write!(f, "m")
          }
          Self::QueryTheme => write!(f, "?996n"),
          Self::ReportTheme(mode) => write!(f, "?997;{}n", *mode as u8),
        }
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DecPrivateMode {
      Code(DecPrivateModeCode),

      Unspecified(u16),
    }

    impl Display for DecPrivateMode {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match *self {
          Self::Code(code) => code as u16,
          Self::Unspecified(code) => code,
        };
        write!(f, "{code}")
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DecPrivateModeCode {
      ApplicationCursorKeys = 1,

      DecAnsiMode = 2,

      Select132Columns = 3,

      SmoothScroll = 4,

      ReverseVideo = 5,

      OriginMode = 6,

      AutoWrap = 7,

      AutoRepeat = 8,

      StartBlinkingCursor = 12,

      ShowCursor = 25,

      ReverseWraparound = 45,

      LeftRightMarginMode = 69,

      SixelDisplayMode = 80,

      MouseTracking = 1000,

      HighlightMouseTracking = 1001,

      ButtonEventMouse = 1002,

      AnyEventMouse = 1003,

      FocusTracking = 1004,

      Utf8Mouse = 1005,

      SGRMouse = 1006,

      RXVTMouse = 1015,

      SGRPixelsMouse = 1016,

      XTermMetaSendsEscape = 1036,

      XTermAltSendsEscape = 1039,

      SaveCursor = 1048,

      ClearAndEnableAlternateScreen = 1049,

      EnableAlternateScreen = 47,

      OptEnableAlternateScreen = 1047,

      BracketedPaste = 2004,

      GraphemeClustering = 2027,

      Theme = 2031,

      UsePrivateColorRegistersForEachGraphic = 1070,

      SynchronizedOutput = 2026,

      MinTTYApplicationEscapeKeyMode = 7727,

      SixelScrollsRight = 8452,

      Win32InputMode = 9001,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TerminalMode {
      Code(TerminalModeCode),

      Unspecified(u16),
    }

    impl Display for TerminalMode {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match *self {
          Self::Code(code) => code as u16,
          Self::Unspecified(code) => code,
        };
        write!(f, "{code}")
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TerminalModeCode {
      KeyboardAction = 2,

      Insert = 4,

      BiDirectionalSupportMode = 8,

      SendReceive = 12,

      AutomaticNewline = 20,

      ShowCursor = 25,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum XtermKeyModifierResource {
      Keyboard = 0,

      CursorKeys = 1,

      FunctionKeys = 2,

      OtherKeys = 4,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DecModeSetting {
      NotRecognized = 0,

      Set = 1,

      Reset = 2,

      PermanentlySet = 3,

      PermanentlyReset = 4,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ThemeMode {
      Dark = 1,

      Light = 2,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MouseReport {
      Sgr1006 {
        x: u16,

        y: u16,

        button: MouseButton,

        modifiers: Modifiers,
      },

      Sgr1016 {
        x_pixels: u16,

        y_pixels: u16,

        button: MouseButton,

        modifiers: Modifiers,
      },
    }

    impl Display for MouseReport {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
          MouseReport::Sgr1006 {
            x,
            y,
            button,
            modifiers,
          } => {
            let mut b = 0;
            if (*modifiers & Modifiers::SHIFT) != Modifiers::NONE {
              b |= 4;
            }
            if (*modifiers & Modifiers::ALT) != Modifiers::NONE {
              b |= 8;
            }
            if (*modifiers & Modifiers::CONTROL) != Modifiers::NONE {
              b |= 16;
            }
            b |= match button {
              MouseButton::Button1Press | MouseButton::Button1Release => 0,
              MouseButton::Button2Press | MouseButton::Button2Release => 1,
              MouseButton::Button3Press | MouseButton::Button3Release => 2,
              MouseButton::Button4Press | MouseButton::Button4Release => 64,
              MouseButton::Button5Press | MouseButton::Button5Release => 65,
              MouseButton::Button6Press | MouseButton::Button6Release => 66,
              MouseButton::Button7Press | MouseButton::Button7Release => 67,
              MouseButton::Button1Drag => 32,
              MouseButton::Button2Drag => 33,
              MouseButton::Button3Drag => 34,
              MouseButton::None => 35,
            };
            let trailer = match button {
              MouseButton::Button1Press
              | MouseButton::Button2Press
              | MouseButton::Button3Press
              | MouseButton::Button4Press
              | MouseButton::Button5Press
              | MouseButton::Button1Drag
              | MouseButton::Button2Drag
              | MouseButton::Button3Drag
              | MouseButton::None => 'M',
              _ => 'm',
            };
            write!(f, "<{b};{x};{y}{trailer}")
          }
          MouseReport::Sgr1016 {
            x_pixels,
            y_pixels,
            button,
            modifiers,
          } => {
            let mut b = 0;
            if (*modifiers & Modifiers::SHIFT) != Modifiers::NONE {
              b |= 4;
            }
            if (*modifiers & Modifiers::ALT) != Modifiers::NONE {
              b |= 8;
            }
            if (*modifiers & Modifiers::CONTROL) != Modifiers::NONE {
              b |= 16;
            }
            b |= match button {
              MouseButton::Button1Press | MouseButton::Button1Release => 0,
              MouseButton::Button2Press | MouseButton::Button2Release => 1,
              MouseButton::Button3Press | MouseButton::Button3Release => 2,
              MouseButton::Button4Press | MouseButton::Button4Release => 64,
              MouseButton::Button5Press | MouseButton::Button5Release => 65,
              MouseButton::Button6Press | MouseButton::Button6Release => 66,
              MouseButton::Button7Press | MouseButton::Button7Release => 67,
              MouseButton::Button1Drag => 32,
              MouseButton::Button2Drag => 33,
              MouseButton::Button3Drag => 34,
              MouseButton::None => 35,
            };
            let trailer = match button {
              MouseButton::Button1Press
              | MouseButton::Button2Press
              | MouseButton::Button3Press
              | MouseButton::Button4Press
              | MouseButton::Button5Press
              | MouseButton::Button1Drag
              | MouseButton::Button2Drag
              | MouseButton::Button3Drag
              | MouseButton::None => 'M',
              _ => 'm',
            };
            write!(f, "<{b};{x_pixels};{y_pixels}{trailer}")
          }
        }
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MouseButton {
      Button1Press,

      Button2Press,

      Button3Press,

      Button4Press,

      Button5Press,

      Button6Press,

      Button7Press,

      Button1Release,

      Button2Release,

      Button3Release,

      Button4Release,

      Button5Release,

      Button6Release,

      Button7Release,

      Button1Drag,

      Button2Drag,

      Button3Drag,

      None,
    }

    bitflags::bitflags! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct KittyKeyboardFlags: u8 {
            const NONE = 0;

            const DISAMBIGUATE_ESCAPE_CODES = 1;

            const REPORT_EVENT_TYPES = 2;

            const REPORT_ALTERNATE_KEYS = 4;

            const REPORT_ALL_KEYS_AS_ESCAPE_CODES = 8;

            const REPORT_ASSOCIATED_TEXT = 16;
        }
    }

    impl Display for KittyKeyboardFlags {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.bits())
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Keyboard {
      QueryFlags,

      ReportFlags(KittyKeyboardFlags),

      PushFlags(KittyKeyboardFlags),

      PopFlags(u8),

      SetFlags {
        flags: KittyKeyboardFlags,
        mode: SetKeyboardFlagsMode,
      },
    }

    impl Display for Keyboard {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
          Self::QueryFlags => write!(f, "?u"),
          Self::ReportFlags(flags) => write!(f, "?{flags}u"),
          Self::PushFlags(flags) => write!(f, ">{flags}u"),
          Self::PopFlags(number) => write!(f, "<{number}u"),
          Self::SetFlags { flags, mode } => write!(f, "={flags};{mode}u"),
        }
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SetKeyboardFlagsMode {
      AssignAll = 1,

      SetSpecified = 2,

      ClearSpecified = 3,
    }

    impl Display for SetKeyboardFlagsMode {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8)
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Device {
      DeviceAttributes(()),

      SoftReset,

      RequestPrimaryDeviceAttributes,

      RequestSecondaryDeviceAttributes,

      RequestTertiaryDeviceAttributes,

      StatusReport,

      RequestTerminalNameAndVersion,

      RequestTerminalParameters(i64),
    }

    impl Display for Device {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
          Self::DeviceAttributes(_) => unimplemented!(),
          Self::SoftReset => write!(f, "!p"),
          Self::RequestPrimaryDeviceAttributes => write!(f, "c"),
          Self::RequestSecondaryDeviceAttributes => write!(f, ">c"),
          Self::RequestTertiaryDeviceAttributes => write!(f, "=c"),
          Self::StatusReport => write!(f, "5n"),
          Self::RequestTerminalNameAndVersion => write!(f, ">q"),
          Self::RequestTerminalParameters(n) => write!(f, "{};1;1;128;128;1;0x", n + 2),
        }
      }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Window {
      DeIconify,

      Iconify,

      MoveWindow {
        x: i64,

        y: i64,
      },

      ResizeWindowPixels {
        width: Option<i64>,

        height: Option<i64>,
      },

      RaiseWindow,

      LowerWindow,

      RefreshWindow,

      ResizeWindowCells {
        width: Option<i64>,

        height: Option<i64>,
      },

      RestoreMaximizedWindow,

      MaximizeWindow,

      MaximizeWindowVertically,

      MaximizeWindowHorizontally,

      UndoFullScreenMode,

      ChangeToFullScreenMode,

      ToggleFullScreen,

      ReportWindowState,

      ReportWindowPosition,

      ReportTextAreaPosition,

      ReportTextAreaSizePixels,

      ReportWindowSizePixels,

      ReportScreenSizePixels,

      ReportCellSizePixels,

      ReportCellSizePixelsResponse {
        width: Option<i64>,

        height: Option<i64>,
      },

      ReportTextAreaSizeCells,

      ReportScreenSizeCells,

      ReportIconLabel,

      ReportWindowTitle,

      PushIconAndWindowTitle,

      PushIconTitle,

      PushWindowTitle,

      PopIconAndWindowTitle,

      PopIconTitle,

      PopWindowTitle,

      ChecksumRectangularArea {
        request_id: i64,

        page_number: i64,

        top: OneBased,

        left: OneBased,

        bottom: OneBased,

        right: OneBased,
      },
    }

    impl Display for Window {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct NumstrOrEmpty(Option<i64>);
        impl Display for NumstrOrEmpty {
          fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if let Some(x) = self.0 {
              write!(f, "{x}")?
            }
            Ok(())
          }
        }

        match self {
          Window::DeIconify => write!(f, "1t"),
          Window::Iconify => write!(f, "2t"),
          Window::MoveWindow { x, y } => write!(f, "3;{x};{y}t"),
          Window::ResizeWindowPixels { width, height } => {
            write!(f, "4;{};{}t", NumstrOrEmpty(*height), NumstrOrEmpty(*width))
          }
          Window::RaiseWindow => write!(f, "5t"),
          Window::LowerWindow => write!(f, "6t"),
          Window::RefreshWindow => write!(f, "7t"),
          Window::ResizeWindowCells { width, height } => {
            write!(f, "8;{};{}t", NumstrOrEmpty(*height), NumstrOrEmpty(*width))
          }
          Window::RestoreMaximizedWindow => write!(f, "9;0t"),
          Window::MaximizeWindow => write!(f, "9;1t"),
          Window::MaximizeWindowVertically => write!(f, "9;2t"),
          Window::MaximizeWindowHorizontally => write!(f, "9;3t"),
          Window::UndoFullScreenMode => write!(f, "10;0t"),
          Window::ChangeToFullScreenMode => write!(f, "10;1t"),
          Window::ToggleFullScreen => write!(f, "10;2t"),
          Window::ReportWindowState => write!(f, "11t"),
          Window::ReportWindowPosition => write!(f, "13t"),
          Window::ReportTextAreaPosition => write!(f, "13;2t"),
          Window::ReportTextAreaSizePixels => write!(f, "14t"),
          Window::ReportWindowSizePixels => write!(f, "14;2t"),
          Window::ReportScreenSizePixels => write!(f, "15t"),
          Window::ReportCellSizePixels => write!(f, "16t"),
          Window::ReportCellSizePixelsResponse { width, height } => {
            write!(f, "6;{};{}t", NumstrOrEmpty(*height), NumstrOrEmpty(*width))
          }
          Window::ReportTextAreaSizeCells => write!(f, "18t"),
          Window::ReportScreenSizeCells => write!(f, "19t"),
          Window::ReportIconLabel => write!(f, "20t"),
          Window::ReportWindowTitle => write!(f, "21t"),
          Window::PushIconAndWindowTitle => write!(f, "22;0t"),
          Window::PushIconTitle => write!(f, "22;1t"),
          Window::PushWindowTitle => write!(f, "22;2t"),
          Window::PopIconAndWindowTitle => write!(f, "23;0t"),
          Window::PopIconTitle => write!(f, "23;1t"),
          Window::PopWindowTitle => write!(f, "23;2t"),
          Window::ChecksumRectangularArea {
            request_id,
            page_number,
            top,
            left,
            bottom,
            right,
          } => write!(
            f,
            "{request_id};{page_number};{top};{left};{bottom};{right}*y"
          ),
        }
      }
    }
  }
  pub mod dcs {
    use std::fmt::{self, Display};

    use crate::termina::style::CursorStyle;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Dcs {
      Request(DcsRequest),

      Response {
        is_request_valid: bool,

        value: DcsResponse,
      },
    }

    impl Display for Dcs {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(super::DCS)?;
        match self {
          Self::Request(request) => write!(f, "$q{request}")?,
          Self::Response {
            is_request_valid,
            value,
          } => write!(f, "{}$r{value}", if *is_request_valid { 1 } else { 0 })?,
        }
        f.write_str(super::ST)
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DcsRequest {
      ActiveStatusDisplay,
      AttributeChangeExtent,
      CharacterAttribute,
      ConformanceLevel,
      ColumnsPerPage,
      LinesPerPage,
      NumberOfLinesPerScreen,
      StatusLineType,
      LeftAndRightMargins,
      TopAndBottomMargins,
      GraphicRendition,
      SetUpLanguage,
      PrinterType,
      RefreshRate,
      DigitalPrintedDataType,
      ProPrinterCharacterSet,
      CommunicationSpeed,
      CommunicationPort,
      ScrollSpeed,
      CursorStyle,
      KeyClickVolume,
      WarningBellVolume,
      MarginBellVolume,
      LockKeyStyle,
      FlowControlType,
      DisconnectDelayTime,
      TransmitRateLimit,
      PortParameter,
    }

    impl Display for DcsRequest {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
          Self::ActiveStatusDisplay => f.write_str("$}"),
          Self::AttributeChangeExtent => write!(f, "*x"),
          Self::CharacterAttribute => write!(f, "\"q"),
          Self::ConformanceLevel => write!(f, "\"p"),
          Self::ColumnsPerPage => write!(f, "$|"),
          Self::LinesPerPage => write!(f, "t"),
          Self::NumberOfLinesPerScreen => write!(f, "*|"),
          Self::StatusLineType => write!(f, "$~"),
          Self::LeftAndRightMargins => write!(f, "s"),
          Self::TopAndBottomMargins => write!(f, "r"),
          Self::GraphicRendition => write!(f, "m"),
          Self::SetUpLanguage => write!(f, "p"),
          Self::PrinterType => write!(f, "$s"),
          Self::RefreshRate => write!(f, "\"t"),
          Self::DigitalPrintedDataType => write!(f, "(p"),
          Self::ProPrinterCharacterSet => write!(f, "*p"),
          Self::CommunicationSpeed => write!(f, "*r"),
          Self::CommunicationPort => write!(f, "*u"),
          Self::ScrollSpeed => write!(f, " p"),
          Self::CursorStyle => write!(f, " q"),
          Self::KeyClickVolume => write!(f, " r"),
          Self::WarningBellVolume => write!(f, " t"),
          Self::MarginBellVolume => write!(f, " u"),
          Self::LockKeyStyle => write!(f, " v"),
          Self::FlowControlType => write!(f, "*s"),
          Self::DisconnectDelayTime => write!(f, "$q"),
          Self::TransmitRateLimit => write!(f, "\"u"),
          Self::PortParameter => write!(f, "+w"),
        }
      }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum DcsResponse {
      GraphicRendition(Vec<super::csi::Sgr>),

      CursorStyle(CursorStyle),
    }

    impl Display for DcsResponse {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
          Self::GraphicRendition(sgrs) => {
            let mut first = true;
            for sgr in sgrs {
              if !first {
                write!(f, ";")?;
              }
              first = false;
              write!(f, "{sgr}")?;
            }
            Ok(())
          }
          Self::CursorStyle(style) => write!(f, "{style} q"),
        }
      }
    }
  }
  pub mod osc {

    use std::fmt::{self, Display};

    use crate::termina::{base64, style::RgbColor};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Osc<'a> {
      SetIconNameAndWindowTitle(&'a str),

      SetWindowTitle(&'a str),

      SetWindowTitleSun(&'a str),

      SetIconName(&'a str),

      SetIconNameSun(&'a str),

      ClearSelection(Selection),

      QuerySelection(Selection),

      SetSelection(Selection, &'a str),

      ChangeDynamicColors(DynamicColorNumber, Vec<ColorOrQuery>),

      ResetDynamicColor(DynamicColorNumber),
    }

    impl Display for Osc<'_> {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(super::OSC)?;
        match self {
          Self::SetIconNameAndWindowTitle(s) => write!(f, "0;{s}")?,
          Self::SetWindowTitle(s) => write!(f, "2;{s}")?,
          Self::SetWindowTitleSun(s) => write!(f, "l{s}")?,
          Self::SetIconName(s) => write!(f, "1;{s}")?,
          Self::SetIconNameSun(s) => write!(f, "L{s}")?,
          Self::ClearSelection(selection) => write!(f, "52;{selection}")?,
          Self::QuerySelection(selection) => write!(f, "52;{selection};?")?,
          Self::SetSelection(selection, content) => write!(
            f,
            "52;{selection};{}",
            base64::base64_encode(content.as_bytes())
          )?,
          Self::ChangeDynamicColors(color, colors) => {
            write!(f, "{}", *color as u8)?;
            for color in colors {
              write!(f, ";{color}")?
            }
          }
          Self::ResetDynamicColor(color) => write!(f, "{}", 100 + *color as u8)?,
        }
        f.write_str(super::ST)?;
        Ok(())
      }
    }

    bitflags::bitflags! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct Selection : u16 {
            const NONE = 0;

            const CLIPBOARD = 1<<1;

            const PRIMARY=1<<2;

            const SELECT=1<<3;

            const CUT0=1<<4;

            const CUT1=1<<5;

            const CUT2=1<<6;

            const CUT3=1<<7;

            const CUT4=1<<8;

            const CUT5=1<<9;

            const CUT6=1<<10;

            const CUT7=1<<11;

            const CUT8=1<<12;

            const CUT9=1<<13;
        }
    }

    impl Display for Selection {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.contains(Self::CLIPBOARD) {
          write!(f, "c")?;
        }
        if self.contains(Self::PRIMARY) {
          write!(f, "p")?;
        }
        if self.contains(Self::SELECT) {
          write!(f, "s")?;
        }
        if self.contains(Self::CUT0) {
          write!(f, "0")?;
        }
        if self.contains(Self::CUT1) {
          write!(f, "1")?;
        }
        if self.contains(Self::CUT2) {
          write!(f, "2")?;
        }
        if self.contains(Self::CUT3) {
          write!(f, "3")?;
        }
        if self.contains(Self::CUT4) {
          write!(f, "4")?;
        }
        if self.contains(Self::CUT5) {
          write!(f, "5")?;
        }
        if self.contains(Self::CUT6) {
          write!(f, "6")?;
        }
        if self.contains(Self::CUT7) {
          write!(f, "7")?;
        }
        if self.contains(Self::CUT8) {
          write!(f, "8")?;
        }
        if self.contains(Self::CUT9) {
          write!(f, "9")?;
        }
        Ok(())
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    pub enum DynamicColorNumber {
      TextForegroundColor = 10,
      TextBackgroundColor = 11,
      TextCursorColor = 12,
      MouseForegroundColor = 13,
      MouseBackgroundColor = 14,
      TektronixForegroundColor = 15,
      TektronixBackgroundColor = 16,
      HighlightBackgroundColor = 17,
      TektronixCursorColor = 18,
      HighlightForegroundColor = 19,
    }

    impl DynamicColorNumber {
      pub(crate) fn from_index(index: u8) -> Option<Self> {
        match index {
          10 => Some(Self::TextForegroundColor),
          11 => Some(Self::TextBackgroundColor),
          12 => Some(Self::TextCursorColor),
          13 => Some(Self::MouseForegroundColor),
          14 => Some(Self::MouseBackgroundColor),
          15 => Some(Self::TektronixForegroundColor),
          16 => Some(Self::TektronixBackgroundColor),
          17 => Some(Self::HighlightBackgroundColor),
          18 => Some(Self::TektronixCursorColor),
          19 => Some(Self::HighlightForegroundColor),
          _ => None,
        }
      }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ColorOrQuery {
      Color(RgbColor),

      Query,
    }

    impl Display for ColorOrQuery {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
          ColorOrQuery::Query => write!(f, "?"),
          ColorOrQuery::Color(c) => {
            write!(
              f,
              "rgb:{red:02x}{red:02x}/{green:02x}{green:02x}/{blue:02x}{blue:02x}",
              red = c.red,
              green = c.green,
              blue = c.blue
            )
          }
        }
      }
    }

    impl From<RgbColor> for ColorOrQuery {
      fn from(color: RgbColor) -> Self {
        Self::Color(color)
      }
    }
  }

  pub(crate) const CSI: &str = "\x1b[";

  pub(crate) const OSC: &str = "\x1b]";

  pub(crate) const ST: &str = "\x1b\\";

  pub(crate) const DCS: &str = "\x1bP";

  pub(crate) const BEL: &str = "\x07";
}
pub mod event {

  use crate::termina::{
    WindowSize,
    escape::{csi::Csi, dcs::Dcs, osc::Osc},
  };

  pub(crate) mod reader {
    use std::{collections::VecDeque, io, sync::Arc, time::Duration};

    use parking_lot::Mutex;

    use super::{
      Event,
      source::{EventSource as _, PlatformEventSource, PlatformWaker, PollTimeout},
    };

    #[derive(Debug, Clone)]
    pub struct EventReader {
      shared: Arc<Mutex<SharedReaderState>>,
    }

    impl EventReader {
      pub(crate) fn new(source: PlatformEventSource) -> Self {
        let shared = SharedReaderState {
          events: VecDeque::with_capacity(32),
          source,
          skipped_events: Vec::with_capacity(32),
        };
        Self {
          shared: Arc::new(Mutex::new(shared)),
        }
      }

      pub(crate) fn waker(&self) -> PlatformWaker {
        let reader = self.shared.lock();
        reader.source.waker()
      }

      pub fn poll<F>(&self, timeout: Option<Duration>, filter: F) -> io::Result<bool>
      where
        F: FnMut(&Event) -> bool,
      {
        let (mut reader, timeout) = if let Some(timeout) = timeout {
          let poll_timeout = PollTimeout::new(Some(timeout));
          if let Some(reader) = self.shared.try_lock_for(timeout) {
            (reader, poll_timeout.leftover())
          } else {
            return Ok(false);
          }
        } else {
          (self.shared.lock(), None)
        };
        reader.poll(timeout, filter)
      }

      pub fn read<F>(&self, filter: F) -> io::Result<Event>
      where
        F: FnMut(&Event) -> bool,
      {
        let mut reader = self.shared.lock();
        reader.read(filter)
      }
    }

    #[derive(Debug)]
    struct SharedReaderState {
      events: VecDeque<Event>,
      source: PlatformEventSource,
      skipped_events: Vec<Event>,
    }

    impl SharedReaderState {
      fn poll<F>(&mut self, timeout: Option<Duration>, mut filter: F) -> io::Result<bool>
      where
        F: FnMut(&Event) -> bool,
      {
        if self.events.iter().any(&mut (filter)) {
          return Ok(true);
        }

        let timeout = PollTimeout::new(timeout);

        loop {
          let maybe_event = match self.source.try_read(timeout.leftover()) {
            Ok(None) => None,
            Ok(Some(event)) => {
              if (filter)(&event) {
                Some(event)
              } else {
                self.skipped_events.push(event);
                None
              }
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => return Ok(false),
            Err(err) => return Err(err),
          };

          if timeout.elapsed() || maybe_event.is_some() {
            self.events.extend(self.skipped_events.drain(..));

            if let Some(event) = maybe_event {
              self.events.push_front(event);
              return Ok(true);
            }

            return Ok(false);
          }
        }
      }

      fn read<F>(&mut self, mut filter: F) -> io::Result<Event>
      where
        F: FnMut(&Event) -> bool,
      {
        let mut skipped_events = VecDeque::new();

        loop {
          while let Some(event) = self.events.pop_front() {
            if (filter)(&event) {
              self.events.extend(skipped_events.drain(..));
              return Ok(event);
            } else {
              skipped_events.push_back(event);
            }
          }
          let _ = self.poll(None, &mut filter)?;
        }
      }
    }
  }
  pub(crate) mod source {
    mod unix {
      use std::{
        io::{self, Read, Write as _},
        os::{
          fd::{AsFd, BorrowedFd},
          unix::net::UnixStream,
        },
        sync::Arc,
        time::Duration,
      };

      use parking_lot::Mutex;
      use rustix::termios;

      use crate::termina::{Event, parse::Parser, terminal::FileDescriptor};

      use super::{EventSource, PollTimeout};

      #[derive(Debug)]
      pub(crate) struct UnixEventSource {
        parser: Parser,
        read: FileDescriptor,
        write: FileDescriptor,
        sigwinch_id: signal_hook::SigId,
        sigwinch_pipe: UnixStream,
        wake_pipe: UnixStream,
        wake_pipe_write: Arc<Mutex<UnixStream>>,
      }

      #[derive(Debug, Clone)]
      pub(crate) struct UnixWaker {
        inner: Arc<Mutex<UnixStream>>,
      }

      impl UnixWaker {
        pub(crate) fn wake(&self) -> io::Result<()> {
          self.inner.lock().write_all(&[0])
        }
      }

      impl UnixEventSource {
        pub(crate) fn new(read: FileDescriptor, write: FileDescriptor) -> io::Result<Self> {
          let (sigwinch_pipe, sigwinch_pipe_write) = UnixStream::pair()?;
          let sigwinch_id = signal_hook::low_level::pipe::register(
            signal_hook::consts::SIGWINCH,
            sigwinch_pipe_write,
          )?;
          sigwinch_pipe.set_nonblocking(true)?;
          let (wake_pipe, wake_pipe_write) = UnixStream::pair()?;
          wake_pipe.set_nonblocking(true)?;
          wake_pipe_write.set_nonblocking(true)?;

          Ok(Self {
            parser: Default::default(),
            read,
            write,
            sigwinch_id,
            sigwinch_pipe,
            wake_pipe,
            wake_pipe_write: Arc::new(Mutex::new(wake_pipe_write)),
          })
        }
      }

      impl Drop for UnixEventSource {
        fn drop(&mut self) {
          signal_hook::low_level::unregister(self.sigwinch_id);
        }
      }

      impl EventSource for UnixEventSource {
        fn waker(&self) -> UnixWaker {
          UnixWaker {
            inner: self.wake_pipe_write.clone(),
          }
        }

        fn try_read(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
          let timeout = PollTimeout::new(timeout);

          loop {
            if let Some(event) = self.parser.pop() {
              return Ok(Some(event));
            }

            let [read_ready, sigwinch_ready, wake_ready] = match poll(
              [
                self.read.as_fd(),
                self.sigwinch_pipe.as_fd(),
                self.wake_pipe.as_fd(),
              ],
              timeout.leftover(),
            ) {
              Ok(ready) => ready,
              Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
              Err(err) => return Err(err),
            };

            if read_ready {
              let mut buffer = [0u8; 64];
              let read_count = read_complete(&mut self.read, &mut buffer)?;
              if read_count > 0 {
                self
                  .parser
                  .parse(&buffer[..read_count], read_count == buffer.len());
              }
              if let Some(event) = self.parser.pop() {
                return Ok(Some(event));
              }
              if read_count == 0 {
                break;
              }
            }

            if sigwinch_ready {
              while read_complete(&self.sigwinch_pipe, &mut [0; 1024])? != 0 {}

              let winsize = termios::tcgetwinsize(&self.write)?;
              let event = Event::WindowResized(winsize.into());
              return Ok(Some(event));
            }

            if wake_ready {
              while read_complete(&self.wake_pipe, &mut [0; 1024])? != 0 {}

              return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Poll operation was woken up",
              ));
            }

            if timeout.leftover().is_some_and(|t| t.is_zero()) {
              break;
            }
          }

          Ok(None)
        }
      }

      fn read_complete<F: Read>(mut file: F, buf: &mut [u8]) -> io::Result<usize> {
        loop {
          match file.read(buf) {
            Ok(read) => return Ok(read),
            Err(err) => match err.kind() {
              io::ErrorKind::WouldBlock => return Ok(0),
              io::ErrorKind::Interrupted => continue,
              _ => return Err(err),
            },
          }
        }
      }

      fn poll(fds: [BorrowedFd<'_>; 3], timeout: Option<Duration>) -> std::io::Result<[bool; 3]> {
        use rustix::event::Timespec;

        fn poll2(fds: [BorrowedFd<'_>; 3], timeout: Option<&Timespec>) -> io::Result<[bool; 3]> {
          use rustix::event::{PollFd, PollFlags};
          let mut fds = [
            PollFd::new(&fds[0], PollFlags::IN),
            PollFd::new(&fds[1], PollFlags::IN),
            PollFd::new(&fds[2], PollFlags::IN),
          ];

          rustix::event::poll(&mut fds, timeout)?;

          Ok([
            fds[0].revents().contains(PollFlags::IN),
            fds[1].revents().contains(PollFlags::IN),
            fds[2].revents().contains(PollFlags::IN),
          ])
        }

        use poll2 as poll_impl;

        let timespec = timeout.map(|timeout| timeout.try_into().unwrap());
        poll_impl(fds, timespec.as_ref())
      }
    }

    use std::time::{Duration, Instant};

    pub(crate) use unix::{UnixEventSource, UnixWaker};

    pub(crate) type PlatformEventSource = UnixEventSource;

    pub(crate) type PlatformWaker = UnixWaker;

    pub(crate) trait EventSource: Send + Sync {
      fn try_read(
        &mut self,
        timeout: Option<Duration>,
      ) -> std::io::Result<Option<crate::termina::Event>>;

      fn waker(&self) -> PlatformWaker;
    }

    #[derive(Debug, Clone)]
    pub(crate) struct PollTimeout {
      timeout: Option<Duration>,
      start: Instant,
    }

    impl PollTimeout {
      pub(crate) fn new(timeout: Option<Duration>) -> Self {
        Self {
          timeout,
          start: Instant::now(),
        }
      }

      pub(crate) fn elapsed(&self) -> bool {
        self
          .timeout
          .map(|timeout| self.start.elapsed() >= timeout)
          .unwrap_or(false)
      }

      pub(crate) fn leftover(&self) -> Option<Duration> {
        self.timeout.map(|timeout| {
          let elapsed = self.start.elapsed();

          if elapsed >= timeout {
            Duration::ZERO
          } else {
            timeout - elapsed
          }
        })
      }
    }
  }

  pub(crate) mod stream {
    use std::{
      io,
      pin::Pin,
      sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
      },
      task::{Context, Poll},
      thread,
      time::Duration,
    };

    use futures_core::Stream;

    use super::{Event, reader::EventReader, source::PlatformWaker};

    pub struct EventStream {
      waker: PlatformWaker,
      filter: Arc<dyn Fn(&Event) -> bool>,
      reader: EventReader,
      stream_wake_task_executed: Arc<AtomicBool>,
      stream_wake_task_should_shutdown: Arc<AtomicBool>,
      task_sender: SyncSender<StreamTask>,
    }

    #[derive(Debug)]
    struct StreamTask {
      stream_waker: std::task::Waker,
      stream_wake_task_executed: Arc<AtomicBool>,
      stream_wake_task_should_shutdown: Arc<AtomicBool>,
    }

    impl EventStream {
      pub fn new<F>(reader: EventReader, filter: F) -> Self
      where
        F: Fn(&Event) -> bool + Send + Sync + 'static,
      {
        let filter = Arc::new(filter);
        let waker = reader.waker();

        let (task_sender, receiver) = mpsc::sync_channel::<StreamTask>(1);

        let task_reader = reader.clone();
        let task_filter = filter.clone();
        thread::spawn(move || {
          while let Ok(task) = receiver.recv() {
            loop {
              if let Ok(true) = task_reader.poll(None, &*task_filter) {
                break;
              }
              if task.stream_wake_task_should_shutdown.load(Ordering::SeqCst) {
                break;
              }
            }
            task
              .stream_wake_task_executed
              .store(false, Ordering::SeqCst);
            task.stream_waker.wake();
          }
        });

        Self {
          waker,
          filter,
          reader,
          stream_wake_task_executed: Default::default(),
          stream_wake_task_should_shutdown: Default::default(),
          task_sender,
        }
      }
    }

    impl Drop for EventStream {
      fn drop(&mut self) {
        self
          .stream_wake_task_should_shutdown
          .store(true, Ordering::SeqCst);
        let _ = self.waker.wake();
      }
    }

    impl Stream for EventStream {
      type Item = io::Result<Event>;

      fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self
          .reader
          .poll(Some(Duration::from_secs(0)), &*self.filter)
        {
          Ok(true) => match self.reader.read(&*self.filter) {
            Ok(event) => Poll::Ready(Some(Ok(event))),
            Err(err) => Poll::Ready(Some(Err(err))),
          },
          Ok(false) => {
            if !self
              .stream_wake_task_executed
              .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
              .unwrap_or_else(|x| x)
            {
              self
                .stream_wake_task_should_shutdown
                .store(false, Ordering::SeqCst);
              let _ = self.task_sender.send(StreamTask {
                stream_waker: cx.waker().clone(),
                stream_wake_task_executed: self.stream_wake_task_executed.clone(),
                stream_wake_task_should_shutdown: self.stream_wake_task_should_shutdown.clone(),
              });
            }
            Poll::Pending
          }
          Err(err) => Poll::Ready(Some(Err(err))),
        }
      }
    }
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum Event {
    Key(KeyEvent),

    Mouse(MouseEvent),

    WindowResized(WindowSize),

    FocusIn,

    FocusOut,

    Paste(String),

    Csi(Csi),

    Osc(Osc<'static>),

    Dcs(Dcs),
  }

  impl Event {
    #[inline]
    pub fn is_escape(&self) -> bool {
      matches!(self, Self::Csi(_) | Self::Dcs(_) | Self::Osc(_))
    }
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct KeyEvent {
    pub code: KeyCode,

    pub kind: KeyEventKind,

    pub modifiers: Modifiers,

    pub state: KeyEventState,
  }

  impl KeyEvent {
    pub(crate) const fn new(code: KeyCode, modifiers: Modifiers) -> Self {
      Self {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
      }
    }
  }

  impl From<KeyCode> for KeyEvent {
    fn from(code: KeyCode) -> Self {
      Self {
        code,
        kind: KeyEventKind::Press,
        modifiers: Modifiers::NONE,
        state: KeyEventState::NONE,
      }
    }
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum KeyEventKind {
    Press,

    Release,

    Repeat,
  }

  bitflags::bitflags! {
      #[derive(Debug, Clone, Copy, PartialEq, Eq)]
      pub struct Modifiers: u8 {
          const NONE = 0;

          const SHIFT = 1;

          const ALT = 1 << 1;

          const CONTROL = 1 << 2;

          const SUPER = 1 << 3;
          const HYPER = 1 << 4;

          const META = 1 << 5;

          const CAPS_LOCK = 1 << 6;

          const NUM_LOCK = 1 << 7;
      }
  }

  bitflags::bitflags! {
      #[derive(Debug, Clone, Copy, PartialEq, Eq)]
      pub struct KeyEventState: u8 {
          const NONE = 0;

          const KEYPAD = 1 << 1;

          const CAPS_LOCK = 1 << 2;

          const NUM_LOCK = 1 << 3;
      }
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum KeyCode {
    Char(char),

    Enter,

    Backspace,

    Tab,

    Escape,

    Left,

    Right,

    Up,

    Down,

    Home,

    End,

    BackTab,

    PageUp,

    PageDown,

    Insert,

    Delete,

    KeypadBegin,

    CapsLock,

    ScrollLock,

    NumLock,

    PrintScreen,

    Pause,

    Menu,

    Null,

    Function(u8),

    Modifier(ModifierKeyCode),

    Media(MediaKeyCode),
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum ModifierKeyCode {
    LeftShift,
    LeftControl,
    LeftAlt,
    LeftSuper,
    LeftHyper,
    LeftMeta,
    RightShift,
    RightControl,
    RightAlt,
    RightSuper,
    RightHyper,
    RightMeta,
    IsoLevel3Shift,
    IsoLevel5Shift,
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum MediaKeyCode {
    Play,
    Pause,
    PlayPause,
    Reverse,
    Stop,
    FastForward,
    Rewind,
    TrackNext,
    TrackPrevious,
    Record,
    LowerVolume,
    RaiseVolume,
    MuteVolume,
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct MouseEvent {
    pub kind: MouseEventKind,

    pub column: u16,

    pub row: u16,

    pub modifiers: Modifiers,
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum MouseEventKind {
    Down(MouseButton),

    Up(MouseButton),

    Drag(MouseButton),

    Moved,

    ScrollDown,

    ScrollUp,

    ScrollLeft,

    ScrollRight,
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum MouseButton {
    Left,
    Right,
    Middle,
  }
}
pub(crate) mod parse {
  use std::{collections::VecDeque, num::NonZeroU16, str};

  use crate::termina::{
    Event,
    escape::{
      self,
      csi::{self, Csi, KittyKeyboardFlags, ThemeMode},
      dcs, osc,
    },
    event::{
      KeyCode, KeyEvent, KeyEventKind, KeyEventState, MediaKeyCode, ModifierKeyCode, Modifiers,
      MouseButton, MouseEvent, MouseEventKind,
    },
    style,
  };

  #[derive(Debug)]
  pub(crate) struct Parser {
    buffer: Vec<u8>,
    events: VecDeque<Event>,
  }

  impl Default for Parser {
    fn default() -> Self {
      Self {
        buffer: Vec::with_capacity(256),
        events: VecDeque::with_capacity(32),
      }
    }
  }

  impl Parser {
    pub(crate) fn pop(&mut self) -> Option<Event> {
      self.events.pop_front()
    }

    pub(crate) fn parse(&mut self, bytes: &[u8], maybe_more: bool) {
      self.buffer.extend_from_slice(bytes);
      self.process_bytes(maybe_more);
    }

    fn process_bytes(&mut self, maybe_more: bool) {
      let mut start = 0;
      for n in 0..self.buffer.len() {
        let end = n + 1;
        match parse_event(
          &self.buffer[start..end],
          maybe_more || end < self.buffer.len(),
        ) {
          Ok(Some(event)) => {
            self.events.push_back(event);
            start = end;
          }
          Ok(None) => continue,
          Err(_) => start = end,
        }
      }
      self.advance(start);
    }

    fn advance(&mut self, len: usize) {
      if len == 0 {
        return;
      }
      let remain = self.buffer.len() - len;
      self.buffer.rotate_left(len);
      self.buffer.truncate(remain);
    }
  }

  #[derive(Debug)]
  struct MalformedSequenceError;

  impl From<str::Utf8Error> for MalformedSequenceError {
    fn from(_: str::Utf8Error) -> Self {
      Self
    }
  }

  type Result<T> = std::result::Result<T, MalformedSequenceError>;

  macro_rules! bail {
    () => {
      return Err(MalformedSequenceError)
    };
  }

  fn parse_event(buffer: &[u8], maybe_more: bool) -> Result<Option<Event>> {
    if buffer.is_empty() {
      return Ok(None);
    }

    match buffer[0] {
      b'\x1B' => {
        if buffer.len() == 1 {
          if maybe_more {
            Ok(None)
          } else {
            Ok(Some(Event::Key(KeyCode::Escape.into())))
          }
        } else {
          match buffer[1] {
            b'O' => {
              if buffer.len() == 2 {
                Ok(None)
              } else {
                match buffer[2] {
                  b'D' => Ok(Some(Event::Key(KeyCode::Left.into()))),
                  b'C' => Ok(Some(Event::Key(KeyCode::Right.into()))),
                  b'A' => Ok(Some(Event::Key(KeyCode::Up.into()))),
                  b'B' => Ok(Some(Event::Key(KeyCode::Down.into()))),
                  b'H' => Ok(Some(Event::Key(KeyCode::Home.into()))),
                  b'F' => Ok(Some(Event::Key(KeyCode::End.into()))),
                  val @ b'P'..=b'S' => {
                    Ok(Some(Event::Key(KeyCode::Function(1 + val - b'P').into())))
                  }
                  _ => bail!(),
                }
              }
            }
            b'[' => parse_csi(buffer),
            b']' => parse_osc(buffer),
            b'P' => parse_dcs(buffer),
            b'\x1B' => Ok(Some(Event::Key(KeyCode::Escape.into()))),
            _ => parse_event(&buffer[1..], maybe_more).map(|event_option| {
              event_option.map(|event| {
                if let Event::Key(key_event) = event {
                  let mut alt_key_event = key_event;
                  alt_key_event.modifiers |= Modifiers::ALT;
                  Event::Key(alt_key_event)
                } else {
                  event
                }
              })
            }),
          }
        }
      }
      b'\r' => Ok(Some(Event::Key(KeyCode::Enter.into()))),
      b'\t' => Ok(Some(Event::Key(KeyCode::Tab.into()))),
      b'\x7F' => Ok(Some(Event::Key(KeyCode::Backspace.into()))),
      b'\0' => Ok(Some(Event::Key(KeyEvent::new(
        KeyCode::Char(' '),
        Modifiers::CONTROL,
      )))),
      c @ b'\x01'..=b'\x1A' => Ok(Some(Event::Key(KeyEvent::new(
        KeyCode::Char((c - 0x1 + b'a') as char),
        Modifiers::CONTROL,
      )))),
      c @ b'\x1C'..=b'\x1F' => Ok(Some(Event::Key(KeyEvent::new(
        KeyCode::Char((c - 0x1C + b'4') as char),
        Modifiers::CONTROL,
      )))),
      _ => parse_utf8_char(buffer).map(|maybe_char| {
        maybe_char.map(|ch| {
          let modifiers = if ch.is_uppercase() {
            Modifiers::SHIFT
          } else {
            Modifiers::NONE
          };
          Event::Key(KeyEvent::new(KeyCode::Char(ch), modifiers))
        })
      }),
    }
  }

  fn parse_utf8_char(buffer: &[u8]) -> Result<Option<char>> {
    assert!(!buffer.is_empty());
    match str::from_utf8(buffer) {
      Ok(s) => Ok(Some(s.chars().next().unwrap())),
      Err(_) => {
        let required_bytes = match buffer[0] {
          (0x00..=0x7F) => 1, // 0xxxxxxx
          (0xC0..=0xDF) => 2, // 110xxxxx 10xxxxxx
          (0xE0..=0xEF) => 3, // 1110xxxx 10xxxxxx 10xxxxxx
          (0xF0..=0xF7) => 4, // 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
          (0x80..=0xBF) | (0xF8..=0xFF) => bail!(),
        };
        if required_bytes > 1 && buffer.len() > 1 {
          for byte in &buffer[1..] {
            if byte & !0b0011_1111 != 0b1000_0000 {
              bail!()
            }
          }
        }
        if buffer.len() < required_bytes {
          Ok(None)
        } else {
          bail!()
        }
      }
    }
  }

  fn parse_csi(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B["));
    if buffer.len() == 2 {
      return Ok(None);
    }
    let maybe_event = match buffer[2] {
      b'[' => match buffer.get(3) {
        None => None,
        Some(b @ b'A'..=b'E') => Some(Event::Key(KeyCode::Function(1 + b - b'A').into())),
        Some(_) => bail!(),
      },
      b'D' => Some(Event::Key(KeyCode::Left.into())),
      b'C' => Some(Event::Key(KeyCode::Right.into())),
      b'A' => Some(Event::Key(KeyCode::Up.into())),
      b'B' => Some(Event::Key(KeyCode::Down.into())),
      b'H' => Some(Event::Key(KeyCode::Home.into())),
      b'F' => Some(Event::Key(KeyCode::End.into())),
      b'Z' => Some(Event::Key(KeyEvent {
        code: KeyCode::BackTab,
        modifiers: Modifiers::SHIFT,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
      })),
      b'M' => return parse_csi_normal_mouse(buffer),
      b'<' => return parse_csi_sgr_mouse(buffer),
      b'I' => Some(Event::FocusIn),
      b'O' => Some(Event::FocusOut),
      b';' => return parse_csi_modifier_key_code(buffer),
      b'P' => Some(Event::Key(KeyCode::Function(1).into())),
      b'Q' => Some(Event::Key(KeyCode::Function(2).into())),
      b'S' => Some(Event::Key(KeyCode::Function(4).into())),
      b'?' => match buffer[buffer.len() - 1] {
        b'u' => return parse_csi_keyboard_enhancement_flags(buffer),
        b'c' => return parse_csi_primary_device_attributes(buffer),
        b'n' => return parse_csi_theme_mode(buffer),
        b'y' => return parse_csi_mode(buffer),
        _ => None,
      },
      b'>' => match buffer[buffer.len() - 2..buffer.len()] {
        [b' ', b'q'] => return parse_csi_cursor_shape_query_response(buffer),
        _ => None,
      },
      b'0'..=b'9' => {
        if buffer.len() == 3 {
          None
        } else {
          let last_byte = buffer[buffer.len() - 1];
          if !(64..=126).contains(&last_byte) {
            None
          } else {
            if buffer.starts_with(b"\x1B[200~") {
              return parse_csi_bracketed_paste(buffer);
            }
            match last_byte {
              b'M' => return parse_csi_rxvt_mouse(buffer),
              b'~' => return parse_csi_special_key_code(buffer),
              b'u' => return parse_csi_u_encoded_key_code(buffer),
              b'R' => return parse_csi_cursor_position(buffer),
              _ => return parse_csi_modifier_key_code(buffer),
            }
          }
        }
      }
      _ => bail!(),
    };
    Ok(maybe_event)
  }

  fn parse_osc(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B]"));
    let Some(buffer) = buffer
      .strip_suffix(escape::ST.as_bytes())
      .or_else(|| buffer.strip_suffix(escape::BEL.as_bytes()))
    else {
      return Ok(None);
    };
    let s = str::from_utf8(&buffer[2..buffer.len()])?;
    let mut split = s.split(';');
    let index = next_parsed::<u8>(&mut split)?;
    let Some(color_number) = osc::DynamicColorNumber::from_index(index) else {
      bail!()
    };
    let Some(color_or_query) = split.next() else {
      bail!()
    };
    let response = match color_or_query {
      "?" => osc::ColorOrQuery::Query,
      _ => osc::ColorOrQuery::Color(color_or_query.parse().map_err(|_| MalformedSequenceError)?),
    };
    Ok(Some(Event::Osc(osc::Osc::ChangeDynamicColors(
      color_number,
      vec![response],
    ))))
  }

  fn next_parsed<T>(iter: &mut dyn Iterator<Item = &str>) -> Result<T>
  where
    T: str::FromStr,
  {
    iter
      .next()
      .ok_or(MalformedSequenceError)?
      .parse::<T>()
      .map_err(|_| MalformedSequenceError)
  }

  fn modifier_and_kind_parsed(iter: &mut dyn Iterator<Item = &str>) -> Result<(u8, u8)> {
    let mut sub_split = iter.next().ok_or(MalformedSequenceError)?.split(':');

    let modifier_mask = next_parsed::<u8>(&mut sub_split)?;

    if let Ok(kind_code) = next_parsed::<u8>(&mut sub_split) {
      Ok((modifier_mask, kind_code))
    } else {
      Ok((modifier_mask, 1))
    }
  }

  fn parse_csi_u_encoded_key_code(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B")); // CSI
    assert!(buffer.ends_with(b"u"));

    let s = str::from_utf8(&buffer[2..buffer.len() - 1])?;
    let mut split = s.split(';');

    let mut codepoints = split.next().ok_or(MalformedSequenceError)?.split(':');

    let codepoint = codepoints
      .next()
      .ok_or(MalformedSequenceError)?
      .parse::<u32>()
      .map_err(|_| MalformedSequenceError)?;

    let (mut modifiers, kind, state_from_modifiers) =
      if let Ok((modifier_mask, kind_code)) = modifier_and_kind_parsed(&mut split) {
        (
          parse_modifiers(modifier_mask),
          parse_key_event_kind(kind_code),
          parse_modifiers_to_state(modifier_mask),
        )
      } else {
        (Modifiers::NONE, KeyEventKind::Press, KeyEventState::NONE)
      };

    let (mut code, state_from_keycode) = {
      if let Some((special_key_code, state)) = translate_functional_key_code(codepoint) {
        (special_key_code, state)
      } else if let Some(c) = char::from_u32(codepoint) {
        (
          match c {
            '\x1B' => KeyCode::Escape,
            '\r' => KeyCode::Enter,
            /*
            '\n' if !crate::termina::terminal::sys::is_raw_mode_enabled() => KeyCode::Enter,
            */
            '\t' => {
              if modifiers.contains(Modifiers::SHIFT) {
                KeyCode::BackTab
              } else {
                KeyCode::Tab
              }
            }
            '\x7F' => KeyCode::Backspace,
            _ => KeyCode::Char(c),
          },
          KeyEventState::empty(),
        )
      } else {
        bail!();
      }
    };

    if let KeyCode::Modifier(modifier_keycode) = code {
      match modifier_keycode {
        ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt => modifiers.set(Modifiers::ALT, true),
        ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => {
          modifiers.set(Modifiers::CONTROL, true)
        }
        ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift => {
          modifiers.set(Modifiers::SHIFT, true)
        }
        ModifierKeyCode::LeftSuper | ModifierKeyCode::RightSuper => {
          modifiers.set(Modifiers::SUPER, true)
        }
        ModifierKeyCode::LeftHyper | ModifierKeyCode::RightHyper => {
          modifiers.set(Modifiers::HYPER, true)
        }
        ModifierKeyCode::LeftMeta | ModifierKeyCode::RightMeta => {
          modifiers.set(Modifiers::META, true)
        }
        _ => {}
      }
    }

    if modifiers.contains(Modifiers::SHIFT) {
      if let Some(shifted_c) = codepoints
        .next()
        .and_then(|codepoint| codepoint.parse::<u32>().ok())
        .and_then(char::from_u32)
      {
        code = KeyCode::Char(shifted_c);
        modifiers.set(Modifiers::SHIFT, false);
      }
    }

    let event = Event::Key(KeyEvent {
      code,
      modifiers,
      kind,
      state: state_from_keycode | state_from_modifiers,
    });

    Ok(Some(event))
  }

  fn parse_modifiers(mask: u8) -> Modifiers {
    let modifier_mask = mask.saturating_sub(1);
    let mut modifiers = Modifiers::empty();
    if modifier_mask & 1 != 0 {
      modifiers |= Modifiers::SHIFT;
    }
    if modifier_mask & 2 != 0 {
      modifiers |= Modifiers::ALT;
    }
    if modifier_mask & 4 != 0 {
      modifiers |= Modifiers::CONTROL;
    }
    if modifier_mask & 8 != 0 {
      modifiers |= Modifiers::SUPER;
    }
    if modifier_mask & 16 != 0 {
      modifiers |= Modifiers::HYPER;
    }
    if modifier_mask & 32 != 0 {
      modifiers |= Modifiers::META;
    }
    modifiers
  }

  fn parse_modifiers_to_state(mask: u8) -> KeyEventState {
    let modifier_mask = mask.saturating_sub(1);
    let mut state = KeyEventState::empty();
    if modifier_mask & 64 != 0 {
      state |= KeyEventState::CAPS_LOCK;
    }
    if modifier_mask & 128 != 0 {
      state |= KeyEventState::NUM_LOCK;
    }
    state
  }

  fn parse_key_event_kind(kind: u8) -> KeyEventKind {
    match kind {
      1 => KeyEventKind::Press,
      2 => KeyEventKind::Repeat,
      3 => KeyEventKind::Release,
      _ => KeyEventKind::Press,
    }
  }

  fn parse_csi_modifier_key_code(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B[")); // CSI
    let s = str::from_utf8(&buffer[2..buffer.len() - 1])?;
    let mut split = s.split(';');

    split.next();

    let (modifiers, kind) =
      if let Ok((modifier_mask, kind_code)) = modifier_and_kind_parsed(&mut split) {
        (
          parse_modifiers(modifier_mask),
          parse_key_event_kind(kind_code),
        )
      } else if buffer.len() > 3 {
        (
          parse_modifiers(
            (buffer[buffer.len() - 2] as char)
              .to_digit(10)
              .ok_or(MalformedSequenceError)? as u8,
          ),
          KeyEventKind::Press,
        )
      } else {
        (Modifiers::NONE, KeyEventKind::Press)
      };
    let key = buffer[buffer.len() - 1];

    let code = match key {
      b'A' => KeyCode::Up,
      b'B' => KeyCode::Down,
      b'C' => KeyCode::Right,
      b'D' => KeyCode::Left,
      b'F' => KeyCode::End,
      b'H' => KeyCode::Home,
      b'P' => KeyCode::Function(1),
      b'Q' => KeyCode::Function(2),
      b'R' => KeyCode::Function(3),
      b'S' => KeyCode::Function(4),
      _ => bail!(),
    };

    let event = Event::Key(KeyEvent {
      code,
      modifiers,
      kind,
      state: KeyEventState::NONE,
    });

    Ok(Some(event))
  }

  fn parse_csi_special_key_code(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B[")); // CSI
    assert!(buffer.ends_with(b"~"));

    let s = str::from_utf8(&buffer[2..buffer.len() - 1])?;
    let mut split = s.split(';');

    let first = next_parsed::<u8>(&mut split)?;

    let (modifiers, kind, state) =
      if let Ok((modifier_mask, kind_code)) = modifier_and_kind_parsed(&mut split) {
        (
          parse_modifiers(modifier_mask),
          parse_key_event_kind(kind_code),
          parse_modifiers_to_state(modifier_mask),
        )
      } else {
        (Modifiers::NONE, KeyEventKind::Press, KeyEventState::NONE)
      };

    let code = match first {
      1 | 7 => KeyCode::Home,
      2 => KeyCode::Insert,
      3 => KeyCode::Delete,
      4 | 8 => KeyCode::End,
      5 => KeyCode::PageUp,
      6 => KeyCode::PageDown,
      v @ 11..=15 => KeyCode::Function(v - 10),
      v @ 17..=21 => KeyCode::Function(v - 11),
      v @ 23..=26 => KeyCode::Function(v - 12),
      v @ 28..=29 => KeyCode::Function(v - 15),
      v @ 31..=34 => KeyCode::Function(v - 17),
      _ => bail!(),
    };

    let event = Event::Key(KeyEvent {
      code,
      modifiers,
      kind,
      state,
    });

    Ok(Some(event))
  }

  fn translate_functional_key_code(codepoint: u32) -> Option<(KeyCode, KeyEventState)> {
    if let Some(keycode) = match codepoint {
      57399 => Some(KeyCode::Char('0')),
      57400 => Some(KeyCode::Char('1')),
      57401 => Some(KeyCode::Char('2')),
      57402 => Some(KeyCode::Char('3')),
      57403 => Some(KeyCode::Char('4')),
      57404 => Some(KeyCode::Char('5')),
      57405 => Some(KeyCode::Char('6')),
      57406 => Some(KeyCode::Char('7')),
      57407 => Some(KeyCode::Char('8')),
      57408 => Some(KeyCode::Char('9')),
      57409 => Some(KeyCode::Char('.')),
      57410 => Some(KeyCode::Char('/')),
      57411 => Some(KeyCode::Char('*')),
      57412 => Some(KeyCode::Char('-')),
      57413 => Some(KeyCode::Char('+')),
      57414 => Some(KeyCode::Enter),
      57415 => Some(KeyCode::Char('=')),
      57416 => Some(KeyCode::Char(',')),
      57417 => Some(KeyCode::Left),
      57418 => Some(KeyCode::Right),
      57419 => Some(KeyCode::Up),
      57420 => Some(KeyCode::Down),
      57421 => Some(KeyCode::PageUp),
      57422 => Some(KeyCode::PageDown),
      57423 => Some(KeyCode::Home),
      57424 => Some(KeyCode::End),
      57425 => Some(KeyCode::Insert),
      57426 => Some(KeyCode::Delete),
      57427 => Some(KeyCode::KeypadBegin),
      _ => None,
    } {
      return Some((keycode, KeyEventState::KEYPAD));
    }

    if let Some(keycode) = match codepoint {
      57358 => Some(KeyCode::CapsLock),
      57359 => Some(KeyCode::ScrollLock),
      57360 => Some(KeyCode::NumLock),
      57361 => Some(KeyCode::PrintScreen),
      57362 => Some(KeyCode::Pause),
      57363 => Some(KeyCode::Menu),
      57376 => Some(KeyCode::Function(13)),
      57377 => Some(KeyCode::Function(14)),
      57378 => Some(KeyCode::Function(15)),
      57379 => Some(KeyCode::Function(16)),
      57380 => Some(KeyCode::Function(17)),
      57381 => Some(KeyCode::Function(18)),
      57382 => Some(KeyCode::Function(19)),
      57383 => Some(KeyCode::Function(20)),
      57384 => Some(KeyCode::Function(21)),
      57385 => Some(KeyCode::Function(22)),
      57386 => Some(KeyCode::Function(23)),
      57387 => Some(KeyCode::Function(24)),
      57388 => Some(KeyCode::Function(25)),
      57389 => Some(KeyCode::Function(26)),
      57390 => Some(KeyCode::Function(27)),
      57391 => Some(KeyCode::Function(28)),
      57392 => Some(KeyCode::Function(29)),
      57393 => Some(KeyCode::Function(30)),
      57394 => Some(KeyCode::Function(31)),
      57395 => Some(KeyCode::Function(32)),
      57396 => Some(KeyCode::Function(33)),
      57397 => Some(KeyCode::Function(34)),
      57398 => Some(KeyCode::Function(35)),
      57428 => Some(KeyCode::Media(MediaKeyCode::Play)),
      57429 => Some(KeyCode::Media(MediaKeyCode::Pause)),
      57430 => Some(KeyCode::Media(MediaKeyCode::PlayPause)),
      57431 => Some(KeyCode::Media(MediaKeyCode::Reverse)),
      57432 => Some(KeyCode::Media(MediaKeyCode::Stop)),
      57433 => Some(KeyCode::Media(MediaKeyCode::FastForward)),
      57434 => Some(KeyCode::Media(MediaKeyCode::Rewind)),
      57435 => Some(KeyCode::Media(MediaKeyCode::TrackNext)),
      57436 => Some(KeyCode::Media(MediaKeyCode::TrackPrevious)),
      57437 => Some(KeyCode::Media(MediaKeyCode::Record)),
      57438 => Some(KeyCode::Media(MediaKeyCode::LowerVolume)),
      57439 => Some(KeyCode::Media(MediaKeyCode::RaiseVolume)),
      57440 => Some(KeyCode::Media(MediaKeyCode::MuteVolume)),
      57441 => Some(KeyCode::Modifier(ModifierKeyCode::LeftShift)),
      57442 => Some(KeyCode::Modifier(ModifierKeyCode::LeftControl)),
      57443 => Some(KeyCode::Modifier(ModifierKeyCode::LeftAlt)),
      57444 => Some(KeyCode::Modifier(ModifierKeyCode::LeftSuper)),
      57445 => Some(KeyCode::Modifier(ModifierKeyCode::LeftHyper)),
      57446 => Some(KeyCode::Modifier(ModifierKeyCode::LeftMeta)),
      57447 => Some(KeyCode::Modifier(ModifierKeyCode::RightShift)),
      57448 => Some(KeyCode::Modifier(ModifierKeyCode::RightControl)),
      57449 => Some(KeyCode::Modifier(ModifierKeyCode::RightAlt)),
      57450 => Some(KeyCode::Modifier(ModifierKeyCode::RightSuper)),
      57451 => Some(KeyCode::Modifier(ModifierKeyCode::RightHyper)),
      57452 => Some(KeyCode::Modifier(ModifierKeyCode::RightMeta)),
      57453 => Some(KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift)),
      57454 => Some(KeyCode::Modifier(ModifierKeyCode::IsoLevel5Shift)),
      _ => None,
    } {
      return Some((keycode, KeyEventState::empty()));
    }

    None
  }

  fn parse_csi_rxvt_mouse(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B[")); // CSI
    assert!(buffer.ends_with(b"M"));

    let s = str::from_utf8(&buffer[2..buffer.len() - 1])?;
    let mut split = s.split(';');

    let cb = next_parsed::<u8>(&mut split)?
      .checked_sub(32)
      .ok_or(MalformedSequenceError)?;
    let (kind, modifiers) = parse_cb(cb)?;

    let cx = next_parsed::<u16>(&mut split)? - 1;
    let cy = next_parsed::<u16>(&mut split)? - 1;

    Ok(Some(Event::Mouse(MouseEvent {
      kind,
      column: cx,
      row: cy,
      modifiers,
    })))
  }

  fn parse_csi_normal_mouse(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B[M")); // CSI M

    if buffer.len() < 6 {
      return Ok(None);
    }

    let cb = buffer[3].checked_sub(32).ok_or(MalformedSequenceError)?;
    let (kind, modifiers) = parse_cb(cb)?;

    let cx = u16::from(buffer[4].saturating_sub(33));
    let cy = u16::from(buffer[5].saturating_sub(33));

    Ok(Some(Event::Mouse(MouseEvent {
      kind,
      column: cx,
      row: cy,
      modifiers,
    })))
  }

  fn parse_csi_sgr_mouse(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B[<")); // CSI <

    if !buffer.ends_with(b"m") && !buffer.ends_with(b"M") {
      return Ok(None);
    }

    let s = str::from_utf8(&buffer[3..buffer.len() - 1])?;
    let mut split = s.split(';');

    let cb = next_parsed::<u8>(&mut split)?;
    let (kind, modifiers) = parse_cb(cb)?;

    let cx = next_parsed::<u16>(&mut split)? - 1;
    let cy = next_parsed::<u16>(&mut split)? - 1;

    let kind = if buffer.last() == Some(&b'm') {
      match kind {
        MouseEventKind::Down(button) => MouseEventKind::Up(button),
        other => other,
      }
    } else {
      kind
    };

    Ok(Some(Event::Mouse(MouseEvent {
      kind,
      column: cx,
      row: cy,
      modifiers,
    })))
  }

  fn parse_cb(cb: u8) -> Result<(MouseEventKind, Modifiers)> {
    let button_number = (cb & 0b0000_0011) | ((cb & 0b1100_0000) >> 4);
    let dragging = cb & 0b0010_0000 == 0b0010_0000;

    let kind = match (button_number, dragging) {
      (0, false) => MouseEventKind::Down(MouseButton::Left),
      (1, false) => MouseEventKind::Down(MouseButton::Middle),
      (2, false) => MouseEventKind::Down(MouseButton::Right),
      (0, true) => MouseEventKind::Drag(MouseButton::Left),
      (1, true) => MouseEventKind::Drag(MouseButton::Middle),
      (2, true) => MouseEventKind::Drag(MouseButton::Right),
      (3, false) => MouseEventKind::Up(MouseButton::Left),
      (3, true) | (4, true) | (5, true) => MouseEventKind::Moved,
      (4, false) => MouseEventKind::ScrollUp,
      (5, false) => MouseEventKind::ScrollDown,
      (6, false) => MouseEventKind::ScrollLeft,
      (7, false) => MouseEventKind::ScrollRight,
      _ => bail!(),
    };

    let mut modifiers = Modifiers::empty();

    if cb & 0b0000_0100 == 0b0000_0100 {
      modifiers |= Modifiers::SHIFT;
    }
    if cb & 0b0000_1000 == 0b0000_1000 {
      modifiers |= Modifiers::ALT;
    }
    if cb & 0b0001_0000 == 0b0001_0000 {
      modifiers |= Modifiers::CONTROL;
    }

    Ok((kind, modifiers))
  }

  fn parse_csi_bracketed_paste(buffer: &[u8]) -> Result<Option<Event>> {
    let buffer = buffer
      .strip_prefix(b"\x1b[200~")
      .expect("asserted by calling functions");

    if let Some(contents) = buffer.strip_suffix(b"\x1b[201~") {
      let paste = String::from_utf8_lossy(contents).to_string();
      Ok(Some(Event::Paste(paste)))
    } else {
      Ok(None)
    }
  }

  fn parse_csi_cursor_position(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B[")); // CSI
    assert!(buffer.ends_with(b"R"));

    let s = str::from_utf8(&buffer[2..buffer.len() - 1])?;

    let mut split = s.split(';');

    let line = next_parsed::<NonZeroU16>(&mut split)?.into();
    let col = next_parsed::<NonZeroU16>(&mut split)?.into();

    Ok(Some(Event::Csi(Csi::Cursor(
      csi::Cursor::ActivePositionReport { line, col },
    ))))
  }

  fn parse_csi_cursor_shape_query_response(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B[>")); // CSI >
    assert!(buffer.ends_with(b" q"));

    if buffer.len() < 5 {
      return Ok(None);
    }

    let s = str::from_utf8(&buffer[3..buffer.len() - 2])?;

    if s.is_empty() {
      return Ok(Some(Event::Csi(Csi::Cursor(csi::Cursor::QueryCursorShape))));
    }

    let caps: Vec<csi::MultiCursorCapability> = s
      .split(';')
      .filter(|part| !part.is_empty())
      .map(|part| {
        part
          .parse::<u8>()
          .map_err(|_| MalformedSequenceError)
          .and_then(|v| csi::MultiCursorCapability::try_from(v).map_err(|_| MalformedSequenceError))
      })
      .collect::<Result<Vec<_>>>()?;

    Ok(Some(Event::Csi(Csi::Cursor(
      csi::Cursor::CursorShapeQueryResponse(caps),
    ))))
  }

  fn parse_csi_keyboard_enhancement_flags(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B[?")); // ESC [ ?
    assert!(buffer.ends_with(b"u"));

    if buffer.len() < 5 {
      return Ok(None);
    }

    let bits = buffer[3];
    let mut flags = KittyKeyboardFlags::empty();

    if bits & 1 != 0 {
      flags |= KittyKeyboardFlags::DISAMBIGUATE_ESCAPE_CODES;
    }
    if bits & 2 != 0 {
      flags |= KittyKeyboardFlags::REPORT_EVENT_TYPES;
    }
    if bits & 4 != 0 {
      flags |= KittyKeyboardFlags::REPORT_ALTERNATE_KEYS;
    }
    if bits & 8 != 0 {
      flags |= KittyKeyboardFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
    }

    Ok(Some(Event::Csi(Csi::Keyboard(csi::Keyboard::ReportFlags(
      flags,
    )))))
  }

  fn parse_csi_primary_device_attributes(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B[?"));
    assert!(buffer.ends_with(b"c"));

    Ok(Some(Event::Csi(Csi::Device(
      csi::Device::DeviceAttributes(()),
    ))))
  }

  fn parse_csi_theme_mode(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B[?"));
    assert!(buffer.ends_with(b"n"));

    let s = str::from_utf8(&buffer[3..buffer.len() - 1])?;

    let mut split = s.split(';');

    if next_parsed::<u16>(&mut split)? != 997 {
      bail!();
    }

    let theme_mode = match next_parsed::<u8>(&mut split)? {
      1 => ThemeMode::Dark,
      2 => ThemeMode::Light,
      _ => bail!(),
    };

    Ok(Some(Event::Csi(Csi::Mode(csi::Mode::ReportTheme(
      theme_mode,
    )))))
  }

  fn parse_csi_mode(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(b"\x1B[?"));
    assert!(buffer.ends_with(b"y"));

    let s = str::from_utf8(&buffer[3..buffer.len() - 1])?;
    let s = match s.strip_suffix('$') {
      Some(s) => s,
      None => bail!(),
    };

    let mut split = s.split(';');

    let mode = match next_parsed::<u16>(&mut split)? {
      2026 => csi::DecPrivateMode::Code(csi::DecPrivateModeCode::SynchronizedOutput),
      2027 => csi::DecPrivateMode::Code(csi::DecPrivateModeCode::GraphemeClustering),
      _ => bail!(),
    };

    let setting = match next_parsed::<u8>(&mut split)? {
      0 | 4 if mode == csi::DecPrivateMode::Code(csi::DecPrivateModeCode::SynchronizedOutput) => {
        csi::DecModeSetting::NotRecognized
      }
      0 => csi::DecModeSetting::NotRecognized,
      1 => csi::DecModeSetting::Set,
      2 => csi::DecModeSetting::Reset,
      3 if mode == csi::DecPrivateMode::Code(csi::DecPrivateModeCode::GraphemeClustering) => {
        csi::DecModeSetting::PermanentlySet
      }
      4 => csi::DecModeSetting::PermanentlyReset,
      _ => bail!(),
    };

    Ok(Some(Event::Csi(Csi::Mode(
      csi::Mode::ReportDecPrivateMode { mode, setting },
    ))))
  }

  fn parse_dcs(buffer: &[u8]) -> Result<Option<Event>> {
    assert!(buffer.starts_with(escape::DCS.as_bytes()));
    if !buffer.ends_with(escape::ST.as_bytes()) {
      return Ok(None);
    }
    match buffer[buffer.len() - 3] {
      b'm' => {
        if buffer.get(3..5) != Some(b"$r") {
          bail!();
        }
        let is_request_valid = match buffer[2] {
          b'1' => true,
          b'0' => false,
          _ => bail!(),
        };
        let s = str::from_utf8(&buffer[5..buffer.len() - 3])?;
        let mut sgrs = Vec::new();
        for sgr in s.split(';') {
          sgrs.push(parse_sgr(sgr)?);
        }
        Ok(Some(Event::Dcs(dcs::Dcs::Response {
          is_request_valid,
          value: dcs::DcsResponse::GraphicRendition(sgrs),
        })))
      }
      _ => bail!(),
    }
  }

  fn parse_sgr(buffer: &str) -> Result<csi::Sgr> {
    use csi::Sgr;
    use style::*;

    let sgr = match buffer {
      "0" => Sgr::Reset,
      "22" => Sgr::Intensity(Intensity::Normal),
      "1" => Sgr::Intensity(Intensity::Bold),
      "2" => Sgr::Intensity(Intensity::Dim),
      "24" => Sgr::Underline(Underline::None),
      "4" => Sgr::Underline(Underline::Single),
      "21" => Sgr::Underline(Underline::Double),
      "4:3 " => Sgr::Underline(Underline::Curly),
      "4:4" => Sgr::Underline(Underline::Dotted),
      "4:5" => Sgr::Underline(Underline::Dashed),
      "25" => Sgr::Blink(Blink::None),
      "5" => Sgr::Blink(Blink::Slow),
      "6" => Sgr::Blink(Blink::Rapid),
      "3" => Sgr::Italic(true),
      "23" => Sgr::Italic(false),
      "7" => Sgr::Reverse(true),
      "27" => Sgr::Reverse(false),
      "8" => Sgr::Invisible(true),
      "28" => Sgr::Invisible(false),
      "9" => Sgr::StrikeThrough(true),
      "29" => Sgr::StrikeThrough(false),
      "53" => Sgr::Overline(true),
      "55" => Sgr::Overline(false),
      "10" => Sgr::Font(Font::Default),
      "11" => Sgr::Font(Font::Alternate(1)),
      "12" => Sgr::Font(Font::Alternate(2)),
      "13" => Sgr::Font(Font::Alternate(3)),
      "14" => Sgr::Font(Font::Alternate(4)),
      "15" => Sgr::Font(Font::Alternate(5)),
      "16" => Sgr::Font(Font::Alternate(6)),
      "17" => Sgr::Font(Font::Alternate(7)),
      "18" => Sgr::Font(Font::Alternate(8)),
      "19" => Sgr::Font(Font::Alternate(9)),
      "75" => Sgr::VerticalAlign(VerticalAlign::BaseLine),
      "73" => Sgr::VerticalAlign(VerticalAlign::SuperScript),
      "74" => Sgr::VerticalAlign(VerticalAlign::SubScript),
      "39" => Sgr::Foreground(ColorSpec::Reset),
      "30" => Sgr::Foreground(ColorSpec::BLACK),
      "31" => Sgr::Foreground(ColorSpec::RED),
      "32" => Sgr::Foreground(ColorSpec::GREEN),
      "33" => Sgr::Foreground(ColorSpec::YELLOW),
      "34" => Sgr::Foreground(ColorSpec::BLUE),
      "35" => Sgr::Foreground(ColorSpec::MAGENTA),
      "36" => Sgr::Foreground(ColorSpec::CYAN),
      "37" => Sgr::Foreground(ColorSpec::WHITE),
      "90" => Sgr::Foreground(ColorSpec::BRIGHT_BLACK),
      "91" => Sgr::Foreground(ColorSpec::BRIGHT_RED),
      "92" => Sgr::Foreground(ColorSpec::BRIGHT_GREEN),
      "93" => Sgr::Foreground(ColorSpec::BRIGHT_YELLOW),
      "94" => Sgr::Foreground(ColorSpec::BRIGHT_BLUE),
      "95" => Sgr::Foreground(ColorSpec::BRIGHT_MAGENTA),
      "96" => Sgr::Foreground(ColorSpec::BRIGHT_CYAN),
      "97" => Sgr::Foreground(ColorSpec::BRIGHT_WHITE),
      "49" => Sgr::Background(ColorSpec::Reset),
      "40" => Sgr::Background(ColorSpec::BLACK),
      "41" => Sgr::Background(ColorSpec::RED),
      "42" => Sgr::Background(ColorSpec::GREEN),
      "43" => Sgr::Background(ColorSpec::YELLOW),
      "44" => Sgr::Background(ColorSpec::BLUE),
      "45" => Sgr::Background(ColorSpec::MAGENTA),
      "46" => Sgr::Background(ColorSpec::CYAN),
      "47" => Sgr::Background(ColorSpec::WHITE),
      "100" => Sgr::Background(ColorSpec::BRIGHT_BLACK),
      "101" => Sgr::Background(ColorSpec::BRIGHT_RED),
      "102" => Sgr::Background(ColorSpec::BRIGHT_GREEN),
      "103" => Sgr::Background(ColorSpec::BRIGHT_YELLOW),
      "104" => Sgr::Background(ColorSpec::BRIGHT_BLUE),
      "105" => Sgr::Background(ColorSpec::BRIGHT_MAGENTA),
      "106" => Sgr::Background(ColorSpec::BRIGHT_CYAN),
      "107" => Sgr::Background(ColorSpec::BRIGHT_WHITE),
      "59" => Sgr::UnderlineColor(ColorSpec::Reset),
      _ => {
        let mut split = buffer.split(':').filter(|s| !s.is_empty());
        let first = next_parsed::<u8>(&mut split)?;
        let color = match next_parsed::<u8>(&mut split)? {
          2 => RgbColor {
            red: next_parsed::<u8>(&mut split)?,
            green: next_parsed::<u8>(&mut split)?,
            blue: next_parsed::<u8>(&mut split)?,
          }
          .into(),
          5 => ColorSpec::PaletteIndex(next_parsed::<u8>(&mut split)?),
          6 => RgbaColor {
            red: next_parsed::<u8>(&mut split)?,
            green: next_parsed::<u8>(&mut split)?,
            blue: next_parsed::<u8>(&mut split)?,
            alpha: next_parsed::<u8>(&mut split)?,
          }
          .into(),
          _ => bail!(),
        };
        match first {
          38 => Sgr::Foreground(color),
          48 => Sgr::Background(color),
          58 => Sgr::UnderlineColor(color),
          _ => bail!(),
        }
      }
    };
    Ok(sgr)
  }
}
pub mod style {

  use std::{
    borrow::Cow,
    fmt::{self, Display},
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
  };

  use crate::termina::escape::{
    self,
    csi::{Csi, Sgr},
  };

  #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
  pub enum Underline {
    #[default]
    None = 0,

    Single = 1,

    Double = 2,

    Curly = 3,

    Dotted = 4,

    Dashed = 5,
  }

  #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
  pub enum CursorStyle {
    #[default]
    Default = 0,
    BlinkingBlock = 1,
    SteadyBlock = 2,
    BlinkingUnderline = 3,
    SteadyUnderline = 4,
    BlinkingBar = 5,
    SteadyBar = 6,
  }

  impl Display for CursorStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", *self as u8)
    }
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
  }

  impl RgbColor {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
      Self { red, green, blue }
    }

    fn channel_from_hex(s: &str) -> Result<u8, InvalidFormatError> {
      if s.is_empty() || s.len() > 4 {
        return Err(InvalidFormatError);
      }
      let color: u16 = u16::from_str_radix(s, 16).map_err(|_| InvalidFormatError)?;
      let divisor: usize = match s.len() {
        1 => 0xf,
        2 => 0xff,
        3 => 0xfff,
        4 => 0xffff,
        _ => return Err(InvalidFormatError),
      };
      Ok(((color as usize) * 0xff / divisor) as u8)
    }
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct InvalidFormatError;

  impl FromStr for RgbColor {
    type Err = InvalidFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
      if let Some(rgb) = s.strip_prefix("rgb:") {
        let mut parts = rgb.split('/').map(Self::channel_from_hex);
        let Some(r) = parts.next().transpose()? else {
          return Err(InvalidFormatError);
        };
        let Some(g) = parts.next().transpose()? else {
          return Err(InvalidFormatError);
        };
        let Some(b) = parts.next().transpose()? else {
          return Err(InvalidFormatError);
        };
        Ok(Self::new(r, g, b))
      } else if let Some(hex) = s.strip_prefix('#') {
        let (r, g, b) = match hex.len() {
          3 => (
            Self::channel_from_hex(&hex[0..1])?,
            Self::channel_from_hex(&hex[1..2])?,
            Self::channel_from_hex(&hex[2..3])?,
          ),
          6 => (
            Self::channel_from_hex(&hex[0..2])?,
            Self::channel_from_hex(&hex[2..4])?,
            Self::channel_from_hex(&hex[4..6])?,
          ),
          9 => (
            Self::channel_from_hex(&hex[0..3])?,
            Self::channel_from_hex(&hex[3..6])?,
            Self::channel_from_hex(&hex[6..9])?,
          ),
          12 => (
            Self::channel_from_hex(&hex[0..4])?,
            Self::channel_from_hex(&hex[4..8])?,
            Self::channel_from_hex(&hex[8..12])?,
          ),
          _ => return Err(InvalidFormatError),
        };
        Ok(Self::new(r, g, b))
      } else {
        Err(InvalidFormatError)
      }
    }
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct RgbaColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
  }

  impl From<RgbaColor> for RgbColor {
    fn from(color: RgbaColor) -> Self {
      Self {
        red: color.red,
        green: color.green,
        blue: color.blue,
      }
    }
  }

  impl From<RgbColor> for RgbaColor {
    fn from(color: RgbColor) -> Self {
      Self {
        red: color.red,
        green: color.green,
        blue: color.blue,
        alpha: 255,
      }
    }
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub(crate) enum AnsiColor {
    Black = 0,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
  }

  pub(crate) type PaletteIndex = u8;

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum ColorSpec {
    Reset,

    PaletteIndex(PaletteIndex),

    TrueColor(RgbaColor),
  }

  impl ColorSpec {
    pub const BLACK: Self = Self::PaletteIndex(AnsiColor::Black as PaletteIndex);
    pub const RED: Self = Self::PaletteIndex(AnsiColor::Red as PaletteIndex);
    pub const GREEN: Self = Self::PaletteIndex(AnsiColor::Green as PaletteIndex);
    pub const YELLOW: Self = Self::PaletteIndex(AnsiColor::Yellow as PaletteIndex);
    pub const BLUE: Self = Self::PaletteIndex(AnsiColor::Blue as PaletteIndex);
    pub const MAGENTA: Self = Self::PaletteIndex(AnsiColor::Magenta as PaletteIndex);
    pub const CYAN: Self = Self::PaletteIndex(AnsiColor::Cyan as PaletteIndex);
    pub const WHITE: Self = Self::PaletteIndex(AnsiColor::White as PaletteIndex);
    pub const BRIGHT_BLACK: Self = Self::PaletteIndex(AnsiColor::BrightBlack as PaletteIndex);
    pub const BRIGHT_RED: Self = Self::PaletteIndex(AnsiColor::BrightRed as PaletteIndex);
    pub const BRIGHT_GREEN: Self = Self::PaletteIndex(AnsiColor::BrightGreen as PaletteIndex);
    pub const BRIGHT_YELLOW: Self = Self::PaletteIndex(AnsiColor::BrightYellow as PaletteIndex);
    pub const BRIGHT_BLUE: Self = Self::PaletteIndex(AnsiColor::BrightBlue as PaletteIndex);
    pub const BRIGHT_MAGENTA: Self = Self::PaletteIndex(AnsiColor::BrightMagenta as PaletteIndex);
    pub const BRIGHT_CYAN: Self = Self::PaletteIndex(AnsiColor::BrightCyan as PaletteIndex);
    pub const BRIGHT_WHITE: Self = Self::PaletteIndex(AnsiColor::BrightWhite as PaletteIndex);
  }

  impl From<AnsiColor> for ColorSpec {
    fn from(color: AnsiColor) -> Self {
      Self::PaletteIndex(color as u8)
    }
  }

  impl From<RgbColor> for ColorSpec {
    fn from(color: RgbColor) -> Self {
      Self::TrueColor(color.into())
    }
  }

  impl From<RgbaColor> for ColorSpec {
    fn from(color: RgbaColor) -> Self {
      Self::TrueColor(color)
    }
  }

  #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
  pub enum Intensity {
    #[default]
    Normal,
    Bold,
    Dim,
  }

  #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
  pub enum Blink {
    #[default]
    None,
    Slow,
    Rapid,
  }

  #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
  pub enum Font {
    #[default]
    Default,

    Alternate(u8),
  }

  #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
  pub enum VerticalAlign {
    #[default]
    BaseLine = 0,
    SuperScript = 1,
    SubScript = 2,
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct Stylized<'a> {
    content: Cow<'a, str>,
    styles: Vec<Sgr>,
  }

  static INITIALIZER: parking_lot::Once = parking_lot::Once::new();
  static NO_COLOR: AtomicBool = AtomicBool::new(false);

  impl Stylized<'_> {
    fn is_ansi_color_disabled() -> bool {
      INITIALIZER.call_once(|| {
        NO_COLOR.store(
          std::env::var("NO_COLOR").is_ok_and(|e| !e.is_empty()),
          Ordering::SeqCst,
        );
      });
      NO_COLOR.load(Ordering::SeqCst)
    }
  }

  impl Display for Stylized<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      let no_color = Self::is_ansi_color_disabled();
      let mut styles = self
        .styles
        .iter()
        .filter(|sgr| {
          !(no_color
            && matches!(
              sgr,
              Sgr::Foreground(_) | Sgr::Background(_) | Sgr::UnderlineColor(_)
            ))
        })
        .peekable();

      if styles.peek().is_none() {
        write!(f, "{}", self.content)?;
      } else {
        write!(f, "{}0", escape::CSI)?;
        for sgr in styles {
          write!(f, ";{sgr}")?;
        }
        write!(f, "m{}{}", self.content, Csi::Sgr(Sgr::Reset))?;
      }
      Ok(())
    }
  }

  pub trait StyleExt<'a>: Sized {
    fn stylized(self) -> Stylized<'a>;

    fn foreground(self, color: impl Into<ColorSpec>) -> Stylized<'a> {
      let mut this = self.stylized();
      this.styles.push(Sgr::Foreground(color.into()));
      this
    }
    fn red(self) -> Stylized<'a> {
      self.foreground(ColorSpec::RED)
    }
    fn yellow(self) -> Stylized<'a> {
      self.foreground(ColorSpec::YELLOW)
    }
    fn green(self) -> Stylized<'a> {
      self.foreground(ColorSpec::GREEN)
    }
    fn underlined(self) -> Stylized<'a> {
      let mut this = self.stylized();
      this.styles.push(Sgr::Underline(Underline::Single));
      this
    }
    fn bold(self) -> Stylized<'a> {
      let mut this = self.stylized();
      this.styles.push(Sgr::Intensity(Intensity::Bold));
      this
    }
  }

  impl<'a> StyleExt<'a> for Cow<'a, str> {
    fn stylized(self) -> Stylized<'a> {
      Stylized {
        content: self,
        styles: Vec::with_capacity(2),
      }
    }
  }

  impl<'a> StyleExt<'a> for &'a str {
    fn stylized(self) -> Stylized<'a> {
      Cow::Borrowed(self).stylized()
    }
  }

  impl StyleExt<'static> for String {
    fn stylized(self) -> Stylized<'static> {
      Cow::<str>::Owned(self).stylized()
    }
  }

  impl<'a> StyleExt<'a> for Stylized<'a> {
    fn stylized(self) -> Stylized<'a> {
      self
    }
  }
}
mod terminal {

  use rustix::termios::{self, Termios};
  use std::{
    fs,
    io::{self, BufWriter, IsTerminal as _, Write as _},
    os::unix::prelude::*,
  };

  use crate::termina::{Event, EventReader, WindowSize, event::source::UnixEventSource};

  const TERMINAL_BUF_SIZE: usize = 4096;

  #[derive(Debug)]
  pub enum FileDescriptor {
    Owned(OwnedFd),

    Borrowed(BorrowedFd<'static>),
  }

  impl AsFd for FileDescriptor {
    fn as_fd(&self) -> BorrowedFd<'_> {
      match self {
        Self::Owned(fd) => fd.as_fd(),
        Self::Borrowed(fd) => *fd,
      }
    }
  }

  impl FileDescriptor {
    const STDIN: Self = Self::Borrowed(rustix::stdio::stdin());

    const STDOUT: Self = Self::Borrowed(rustix::stdio::stdout());

    fn try_clone(&self) -> io::Result<Self> {
      let this = match self {
        Self::Owned(fd) => Self::Owned(fd.try_clone()?),
        Self::Borrowed(fd) => Self::Borrowed(*fd),
      };
      Ok(this)
    }
  }

  impl io::Read for FileDescriptor {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
      let read = rustix::io::read(&self, buf)?;
      Ok(read)
    }
  }

  impl io::Write for FileDescriptor {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
      let written = rustix::io::write(self, buf)?;
      Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
      Ok(())
    }
  }

  fn open_pty() -> io::Result<(FileDescriptor, FileDescriptor)> {
    let read = if io::stdin().is_terminal() {
      FileDescriptor::STDIN
    } else {
      open_dev_tty()?
    };
    let write = if io::stdout().is_terminal() {
      FileDescriptor::STDOUT
    } else {
      open_dev_tty()?
    };

    Ok((read, write))
  }

  fn open_dev_tty() -> io::Result<FileDescriptor> {
    let file = fs::OpenOptions::new()
      .read(true)
      .write(true)
      .open("/dev/tty")?;
    Ok(FileDescriptor::Owned(file.into()))
  }

  impl From<termios::Winsize> for WindowSize {
    fn from(size: termios::Winsize) -> Self {
      Self {
        cols: size.ws_col,
        rows: size.ws_row,
        pixel_width: Some(size.ws_xpixel),
        pixel_height: Some(size.ws_ypixel),
      }
    }
  }

  #[derive(Debug)]
  pub struct UnixTerminal {
    reader: EventReader,
    write: BufWriter<FileDescriptor>,
    original_termios: Termios,
    has_panic_hook: bool,
  }

  impl UnixTerminal {
    pub fn new() -> io::Result<Self> {
      let (read, write) = open_pty()?;
      let source = UnixEventSource::new(read, write.try_clone()?)?;
      let original_termios = termios::tcgetattr(&write)?;
      let reader = EventReader::new(source);

      Ok(Self {
        reader,
        write: BufWriter::with_capacity(TERMINAL_BUF_SIZE, write),
        original_termios,
        has_panic_hook: false,
      })
    }
  }

  impl Terminal for UnixTerminal {
    fn enter_raw_mode(&mut self) -> io::Result<()> {
      let mut termios = termios::tcgetattr(self.write.get_ref())?;
      termios.make_raw();
      termios::tcsetattr(
        self.write.get_ref(),
        termios::OptionalActions::Flush,
        &termios,
      )?;

      Ok(())
    }

    fn enter_cooked_mode(&mut self) -> io::Result<()> {
      termios::tcsetattr(
        self.write.get_ref(),
        termios::OptionalActions::Now,
        &self.original_termios,
      )?;
      Ok(())
    }

    fn get_dimensions(&self) -> io::Result<WindowSize> {
      let winsize = termios::tcgetwinsize(self.write.get_ref())?;
      let mut size: WindowSize = winsize.into();

      if size.cols == 0 || size.rows == 0 {
        if let Some(rows) = std::env::var("LINES")
          .ok()
          .and_then(|l| l.parse::<u16>().ok())
        {
          size.rows = rows;
        }
        if let Some(cols) = std::env::var("COLUMNS")
          .ok()
          .and_then(|c| c.parse::<u16>().ok())
        {
          size.cols = cols;
        }
      }
      if size.cols == 0 || size.rows == 0 {
        Err(io::Error::new(
          io::ErrorKind::Other,
          "cannot read non-zero cols/rows from ioctl or COLUMNS/LINES environment variables",
        ))
      } else {
        Ok(size)
      }
    }

    fn event_reader(&self) -> EventReader {
      self.reader.clone()
    }

    fn poll<F: Fn(&Event) -> bool>(
      &self,
      filter: F,
      timeout: Option<std::time::Duration>,
    ) -> io::Result<bool> {
      self.reader.poll(timeout, filter)
    }

    fn read<F: Fn(&Event) -> bool>(&self, filter: F) -> io::Result<Event> {
      self.reader.read(filter)
    }

    fn set_panic_hook(&mut self, f: impl Fn(&mut FileDescriptor) + Send + Sync + 'static) {
      let original_termios = self.original_termios.clone();
      let hook = std::panic::take_hook();
      std::panic::set_hook(Box::new(move |info| {
        if let Ok((_read, mut write)) = open_pty() {
          f(&mut write);
          let _ = termios::tcsetattr(write, termios::OptionalActions::Now, &original_termios);
        }
        hook(info);
      }));
      self.has_panic_hook = true;
    }
  }

  impl Drop for UnixTerminal {
    fn drop(&mut self) {
      if !self.has_panic_hook || !std::thread::panicking() {
        let _ = self.flush();
        let _ = self.enter_cooked_mode();
      }
    }
  }

  impl io::Write for UnixTerminal {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
      self.write.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
      self.write.flush()
    }
  }

  use std::time::Duration;

  pub type PlatformTerminal = UnixTerminal;

  pub type PlatformHandle = FileDescriptor;

  pub trait Terminal: io::Write {
    fn enter_raw_mode(&mut self) -> io::Result<()>;

    fn enter_cooked_mode(&mut self) -> io::Result<()>;

    fn get_dimensions(&self) -> io::Result<WindowSize>;

    fn event_reader(&self) -> EventReader;

    fn poll<F: Fn(&Event) -> bool>(&self, filter: F, timeout: Option<Duration>)
    -> io::Result<bool>;

    fn read<F: Fn(&Event) -> bool>(&self, filter: F) -> io::Result<Event>;
    fn set_panic_hook(&mut self, f: impl Fn(&mut PlatformHandle) + Send + Sync + 'static);
  }
}

use std::{fmt, num::NonZeroU16};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OneBased(NonZeroU16);

impl OneBased {
  pub const fn new(n: u16) -> Option<Self> {
    match NonZeroU16::new(n) {
      Some(n) => Some(Self(n)),
      None => None,
    }
  }

  pub const fn from_zero_based(n: u16) -> Self {
    assert!(n < u16::MAX);
    Self(unsafe { NonZeroU16::new_unchecked(n + 1) })
  }

  pub const fn get(self) -> u16 {
    self.0.get()
  }

  pub const fn get_zero_based(self) -> u16 {
    self.get() - 1
  }
}

impl Default for OneBased {
  fn default() -> Self {
    Self(unsafe { NonZeroU16::new_unchecked(1) })
  }
}

impl fmt::Display for OneBased {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.0.fmt(f)
  }
}

impl From<NonZeroU16> for OneBased {
  fn from(n: NonZeroU16) -> Self {
    Self(n)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
  #[doc(alias = "width")]
  pub cols: u16,

  #[doc(alias = "height")]
  pub rows: u16,

  pub(crate) pixel_width: Option<u16>,

  pub(crate) pixel_height: Option<u16>,
}
pub use event::stream::EventStream;
pub use event::{Event, reader::EventReader};
pub use terminal::{PlatformHandle, PlatformTerminal, Terminal};
