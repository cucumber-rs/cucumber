use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::time::SystemTime;

use error_chain::error_chain;
use walkdir::WalkDir;

pub fn compile_test(root_dir: &str, out_dir: &str, target_triple: &str, test_text: &str) {
    handle_test(
        root_dir,
        out_dir,
        target_triple,
        test_text,
        CompileType::Check,
    );
}

pub fn run_test(root_dir: &str, out_dir: &str, target_triple: &str, test_text: &str) {
    handle_test(
        root_dir,
        out_dir,
        target_triple,
        test_text,
        CompileType::Full,
    );
}

fn handle_test(
    root_dir: &str,
    target_dir: &str,
    target_triple: &str,
    test_text: &str,
    compile_type: CompileType,
) {
    let out_dir = tempfile::Builder::new()
        .prefix("rust-skeptic")
        .tempdir()
        .unwrap();
    let testcase_path = out_dir.path().join("test.rs");
    fs::write(&testcase_path, test_text.as_bytes()).unwrap();

    // OK, here's where a bunch of magic happens using assumptions
    // about cargo internals. We are going to use rustc to compile
    // the examples, but to do that we've got to tell it where to
    // look for the rlibs with the -L flag, and what their names
    // are with the --extern flag. This is going to involve
    // parsing fingerprints out of the lockfile and looking them
    // up in the fingerprint file.

    let root_dir = PathBuf::from(root_dir);
    let mut target_dir = PathBuf::from(target_dir);
    target_dir.pop();
    target_dir.pop();
    target_dir.pop();
    let mut deps_dir = target_dir.clone();
    deps_dir.push("deps");

    let rustc = env::var("RUSTC").unwrap_or_else(|_| String::from("rustc"));
    let mut cmd = Command::new(rustc);
    cmd.arg(testcase_path)
        .arg("--verbose")
        .arg("--crate-type=bin");

    // Find the edition

    // This has to come before "-L".
    let metadata_path = root_dir.join("Cargo.toml");
    let metadata = get_cargo_meta(&metadata_path).expect("failed to read Cargo.toml");
    let edition = metadata
        .packages
        .iter()
        .map(|package| &package.edition)
        .max_by_key(|edition| u64::from_str(edition).unwrap())
        .unwrap()
        .clone();
    if edition != "2015" {
        cmd.arg(format!("--edition={}", edition));
    }

    cmd.arg("-L")
        .arg(&target_dir)
        .arg("-L")
        .arg(&deps_dir)
        .arg("--target")
        .arg(&target_triple);

    for dep in get_rlib_dependencies(root_dir, target_dir).expect("failed to read dependencies") {
        cmd.arg("--extern");
        cmd.arg(format!(
            "{}={}",
            dep.libname,
            dep.rlib.to_str().expect("filename not utf8"),
        ));
    }

    let binary_path = out_dir.path().join("out.exe");
    match compile_type {
        CompileType::Full => cmd.arg("-o").arg(&binary_path),
        CompileType::Check => cmd.arg(format!(
            "--emit=dep-info={0}.d,metadata={0}.m",
            binary_path.display()
        )),
    };

    interpret_output(cmd);

    if let CompileType::Check = compile_type {
        return;
    }

    let mut cmd = Command::new(binary_path);
    cmd.current_dir(out_dir.path());
    interpret_output(cmd);
}

fn interpret_output(mut command: Command) {
    let output = command.output().unwrap();
    print!("{}", String::from_utf8(output.stdout).unwrap());
    eprint!("{}", String::from_utf8(output.stderr).unwrap());
    if !output.status.success() {
        panic!("Command failed:\n{:?}", command);
    }
}

// Retrieve the exact dependencies for a given build by
// cross-referencing the lockfile with the fingerprint file
fn get_rlib_dependencies(root_dir: PathBuf, target_dir: PathBuf) -> Result<Vec<Fingerprint>> {
    let lock = LockedDeps::from_path(root_dir).or_else(|_| {
        // could not find Cargo.lock in $CARGO_MAINFEST_DIR
        // try relative to target_dir
        let mut root_dir = target_dir.clone();
        root_dir.pop();
        root_dir.pop();
        LockedDeps::from_path(root_dir)
    })?;

    let fingerprint_dir = target_dir.join(".fingerprint/");
    let locked_deps: HashMap<String, String> = lock.collect();
    let mut found_deps: HashMap<String, Fingerprint> = HashMap::new();

    for finger in WalkDir::new(fingerprint_dir)
        .into_iter()
        .filter_map(|v| Fingerprint::from_path(v.ok()?.path()).ok())
    {
        let locked_ver = match locked_deps.get(&finger.name()) {
            Some(ver) => ver,
            None => continue,
        };

        // TODO this should be refactored to something more readable
        match (found_deps.entry(finger.name()), finger.version()) {
            (Entry::Occupied(mut e), Some(ver)) => {
                // we find better match only if it is exact version match
                // and has fresher build time
                if *locked_ver == ver && e.get().mtime < finger.mtime {
                    e.insert(finger);
                }
            }
            (Entry::Vacant(e), ver) => {
                // we see an exact match or unversioned version
                if ver.unwrap_or_else(|| locked_ver.clone()) == *locked_ver {
                    e.insert(finger);
                }
            }
            _ => (),
        }
    }

    Ok(found_deps
        .into_iter()
        .filter_map(|(_, val)| if val.rlib.exists() { Some(val) } else { None })
        .collect())
}

