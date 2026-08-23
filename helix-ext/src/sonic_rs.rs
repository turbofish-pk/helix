#![allow(clippy::needless_lifetimes)]
#![allow(unsafe_op_in_unsafe_fn)]

use bytes::Bytes;
use faststr::FastStr;

use sonic_number::{Error as NumberError, ParserNumber, parse_number};
use sonic_simd::{BitMask, Mask, Simd, i8x32, m8x32, u8x32, u8x64};
use std::{
  borrow::Cow,
  error,
  fmt::{Debug, Result as FmtResult},
  num::NonZeroU8,
  ops::Deref,
  pin::Pin,
  ptr::NonNull,
  slice::{from_raw_parts, from_raw_parts_mut},
  str::{FromStr, from_utf8_unchecked},
};

use core::{
  fmt::{self, Display},
  result,
};
use serde::{
  de::{self, Expected, Unexpected},
  forward_to_deserialize_any,
};

use thiserror::Error as ErrorTrait;

macro_rules! tri {
  ($e:expr $(,)?) => {
    match $e {
      Ok(val) => val,
      Err(err) => {
        return Err(err);
      }
    }
  };
}

enum ParsedSlice<'b, 'c> {
  Borrowed {
    slice: &'b [u8],
    buf: &'c mut Vec<u8>,
  },
  Copied(&'c mut Vec<u8>),
}
pub struct SonicRsError {
  pub err: Box<SonicRsErrorImpl>,
}

pub type SonicRsResult<T> = result::Result<T, SonicRsError>;

impl SonicRsError {
  fn line(&self) -> usize {
    self.err.line
  }

  fn offset(&self) -> usize {
    self.err.index
  }
}

#[allow(clippy::fallible_impl_from)]
impl From<SonicRsError> for std::io::Error {
  fn from(j: SonicRsError) -> Self {
    match j.err.code {
      SonicRsErrorCode::EofWhileParsing => {
        std::io::Error::new(std::io::ErrorKind::UnexpectedEof, j)
      }
      _ => std::io::Error::new(std::io::ErrorKind::InvalidData, j),
    }
  }
}
pub struct SonicRsErrorImpl {
  code: SonicRsErrorCode,
  index: usize,
  line: usize,
  column: usize,
  descript: Option<String>,
}

#[derive(ErrorTrait, Debug)]
#[non_exhaustive]
enum SonicRsErrorCode {
  #[error("{0}")]
  Message(Cow<'static, str>),
  #[error("EOF while parsing")]
  EofWhileParsing,
  #[error("Expected this character to be a ':' while parsing")]
  ExpectedColon,
  #[error("Expected this character to be either a ',' or a ']' while parsing")]
  ExpectedArrayCommaOrEnd,
  #[error("Expected this character to be either a ',' or a '}}' while parsing")]
  ExpectedObjectCommaOrEnd,
  #[error("Invalid literal (`true`, `false`, or a `null`) while parsing")]
  InvalidLiteral,
  #[error("Invalid JSON value")]
  InvalidJsonValue,
  #[error("Invalid escape chars")]
  InvalidEscape,
  #[error("Invalid number")]
  InvalidNumber,
  #[error("Number is bigger than the maximum value of its type")]
  NumberOutOfRange,
  #[error("Invalid unicode code point")]
  InvalidUnicodeCodePoint,
  #[error("Invalid UTF-8 characters in json")]
  InvalidUTF8,
  #[error("Control character found while parsing a string")]
  ControlCharacterWhileParsingString,
  #[error("Expected this character to be '\"' or '}}'")]
  ExpectObjectKeyOrEnd,
  #[error("JSON has a comma after the last value in an array or object")]
  TrailingComma,
  #[error("JSON has non-whitespace trailing characters after the value")]
  TrailingCharacters,
  #[error("Encountered nesting of JSON maps and arrays more than 128 layers deep")]
  RecursionLimitExceeded,
  #[error("Invalid surrogate Unicode code point")]
  InvalidSurrogateUnicodeCodePoint,
  #[error("Float number must be finite, not be Infinity or NaN")]
  FloatMustBeFinite,
  #[error("Expect a numeric key in Value")]
  ExpectedNumericKey,
  #[error("Expect a quote")]
  ExpectedQuote,
}

impl From<NumberError> for SonicRsErrorCode {
  fn from(err: NumberError) -> Self {
    match err {
      NumberError::InvalidNumber => SonicRsErrorCode::InvalidNumber,
      NumberError::FloatMustBeFinite => SonicRsErrorCode::FloatMustBeFinite,
    }
  }
}

impl SonicRsError {
  #[cold]
  fn syntax(code: SonicRsErrorCode, json: &[u8], index: usize) -> Self {
    let position = Position::from_index(index, json);
    let mut start = index.saturating_sub(8);
    let mut end = if index + 8 > json.len() {
      json.len()
    } else {
      index + 8
    };

    while start > 0 && index - start <= 16 && (json[start] & 0b1100_0000) == 0b1000_0000 {
      start -= 1;
    }

    while end < json.len() && end - index <= 16 && (json[end - 1] & 0b1100_0000) == 0b1000_0000 {
      end += 1;
    }

    let fragment = String::from_utf8_lossy(&json[start..end]).to_string();
    let left = index - start;
    let right = if end - index > 1 {
      end - (index + 1)
    } else {
      0
    };
    let mask = ".".repeat(left) + "^" + &".".repeat(right);
    let descript = format!("\n\n\t{fragment}\n\t{mask}\n");

    SonicRsError {
      err: Box::new(SonicRsErrorImpl {
        code,
        line: position.line,
        column: position.column,
        index,
        descript: Some(descript),
      }),
    }
  }

  #[cold]
  fn error_code(self) -> SonicRsErrorCode {
    self.err.code
  }
}

impl serde::de::StdError for SonicRsError {
  fn source(&self) -> Option<&(dyn error::Error + 'static)> {
    None
  }
}

impl Display for SonicRsError {
  fn fmt(&self, f: &mut fmt::Formatter) -> FmtResult {
    Display::fmt(&*self.err, f)
  }
}

impl Display for SonicRsErrorImpl {
  fn fmt(&self, f: &mut fmt::Formatter) -> FmtResult {
    if self.line != 0 {
      write!(
        f,
        "{} at line {} column {}{}",
        self.code,
        self.line,
        self.column,
        self.descript.as_ref().unwrap_or(&"".to_string())
      )
    } else {
      write!(f, "{}", self.code)
    }
  }
}

impl Debug for SonicRsError {
  fn fmt(&self, f: &mut fmt::Formatter) -> FmtResult {
    Display::fmt(&self, f)
  }
}

impl de::Error for SonicRsError {
  #[cold]
  fn custom<T: Display>(msg: T) -> SonicRsError {
    make_error(msg.to_string())
  }

  #[cold]
  fn invalid_type(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
    if let de::Unexpected::Unit = unexp {
      make_error(format!("invalid type: null, expected {exp}"))
    } else {
      make_error(format!("invalid type: {unexp}, expected {exp}"))
    }
  }
}

#[cold]
fn make_error(mut msg: String) -> SonicRsError {
  let (line, column) = parse_line_col(&mut msg).unwrap_or((0, 0));
  SonicRsError {
    err: Box::new(SonicRsErrorImpl {
      code: SonicRsErrorCode::Message(msg.into()),
      line,
      index: 0,
      column,
      descript: None,
    }),
  }
}

fn parse_line_col(msg: &mut String) -> Option<(usize, usize)> {
  let start_of_suffix = msg.rfind(" at line ")?;

  let start_of_line = start_of_suffix + " at line ".len();
  let mut end_of_line = start_of_line;
  while starts_with_digit(&msg[end_of_line..]) {
    end_of_line += 1;
  }

  if !msg[end_of_line..].starts_with(" column ") {
    return None;
  }

  let start_of_column = end_of_line + " column ".len();
  let mut end_of_column = start_of_column;
  while starts_with_digit(&msg[end_of_column..]) {
    end_of_column += 1;
  }

  if end_of_column < msg.len() {
    return None;
  }

  let line = match usize::from_str(&msg[start_of_line..end_of_line]) {
    Ok(line) => line,
    Err(_) => return None,
  };
  let column = match usize::from_str(&msg[start_of_column..end_of_column]) {
    Ok(column) => column,
    Err(_) => return None,
  };

  msg.truncate(start_of_suffix);
  Some((line, column))
}

fn starts_with_digit(slice: &str) -> bool {
  match slice.as_bytes().first() {
    None => false,
    Some(&byte) => byte.is_ascii_digit(),
  }
}

enum Reference<'b, 'c, T>
where
  T: ?Sized + 'static,
{
  Borrowed(&'b T),
  Copied(&'c T),
}

impl<'b, 'c> From<Reference<'b, 'c, str>> for Cow<'b, str> {
  fn from(value: Reference<'b, 'c, str>) -> Self {
    match value {
      Reference::Borrowed(b) => Cow::Owned(b.to_string()),
      Reference::Copied(c) => Cow::Owned(c.to_string()),
    }
  }
}

impl<'b, 'c, T: Debug + ?Sized + 'static> Debug for Reference<'b, 'c, T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Borrowed(c) => write!(f, "Borrowed({c:?})"),
      Self::Copied(c) => write!(f, "Copied({c:?})"),
    }
  }
}

impl<'b, 'c, T> Deref for Reference<'b, 'c, T>
where
  T: ?Sized + 'static,
{
  type Target = T;

  fn deref(&self) -> &Self::Target {
    match *self {
      Reference::Borrowed(b) => b,
      Reference::Copied(c) => c,
    }
  }
}

impl<'b, 'c> Deref for ParsedSlice<'b, 'c> {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    match self {
      ParsedSlice::Borrowed { slice, buf: _ } => slice,
      ParsedSlice::Copied(c) => c.as_slice(),
    }
  }
}

fn as_str(data: &[u8]) -> &str {
  debug_assert!(from_utf8(data).is_ok(), "invalid utf-8 in as_str");
  unsafe { from_utf8_unchecked(data) }
}

