//! tmosh — on SSH/mosh login, offer to attach to a detached tmux session,
//! create a new one, or drop to the shell. Esc is always the escape hatch.

mod menu;
mod tmux;
mod update;

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("--version" | "-V") => {
            println!("tmosh {VERSION}");
            update::flush();
            return ExitCode::SUCCESS;
        }
        Some("--help" | "-h") => {
            print_help();
            return ExitCode::SUCCESS;
        }
        Some("--update") => {
            return ExitCode::from(update::run_foreground(VERSION) as u8);
        }
        Some("--self-update-bg") => {
            // Internal: spawned detached by maybe_check_in_background.
            update::run_background(VERSION);
            return ExitCode::SUCCESS;
        }
        Some("--init") => {
            print_shell_init();
            return ExitCode::SUCCESS;
        }
        Some(other) => {
            eprintln!("tmosh: unknown argument '{other}'\n");
            print_help();
            return ExitCode::FAILURE;
        }
        None => {}
    }

    run()
}

fn run() -> ExitCode {
    // Never hijack a non-interactive shell (scp, rsync, scripts, etc.).
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return ExitCode::SUCCESS;
    }

    // Already inside tmux? Do nothing, just hand control back to the shell.
    if std::env::var_os("TMUX").is_some() {
        return ExitCode::SUCCESS;
    }

    // Fire off a throttled, detached update check; never blocks the menu.
    update::maybe_check_in_background();

    if !tmux::is_available() {
        eprintln!("tmosh: tmux not found on PATH — continuing to shell.");
        return ExitCode::SUCCESS;
    }

    // Only offer detached sessions as attach candidates — a session already
    // attached elsewhere shouldn't be in the list.
    let sessions: Vec<_> = tmux::list_sessions()
        .into_iter()
        .filter(|s| !s.attached)
        .collect();

    let choice = match menu::run(&sessions, VERSION) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("tmosh: {e} — continuing to shell.");
            return ExitCode::SUCCESS;
        }
    };

    match choice {
        menu::Choice::Shell => ExitCode::SUCCESS, // escape hatch → return to shell
        menu::Choice::Attach(name) => {
            let err = tmux::attach(&name); // exec replaces us on success
            eprintln!("tmosh: failed to attach to '{name}': {err}");
            ExitCode::FAILURE
        }
        menu::Choice::NewSession => {
            let name = prompt_session_name().unwrap_or_default();
            let err = tmux::new_session(&name); // exec replaces us on success
            eprintln!("tmosh: failed to create session: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Prompts (cooked mode, on stderr) for an optional new session name.
fn prompt_session_name() -> Option<String> {
    let mut err = io::stderr();
    let _ = write!(err, "  New session name (empty for default): ");
    let _ = err.flush();
    let mut line = String::new();
    io::stdin().read_line(&mut line).ok()?;
    let name = line.trim().to_string();
    Some(name)
}

fn print_help() {
    println!(
        "tmosh {VERSION} — tmux session picker for SSH/mosh login

USAGE:
    tmosh              Launch the interactive session picker
    tmosh --update     Check for and install the latest release
    tmosh --init       Print the shell snippet to add to your rc file
    tmosh --version    Print version
    tmosh --help       Show this help

In the picker: ↑/↓ to move, enter to select, esc to drop to the shell."
    );
}

/// The snippet users add to ~/.bashrc / ~/.zshrc. Guards against running in
/// non-interactive shells and inside an existing tmux.
fn print_shell_init() {
    print!(
        r#"# >>> tmosh >>>
# Launch tmosh on interactive login shells (skips inside tmux & non-tty).
if command -v tmosh >/dev/null 2>&1; then
  case $- in
    *i*)
      if [ -z "$TMUX" ] && [ -t 1 ]; then
        tmosh
      fi
      ;;
  esac
fi
# <<< tmosh <<<
"#
    );
}