// An iterator over the root dependencies in a lockfile
#[derive(Debug)]
struct LockedDeps {
    dependencies: Vec<String>,
}

fn get_cargo_meta<P: AsRef<Path> + std::convert::AsRef<std::ffi::OsStr>>(
    path: P,
) -> Result<cargo_metadata::Metadata> {
    Ok(cargo_metadata::MetadataCommand::new()
        .manifest_path(&path)
        .exec()?)
}

impl LockedDeps {
    fn from_path<P: AsRef<Path>>(path: P) -> Result<LockedDeps> {
        let path = path.as_ref().join("Cargo.toml");
        let metadata = get_cargo_meta(&path)?;
        let workspace_members = metadata.workspace_members;
        let deps = metadata
            .resolve
            .ok_or("Missing dependency metadata")?
            .nodes
            .into_iter()
            .filter(|node| workspace_members.contains(&node.id))
            .flat_map(|node| node.dependencies.into_iter())
            .chain(workspace_members.clone());

        Ok(LockedDeps {
            dependencies: deps.map(|node| node.repr).collect(),
        })
    }
}

impl Iterator for LockedDeps {
    type Item = (String, String);

    fn next(&mut self) -> Option<(String, String)> {
        let dep = self.dependencies.pop()?;
        let mut parts = dep.split_whitespace();
        let name = parts.next()?;
        let val = parts.next()?;
        Some((name.replace('-', "_"), val.to_owned()))
    }
}

#[derive(Debug)]
struct Fingerprint {
    libname: String,
    version: Option<String>, // version might not be present on path or vcs deps
    rlib: PathBuf,
    mtime: SystemTime,
}

fn guess_ext(mut path: PathBuf, exts: &[&str]) -> Result<PathBuf> {
    for ext in exts {
        path.set_extension(ext);
        if path.exists() {
            return Ok(path);
        }
    }
    Err(ErrorKind::Fingerprint.into())
}

impl Fingerprint {
    fn from_path<P: AsRef<Path>>(path: P) -> Result<Fingerprint> {
        let path = path.as_ref();

        // Use the parent path to get libname and hash, replacing - with _
        let mut captures = path
            .parent()
            .and_then(Path::file_stem)
            .and_then(OsStr::to_str)
            .ok_or(ErrorKind::Fingerprint)?
            .rsplit('-');
        let hash = captures.next().ok_or(ErrorKind::Fingerprint)?;
        let mut libname_parts = captures.collect::<Vec<_>>();
        libname_parts.reverse();
        let libname = libname_parts.join("_");

        path.extension()
            .and_then(|e| if e == "json" { Some(e) } else { None })
            .ok_or(ErrorKind::Fingerprint)?;

        let mut rlib = PathBuf::from(path);
        rlib.pop();
        rlib.pop();
        rlib.pop();
        let mut dll = rlib.clone();
        rlib.push(format!("deps/lib{}-{}", libname, hash));
        dll.push(format!("deps/{}-{}", libname, hash));
        rlib = guess_ext(rlib, &["rlib", "so", "dylib"]).or_else(|_| guess_ext(dll, &["dll"]))?;

        Ok(Fingerprint {
            libname,
            version: None,
            rlib,
            mtime: fs::metadata(path)?.modified()?,
        })
    }

    fn name(&self) -> String {
        self.libname.clone()
    }

    fn version(&self) -> Option<String> {
        self.version.clone()
    }
}

error_chain! {
    errors { Fingerprint }
    foreign_links {
        Io(std::io::Error);
        Metadata(cargo_metadata::Error);
    }
}

#[derive(Clone, Copy)]
enum CompileType {
    Full,
    Check,
}