macro_rules! impl_get_escaped_branchless {
  ($name:ident, $ty:ty, $even_bits:expr) => {
    #[inline(always)]
    fn $name(prev_escaped: &mut $ty, backslash: $ty) -> $ty {
      const EVEN_BITS: $ty = $even_bits;
      let backslash = backslash & (!*prev_escaped);
      let follows_escape = (backslash << 1) | *prev_escaped;
      let odd_sequence_starts = backslash & !EVEN_BITS & !follows_escape;
      let (sequences_starting_on_even_bits, overflow) =
        odd_sequence_starts.overflowing_add(backslash);
      *prev_escaped = overflow as $ty;
      let invert_mask = sequences_starting_on_even_bits << 1;
      (EVEN_BITS ^ invert_mask) & follows_escape
    }
  };
}

impl_get_escaped_branchless!(get_escaped_branchless_u32, u32, 0x5555_5555);
impl_get_escaped_branchless!(get_escaped_branchless_u64, u64, 0x5555_5555_5555_5555);

macro_rules! perr {
  ($self:ident, $err:expr) => {{ Err($self.error($err)) }};
}

#[inline(always)]
fn is_whitespace(ch: u8) -> bool {
  const SPACE_MASK: u64 = (1u64 << b' ') | (1u64 << b'\r') | (1u64 << b'\n') | (1u64 << b'\t');
  1u64
    .checked_shl(ch as u32)
    .is_some_and(|v| v & SPACE_MASK != 0)
}

#[inline(always)]
fn get_string_bits(data: &[u8; 64], prev_instring: &mut u64, prev_escaped: &mut u64) -> u64 {
  let v = unsafe { u8x64::from_slice_unaligned_unchecked(data) };

  let bs_bits = (v.eq(&u8x64::splat(b'\\'))).bitmask();
  let escaped: u64;
  if bs_bits != 0 {
    escaped = get_escaped_branchless_u64(prev_escaped, bs_bits);
  } else {
    escaped = *prev_escaped;
    *prev_escaped = 0;
  }
  let quote_bits = (v.eq(&u8x64::splat(b'"'))).bitmask() & !escaped;
  let in_string = unsafe { prefix_xor(quote_bits) ^ *prev_instring };
  *prev_instring = (in_string as i64 >> 63) as u64;
  in_string
}

#[inline(always)]
fn skip_container_loop(
  input: &[u8; 64],        /* a 64-bytes slice from json */
  prev_instring: &mut u64, /* the bitmap of last string */
  prev_escaped: &mut u64,
  lbrace_num: &mut usize,
  rbrace_num: &mut usize,
  left: u8,
  right: u8,
) -> Option<NonZeroU8> {
  let instring = get_string_bits(input, prev_instring, prev_escaped);
  let v = unsafe { u8x64::from_slice_unaligned_unchecked(input) };
  let last_lbrace_num = *lbrace_num;
  let mut rbrace = (v.eq(&u8x64::splat(right))).bitmask() & !instring;
  let lbrace = (v.eq(&u8x64::splat(left))).bitmask() & !instring;
  while rbrace != 0 {
    *rbrace_num += 1;
    *lbrace_num = last_lbrace_num + (lbrace & (rbrace - 1)).count_ones() as usize;
    let is_closed = lbrace_num < rbrace_num;
    if is_closed {
      debug_assert_eq!(*rbrace_num, *lbrace_num + 1);
      let cnt = rbrace.trailing_zeros() + 1;
      return unsafe { Some(NonZeroU8::new_unchecked(cnt as u8)) };
    }
    rbrace &= rbrace - 1;
  }
  *lbrace_num = last_lbrace_num + lbrace.count_ones() as usize;
  None
}

struct Parser<R> {
  read: R,
  error_index: usize,   // mark the error position
  nospace_bits: u64,    // SIMD marked nospace bitmap
  nospace_start: isize, // the start position of nospace_bits
  cfg: DeserializeCfg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseStatus {
  None,
  HasEscaped,
}
use crate::sonic_rs::SonicRsErrorCode::*;
impl<'de, R> Parser<R>
where
  R: Reader<'de>,
{
  fn new(read: R) -> Self {
    Self {
      read,
      error_index: usize::MAX,
      nospace_bits: 0,
      nospace_start: -128,
      cfg: DeserializeCfg::default(),
    }
  }

  #[inline(always)]
  fn error_index(&self) -> usize {
    std::cmp::min(self.error_index, self.read.index().saturating_sub(1))
  }

  #[cold]
  fn error(&self, mut reason: SonicRsErrorCode) -> SonicRsError {
    if let Err(e) = self.read.check_utf8_final() {
      return e;
    }

    let mut index = self.error_index();
    let len = self.read.as_u8_slice().len();
    if index > len {
      reason = EofWhileParsing;
      index = len;
    }
    SonicRsError::syntax(reason, self.read.origin_input(), index)
  }

  #[cold]
  fn fix_position(&self, err: SonicRsError) -> SonicRsError {
    if err.line() == 0 {
      self.error(err.error_code())
    } else {
      err
    }
  }

  #[inline(always)]
  fn parse_number(&mut self, first: u8) -> SonicRsResult<ParserNumber> {
    let reader = &mut self.read;
    let neg = first == b'-';
    let mut now = reader.index() - (!neg as usize);
    let data = reader.as_u8_slice();
    let ret = parse_number(data, &mut now, neg);
    reader.set_index(now);
    ret.map_err(|err| self.error(err.into()))
  }

  #[inline(always)]
  fn parse_str<'own>(
    &mut self,
    buf: &'own mut Vec<u8>,
  ) -> SonicRsResult<Reference<'de, 'own, str>> {
    match self.parse_string_raw(buf) {
      Ok(ParsedSlice::Copied(buf)) => {
        if self.check_invalid_utf8(self.cfg.utf8_lossy)? {
          let repr = String::from_utf8_lossy(buf.as_ref()).into_owned();
          *buf = repr.into_bytes();
        }
        let slice = unsafe { from_utf8_unchecked(buf.as_slice()) };
        Ok(Reference::Copied(slice))
      }
      Ok(ParsedSlice::Borrowed { slice, buf }) => {
        if self.check_invalid_utf8(self.cfg.utf8_lossy)? {
          let repr = String::from_utf8_lossy(slice).into_owned();
          *buf = repr.into_bytes();
          let slice = unsafe { from_utf8_unchecked(buf) };
          Ok(Reference::Copied(slice))
        } else {
          Ok(Reference::Borrowed(unsafe { from_utf8_unchecked(slice) }))
        }
      }
      Err(e) => Err(e),
    }
  }

  fn check_invalid_utf8(&mut self, allowed: bool) -> SonicRsResult<bool> {
    let invalid = self.read.next_invalid_utf8();
    if invalid >= self.read.index() {
      return Ok(false);
    }

    if !allowed {
      Err(SonicRsError::syntax(
        SonicRsErrorCode::InvalidUTF8,
        self.read.origin_input(),
        invalid,
      ))
    } else {
      self.read.check_invalid_utf8();
      Ok(true)
    }
  }

  fn parse_escaped_utf8(&mut self) -> SonicRsResult<u32> {
    let point1 = if let Some(asc) = self.read.next_n(4) {
      unsafe { hex_to_u32_nocheck(&*(asc.as_ptr() as *const _ as *const [u8; 4])) }
    } else {
      return perr!(self, EofWhileParsing);
    };

    if (0xD800..0xDC00).contains(&point1) {
      let point2 = if let Some(asc) = self.read.next_n(6) {
        if asc[0] != b'\\' || asc[1] != b'u' {
          if self.cfg.utf8_lossy {
            let idx = self.read.index();
            self.read.set_index(idx - 6);
            return Ok(0xFFFD);
          } else {
            return perr!(self, InvalidSurrogateUnicodeCodePoint);
          }
        }
        unsafe { hex_to_u32_nocheck(&*(asc.as_ptr().add(2) as *const _ as *const [u8; 4])) }
      } else if self.cfg.utf8_lossy {
        return Ok(0xFFFD);
      } else {
        return perr!(self, InvalidSurrogateUnicodeCodePoint);
      };

      /* calcute the real code point */
      let low_bit = point2.wrapping_sub(0xdc00);
      if (low_bit >> 10) != 0 {
        if self.cfg.utf8_lossy {
          let idx = self.read.index();
          self.read.set_index(idx - 6);
          return Ok(0xFFFD);
        } else {
          return perr!(self, InvalidSurrogateUnicodeCodePoint);
        }
      }

      Ok((((point1 - 0xd800) << 10) | low_bit).wrapping_add(0x10000))
    } else if (0xDC00..0xE000).contains(&point1) {
      if self.cfg.utf8_lossy {
        Ok(0xFFFD)
      } else {
        perr!(self, InvalidSurrogateUnicodeCodePoint)
      }
    } else {
      Ok(point1)
    }
  }

