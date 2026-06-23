//! Thin wrapper around the `tmux` CLI.

use std::process::Command;

/// A tmux session as reported by `tmux list-sessions`.
#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    /// Number of windows in the session.
    pub windows: u32,
    /// Whether at least one client is currently attached.
    pub attached: bool,
    /// Human-readable "last activity" hint, e.g. "2h ago".
    pub activity: String,
}

/// Returns true if a `tmux` binary is on the PATH.
pub fn is_available() -> bool {
    Command::new("tmux")
        .arg("-V")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Lists existing tmux sessions. Returns an empty vec when the server isn't
/// running or there are no sessions (tmux exits non-zero in that case).
pub fn list_sessions() -> Vec<Session> {
    // Custom format keeps parsing trivial and stable across tmux versions.
    let fmt = "#{session_name}\t#{session_windows}\t#{session_attached}\t#{session_activity}";
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", fmt])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let name = parts.next()?.to_string();
            let windows = parts.next()?.parse().unwrap_or(0);
            let attached = parts.next().map(|s| s != "0").unwrap_or(false);
            let activity_epoch: i64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            Some(Session {
                name,
                windows,
                attached,
                activity: humanize_since(now - activity_epoch),
            })
        })
        .collect()
}

/// Replaces the current process with `tmux attach` to the given session.
/// Never returns on success.
pub fn attach(name: &str) -> std::io::Error {
    exec(Command::new("tmux").args(["attach-session", "-t", name]))
}

/// Replaces the current process with `tmux new-session`. Never returns on
/// success. An empty name lets tmux pick the default numeric name.
pub fn new_session(name: &str) -> std::io::Error {
    let mut cmd = Command::new("tmux");
    cmd.arg("new-session");
    if !name.is_empty() {
        cmd.args(["-s", name]);
    }
    exec(&mut cmd)
}

/// Replaces the current process image with `cmd` via execvp(2). On success it
/// does not return; on failure the returned error explains why.
fn exec(cmd: &mut Command) -> std::io::Error {
    use std::os::unix::process::CommandExt;
    cmd.exec()
}

/// Turns a duration in seconds into a compact "Nx ago" string.
fn humanize_since(secs: i64) -> String {
    if secs < 0 {
        return "just now".into();
    }
    let (n, unit) = match secs {
        0..=59 => return "just now".into(),
        60..=3599 => (secs / 60, "m"),
        3600..=86399 => (secs / 3600, "h"),
        _ => (secs / 86400, "d"),
    };
    format!("{n}{unit} ago")
}

#[cfg(test)]
mod tests {
    use super::humanize_since;

    #[test]
    fn humanizes_durations() {
        assert_eq!(humanize_since(-5), "just now");
        assert_eq!(humanize_since(10), "just now");
        assert_eq!(humanize_since(120), "2m ago");
        assert_eq!(humanize_since(7200), "2h ago");
        assert_eq!(humanize_since(172800), "2d ago");
    }
}
