use std::path::PathBuf;

fn page(body: &str) -> String {
    body.replace("<html lang=\"ja\">", "<html lang=\"ja\" data-site=\"1\">")
        .replace("href=\"/style.css\"", "href=\"style.css\"")
        .replace("src=\"/app.js\"", "src=\"site.js\"")
        .replace("href=\"/\"", "href=\"#/\"")
}

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "site".into()));
    for (name, body) in ats5::web::FILES {
        let path = out.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        match *name {
            "index.html" => std::fs::write(&path, page(body)).unwrap(),
            _ => std::fs::write(&path, body).unwrap(),
        }
    }
}