  unsafe fn parse_escaped_char(&mut self, buf: &mut Vec<u8>) -> SonicRsResult<()> {
    'escape: loop {
      match self.read.next() {
        Some(b'u') => {
          let code = self.parse_escaped_utf8()?;
          buf.reserve(4);
          let ptr = buf.as_mut_ptr().add(buf.len());
          let cnt = codepoint_to_utf8(code, ptr);
          if cnt == 0 {
            return perr!(self, InvalidUnicodeCodePoint);
          }
          buf.set_len(buf.len() + cnt);
        }
        Some(c) if ESCAPED_TAB[c as usize] != 0 => {
          buf.push(ESCAPED_TAB[c as usize]);
        }
        None => return perr!(self, EofWhileParsing),
        _ => return perr!(self, InvalidEscape),
      }

      if self.read.peek() == Some(b'\\') {
        self.read.eat(1);
        continue 'escape;
      }
      break 'escape;
    }
    Ok(())
  }

  unsafe fn parse_string_escaped<'own>(
    &mut self,
    buf: &'own mut Vec<u8>,
  ) -> SonicRsResult<ParsedSlice<'de, 'own>> {
    let mut block: StringBlock<u32>;

    self.parse_escaped_char(buf)?;

    while let Some(chunk) = self.read.peek_n(StringBlock::LANES) {
      buf.reserve(StringBlock::LANES);
      let v = unsafe { load(chunk.as_ptr()) };
      block = StringBlock::new(&v);

      if block.has_unescaped() {
        self.read.eat(block.unescaped_index());
        return perr!(self, ControlCharacterWhileParsingString);
      }

      let chunk = from_raw_parts_mut(buf.as_mut_ptr().add(buf.len()), StringBlock::LANES);
      v.write_to_slice_unaligned_unchecked(chunk);

      if block.has_quote_first() {
        let cnt = block.quote_index();
        buf.set_len(buf.len() + cnt);

        self.read.eat(cnt + 1);
        return Ok(ParsedSlice::Copied(buf));
      }

      if block.has_backslash() {
        let cnt = block.bs_index();
        self.read.eat(cnt + 1);
        buf.set_len(buf.len() + cnt);
        self.parse_escaped_char(buf)?;
      } else {
        buf.set_len(buf.len() + StringBlock::LANES);
        self.read.eat(StringBlock::LANES);
      }
    }

    while let Some(c) = self.read.peek() {
      match c {
        b'"' => {
          self.read.eat(1);
          return Ok(ParsedSlice::Copied(buf));
        }
        b'\\' => {
          self.read.eat(1);
          self.parse_escaped_char(buf)?;
        }
        b'\x00'..=b'\x1f' => return perr!(self, ControlCharacterWhileParsingString),
        _ => {
          buf.push(c);
          self.read.eat(1);
        }
      }
    }

    perr!(self, EofWhileParsing)
  }

  #[inline(always)]
  fn parse_string_raw<'own>(
    &mut self,
    buf: &'own mut Vec<u8>,
  ) -> SonicRsResult<ParsedSlice<'de, 'own>> {
    let start = self.read.index();

    let mut block: StringBlock<u32>;

    while let Some(chunk) = self.read.peek_n(StringBlock::LANES) {
      let v = unsafe { load(chunk.as_ptr()) };
      block = StringBlock::new(&v);

      if block.has_quote_first() {
        let cnt = block.quote_index();
        self.read.eat(cnt + 1);
        let slice = self.read.slice_unchecked(start, self.read.index() - 1);
        return Ok(ParsedSlice::Borrowed { slice, buf });
      }

      if block.has_unescaped() {
        self.read.eat(block.unescaped_index());
        return perr!(self, ControlCharacterWhileParsingString);
      }

      if block.has_backslash() {
        let cnt = block.bs_index();
        self.read.eat(cnt + 1);

        buf.clear();
        buf.extend_from_slice(&self.read.as_u8_slice()[start..self.read.index() - 1]);

        return unsafe { self.parse_string_escaped(buf) };
      }

      self.read.eat(StringBlock::LANES);
      continue;
    }

    while let Some(c) = self.read.peek() {
      match c {
        b'"' => {
          self.read.eat(1);
          let slice = self.read.slice_unchecked(start, self.read.index() - 1);
          return Ok(ParsedSlice::Borrowed { slice, buf });
        }
        b'\\' => {
          buf.clear();
          buf.extend_from_slice(self.read.slice_unchecked(start, self.read.index()));
          self.read.eat(1);
          return unsafe { self.parse_string_escaped(buf) };
        }
        b'\x00'..=b'\x1f' => return perr!(self, ControlCharacterWhileParsingString),
        _ => self.read.eat(1),
      }
    }
    perr!(self, EofWhileParsing)
  }

  #[inline(always)]
  fn get_next_token<const N: usize>(&mut self, tokens: [u8; N], advance: usize) -> Option<u8> {
    let r = &mut self.read;
    const LANS: usize = u8x32::LANES;
    while let Some(chunk) = r.peek_n(LANS) {
      let v = unsafe { u8x32::from_slice_unaligned_unchecked(chunk) };
      let mut vor = m8x32::splat(false);
      for t in tokens.iter().take(N) {
        vor |= v.eq(&u8x32::splat(*t));
      }
      let next = vor.bitmask();
      if next != 0 {
        let cnt = next.trailing_zeros() as usize;
        let ch = chunk[cnt];
        r.eat(cnt + advance);
        return Some(ch);
      }
      r.eat(LANS);
    }

    while let Some(ch) = r.peek() {
      for t in tokens.iter().take(N) {
        if ch == *t {
          r.eat(advance);
          return Some(ch);
        }
      }
      r.eat(1)
    }
    None
  }

  #[inline(always)]
  unsafe fn skip_string_unchecked(&mut self) -> SonicRsResult<ParseStatus> {
    const LANS: usize = u8x32::LANES;
    let r = &mut self.read;
    let mut quote_bits;
    let mut escaped;
    let mut prev_escaped = 0;
    let mut status = ParseStatus::None;

    while let Some(chunk) = r.peek_n(LANS) {
      let v = unsafe { u8x32::from_slice_unaligned_unchecked(chunk) };
      let bs_bits = (v.eq(&u8x32::splat(b'\\'))).bitmask();
      quote_bits = (v.eq(&u8x32::splat(b'"'))).bitmask();
      if ((quote_bits.wrapping_sub(1)) & bs_bits) != 0 || prev_escaped != 0 {
        escaped = get_escaped_branchless_u32(&mut prev_escaped, bs_bits);
        status = ParseStatus::HasEscaped;
        quote_bits &= !escaped;
      }
      if quote_bits != 0 {
        r.eat(quote_bits.trailing_zeros() as usize + 1);
        return Ok(status);
      }
      r.eat(LANS)
    }

    if prev_escaped != 0 {
      r.eat(1)
    }

    while let Some(ch) = r.peek() {
      if ch == b'\\' {
        if r.remain() < 2 {
          break;
        }
        status = ParseStatus::HasEscaped;
        r.eat(2);
        continue;
      }
      r.eat(1);
      if ch == b'"' {
        return Ok(status);
      }
    }
    perr!(self, EofWhileParsing)
  }

  fn skip_escaped_chars(&mut self) -> SonicRsResult<()> {
    match self.read.peek() {
      Some(b'u') => {
        if self.read.remain() < 6 {
          return perr!(self, EofWhileParsing);
        } else {
          self.read.eat(5);
        }
      }
      Some(c) => {
        if self.read.next().is_none() {
          return perr!(self, EofWhileParsing);
        }
        if ESCAPED_TAB[c as usize] == 0 {
          return perr!(self, InvalidEscape);
        }
      }
      None => return perr!(self, EofWhileParsing),
    }
    Ok(())
  }

  #[inline(always)]
  fn skip_string(&mut self) -> SonicRsResult<ParseStatus> {
    const LANS: usize = u8x32::LANES;

    let mut status = ParseStatus::None;
    while let Some(chunk) = self.read.peek_n(LANS) {
      let v = unsafe { u8x32::from_slice_unaligned_unchecked(chunk) };
      let v_bs = v.eq(&u8x32::splat(b'\\'));
      let v_quote = v.eq(&u8x32::splat(b'"'));
      let v_cc = v.le(&u8x32::splat(0x1f));
      let mask = (v_bs | v_quote | v_cc).bitmask();

      if mask != 0 {
        let cnt = mask.trailing_zeros() as usize;
        self.read.eat(cnt + 1);

        match chunk[cnt] {
          b'\\' => {
            self.skip_escaped_chars()?;
            status = ParseStatus::HasEscaped;
          }
          b'\"' => return Ok(status),
          0..=0x1f => return perr!(self, ControlCharacterWhileParsingString),
          _ => unreachable!(),
        }
      } else {
        self.read.eat(LANS)
      }
    }

    while let Some(ch) = self.read.next() {
      match ch {
        b'\\' => {
          self.skip_escaped_chars()?;
          status = ParseStatus::HasEscaped;
        }
        b'"' => return Ok(status),
        0..=0x1f => return perr!(self, ControlCharacterWhileParsingString),
        _ => {}
      }
    }
    perr!(self, EofWhileParsing)
  }

  #[inline(always)]
  fn parse_object_clo(&mut self) -> SonicRsResult<()> {
    if let Some(ch) = self.read.peek() {
      if ch == b':' {
        self.read.eat(1);
        return Ok(());
      }

      match self.skip_space() {
        Some(b':') => Ok(()),
        Some(_) => perr!(self, ExpectedColon),
        None => perr!(self, EofWhileParsing),
      }
    } else {
      perr!(self, EofWhileParsing)
    }
  }

  #[inline(always)]
  fn parse_array_end(&mut self) -> SonicRsResult<()> {
    match self.skip_space() {
      Some(b']') => Ok(()),
      Some(_) => perr!(self, ExpectedArrayCommaOrEnd),
      None => perr!(self, EofWhileParsing),
    }
  }

  #[inline(always)]
  fn skip_object(&mut self) -> SonicRsResult<()> {
    match self.skip_space() {
      Some(b'}') => return Ok(()),
      Some(b'"') => {}
      None => return perr!(self, EofWhileParsing),
      Some(_) => return perr!(self, ExpectObjectKeyOrEnd),
    }

    loop {
      self.skip_string()?;
      self.parse_object_clo()?;
      self.skip_one(true)?;

      match self.skip_space() {
        Some(b'}') => return Ok(()),
        Some(b',') => match self.skip_space() {
          Some(b'"') => continue,
          _ => return perr!(self, ExpectObjectKeyOrEnd),
        },
        None => return perr!(self, EofWhileParsing),
        Some(_) => return perr!(self, ExpectedObjectCommaOrEnd),
      }
    }
  }

  #[inline(always)]
  fn skip_array(&mut self) -> SonicRsResult<()> {
    match self.skip_space_peek() {
      Some(b']') => {
        self.read.eat(1);
        return Ok(());
      }
      None => return perr!(self, EofWhileParsing),
      _ => {}
    }

    loop {
      self.skip_one(true)?;
      match self.skip_space() {
        Some(b']') => return Ok(()),
        Some(b',') => continue,
        None => return perr!(self, EofWhileParsing),
        _ => return perr!(self, ExpectedArrayCommaOrEnd),
      }
    }
  }

  #[inline(always)]
  fn skip_container(&mut self, left: u8, right: u8) -> SonicRsResult<()> {
    let mut prev_instring = 0;
    let mut prev_escaped = 0;
    let mut rbrace_num = 0;
    let mut lbrace_num = 0;
    let reader = &mut self.read;

    while let Some(chunk) = reader.peek_n(64) {
      let input = unsafe { &*(chunk.as_ptr() as *const [_; 64]) };
      if let Some(count) = skip_container_loop(
        input,
        &mut prev_instring,
        &mut prev_escaped,
        &mut lbrace_num,
        &mut rbrace_num,
        left,
        right,
      ) {
        reader.eat(count.get() as usize);
        return Ok(());
      }
      reader.eat(64);
    }

    let mut remain = [0u8; 64];
    {
      let n = reader.remain();
      debug_assert!(n <= 64);
      remain[..n].copy_from_slice(reader.peek_n(n).unwrap());
    }
    if let Some(count) = skip_container_loop(
      &remain,
      &mut prev_instring,
      &mut prev_escaped,
      &mut lbrace_num,
      &mut rbrace_num,
      left,
      right,
    ) {
      reader.eat(count.get() as usize);
      return Ok(());
    }

    perr!(self, EofWhileParsing)
  }

  #[inline(always)]
  fn skip_space(&mut self) -> Option<u8> {
    let reader = &mut self.read;
    if let Some(ch) = reader.next() {
      if !is_whitespace(ch) {
        return Some(ch);
      }
    }
    if let Some(ch) = reader.next() {
      if !is_whitespace(ch) {
        return Some(ch);
      }
    }

    let nospace_offset = (reader.index() as isize) - self.nospace_start;
    if nospace_offset < 64 {
      let bitmap = {
        let mask = !((1 << nospace_offset) - 1);
        self.nospace_bits & mask
      };
      if bitmap != 0 {
        let cnt = bitmap.trailing_zeros() as usize;
        let ch = reader.at(self.nospace_start as usize + cnt);
        reader.set_index(self.nospace_start as usize + cnt + 1);

        return Some(ch);
      } else {
        reader.set_index(self.nospace_start as usize + 64);
      }
    }

    while let Some(chunk) = reader.peek_n(64) {
      let chunk = unsafe { &*(chunk.as_ptr() as *const [_; 64]) };
      let bitmap = unsafe { get_nonspace_bits(chunk) };
      if bitmap != 0 {
        self.nospace_bits = bitmap;
        self.nospace_start = reader.index() as isize;
        let cnt = bitmap.trailing_zeros() as usize;
        let ch = chunk[cnt];
        reader.eat(cnt + 1);

        return Some(ch);
      }
      reader.eat(64)
    }

    while let Some(ch) = reader.next() {
      if !is_whitespace(ch) {
        return Some(ch);
      }
    }
    None
  }

  #[inline(always)]
  fn skip_space_peek(&mut self) -> Option<u8> {
    let ret = self.skip_space()?;
    self.read.backward(1);
    Some(ret)
  }

  #[inline(always)]
  fn parse_literal(&mut self, literal: &str) -> SonicRsResult<()> {
    let reader = &mut self.read;
    if let Some(chunk) = reader.next_n(literal.len()) {
      if chunk == literal.as_bytes() {
        Ok(())
      } else {
        perr!(self, InvalidLiteral)
      }
    } else {
      perr!(self, EofWhileParsing)
    }
  }

  #[inline(always)]
  fn skip_number_unsafe(&mut self) -> SonicRsResult<()> {
    let _ = self.get_next_token([b']', b'}', b','], 0);
    Ok(())
  }

  #[inline(always)]
  fn skip_exponent(&mut self) -> SonicRsResult<()> {
    if let Some(ch) = self.read.peek() {
      if ch == b'-' || ch == b'+' {
        self.read.eat(1);
      }
    }
    self.skip_single_digit()?;
    while matches!(self.read.peek(), Some(b'0'..=b'9')) {
      self.read.eat(1);
    }
    Ok(())
  }

  #[inline(always)]
  fn skip_single_digit(&mut self) -> SonicRsResult<u8> {
    if let Some(ch) = self.read.next() {
      if !ch.is_ascii_digit() {
        perr!(self, InvalidNumber)
      } else {
        Ok(ch)
      }
    } else {
      perr!(self, EofWhileParsing)
    }
  }

  #[inline(always)]
  fn skip_number(&mut self, first: u8) -> SonicRsResult<&'de str> {
    let start = self.read.index() - 1;
    self.do_skip_number(first)?;
    let end = self.read.index();
    Ok(as_str(self.read.slice_unchecked(start, end)))
  }

  #[inline(always)]
  fn do_skip_number(&mut self, mut first: u8) -> SonicRsResult<()> {
    if first == b'-' {
      first = self.skip_single_digit()?;
    }

    let second = self.read.peek();
    if first == b'0' && matches!(second, Some(b'0'..=b'9')) {
      return perr!(self, InvalidNumber);
    }

    let mut is_float: bool = false;
    match second {
      Some(b'0'..=b'9') => self.read.eat(1),
      Some(b'.') => {
        is_float = true;
        self.read.eat(1);
        self.skip_single_digit()?;
      }
      Some(b'e' | b'E') => {
        self.read.eat(1);
        return self.skip_exponent();
      }
      _ => return Ok(()),
    }

    const LANES: usize = i8x32::LANES;
    while let Some(chunk) = self.read.peek_n(LANES) {
      let v = unsafe { i8x32::from_slice_unaligned_unchecked(chunk) };
      let zero = i8x32::splat(b'0' as i8);
      let nine = i8x32::splat(b'9' as i8);
      let mut nondigits = (zero.gt(&v) | v.gt(&nine)).bitmask();
      if nondigits != 0 {
        let mut cnt = nondigits.trailing_zeros() as usize;
        let ch = chunk[cnt];
        if ch == b'.' && !is_float {
          self.read.eat(cnt + 1);
          self.skip_single_digit()?;

          cnt += 2;
          if cnt >= LANES {
            is_float = true;
            continue;
          }

          nondigits = nondigits.wrapping_shr(cnt as u32);
          if nondigits != 0 {
            let offset = nondigits.trailing_zeros() as usize;
            let ch = chunk[cnt + offset];
            if ch == b'e' || ch == b'E' {
              self.read.eat(offset + 1);
              return self.skip_exponent();
            } else {
              self.read.eat(offset);
              return Ok(());
            }
          } else {
            self.read.eat(32 - cnt);
            is_float = true;
            continue;
          }
        } else if ch == b'e' || ch == b'E' {
          self.read.eat(cnt + 1);
          return self.skip_exponent();
        } else {
          self.read.eat(cnt);
          return Ok(());
        }
      }
      self.read.eat(32);
    }

    while matches!(self.read.peek(), Some(b'0'..=b'9')) {
      self.read.eat(1);
    }

    match self.read.peek() {
      Some(b'.') if !is_float => {
        self.read.eat(1);
        self.skip_single_digit()?;
        while matches!(self.read.peek(), Some(b'0'..=b'9')) {
          self.read.eat(1);
        }
        match self.read.peek() {
          Some(b'e' | b'E') => {
            self.read.eat(1);
            return self.skip_exponent();
          }
          _ => return Ok(()),
        }
      }
      Some(b'e' | b'E') => {
        self.read.eat(1);
        return self.skip_exponent();
      }
      _ => {}
    }
    Ok(())
  }

  fn skip_one(&mut self, checked: bool) -> SonicRsResult<(&'de [u8], ParseStatus)> {
    let ch = match self.skip_space() {
      Some(ch) => ch,
      None => return perr!(self, EofWhileParsing),
    };
    let start = self.read.index() - 1;
    let mut status = ParseStatus::None;
    match ch {
      c @ b'-' | c @ b'0'..=b'9' => {
        if checked {
          self.skip_number(c)?;
        } else {
          self.skip_number_unsafe()?;
        }
        Ok(())
      }
      b'"' => {
        status = if checked {
          self.skip_string()?
        } else {
          unsafe { self.skip_string_unchecked() }?
        };
        Ok(())
      }
      b'{' => {
        if checked {
          self.skip_object()
        } else {
          self.skip_container(b'{', b'}')
        }
      }
      b'[' => {
        if checked {
          self.skip_array()
        } else {
          self.skip_container(b'[', b']')
        }
      }
      b't' => self.parse_literal("rue"),
      b'f' => self.parse_literal("alse"),
      b'n' => self.parse_literal("ull"),
      _ => perr!(self, InvalidJsonValue),
    }?;
    let slice = self.read.slice_unchecked(start, self.read.index());
    Ok((slice, status))
  }

  #[inline(always)]
  fn parse_trailing(&mut self) -> SonicRsResult<()> {
    let exceed = self.read.index() > self.read.as_u8_slice().len();
    if exceed {
      return perr!(self, EofWhileParsing);
    }

    let remain = self.read.remain() > 0;
    if !remain {
      return Ok(());
    }

    let last = self.skip_space();
    let exceed = self.read.index() > self.read.as_u8_slice().len();
    if last.is_some() && !exceed {
      perr!(self, TrailingCharacters)
    } else {
      Ok(())
    }
  }

  #[cold]
  fn peek_invalid_type(&mut self, peek: u8, exp: &dyn Expected) -> SonicRsError {
    let err = match peek {
      b'n' => {
        if let Err(err) = self.parse_literal("ull") {
          return err;
        }
        de::Error::invalid_type(Unexpected::Unit, exp)
      }
      b't' => {
        if let Err(err) = self.parse_literal("rue") {
          return err;
        }
        de::Error::invalid_type(Unexpected::Bool(true), exp)
      }
      b'f' => {
        if let Err(err) = self.parse_literal("alse") {
          return err;
        }
        de::Error::invalid_type(Unexpected::Bool(false), exp)
      }
      c @ b'-' | c @ b'0'..=b'9' => match self.parse_number(c) {
        Ok(n) => invalid_type_number(&n, exp),
        Err(err) => return err,
      },
      b'"' => {
        let mut scratch = Vec::new();
        match self.parse_str(&mut scratch) {
          Ok(s) if std::str::from_utf8(s.as_bytes()).is_ok() => {
            de::Error::invalid_type(Unexpected::Str(&s), exp)
          }
          Ok(s) => de::Error::invalid_type(Unexpected::Bytes(s.as_bytes()), exp),
          Err(err) => return err,
        }
      }
      b'[' => {
        self.read.backward(1);

        match self.skip_one(true) {
          Ok(_) => de::Error::invalid_type(Unexpected::Seq, exp),
          Err(err) => return err,
        }
      }
      b'{' => {
        self.read.backward(1);
        match self.skip_one(true) {
          Ok(_) => de::Error::invalid_type(Unexpected::Map, exp),
          Err(err) => return err,
        }
      }
      _ => self.error(SonicRsErrorCode::InvalidJsonValue),
    };
    self.fix_position(err)
  }
}

