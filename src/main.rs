extern crate cargo;
extern crate clap;

use clap::{App, Arg};
use cargo::{CargoError, CargoResult, Config};
use cargo::ops::{compile_with_exec, CompileOptions};
use cargo::core::{PackageId, Target, Workspace, Verbosity};
use cargo::core::compiler::{CompileMode, DefaultExecutor, Executor};
use cargo::util::ProcessBuilder;
use cargo::util::important_paths::find_root_manifest_for_wd;
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::fs::OpenOptions;

fn touch_fingerprint(cmd: ProcessBuilder, id: &PackageId) {
    let mut crate_name = String::new();
    let mut crate_type = String::new();
    let mut out_dir = String::new();
    let mut extra_filename = String::new();
    // Don't support OsString for now by casting to String
    let args = cmd
        .get_args()
        .iter()
        .map(|x| x.clone().into_string().unwrap())
        .collect::<Vec<String>>();
    for (i, arg) in args.iter().enumerate() {
        if arg == "--crate-name" {
            crate_name = args[i + 1].clone();
        } else if arg == "--crate-type" {
            crate_type = args[i + 1].clone();
        } else {
            if arg.starts_with("extra-filename") {
                extra_filename = arg
                    .split("=")
                    .collect::<Vec<&str>>()[1]
                    .to_string();
            } else if arg.starts_with("dependency") {
                out_dir = arg
                    .split("=")
                    .collect::<Vec<&str>>()[1]
                    .to_string();
            }
        }
    }
    if crate_name == "build_script_build" {
        crate_type = "build-script".to_string();
    }
    let package_name = id.name();
    let file_path = format!(
        "{}/../.fingerprint/{}{}/dep-{}-{}{}",
        out_dir,
        package_name,
        extra_filename,
        crate_type,
        crate_name,
        extra_filename,
    );
    //println!("{}", file_path);
    OpenOptions::new()
        .create(true)
        .write(true)
        .open(Path::new(file_path.as_str()))
        .unwrap();
}

struct BuildDepsExecutor;

impl Executor for BuildDepsExecutor {
    fn exec(
        &self,
        cmd: ProcessBuilder,
        _id: &PackageId,
        _target: &Target,
        _mode: CompileMode,
    ) -> CargoResult<()> {
        if !_id.source_id().is_path() {
            cmd.exec()?;
        } else {
            println!("Skipping {}", _id.name());
            touch_fingerprint(cmd, _id);
        }
        Ok(())
    }
}

fn build_deps(cwd: &Path, release: bool, deps_only: bool) -> Result<(), CargoError> {
    let config = Config::default()?;
    config.shell().set_verbosity(Verbosity::Normal);
    let manifest = find_root_manifest_for_wd(&cwd)?;
    let ws = Workspace::new(&manifest, &config)?;

    let mut options = CompileOptions::new(&config, CompileMode::Build)?;
    options.build_config.release = release;
    let exec : Arc<Executor> = match deps_only {
        true => Arc::new(BuildDepsExecutor),
        false => Arc::new(DefaultExecutor),
    };
    match compile_with_exec(&ws, &options, &exec) {
        Ok(_) => Ok(()),
        Err(e) => Err(e)
    }
}

fn main() {
    let matched_args = App::new("cargo build-deps")
        .arg(Arg::with_name("build-deps"))
        .arg(Arg::with_name("release").long("release"))
        .arg(Arg::with_name("build-all").long("build-all"))
        .get_matches();
    let release = matched_args.is_present("release");
    let deps_only = !matched_args.is_present("build-all");
    let cwd = env::current_dir().unwrap();
    match build_deps(&cwd, release, deps_only) {
        Ok(_) => (),
        Err(e) => println!("{}", e)
    }
}
