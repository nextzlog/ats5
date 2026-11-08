mod host;
mod rules;

pub use host::preload;
pub use rules::{builtin, load, LuaContest};

use ats5_core::Extension;
use rxsl::ruler::Program;

pub struct Rules {
    pub files: Vec<(String, String)>,
}

impl Rules {
    pub fn builtin() -> Self {
        let files = builtin()
            .iter()
            .map(|(n, s)| (n.to_string(), s.to_string()))
            .collect();
        Rules { files }
    }

    pub fn only(self, select: Option<&str>) -> Self {
        let Some(select) = select else {
            return self;
        };
        let stem = |n: &str| {
            let base = n.rsplit(['/', '\\']).next().unwrap_or(n);
            base.trim_end_matches(".lua").to_ascii_lowercase()
        };
        let want = select.to_ascii_lowercase();
        let hit: Vec<(String, String)> = self
            .files
            .iter()
            .filter(|(n, _)| stem(n) == want)
            .cloned()
            .collect();
        match hit.is_empty() {
            true => self,
            false => Rules { files: hit },
        }
    }
}

impl Extension for Rules {
    fn name(&self) -> &str {
        "Luau rules"
    }

    fn contests(&self) -> Vec<Box<dyn Program>> {
        let mut out: Vec<Box<dyn Program>> = Vec::new();
        for (name, source) in &self.files {
            match load(name, source) {
                Ok(c) => out.push(Box::new(c)),
                Err(e) => eprintln!("cannot load {name}: {e}"),
            }
        }
        out
    }
}