const MAX_ALLOWED_DEPTH: u8 = u8::MAX;

struct Deserializer<R> {
  parser: Parser<R>,
  scratch: Vec<u8>,
  remaining_depth: u8,
}

impl<'de, R: Reader<'de>> Deserializer<R> {
  fn new(read: R) -> Self {
    Self {
      parser: Parser::new(read),
      scratch: Vec::new(),
      remaining_depth: MAX_ALLOWED_DEPTH,
    }
  }
}

impl<'de, R: Reader<'de>> Deserializer<R> {
  #[inline]
  fn with_depth_limit<F, T>(&mut self, f: F) -> SonicRsResult<T>
  where
    F: FnOnce(&mut Self) -> SonicRsResult<T>,
  {
    self.remaining_depth -= 1;
    if self.remaining_depth == 0 {
      return Err(self.parser.error(RecursionLimitExceeded));
    }
    let result = f(self);
    self.remaining_depth += 1;
    result
  }
}

fn visit_number<'de, V>(num: &ParserNumber, visitor: V) -> SonicRsResult<V::Value>
where
  V: de::Visitor<'de>,
{
  match *num {
    ParserNumber::Float(x) => visitor.visit_f64(x),
    ParserNumber::Unsigned(x) => visitor.visit_u64(x),
    ParserNumber::Signed(x) => visitor.visit_i64(x),
  }
}

