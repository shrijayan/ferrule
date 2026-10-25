//! `ferrule-gen convert`: the only module in the converter crate that touches the file system.
//! Port of `converter/src/cli.ts`.

use std::io::Write;

/// Run the CLI on `args` (after the subcommand); returns the exit code (0 ok, 1 error
/// diagnostics, 2 usage or input error).
///
/// # Panics
///
/// Until the port lands.
pub fn main(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let _ = (args, stdout, stderr);
    todo!("port of converter/src/cli.ts main")
}
