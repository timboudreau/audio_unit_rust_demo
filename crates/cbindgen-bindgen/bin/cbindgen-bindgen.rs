//! Builds the C and C++ bindings for our effects.

use cbindgen::Bindings;
use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process,
};

const CONFIG_FILE_NAME: &str = "cbindgen.toml";

// Crate names
const MOCK_DSP_LIB: &str = "mock_dsp_lib";

// Metadata supplied by build system
const DIR: &str = env!("CARGO_MANIFEST_DIR");
const DEST: &str = env!("OUT_DIR");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

pub fn main() {
    // For now, keep these as separate methods - we may want to post-process the headers,
    // for example, eliminating gazillions of unused ramps_n methods.
    generate_mock_effect_bindings();
}

fn generate_mock_effect_bindings() {
    generic_generate_bindings(MOCK_DSP_LIB);
}

fn generic_generate_bindings(lib: &'static str) {
    println!("\nGenerate {lib} bindings");
    let path = find_crate_dir(lib);

    log(format!("Building c-bindings for {lib} from {PKG_NAME}"));

    let bindings = run_cbindgen::<PathBuf>(&path, config_file(lib));
    let dest = output_file(lib, "h");

    log(format!(
        "Write bindings for {lib} to {}",
        dest.to_string_lossy()
    ));

    write_bindings(dest, bindings, needs_hack_includes(&path));
}

/// Allows a "pretend mode" if -p or --p is passed, for debugging
fn is_pretend() -> bool {
    for a in std::env::args() {
        match a.as_str() {
            "-p" | "--pretend" => return true,
            _ => (),
        }
    }
    false
}

fn find_crate_dir(name: &'static str) -> PathBuf {
    let mut path = PathBuf::from(DIR);
    path.pop();
    path.push(name);
    if !path.exists() {
        panic!(
            "No such create dir for '{}': {}",
            name,
            path.to_string_lossy()
        );
    }
    path
}

fn needs_hack_includes(crate_dir: &PathBuf) -> bool {
    let mut marker_file = crate_dir.clone();
    marker_file.push("hack_cbindgen_includes");
    marker_file.exists()
}

fn config_file(crate_name: &'static str) -> Option<PathBuf> {
    let mut dir = find_crate_dir(crate_name);
    let mut config_file = dir.clone();
    config_file.push(CONFIG_FILE_NAME);
    if config_file.exists() {
        println!("Use config file {}", config_file.to_string_lossy());
        Some(config_file)
    } else {
        // for, e.g. plugins/*, which nearly all use the same config file,
        // allow a single copy of it in the parent folder.
        if dir.pop() {
            config_file = dir;
            config_file.push(CONFIG_FILE_NAME);
            if config_file.exists() {
                println!("Use config file {}", config_file.to_string_lossy());
                return Some(config_file);
            }
        }
        println!("No {} found for {}", CONFIG_FILE_NAME, crate_name);
        None
    }
}

fn output_file(name: &'static str, extension: &'static str) -> PathBuf {
    let pb = PathBuf::from(name);
    let name = if pb.components().count() > 1 {
        if let Some(fname) = pb.file_name() {
            println!(
                "Strip leading components from {} to get output file name {}",
                name,
                fname.to_string_lossy()
            );
            fname.to_string_lossy().to_string()
        } else {
            panic!("Could not find a file name in {}", pb.to_string_lossy());
        }
    } else {
        name.to_string()
    };

    let mut dest = PathBuf::from(DEST);
    // we are in, e.g. build/somepackage/out...
    dest.pop();
    dest.pop();
    dest.pop();
    dest.push(&name);
    dest.set_extension(extension);
    dest
}

/// A torturous way to get cargo not to swallow the output:
fn log(what: impl Into<String>) {
    let s: String = what.into();
    println!("{}", s);
}

/// A torturous way to get cargo not to swallow the info that would lead to diagnosis.
fn log_error(what: impl Into<String>) {
    let s: String = what.into();
    println!("{}", s);
}

/// Actually run cbindgen with an optional supplied config
fn run_cbindgen<P: AsRef<Path>>(crate_dir: &PathBuf, config: Option<P>) -> Bindings {
    // Pending: Find the lock file in ../../Cargo.lock and pass it somehow - would be
    // --lockfile on the command-line.
    // Doesn't seem like it's desperately needed since everything works without it,
    // but for completeness.
    if let Some(config_path) = config {
        let pth: &Path = config_path.as_ref();
        let config = match cbindgen::Config::from_file(pth) {
            Ok(config) => config,
            Err(e) => {
                log_error(format!(
                    "Error loading cbindgen config from {}: {}",
                    pth.to_string_lossy(),
                    e
                ));
                process::exit(3);
            }
        };
        match cbindgen::generate_with_config(crate_dir, config) {
            Ok(b) => b,
            Err(e) => {
                let nm = crate_dir.file_name().unwrap().to_string_lossy();
                log_error(format!("Error in cbindgen for {nm}: {:#?}", e));
                process::exit(2);
            }
        }
    } else {
        match cbindgen::generate(crate_dir) {
            Ok(b) => b,
            Err(e) => {
                let nm = crate_dir.file_name().unwrap().to_string_lossy();
                log_error(format!("Error in cbindgen for {nm}: {:#?}", e));
                process::exit(1);
            }
        }
    }
}

fn write_bindings(dest: PathBuf, bindings: Bindings, hack_includes: bool) {
    if is_pretend() {
        println!(
            "# Pretend mode. Will not output to {}",
            dest.to_string_lossy()
        );
        bindings.write(std::io::stdout());
        println!("# ----------------------------------------------");
    } else {
        bindings.write_to_file(&dest);
        if hack_includes {
            // Sigh.  When included in a framework module map, Xcode wants headers that are
            // sort-of C++ but sort of not.
            // We still need the C convention of including the struct keyword in functions that
            // return structs, but #include statements must be angle-bracketed, not quoted, or
            // xcodebuild will have a sad.
            replace_quoted_includes_with_angle_bracketed(dest);
        }
    }
}

fn replace_quoted_includes_with_angle_bracketed(file: PathBuf) {
    let fl = std::fs::File::open(&file).expect("File was not written?");
    let lines = BufReader::new(fl).lines();
    let mut output = String::new();
    for line in lines.flatten() {
        if output.len() > 0 {
            output.push('\n');
        }
        if line.starts_with("#include ") {
            if let Some(open_quote) = line.find(|ch| ch == '"') {
                if let Some(close_quote) = line.rfind(|ch| ch == '"') {
                    let sub = &line[(open_quote + 1)..close_quote];
                    println!("Replace quoted import of '{}'", sub);

                    output.push_str("#include <");
                    output.push_str(sub);
                    output.push('>');
                }
            }
        } else {
            output.push_str(&line);
        }
    }
    output.push('\n'); // always end with a newline.
    std::fs::write(file, output).expect("Could not write quote-to-bracket converted file");
}