fn invalid_type_number(num: &ParserNumber, exp: &dyn Expected) -> SonicRsError {
  match *num {
    ParserNumber::Float(x) => de::Error::invalid_type(Unexpected::Float(x), exp),
    ParserNumber::Unsigned(x) => de::Error::invalid_type(Unexpected::Unsigned(x), exp),
    ParserNumber::Signed(x) => de::Error::invalid_type(Unexpected::Signed(x), exp),
  }
}

macro_rules! impl_deserialize_number {
  ($method:ident) => {
    fn $method<V>(self, visitor: V) -> SonicRsResult<V::Value>
    where
      V: de::Visitor<'de>,
    {
      self.deserialize_number(visitor)
    }
  };
}

impl<'de, R: Reader<'de>> Deserializer<R> {
  #[inline]
  fn fix_position<T>(&self, result: SonicRsResult<T>) -> SonicRsResult<T> {
    result.map_err(|err| self.parser.fix_position(err))
  }

  fn deserialize_number<V>(&mut self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    let Some(peek) = self.parser.skip_space() else {
      return Err(self.parser.error(EofWhileParsing));
    };

    let value = match peek {
      c @ b'-' | c @ b'0'..=b'9' => visit_number(&tri!(self.parser.parse_number(c)), visitor),
      _ => Err(self.peek_invalid_type(peek, &visitor)),
    };

    self.fix_position(value)
  }

  #[cold]
  fn peek_invalid_type(&mut self, peek: u8, exp: &dyn Expected) -> SonicRsError {
    self.parser.peek_invalid_type(peek, exp)
  }

  fn end_seq(&mut self) -> SonicRsResult<()> {
    self.parser.parse_array_end()
  }

  fn end_map(&mut self) -> SonicRsResult<()> {
    match self.parser.skip_space() {
      Some(b'}') => Ok(()),
      Some(b',') => Err(self.parser.error(SonicRsErrorCode::TrailingComma)),
      Some(_) => Err(
        self
          .parser
          .error(SonicRsErrorCode::ExpectedObjectCommaOrEnd),
      ),
      None => Err(self.parser.error(SonicRsErrorCode::EofWhileParsing)),
    }
  }

  fn scan_integer128(&mut self, buf: &mut String) -> SonicRsResult<()> {
    match self.parser.read.peek() {
      Some(b'0') => {
        buf.push('0');
        self.parser.read.eat(1);
        if let Some(ch) = self.parser.read.peek() {
          if ch.is_ascii_digit() {
            return Err(self.parser.error(SonicRsErrorCode::InvalidNumber));
          }
        }
        Ok(())
      }
      Some(c) if c.is_ascii_digit() => {
        buf.push(c as char);
        self.parser.read.eat(1);
        while let c @ b'0'..=b'9' = self.parser.read.peek().unwrap_or_default() {
          self.parser.read.eat(1);
          buf.push(c as char);
        }
        Ok(())
      }
      _ => Err(self.parser.error(SonicRsErrorCode::InvalidNumber)),
    }
  }
}

impl<'de, 'a, R: Reader<'de>> de::Deserializer<'de> for &'a mut Deserializer<R> {
  type Error = SonicRsError;
  #[inline]
  fn deserialize_any<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    let Some(peek) = self.parser.skip_space() else {
      return Err(self.parser.error(EofWhileParsing));
    };

    let value = match peek {
      b'n' => {
        tri!(self.parser.parse_literal("ull"));
        visitor.visit_unit()
      }
      b't' => {
        tri!(self.parser.parse_literal("rue"));
        visitor.visit_bool(true)
      }
      b'f' => {
        tri!(self.parser.parse_literal("alse"));
        visitor.visit_bool(false)
      }
      c @ b'-' | c @ b'0'..=b'9' => visit_number(&tri!(self.parser.parse_number(c)), visitor),
      b'"' => match tri!(self.parser.parse_str(&mut self.scratch)) {
        Reference::Borrowed(s) => visitor.visit_borrowed_str(s),
        Reference::Copied(s) => visitor.visit_str(s),
      },
      b'[' => {
        let ret = self.with_depth_limit(|de| visitor.visit_seq(SeqAccess::new(de)));
        match (ret, self.end_seq()) {
          (Ok(ret), Ok(())) => Ok(ret),
          (Err(err), _) | (_, Err(err)) => Err(err),
        }
      }
      b'{' => {
        let ret = self.with_depth_limit(|de| visitor.visit_map(MapAccess::new(de)));
        match (ret, self.end_map()) {
          (Ok(ret), Ok(())) => Ok(ret),
          (Err(err), _) | (_, Err(err)) => Err(err),
        }
      }
      _ => Err(self.parser.error(SonicRsErrorCode::InvalidJsonValue)),
    };

    match value {
      Ok(value) => Ok(value),
      Err(err) => Err(self.parser.fix_position(err)),
    }
  }

  fn deserialize_bool<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    let Some(peek) = self.parser.skip_space() else {
      return Err(self.parser.error(SonicRsErrorCode::EofWhileParsing));
    };

    let value = match peek {
      b't' => {
        tri!(self.parser.parse_literal("rue"));
        visitor.visit_bool(true)
      }
      b'f' => {
        tri!(self.parser.parse_literal("alse"));
        visitor.visit_bool(false)
      }
      _ => Err(self.peek_invalid_type(peek, &visitor)),
    };

    self.fix_position(value)
  }

  impl_deserialize_number!(deserialize_i8);
  impl_deserialize_number!(deserialize_i16);
  impl_deserialize_number!(deserialize_i32);
  impl_deserialize_number!(deserialize_i64);
  impl_deserialize_number!(deserialize_u8);
  impl_deserialize_number!(deserialize_u16);
  impl_deserialize_number!(deserialize_u32);
  impl_deserialize_number!(deserialize_u64);
  impl_deserialize_number!(deserialize_f32);
  impl_deserialize_number!(deserialize_f64);

  fn deserialize_i128<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    let mut buf = String::new();
    match self.parser.skip_space_peek() {
      Some(b'-') => {
        buf.push('-');
        self.parser.read.eat(1);
      }
      Some(_) => {}
      None => {
        return Err(self.parser.error(SonicRsErrorCode::EofWhileParsing));
      }
    };

    tri!(self.scan_integer128(&mut buf));

    let value = match buf.parse() {
      Ok(int) => visitor.visit_i128(int),
      Err(_) => {
        return Err(self.parser.error(SonicRsErrorCode::NumberOutOfRange));
      }
    };

    self.fix_position(value)
  }

  fn deserialize_u128<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    match self.parser.skip_space_peek() {
      Some(b'-') => {
        return Err(self.parser.error(SonicRsErrorCode::NumberOutOfRange));
      }
      Some(_) => {}
      None => {
        return Err(self.parser.error(SonicRsErrorCode::EofWhileParsing));
      }
    }

    let mut buf = String::new();
    tri!(self.scan_integer128(&mut buf));

    let value = match buf.parse() {
      Ok(int) => visitor.visit_u128(int),
      Err(_) => {
        return Err(self.parser.error(SonicRsErrorCode::NumberOutOfRange));
      }
    };

    self.fix_position(value)
  }

  fn deserialize_char<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    self.deserialize_str(visitor)
  }

  fn deserialize_str<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    let Some(peek) = self.parser.skip_space() else {
      return Err(self.parser.error(SonicRsErrorCode::EofWhileParsing));
    };

    let value = match peek {
      b'"' => match tri!(self.parser.parse_str(&mut self.scratch)) {
        Reference::Borrowed(s) => visitor.visit_borrowed_str(s),
        Reference::Copied(s) => visitor.visit_str(s),
      },
      _ => Err(self.peek_invalid_type(peek, &visitor)),
    };

    self.fix_position(value)
  }

  fn deserialize_string<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    self.deserialize_str(visitor)
  }

  fn deserialize_bytes<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    let Some(peek) = self.parser.skip_space() else {
      return Err(self.parser.error(SonicRsErrorCode::EofWhileParsing));
    };

    let value = match peek {
      b'"' => match tri!(self.parser.parse_string_raw(&mut self.scratch)) {
        ParsedSlice::Borrowed { slice: b, buf: _ } => visitor.visit_borrowed_bytes(b),
        ParsedSlice::Copied(b) => visitor.visit_bytes(b),
      },
      b'[' => {
        self.parser.read.backward(1);
        self.deserialize_seq(visitor)
      }
      _ => Err(self.peek_invalid_type(peek, &visitor)),
    };

    let _ = self.parser.check_invalid_utf8(true)?;
    self.fix_position(value)
  }

  #[inline]
  fn deserialize_byte_buf<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    self.deserialize_bytes(visitor)
  }

  #[inline]
  fn deserialize_option<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    match self.parser.skip_space_peek() {
      Some(b'n') => {
        self.parser.read.eat(1);
        tri!(self.parser.parse_literal("ull"));
        visitor.visit_none()
      }
      _ => visitor.visit_some(self),
    }
  }

  fn deserialize_unit<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    let Some(peek) = self.parser.skip_space() else {
      return Err(self.parser.error(SonicRsErrorCode::EofWhileParsing));
    };

    let value = match peek {
      b'n' => {
        tri!(self.parser.parse_literal("ull"));
        visitor.visit_unit()
      }
      _ => Err(self.peek_invalid_type(peek, &visitor)),
    };

    self.fix_position(value)
  }

  fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    self.deserialize_unit(visitor)
  }

  #[inline]
  fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    visitor.visit_newtype_struct(self)
  }

  fn deserialize_seq<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    let Some(peek) = self.parser.skip_space() else {
      return Err(self.parser.error(SonicRsErrorCode::EofWhileParsing));
    };

    let value = match peek {
      b'[' => {
        let ret = self.with_depth_limit(|de| visitor.visit_seq(SeqAccess::new(de)));
        match (ret, self.end_seq()) {
          (Ok(ret), Ok(())) => Ok(ret),
          (Err(err), _) | (_, Err(err)) => Err(err),
        }
      }
      _ => return Err(self.peek_invalid_type(peek, &visitor)),
    };
    self.fix_position(value)
  }

  fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    self.deserialize_seq(visitor)
  }

  fn deserialize_tuple_struct<V>(
    self,
    _name: &'static str,
    _len: usize,
    visitor: V,
  ) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    self.deserialize_seq(visitor)
  }

  fn deserialize_map<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    let Some(peek) = self.parser.skip_space() else {
      return Err(self.parser.error(SonicRsErrorCode::EofWhileParsing));
    };

    let value = match peek {
      b'{' => {
        let ret = self.with_depth_limit(|de| visitor.visit_map(MapAccess::new(de)));
        match (ret, self.end_map()) {
          (Ok(ret), Ok(())) => Ok(ret),
          (Err(err), _) | (_, Err(err)) => Err(err),
        }
      }
      _ => return Err(self.peek_invalid_type(peek, &visitor)),
    };
    self.fix_position(value)
  }

  fn deserialize_struct<V>(
    self,
    _name: &'static str,
    _fields: &'static [&'static str],
    visitor: V,
  ) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    let Some(peek) = self.parser.skip_space() else {
      return Err(self.parser.error(SonicRsErrorCode::EofWhileParsing));
    };

    let value = match peek {
      b'[' => {
        let ret = self.with_depth_limit(|de| visitor.visit_seq(SeqAccess::new(de)));
        match (ret, self.end_seq()) {
          (Ok(ret), Ok(())) => Ok(ret),
          (Err(err), _) | (_, Err(err)) => Err(err),
        }
      }
      b'{' => {
        let ret = self.with_depth_limit(|de| visitor.visit_map(MapAccess::new(de)));
        match (ret, self.end_map()) {
          (Ok(ret), Ok(())) => Ok(ret),
          (Err(err), _) | (_, Err(err)) => Err(err),
        }
      }
      _ => return Err(self.peek_invalid_type(peek, &visitor)),
    };

    self.fix_position(value)
  }

  #[inline]
  fn deserialize_enum<V>(
    self,
    _name: &str,
    _variants: &'static [&'static str],
    visitor: V,
  ) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    match self.parser.skip_space_peek() {
      Some(b'{') => {
        self.parser.read.eat(1);
        let value = self.with_depth_limit(|de| visitor.visit_enum(VariantAccess::new(de)))?;

        match self.parser.skip_space() {
          Some(b'}') => Ok(value),
          Some(_) => Err(self.parser.error(SonicRsErrorCode::InvalidJsonValue)),
          None => Err(self.parser.error(SonicRsErrorCode::EofWhileParsing)),
        }
      }
      Some(b'"') => visitor.visit_enum(UnitVariantAccess::new(self)),
      Some(_) => Err(self.parser.error(SonicRsErrorCode::InvalidJsonValue)),
      None => Err(self.parser.error(SonicRsErrorCode::EofWhileParsing)),
    }
  }

  fn deserialize_identifier<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    self.deserialize_str(visitor)
  }

  fn deserialize_ignored_any<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    tri!(self.parser.skip_one(true));
    visitor.visit_unit()
  }
}

