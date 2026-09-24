//! External command execution.

use std::path::PathBuf;

/// A command to execute with its arguments.
///
/// kk only needs plain command + args execution (stdin/stdout/stderr are
/// captured / discarded by default).
#[derive(Debug, Clone)]
pub struct ExternalCommand {
    /// Path to the executable command.
    pub command: PathBuf,

    /// Command line arguments to pass to the executable.
    pub args: Vec<String>,
}

impl ExternalCommand {
    /// Executes the command, capturing stdout and stderr.
    pub fn execute(&self) -> std::io::Result<std::process::Output> {
        let mut cmd = std::process::Command::new(&self.command);
        for arg in &self.args {
            cmd.arg(arg);
        }
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.output()
    }

    /// Returns a command line representation for display purposes.
    pub fn command_line(&self) -> impl '_ + std::fmt::Display {
        CommandLine {
            command: &self.command,
            args: &self.args,
        }
    }
}

struct CommandLine<'a> {
    command: &'a PathBuf,
    args: &'a [String],
}

impl std::fmt::Display for CommandLine<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.command.display())?;
        for arg in self.args {
            if arg.is_empty() || arg.chars().any(|c| c.is_control() || c.is_whitespace()) {
                write!(f, " {arg:?}")?;
            } else {
                write!(f, " {arg}")?;
            }
        }
        Ok(())
    }
}
