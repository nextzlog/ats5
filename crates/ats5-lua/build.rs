use std::path::PathBuf;

fn urls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("ats.dat(\"") {
        let tail = &rest[at + 9..];
        let Some(end) = tail.find('"') else {
            break;
        };
        out.push(tail[..end].to_string());
        rest = &tail[end..];
    }
    out
}

fn main() {
    let rules = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../rules");
    let root = rules.join("lua");
    let data = rules.join("dat");
    println!("cargo:rerun-if-changed={}", data.display());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "lua"))
        .collect();
    files.sort();
    let mut rules = String::from("pub const BUILTIN: &[(&str, &str)] = &[\n");
    let mut dats = String::from("pub const DATS: &[(&str, &[u8])] = &[\n");
    let mut seen = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let full = path.canonicalize().unwrap().display().to_string();
        rules.push_str(&format!("    ({name:?}, include_str!({full:?})),\n"));
        for url in urls(&std::fs::read_to_string(&path).unwrap()) {
            if seen.contains(&url) {
                continue;
            }
            seen.push(url.clone());
            let name = url.rsplit('/').next().unwrap_or_default();
            let dat = data.join(name);
            match dat.canonicalize() {
                Ok(dat) => {
                    let dat = dat.display().to_string();
                    dats.push_str(&format!("    ({url:?}, include_bytes!({dat:?})),\n"));
                }
                Err(_) => {
                    println!("cargo:warning=rules/{name} is missing; {url} is fetched at startup")
                }
            }
        }
    }
    rules.push_str("];\n");
    dats.push_str("];\n");
    std::fs::write(out.join("rules.rs"), rules + &dats).unwrap();
}
