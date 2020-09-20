//! This code is heavily based on the [rust-playground code][rust-playground]
//!
//! [rust-playground]: (https://github.com/integer32llc/rust-playground/tree/master/ui

use log;
use snafu::{ResultExt, Snafu};
use std::{
    fmt,
    fs::{self, File},
    io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    string,
    time::Duration,
};
use tempdir::TempDir;
use tokio::process::Command;

const DOCKER_CONTAINER_NAME: &str = "lowlvl/playground";
const DOCKER_PROCESS_TIMEOUT_SOFT: Duration = Duration::from_secs(10);
const DOCKER_PROCESS_TIMEOUT_HARD: Duration = Duration::from_secs(12);

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unable to create temporary directory: {}", source))]
    UnableToCreateTempDir { source: io::Error },
    #[snafu(display("Unable to create output directory: {}", source))]
    UnableToCreateOutputDir { source: io::Error },
    #[snafu(display("Unable to set permissions for output directory: {}", source))]
    UnableToSetOutputPermissions { source: io::Error },
    #[snafu(display("Unable to create source file: {}", source))]
    UnableToCreateSourceFile { source: io::Error },
    #[snafu(display("Unable to set permissions for source file: {}", source))]
    UnableToSetSourcePermissions { source: io::Error },
    #[snafu(display("Unable to execute the compiler: {}", source))]
    UnableToExecuteCompiler { source: io::Error },
    #[snafu(display("Compiler execution took longer than {} ms", timeout.as_millis()))]
    CompilerExecutionTimedOut {
        source: tokio::time::Elapsed,
        timeout: Duration,
    },
    #[snafu(display("Unable to read output file: {}", source))]
    UnableToReadOutput { source: io::Error },
    #[snafu(display("Output was not valid UTF-8: {}", source))]
    OutputNotUtf8 { source: string::FromUtf8Error },
    #[snafu(display("Output was missing"))]
    OutputMissing,
    #[snafu(display("Release was missing from the version output"))]
    VersionReleaseMissing,
    #[snafu(display("Commit hash was missing from the version output"))]
    VersionHashMissing,
    #[snafu(display("Commit date was missing from the version output"))]
    VersionDateMissing,
}

pub type Result<T, E = Error> = ::std::result::Result<T, E>;

#[derive(Debug)]
pub struct CompileResponse {
    pub success: bool,
    pub wasm: Option<File>,
    pub stdout: String,
    pub stderr: String,
}

pub struct Sandbox {
    // This is a phantom var to hold a temporary dir
    #[allow(dead_code)]
    scratch: TempDir,
    input_file: PathBuf,
    output_dir: PathBuf,
}

// We must create a world-writable files (rustfmt) and directories
// (LLVM IR) so that the process inside the Docker container can write
// into it.
//
// This problem does *not* occur when using the indirection of
// docker-machine.
fn wide_open_permissions() -> std::fs::Permissions {
    PermissionsExt::from_mode(0o777)
}

async fn run_command_with_timeout(mut command: Command) -> Result<std::process::Output> {
    let timeout = DOCKER_PROCESS_TIMEOUT_HARD;

    tokio::time::timeout(timeout, command.output())
        .await
        .context(CompilerExecutionTimedOut { timeout })?
        .context(UnableToExecuteCompiler)
}

impl Sandbox {
    pub fn new() -> Result<Self> {
        let scratch = TempDir::new("playground").context(UnableToCreateTempDir)?;
        let input_file = scratch.path().join("input.rs");
        let output_dir = scratch.path().join("output");

        fs::create_dir(&output_dir).context(UnableToCreateOutputDir)?;
        fs::set_permissions(&output_dir, wide_open_permissions())
            .context(UnableToSetOutputPermissions)?;

        Ok(Sandbox {
            scratch,
            input_file,
            output_dir,
        })
    }

    pub async fn compile(&self, code: &str) -> Result<CompileResponse> {
        self.write_source_code(code)?;

        let command = self.compile_command();

        let output = run_command_with_timeout(command).await?;

        // The compiler writes the file to a name like
        // `req.wasm`, so we just find the first with the right extension.
        let mut file = PathBuf::new();
        file.push(&self.output_dir);
        file.push("result.wasm");

        let stdout = vec_to_str(output.stdout)?;
        let mut stderr = vec_to_str(output.stderr)?;

        let wasm = if file.exists() {
            Some(read(&file)?)
        } else {
            // If we didn't find the file, it's *most* likely that
            // the user's code was invalid. Tack on our own error
            // to the compiler's error instead of failing the
            // request.
            use self::fmt::Write;
            write!(&mut stderr, "\nUnable to locate output file",)
                .expect("Unable to write to a string");
            None
        };

        Ok(CompileResponse {
            success: output.status.success(),
            wasm,
            stdout,
            stderr,
        })
    }

    fn write_source_code(&self, code: &str) -> Result<()> {
        fs::write(&self.input_file, code).context(UnableToCreateSourceFile)?;
        fs::set_permissions(&self.input_file, wide_open_permissions())
            .context(UnableToSetSourcePermissions)?;

        log::debug!(
            "Wrote {} bytes of source to {}",
            code.len(),
            self.input_file.display()
        );
        Ok(())
    }

    fn compile_command(&self) -> Command {
        let mut cmd = self.docker_command();

        cmd.arg(DOCKER_CONTAINER_NAME)
            .arg("rustc-wasm")
            .args(&["-o", "/playground-result/result.wasm"])
            .arg("input.rs");

        log::debug!("Compilation command is {:?}", cmd);

        cmd
    }

    fn docker_command(&self) -> Command {
        let mut mount_input_file = self.input_file.as_os_str().to_os_string();
        mount_input_file.push(":");
        mount_input_file.push("/playground/input.rs");

        let mut mount_output_dir = self.output_dir.as_os_str().to_os_string();
        mount_output_dir.push(":");
        mount_output_dir.push("/playground-result");

        let mut cmd = basic_secure_docker_command();

        cmd.arg("--volume")
            .arg(&mount_input_file)
            .arg("--volume")
            .arg(&mount_output_dir);

        cmd
    }
}

fn read(path: &Path) -> Result<File> {
    let f = match File::open(path) {
        Ok(f) => f,
        e => e.context(UnableToReadOutput)?,
    };
    Ok(f)
}

fn basic_secure_docker_command() -> Command {
    let mut cmd = Command::new("docker");

    cmd.arg("run")
        .arg("--rm")
        .arg("--cap-drop=ALL")
        .arg("--cap-add=DAC_OVERRIDE")
        .arg("--security-opt=no-new-privileges")
        .args(&["--workdir", "/playground"])
        .args(&["--net", "none"])
        .args(&["--memory", "256m"])
        .args(&["--memory-swap", "320m"])
        .args(&[
            "--env",
            &format!(
                "PLAYGROUND_TIMEOUT={}",
                DOCKER_PROCESS_TIMEOUT_SOFT.as_secs()
            ),
        ])
        .args(&["--pids-limit", "512"]);

    cmd.kill_on_drop(true);

    cmd
}

fn vec_to_str(v: Vec<u8>) -> Result<String> {
    String::from_utf8(v).context(OutputNotUtf8)
}