struct SeqAccess<'a, R: 'a> {
  de: &'a mut Deserializer<R>,
  first: bool, // first is marked as
}

impl<'a, R: 'a> SeqAccess<'a, R> {
  fn new(de: &'a mut Deserializer<R>) -> Self {
    SeqAccess { de, first: true }
  }
}

impl<'de, 'a, R: Reader<'de> + 'a> de::SeqAccess<'de> for SeqAccess<'a, R> {
  type Error = SonicRsError;

  fn next_element_seed<T>(&mut self, seed: T) -> SonicRsResult<Option<T::Value>>
  where
    T: de::DeserializeSeed<'de>,
  {
    match self.de.parser.skip_space_peek() {
      Some(b']') => Ok(None), // we will check the ending brace after `visit_seq`
      Some(b',') if !self.first => {
        self.de.parser.read.eat(1);
        Ok(Some(tri!(seed.deserialize(&mut *self.de))))
      }
      Some(_) => {
        if self.first {
          self.first = false;
          Ok(Some(tri!(seed.deserialize(&mut *self.de))))
        } else {
          self.de.parser.read.eat(1); // makes the error position is correct
          Err(
            self
              .de
              .parser
              .error(SonicRsErrorCode::ExpectedArrayCommaOrEnd),
          )
        }
      }
      None => Err(self.de.parser.error(SonicRsErrorCode::EofWhileParsing)),
    }
  }
}

struct MapAccess<'a, R: 'a> {
  de: &'a mut Deserializer<R>,
  first: bool,
}

impl<'a, R: 'a> MapAccess<'a, R> {
  fn new(de: &'a mut Deserializer<R>) -> Self {
    MapAccess { de, first: true }
  }
}

impl<'de, 'a, R: Reader<'de> + 'a> de::MapAccess<'de> for MapAccess<'a, R> {
  type Error = SonicRsError;

  #[inline(always)]
  fn next_key_seed<K>(&mut self, seed: K) -> SonicRsResult<Option<K::Value>>
  where
    K: de::DeserializeSeed<'de>,
  {
    let peek = match self.de.parser.skip_space_peek() {
      Some(b'}') => {
        return Ok(None);
      }
      Some(b',') if !self.first => {
        self.de.parser.read.eat(1);
        self.de.parser.skip_space()
      }
      Some(b) => {
        self.de.parser.read.eat(1);
        if self.first {
          self.first = false;
          Some(b)
        } else {
          return Err(
            self
              .de
              .parser
              .error(SonicRsErrorCode::ExpectedObjectCommaOrEnd),
          );
        }
      }
      None => {
        return Err(self.de.parser.error(SonicRsErrorCode::EofWhileParsing));
      }
    };

    match peek {
      Some(b'"') => seed.deserialize(MapKey { de: &mut *self.de }).map(Some),
      Some(b'}') => Err(self.de.parser.error(SonicRsErrorCode::TrailingComma)),
      Some(_) => Err(self.de.parser.error(SonicRsErrorCode::ExpectObjectKeyOrEnd)),
      None => Err(self.de.parser.error(SonicRsErrorCode::EofWhileParsing)),
    }
  }

  #[inline(always)]
  fn next_value<V>(&mut self) -> SonicRsResult<V>
  where
    V: de::Deserialize<'de>,
  {
    use std::marker::PhantomData;
    self.next_value_seed(PhantomData)
  }

  #[inline(always)]
  fn next_entry<K, V>(&mut self) -> SonicRsResult<Option<(K, V)>>
  where
    K: de::Deserialize<'de>,
    V: de::Deserialize<'de>,
  {
    use std::marker::PhantomData;
    self.next_entry_seed(PhantomData, PhantomData)
  }

  #[inline(always)]
  fn next_value_seed<V>(&mut self, seed: V) -> SonicRsResult<V::Value>
  where
    V: de::DeserializeSeed<'de>,
  {
    tri!(self.de.parser.parse_object_clo());
    seed.deserialize(&mut *self.de)
  }
}

struct VariantAccess<'a, R: 'a> {
  de: &'a mut Deserializer<R>,
}

impl<'a, R: 'a> VariantAccess<'a, R> {
  fn new(de: &'a mut Deserializer<R>) -> Self {
    VariantAccess { de }
  }
}

impl<'de, 'a, R: Reader<'de> + 'a> de::EnumAccess<'de> for VariantAccess<'a, R> {
  type Error = SonicRsError;
  type Variant = Self;

  fn variant_seed<V>(self, seed: V) -> SonicRsResult<(V::Value, Self)>
  where
    V: de::DeserializeSeed<'de>,
  {
    let val = tri!(seed.deserialize(&mut *self.de));
    tri!(self.de.parser.parse_object_clo());
    Ok((val, self))
  }
}

impl<'de, 'a, R: Reader<'de> + 'a> de::VariantAccess<'de> for VariantAccess<'a, R> {
  type Error = SonicRsError;

  fn unit_variant(self) -> SonicRsResult<()> {
    de::Deserialize::deserialize(self.de)
  }

  fn newtype_variant_seed<T>(self, seed: T) -> SonicRsResult<T::Value>
  where
    T: de::DeserializeSeed<'de>,
  {
    seed.deserialize(self.de)
  }

  fn tuple_variant<V>(self, _len: usize, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    de::Deserializer::deserialize_seq(self.de, visitor)
  }

  fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
  }
}

struct UnitVariantAccess<'a, R: 'a> {
  de: &'a mut Deserializer<R>,
}

impl<'a, R: 'a> UnitVariantAccess<'a, R> {
  fn new(de: &'a mut Deserializer<R>) -> Self {
    UnitVariantAccess { de }
  }
}

impl<'de, 'a, R: Reader<'de> + 'a> de::EnumAccess<'de> for UnitVariantAccess<'a, R> {
  type Error = SonicRsError;
  type Variant = Self;

