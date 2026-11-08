use std::path::{Path, PathBuf};
use std::process::Command;

use oxc::allocator::Allocator;
use oxc::codegen::Codegen;
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;
use oxc::transformer::{TransformOptions, Transformer};

fn transpile(path: &Path) -> String {
    let source = std::fs::read_to_string(path).unwrap();
    let allocator = Allocator::default();
    let kind = SourceType::from_path(path).unwrap();
    let parsed = Parser::new(&allocator, &source, kind).parse();
    if !parsed.diagnostics.is_empty() {
        panic!("{}: {:?}", path.display(), parsed.diagnostics);
    }
    let mut program = parsed.program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let options = TransformOptions::from_target("es2022").unwrap();
    let done =
        Transformer::new(&allocator, path, &options).build_with_scoping(scoping, &mut program);
    if !done.diagnostics.is_empty() {
        panic!("{}: {:?}", path.display(), done.diagnostics);
    }
    Codegen::new().build(&program).code
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn web(out: &Path) {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("web");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut sources = Vec::new();
    walk(&root, &mut sources);
    let dest = out.join("web");
    let mut table = String::from("pub const FILES: &[(&str, &str)] = &[\n");
    for path in sources {
        let rel = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let (name, body) = match path.extension().and_then(|e| e.to_str()) {
            Some("ts") => (
                rel.trim_end_matches(".ts").to_string() + ".js",
                transpile(&path),
            ),
            Some("html" | "css") => (rel.clone(), std::fs::read_to_string(&path).unwrap()),
            _ => continue,
        };
        let target = dest.join(&name);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, body).unwrap();
        table.push_str(&format!(
            "    ({name:?}, include_str!({:?})),\n",
            target.display().to_string()
        ));
    }
    table.push_str("];\n");
    std::fs::write(out.join("web.rs"), table).unwrap();
}

fn wasm(out: &Path) {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let dest = out.join("ats5.wasm");
    for dir in ["crates", "Cargo.toml"] {
        println!("cargo:rerun-if-changed={}", root.join(dir).display());
    }
    println!("cargo:rerun-if-env-changed=ATS5_SKIP_WASM");
    if std::env::var("ATS5_SKIP_WASM").is_ok() {
        std::fs::write(&dest, []).unwrap();
        return;
    }
    let target = out.join("wasm");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(&root)
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
            "-p",
            "ats5-wasm",
        ])
        .env("CARGO_TARGET_DIR", &target)
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_BUILD_TARGET")
        .status();
    let built = target.join("wasm32-unknown-unknown/release/ats5_wasm.wasm");
    match status {
        Ok(s) if s.success() && built.exists() => {
            std::fs::copy(&built, &dest).unwrap();
        }
        _ => {
            println!("cargo:warning=the browser check is disabled: install the wasm32-unknown-unknown target");
            std::fs::write(&dest, []).unwrap();
        }
    }
}

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    web(&out);
    wasm(&out);
}
