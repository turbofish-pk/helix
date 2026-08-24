// Implementation reference: https://github.com/neovim/neovim/blob/f2906a4669a2eef6d7bf86a29648793d63c98949/runtime/autoload/provider/clipboard.vim#L68-L152
//
// NOTE(pk): clipboard only (Ctrl+C / Ctrl+V). No primary selection.
// // NOTE(pk):
// - clipboard = Ctrl+C / Ctrl+V. Works on every OS. Helix calls it register +.
// - primary = highlight with mouse, paste with middle-click. Linux/X11/Wayland only. Helix calls it register *.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum ClipboardError {
  #[error(transparent)]
  IoError(#[from] std::io::Error),
  #[error("could not convert terminal output to UTF-8: {0}")]
  FromUtf8Error(#[from] std::string::FromUtf8Error),
  #[error("clipboard provider command failed")]
  CommandFailed,
  #[error("failed to write to clipboard provider's stdin")]
  StdinWriteFailed,
  #[error("clipboard provider did not return any contents")]
  MissingStdout,
  #[error("This clipboard provider does not support reading")]
  ReadingNotSupported,
}

type Result<T> = std::result::Result<T, ClipboardError>;

pub use external::ClipboardProvider;

mod external {
  use super::{ClipboardError, Cow, Deserialize, Result, Serialize};

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
  pub struct Command {
    command: Cow<'static, str>,
    #[serde(default)]
    args: Cow<'static, [Cow<'static, str>]>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
  #[serde(rename_all = "kebab-case")]
  pub struct CommandProvider {
    yank: Command,
    paste: Command,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
  #[serde(rename_all = "kebab-case")]
  #[allow(clippy::large_enum_variant)]
  pub enum ClipboardProvider {
    Wayland,
    XClip,
    XSel,
    #[cfg(feature = "term")]
    Termcode,
    Custom(CommandProvider),
    None,
  }

  impl Default for ClipboardProvider {
    fn default() -> Self {
      use helix_stdx::env::{binary_exists, env_var_is_set};

      fn is_exit_success(program: &str, args: &[&str]) -> bool {
        std::process::Command::new(program)
          .args(args)
          .output()
          .ok()
          .and_then(|out| out.status.success().then_some(()))
          .is_some()
      }

      if cfg!(feature = "term") && env_var_is_set("WEZTERM_UNIX_SOCKET") && binary_exists("wezterm")
      {
        #[cfg(feature = "term")]
        return Self::Termcode;
        #[cfg(not(feature = "term"))]
        return Self::None;
      } else if env_var_is_set("WAYLAND_DISPLAY")
        && binary_exists("wl-copy")
        && binary_exists("wl-paste")
      {
        Self::Wayland
      } else if env_var_is_set("DISPLAY") && binary_exists("xclip") {
        Self::XClip
      } else if env_var_is_set("DISPLAY")
                && binary_exists("xsel")
                // FIXME: check performance of is_exit_success
                && is_exit_success("xsel", &["-o", "-b"])
      {
        Self::XSel
      } else {
        #[cfg(feature = "term")]
        return Self::Termcode;
        #[cfg(not(feature = "term"))]
        return Self::None;
      }
    }
  }

  impl ClipboardProvider {
    #[must_use]
    pub fn name(&self) -> Cow<'_, str> {
      fn builtin_name<'a>(name: &'static str, provider: &'static CommandProvider) -> Cow<'a, str> {
        if provider.yank.command == provider.paste.command {
          Cow::Owned(format!(
            "{} ({}+{})",
            name, provider.yank.command, provider.paste.command
          ))
        } else {
          Cow::Owned(format!("{} ({})", name, provider.yank.command))
        }
      }

      match self {
        // These names should match the config option names from Serde
        Self::Wayland => builtin_name("wayland", &WL_CLIPBOARD),
        Self::XClip => builtin_name("x-clip", &XCLIP),
        Self::XSel => builtin_name("x-sel", &XSEL),
        #[cfg(feature = "term")]
        Self::Termcode => "termcode".into(),
        Self::Custom(command_provider) => Cow::Owned(format!(
          "custom ({}+{})",
          command_provider.yank.command, command_provider.paste.command
        )),
        Self::None => "none".into(),
      }
    }

    pub fn get_contents(&self) -> Result<String> {
      fn yank_from(provider: &CommandProvider) -> Result<String> {
        execute_command(&provider.yank, None, true)?.ok_or(ClipboardError::MissingStdout)
      }

      match self {
        Self::Wayland => yank_from(&WL_CLIPBOARD),
        Self::XClip => yank_from(&XCLIP),
        Self::XSel => yank_from(&XSEL),
        #[cfg(feature = "term")]
        Self::Termcode => Err(ClipboardError::ReadingNotSupported),
        Self::Custom(command_provider) => yank_from(command_provider),
        Self::None => Err(ClipboardError::ReadingNotSupported),
      }
    }

    pub fn set_contents(&self, content: &str) -> Result<()> {
      fn paste_to(provider: &CommandProvider, content: &str) -> Result<()> {
        execute_command(&provider.paste, Some(content), false).map(|_| ())
      }

      match self {
        Self::Wayland => paste_to(&WL_CLIPBOARD, content),
        Self::XClip => paste_to(&XCLIP, content),
        Self::XSel => paste_to(&XSEL, content),
        #[cfg(feature = "term")]
        Self::Termcode => {
          use helix_ext::termina::escape::osc::{self, Osc};
          use std::io::Write;
          // NOTE: it would be ideal to have the terminal execute this but it _should_
          // work to send this over stdout instead.
          let mut stdout = std::io::stdout().lock();
          write!(
            stdout,
            "{}",
            Osc::SetSelection(osc::Selection::CLIPBOARD, content)
          )?;
          stdout.flush()?;
          Ok(())
        }
        Self::Custom(command_provider) => paste_to(command_provider, content),
        Self::None => Ok(()),
      }
    }
  }

  macro_rules! command_provider {
        ($name:ident,
         yank => $yank_cmd:literal $( , $yank_arg:literal )* ;
         paste => $paste_cmd:literal $( , $paste_arg:literal )* ; ) => {
            const $name: CommandProvider = CommandProvider {
                yank: Command {
                    command: Cow::Borrowed($yank_cmd),
                    args: Cow::Borrowed(&[ $( Cow::Borrowed($yank_arg) ),* ])
                },
                paste: Command {
                    command: Cow::Borrowed($paste_cmd),
                    args: Cow::Borrowed(&[ $( Cow::Borrowed($paste_arg) ),* ])
                },
            };
        };
    }

  command_provider! {
      WL_CLIPBOARD,
      yank => "wl-paste", "--no-newline";
      paste => "wl-copy", "--type", "text/plain";
  }
  command_provider! {
      XCLIP,
      yank => "xclip", "-o", "-selection", "clipboard";
      paste => "xclip", "-i", "-selection", "clipboard";
  }
  command_provider! {
      XSEL,
      yank => "xsel", "-o", "-b";
      paste => "xsel", "-i", "-b";
  }

  fn execute_command(
    cmd: &Command,
    input: Option<&str>,
    pipe_output: bool,
  ) -> Result<Option<String>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let stdin = input.map_or_else(Stdio::null, |_| Stdio::piped());
    let stdout = pipe_output.then(Stdio::piped).unwrap_or_else(Stdio::null);

    let mut command: Command = Command::new(cmd.command.as_ref());

    #[allow(unused_mut)]
    let mut command_mut: &mut Command = command
      .args(cmd.args.iter().map(AsRef::as_ref))
      .stdin(stdin)
      .stdout(stdout)
      .stderr(Stdio::null());

    // Fix for https://github.com/helix-editor/helix/issues/5424

    {
      use std::os::unix::process::CommandExt;

      unsafe {
        command_mut = command_mut.pre_exec(|| match libc::setsid() {
          -1 => Err(std::io::Error::last_os_error()),
          _ => Ok(()),
        });
      }
    }

    let mut child = command_mut.spawn()?;

    if let Some(input) = input {
      let mut stdin = child.stdin.take().ok_or(ClipboardError::StdinWriteFailed)?;
      stdin
        .write_all(input.as_bytes())
        .map_err(|_| ClipboardError::StdinWriteFailed)?;
    }

    // TODO: add timer?
    let output = child.wait_with_output()?;

    if !output.status.success() {
      log::error!(
        "clipboard provider {} failed with stderr: \"{}\"",
        cmd.command,
        String::from_utf8_lossy(&output.stderr)
      );
      return Err(ClipboardError::CommandFailed);
    }

    if pipe_output {
      Ok(Some(String::from_utf8(output.stdout)?))
    } else {
      Ok(None)
    }
  }
}