  fn variant_seed<V>(self, seed: V) -> SonicRsResult<(V::Value, Self)>
  where
    V: de::DeserializeSeed<'de>,
  {
    let variant = tri!(seed.deserialize(&mut *self.de));
    Ok((variant, self))
  }
}

impl<'de, 'a, R: Reader<'de> + 'a> de::VariantAccess<'de> for UnitVariantAccess<'a, R> {
  type Error = SonicRsError;

  fn unit_variant(self) -> SonicRsResult<()> {
    Ok(())
  }

  fn newtype_variant_seed<T>(self, _seed: T) -> SonicRsResult<T::Value>
  where
    T: de::DeserializeSeed<'de>,
  {
    Err(de::Error::invalid_type(
      Unexpected::UnitVariant,
      &"newtype variant",
    ))
  }

  fn tuple_variant<V>(self, _len: usize, _visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    Err(de::Error::invalid_type(
      Unexpected::UnitVariant,
      &"tuple variant",
    ))
  }

  fn struct_variant<V>(
    self,
    _fields: &'static [&'static str],
    _visitor: V,
  ) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    Err(de::Error::invalid_type(
      Unexpected::UnitVariant,
      &"struct variant",
    ))
  }
}

struct MapKey<'a, R: 'a> {
  de: &'a mut Deserializer<R>,
}

macro_rules! deserialize_numeric_key {
  ($method:ident) => {
    fn $method<V>(self, visitor: V) -> SonicRsResult<V::Value>
    where
      V: de::Visitor<'de>,
    {
      let value = tri!(self.de.deserialize_number(visitor));
      if self.de.parser.read.next() != Some(b'"') {
        return Err(self.de.parser.error(SonicRsErrorCode::ExpectedQuote));
      }

      Ok(value)
    }
  };

  ($method:ident, $delegate:ident) => {
    fn $method<V>(self, visitor: V) -> SonicRsResult<V::Value>
    where
      V: de::Visitor<'de>,
    {
      match self.de.parser.read.peek() {
        Some(b'0'..=b'9' | b'-') => {}
        _ => return Err(self.de.parser.error(SonicRsErrorCode::ExpectedNumericKey)),
      }

      let value = tri!(self.de.$delegate(visitor));

      if self.de.parser.read.next() != Some(b'"') {
        return Err(self.de.parser.error(SonicRsErrorCode::ExpectedQuote));
      }

      Ok(value)
    }
  };
}

impl<'de, 'a, R> de::Deserializer<'de> for MapKey<'a, R>
where
  R: Reader<'de>,
{
  type Error = SonicRsError;

  #[inline]
  fn deserialize_any<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    self.de.scratch.clear();
    match tri!(self.de.parser.parse_str(&mut self.de.scratch)) {
      Reference::Borrowed(s) => visitor.visit_borrowed_str(s),
      Reference::Copied(s) => visitor.visit_str(s),
    }
  }

  deserialize_numeric_key!(deserialize_i8);
  deserialize_numeric_key!(deserialize_i16);
  deserialize_numeric_key!(deserialize_i32);
  deserialize_numeric_key!(deserialize_i64);
  deserialize_numeric_key!(deserialize_i128, deserialize_i128);
  deserialize_numeric_key!(deserialize_u8);
  deserialize_numeric_key!(deserialize_u16);
  deserialize_numeric_key!(deserialize_u32);
  deserialize_numeric_key!(deserialize_u64);
  deserialize_numeric_key!(deserialize_u128, deserialize_u128);
  deserialize_numeric_key!(deserialize_f32);
  deserialize_numeric_key!(deserialize_f64);

  fn deserialize_bool<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    let mut value = match self.de.parser.read.next() {
      Some(b't') => {
        tri!(self.de.parser.parse_literal("rue"));
        visitor.visit_bool(true)
      }
      Some(b'f') => {
        tri!(self.de.parser.parse_literal("alse"));
        visitor.visit_bool(false)
      }
      None => Err(self.de.parser.error(SonicRsErrorCode::EofWhileParsing)),
      Some(peek) => Err(self.de.peek_invalid_type(peek, &visitor)),
    };

    if self.de.parser.read.next() != Some(b'"') {
      value = Err(self.de.parser.error(SonicRsErrorCode::ExpectedQuote));
    }

    match value {
      Ok(value) => Ok(value),
      Err(err) => Err(self.de.parser.fix_position(err)),
    }
  }

  #[inline]
  fn deserialize_option<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    visitor.visit_some(self)
  }

  #[inline]
  fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    visitor.visit_newtype_struct(self)
  }

  #[inline]
  fn deserialize_enum<V>(
    self,
    name: &'static str,
    variants: &'static [&'static str],
    visitor: V,
  ) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    self.de.parser.read.backward(1);
    self.de.deserialize_enum(name, variants, visitor)
  }

  #[inline]
  fn deserialize_bytes<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    self.de.parser.read.backward(1);
    self.de.deserialize_bytes(visitor)
  }

  #[inline]
  fn deserialize_byte_buf<V>(self, visitor: V) -> SonicRsResult<V::Value>
  where
    V: de::Visitor<'de>,
  {
    self.de.parser.read.backward(1);
    self.de.deserialize_bytes(visitor)
  }

  forward_to_deserialize_any! {
      char str string unit unit_struct seq tuple tuple_struct map struct
      identifier ignored_any
  }
}

fn from_trait<'de, R, T>(read: R) -> SonicRsResult<T>
where
  R: Reader<'de>,
  T: de::Deserialize<'de>,
{
  let len = read.as_u8_slice().len();
  if len > u32::MAX as _ {
    return Err(make_error(format!(
      "Only support JSON less than 4 GB, the input JSON is too large here, len is {len}"
    )));
  }

  let mut de = Deserializer::new(read);

  let value = tri!(de::Deserialize::deserialize(&mut de));

  tri!(de.parser.parse_trailing());

  tri!(de.parser.read.check_utf8_final());
  Ok(value)
}

pub fn from_slice<'a, T>(json: &'a [u8]) -> SonicRsResult<T>
where
  T: de::Deserialize<'a>,
{
  from_trait(Read::new(json, true))
}

#[derive(Debug, Clone, Copy, Default)]
struct DeserializeCfg {
  utf8_lossy: bool,
}

#[doc(hidden)]
#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum JsonSlice<'de> {
  Raw(&'de [u8]),
  FastStr(FastStr), // note: FastStr maybe inlined and in the stack.
}

impl Default for JsonSlice<'_> {
  fn default() -> Self {
    JsonSlice::Raw(&b"null"[..])
  }
}

impl<'de> From<FastStr> for JsonSlice<'de> {
  fn from(value: FastStr) -> Self {
    JsonSlice::FastStr(value)
  }
}

impl<'de> From<Bytes> for JsonSlice<'de> {
  fn from(value: Bytes) -> Self {
    JsonSlice::FastStr(unsafe { FastStr::from_bytes_unchecked(value) })
  }
}

impl<'de> From<&'de [u8]> for JsonSlice<'de> {
  fn from(value: &'de [u8]) -> Self {
    JsonSlice::Raw(value)
  }
}

impl<'de> From<&'de str> for JsonSlice<'de> {
  fn from(value: &'de str) -> Self {
    JsonSlice::Raw(value.as_bytes())
  }
}

impl<'de> From<&'de String> for JsonSlice<'de> {
  fn from(value: &'de String) -> Self {
    JsonSlice::Raw(value.as_bytes())
  }
}

impl From<String> for JsonSlice<'_> {
  fn from(value: String) -> Self {
    JsonSlice::FastStr(FastStr::new(value))
  }
}

impl<'de> AsRef<[u8]> for JsonSlice<'de> {
  fn as_ref(&self) -> &[u8] {
    match self {
      Self::Raw(r) => r,
      Self::FastStr(s) => s.as_bytes(),
    }
  }
}

trait JsonInput<'de>: Sealed {
  fn to_json_slice(&self) -> JsonSlice<'de>;
}

impl<'de> JsonInput<'de> for &'de [u8] {
  fn to_json_slice(&self) -> JsonSlice<'de> {
    JsonSlice::Raw(self)
  }
}

impl<'de> JsonInput<'de> for &'de str {
  fn to_json_slice(&self) -> JsonSlice<'de> {
    JsonSlice::Raw((*self).as_bytes())
  }
}

impl<'de> JsonInput<'de> for &'de Bytes {
  fn to_json_slice(&self) -> JsonSlice<'de> {
    let bytes = self.as_ref();
    let newed = self.slice_ref(bytes);
    JsonSlice::FastStr(unsafe { FastStr::from_bytes_unchecked(newed) })
  }
}

impl<'de> JsonInput<'de> for &'de FastStr {
  fn to_json_slice(&self) -> JsonSlice<'de> {
    JsonSlice::FastStr((**self).clone())
  }
}

impl<'de> JsonInput<'de> for &'de String {
  fn to_json_slice(&self) -> JsonSlice<'de> {
    JsonSlice::Raw(self.as_bytes())
  }
}

struct Position {
  line: usize,
  column: usize,
}

impl Position {
  fn from_index(mut i: usize, data: &[u8]) -> Self {
    i = i.min(data.len());
    let mut position = Position { line: 1, column: 1 };
    for ch in &data[..i] {
      match *ch {
        b'\n' => {
          position.line += 1;
          position.column = 1;
        }
        _ => {
          position.column += 1;
        }
      }
    }
    position
  }
}

#[doc(hidden)]
trait Reader<'de>: Sealed {
  fn remain(&self) -> usize;
  fn eat(&mut self, n: usize);
  fn backward(&mut self, n: usize);
  fn peek_n(&self, n: usize) -> Option<&'de [u8]>;
  fn peek(&self) -> Option<u8>;
  fn index(&self) -> usize;
  fn at(&self, index: usize) -> u8;
  fn set_index(&mut self, index: usize);
  fn next_n(&mut self, n: usize) -> Option<&'de [u8]>;

  #[inline(always)]
  fn next(&mut self) -> Option<u8> {
    self.peek().inspect(|_| {
      self.eat(1);
    })
  }

  fn slice_unchecked(&self, start: usize, end: usize) -> &'de [u8];

  fn as_u8_slice(&self) -> &'de [u8];

  fn check_utf8_final(&self) -> SonicRsResult<()>;

  fn next_invalid_utf8(&self) -> usize;

  fn check_invalid_utf8(&mut self);

  fn origin_input(&self) -> &'de [u8] {
    self.as_u8_slice()
  }
}

