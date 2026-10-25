//! Resolve the user's login-shell environment (so a GUI started from a desktop launcher still
//! gets variables set in shell profiles, e.g. `CLAUDE_CONFIG_DIR`).

use std::collections::BTreeMap;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// Printed right before the environment JSON so shell noise can be skipped.
pub const ENV_MARKER: &str = "__GANTRY_ENV__";

/// Current process environment as a JSON object (what `gantry internal-print-env` prints).
pub fn print_env_json() -> String {
    let env: BTreeMap<String, String> = std::env::vars().collect();
    serde_json::to_string(&env).expect("string map serializes")
}

/// Extract the environment printed after [`ENV_MARKER`].
pub fn parse_env_output(stdout: &str) -> Option<Vec<(String, String)>> {
    let (_, json) = stdout.split_once(ENV_MARKER)?;
    let env: BTreeMap<String, String> = serde_json::from_str(json.trim()).ok()?;
    Some(env.into_iter().collect())
}

/// Quote `text` for a POSIX shell.
pub fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// The `PATH` value in the output of macOS's `/usr/libexec/path_helper -s` (`PATH="…"; export PATH;`).
pub fn parse_path_helper(stdout: &str) -> Option<&str> {
    let (_, rest) = stdout.split_once("PATH=\"")?;
    rest.split_once('"').map(|(path, _)| path)
}

/// A `PATH` for when the login shell cannot be read on macOS, where an app opened from Finder only gets
/// `/usr/bin:/bin:/usr/sbin:/sbin`: where Claude's installer and Homebrew put programs, then `system` (from
/// `path_helper`), then `current`; each folder once.
pub fn fallback_path(home: &str, system: Option<&str>, current: &str) -> String {
    let local = format!("{home}/.local/bin");
    let first = [local.as_str(), "/opt/homebrew/bin", "/opt/homebrew/sbin", "/usr/local/bin"];
    let mut folders: Vec<&str> = Vec::new();
    for folder in first.into_iter().chain(system.unwrap_or_default().split(':')).chain(current.split(':')) {
        if !folder.is_empty() && !folders.contains(&folder) {
            folders.push(folder);
        }
    }
    folders.join(":")
}

/// Run `shell -l -i -c "<marker>; <print_command>"` and parse its output; `None` on failure or timeout.
pub fn resolve_login_env(shell: &str, print_command: &str, timeout: Duration) -> Option<Vec<(String, String)>> {
    let mut child = Command::new(shell)
        .args(["-l", "-i", "-c", &format!("printf '%s' {ENV_MARKER}; {print_command}")])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stdout.read_to_string(&mut text);
        let _ = tx.send(text);
    });
    let result = rx.recv_timeout(timeout).ok().and_then(|text| parse_env_output(&text));
    let _ = child.kill();
    let _ = child.wait();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_env_after_marker_ignoring_shell_noise() {
        let stdout = format!("Welcome to fish!\nlast login: …\n{ENV_MARKER}{{\"A\":\"1\",\"B\":\"x y\"}}\n");
        let mut env = parse_env_output(&stdout).unwrap();
        env.sort();
        assert_eq!(env, vec![("A".into(), "1".into()), ("B".into(), "x y".into())]);
        assert_eq!(parse_env_output("no marker here"), None);
        assert_eq!(parse_env_output(&format!("{ENV_MARKER}not json")), None);
    }

    #[test]
    fn quoting_survives_single_quotes() {
        assert_eq!(shell_quote("/a b/it's"), r#"'/a b/it'\''s'"#);
    }

    #[test]
    fn reads_path_helper_output() {
        let stdout =
            "PATH=\"/usr/local/bin:/usr/bin:/bin\"; export PATH;\nMANPATH=\"/usr/share/man:\"; export MANPATH;\n";
        assert_eq!(parse_path_helper(stdout), Some("/usr/local/bin:/usr/bin:/bin"));
        assert_eq!(parse_path_helper(""), None);
    }

    #[test]
    fn fallback_path_puts_user_folders_first_once() {
        let path = fallback_path("/Users/a", Some("/usr/local/bin:/usr/bin:/bin"), "/usr/bin:/bin:/usr/sbin:/sbin");
        assert_eq!(
            path,
            "/Users/a/.local/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
        );
        assert_eq!(
            fallback_path("/h", None, "/bin"),
            "/h/.local/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/bin"
        );
    }

    #[test]
    fn print_env_json_round_trips() {
        let json = print_env_json();
        let env = parse_env_output(&format!("{ENV_MARKER}{json}")).unwrap();
        // Windows spells it `Path` (names are case-insensitive there).
        assert!(env.iter().any(|(k, _)| k.eq_ignore_ascii_case("PATH")));
    }

    #[cfg(unix)]
    #[test]
    fn resolves_env_through_a_login_shell() {
        let env = resolve_login_env("/bin/sh", r#"printf '{"GANTRY_T":"yes"}'"#, Duration::from_secs(5)).unwrap();
        assert_eq!(env, vec![("GANTRY_T".into(), "yes".into())]);
    }

    #[cfg(unix)]
    #[test]
    fn hanging_shell_times_out() {
        let started = std::time::Instant::now();
        assert_eq!(resolve_login_env("/bin/sh", "sleep 10", Duration::from_millis(300)), None);
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