enum PinnedInput<'a> {
  FastStr(Pin<Box<FastStr>>),
  Slice(&'a [u8]),
}

impl<'a> PinnedInput<'a> {
  unsafe fn as_ptr(&self) -> NonNull<[u8]> {
    match self {
      Self::FastStr(f) => f.as_bytes().into(),
      Self::Slice(slice) => (*slice).into(),
    }
  }
}

impl<'a> From<JsonSlice<'a>> for PinnedInput<'a> {
  fn from(input: JsonSlice<'a>) -> Self {
    match input {
      JsonSlice::Raw(slice) => Self::Slice(slice),
      JsonSlice::FastStr(f) => Self::FastStr(Pin::new(Box::new(f))),
    }
  }
}

struct Read<'a> {
  input: PinnedInput<'a>,
  index: usize,
  next_invalid_utf8: usize,
}

impl<'a> Read<'a> {
  fn new(slice: &'a [u8], validate_utf8: bool) -> Self {
    Self::new_in(slice.to_json_slice(), validate_utf8)
  }

  fn new_in(input: JsonSlice<'a>, validate_utf8: bool) -> Self {
    let input: PinnedInput<'a> = input.into();
    let slice: NonNull<[u8]> = unsafe { input.as_ptr() };

    let next_invalid_utf8 = validate_utf8
      .then(|| {
        from_utf8(unsafe { slice.as_ref() })
          .err()
          .map(|e| e.offset())
      })
      .flatten()
      .unwrap_or(usize::MAX);

    Self {
      input,
      index: 0,
      next_invalid_utf8,
    }
  }

  #[inline(always)]
  fn slice(&self) -> &'a [u8] {
    unsafe { self.input.as_ptr().as_ref() }
  }
}

impl<'a> Reader<'a> for Read<'a> {
  #[inline(always)]
  fn remain(&self) -> usize {
    self.slice().len() - self.index
  }

  #[inline(always)]
  fn peek_n(&self, n: usize) -> Option<&'a [u8]> {
    let end = self.index + n;
    (end <= self.slice().len()).then(|| &self.slice()[self.index..end])
  }

  #[inline(always)]
  fn set_index(&mut self, index: usize) {
    self.index = index
  }

  #[inline(always)]
  fn peek(&self) -> Option<u8> {
    if self.index < self.slice().len() {
      Some(self.slice()[self.index])
    } else {
      None
    }
  }

  #[inline(always)]
  fn at(&self, index: usize) -> u8 {
    self.slice()[index]
  }

  #[inline(always)]
  fn next_n(&mut self, n: usize) -> Option<&'a [u8]> {
    let new_index = self.index + n;
    if new_index <= self.slice().len() {
      let ret = &self.slice()[self.index..new_index];
      self.index = new_index;
      Some(ret)
    } else {
      None
    }
  }

  #[inline(always)]
  fn index(&self) -> usize {
    self.index
  }

  #[inline(always)]
  fn eat(&mut self, n: usize) {
    self.index += n;
  }

  #[inline(always)]
  fn backward(&mut self, n: usize) {
    self.index -= n;
  }

  #[inline(always)]
  fn slice_unchecked(&self, start: usize, end: usize) -> &'a [u8] {
    &self.slice()[start..end]
  }

  #[inline(always)]
  fn as_u8_slice(&self) -> &'a [u8] {
    self.slice()
  }

  #[inline(always)]
  fn check_utf8_final(&self) -> SonicRsResult<()> {
    if self.next_invalid_utf8 == usize::MAX {
      Ok(())
    } else {
      Err(SonicRsError::syntax(
        SonicRsErrorCode::InvalidUTF8,
        self.origin_input(),
        self.next_invalid_utf8,
      ))
    }
  }

  fn check_invalid_utf8(&mut self) {
    self.next_invalid_utf8 = match from_utf8(&self.origin_input()[self.index..]) {
      Ok(_) => usize::MAX,
      Err(e) => self.index + e.offset(),
    };
  }

  fn next_invalid_utf8(&self) -> usize {
    self.next_invalid_utf8
  }
}

trait Sealed {}

impl Sealed for usize {}

impl Sealed for str {}

impl Sealed for std::string::String {}

impl Sealed for FastStr {}

impl Sealed for Bytes {}

impl Sealed for u8 {}

impl<'de> Sealed for Read<'de> {}

impl<'a, T> Sealed for &'a T where T: ?Sized + Sealed {}

impl<T> Sealed for [T] where T: Sized + Sealed {}

const ESCAPED_TAB: [u8; 256] = [
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, b'"', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b'/', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b'\\',
  0, 0, 0, 0, 0, b'\x08', /* \b */
  0, 0, 0, b'\x0c', /* \f */
  0, 0, 0, 0, 0, 0, 0, b'\n', 0, 0, 0, b'\r', 0, b'\t', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[derive(Debug)]
struct StringBlock<B: BitMask> {
  bs_bits: B,
  quote_bits: B,
  unescaped_bits: B,
}

impl StringBlock<u32> {
  const LANES: usize = 32;

  #[inline]
  fn new(v: &u8x32) -> Self {
    Self {
      bs_bits: (v.eq(&u8x32::splat(b'\\'))).bitmask(),
      quote_bits: (v.eq(&u8x32::splat(b'"'))).bitmask(),
      unescaped_bits: (v.le(&u8x32::splat(0x1f))).bitmask(),
    }
  }
}

impl<B: BitMask> StringBlock<B> {
  #[inline(always)]
  fn has_unescaped(&self) -> bool {
    self.unescaped_bits.before(&self.quote_bits)
  }

  #[inline(always)]
  fn has_quote_first(&self) -> bool {
    self.quote_bits.before(&self.bs_bits) && !self.has_unescaped()
  }

  #[inline(always)]
  fn has_backslash(&self) -> bool {
    self.bs_bits.before(&self.quote_bits)
  }

  #[inline(always)]
  fn quote_index(&self) -> usize {
    self.quote_bits.first_offset()
  }

  #[inline(always)]
  fn bs_index(&self) -> usize {
    self.bs_bits.first_offset()
  }

  #[inline(always)]
  fn unescaped_index(&self) -> usize {
    self.unescaped_bits.first_offset()
  }
}

#[inline(always)]
unsafe fn load<V: Simd>(ptr: *const u8) -> V {
  let chunk = from_raw_parts(ptr, V::LANES);
  V::from_slice_unaligned_unchecked(chunk)
}

const DIGIT_TO_VAL32: [u32; 886] = [
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0x0, 0x10, 0x20, 0x30,
  0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xa0, 0xb0, 0xc0, 0xd0, 0xe0, 0xf0, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xa0, 0xb0,
  0xc0, 0xd0, 0xe0, 0xf0, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0x0, 0x100, 0x200, 0x300, 0x400,
  0x500, 0x600, 0x700, 0x800, 0x900, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xa00, 0xb00, 0xc00, 0xd00, 0xe00, 0xf00, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xa00, 0xb00, 0xc00, 0xd00, 0xe00, 0xf00, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0x0, 0x1000,
  0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0x7000, 0x8000, 0x9000, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xa000, 0xb000, 0xc000, 0xd000,
  0xe000, 0xf000, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xa000, 0xb000, 0xc000, 0xd000, 0xe000, 0xf000,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
  0xFFFFFFFF,
];

#[inline(always)]
unsafe fn hex_to_u32_nocheck(src: &[u8; 4]) -> u32 {
  let v1 = DIGIT_TO_VAL32[630 + src[0] as usize];
  let v2 = DIGIT_TO_VAL32[420 + src[1] as usize];
  let v3 = DIGIT_TO_VAL32[210 + src[2] as usize];
  let v4 = DIGIT_TO_VAL32[src[3] as usize];
  v1 | v2 | v3 | v4
}

unsafe fn codepoint_to_utf8(cp: u32, c: *mut u8) -> usize {
  if cp <= 0x7F {
    unsafe { *c = cp as u8 };
    1 // ascii
  } else if cp <= 0x7FF {
    unsafe {
      *c = (((cp >> 6) + 192) & 0xFF) as u8;
      *(c.offset(1)) = ((cp & 63) + 128) as u8;
    }
    2 // universal plane
  } else if cp <= 0xFFFF {
    unsafe {
      *c = (((cp >> 12) + 224) & 0xFF) as u8;
      *(c.offset(1)) = (((cp >> 6) & 63) + 128) as u8;
      *(c.offset(2)) = ((cp & 63) + 128) as u8;
    }
    3
  } else if cp <= 0x10FFFF {
    unsafe {
      *c = ((cp >> 18) + 240) as u8;
      *(c.offset(1)) = (((cp >> 12) & 63) + 128) as u8;
      *(c.offset(2)) = (((cp >> 6) & 63) + 128) as u8;
      *(c.offset(3)) = ((cp & 63) + 128) as u8;
    }
    4
  } else {
    0 // bad r
  }
}

#[inline]
fn from_utf8(data: &[u8]) -> SonicRsResult<&str> {
  simdutf8::basic::from_utf8(data).or_else(|_| from_utf8_compat(data))
}

#[cold]
fn from_utf8_compat(data: &[u8]) -> SonicRsResult<&str> {
  simdutf8::compat::from_utf8(data)
    .map_err(|e| SonicRsError::syntax(SonicRsErrorCode::InvalidUTF8, data, e.valid_up_to()))
}

unsafe fn prefix_xor(bitmask: u64) -> u64 {
  let mut bitmask = bitmask;
  bitmask ^= bitmask << 1;
  bitmask ^= bitmask << 2;
  bitmask ^= bitmask << 4;
  bitmask ^= bitmask << 8;
  bitmask ^= bitmask << 16;
  bitmask ^= bitmask << 32;
  bitmask
}

#[inline(always)]
unsafe fn get_nonspace_bits(data: &[u8; 64]) -> u64 {
  let mut mask: u64 = 0;
  for (i, p) in data.iter().enumerate() {
    if !matches!(*p, b'\t' | b'\n' | b'\r' | b' ') {
      mask |= 1 << i;
    }
  }
  mask
}
