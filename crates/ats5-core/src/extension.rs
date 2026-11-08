use rxsl::ruler::Program;
use rxsl::{Format, Item};

pub trait Extension: Send + Sync {
    fn name(&self) -> &str;

    fn contests(&self) -> Vec<Box<dyn Program>> {
        Vec::new()
    }

    fn formats(&self) -> Vec<Box<dyn Format>> {
        Vec::new()
    }
}

#[derive(Default)]
pub struct Builder {
    entries: Vec<(i32, Box<dyn Extension>)>,
}

impl Builder {
    pub fn extension<E: Extension + 'static>(&mut self, order: i32, make: impl FnOnce() -> E) {
        self.entries.push((order, Box::new(make())));
    }

    pub fn build(mut self) -> Registry {
        self.entries.sort_by_key(|(order, _)| *order);
        let extensions: Vec<Box<dyn Extension>> =
            self.entries.into_iter().map(|(_, e)| e).collect();
        let contests = extensions.iter().flat_map(|e| e.contests()).collect();
        let formats = extensions.iter().flat_map(|e| e.formats()).collect();
        Registry {
            extensions,
            contests,
            formats,
        }
    }
}

pub struct Registry {
    extensions: Vec<Box<dyn Extension>>,
    contests: Vec<Box<dyn Program>>,
    formats: Vec<Box<dyn Format>>,
}

impl Registry {
    pub fn extensions(&self) -> impl Iterator<Item = &dyn Extension> {
        self.extensions.iter().map(|e| e.as_ref())
    }

    pub fn contests(&self) -> impl Iterator<Item = &dyn Program> {
        self.contests.iter().map(|c| c.as_ref())
    }

    pub fn contest(&self, name: &str) -> Option<&dyn Program> {
        self.contests().find(|c| c.name() == name)
    }

    pub fn formats(&self) -> impl Iterator<Item = &dyn Format> {
        self.formats.iter().map(|f| f.as_ref())
    }

    pub fn decode(&self, file: &str, bytes: &[u8]) -> Result<Vec<Item>, String> {
        let ext = file.rsplit_once('.').map(|(_, e)| e).unwrap_or_default();
        let by_ext = |f: &&dyn Format| f.extensions().iter().any(|e| e.eq_ignore_ascii_case(ext));
        let mut order: Vec<&dyn Format> = self.formats().filter(by_ext).collect();
        order.extend(self.formats().filter(|f| !by_ext(f)));
        if let Ok(items) = rxsl::sheet::unpack(bytes) {
            if !items.is_empty() {
                return Ok(items);
            }
        }
        let mut causes = Vec::new();
        for format in order {
            match format.read(bytes) {
                Ok(items) if !items.is_empty() => return Ok(items),
                Ok(_) => causes.push(format!("[{}]: empty", format.name())),
                Err(e) => causes.push(format!("[{}]: {e}", format.name())),
            }
        }
        Err(causes.join("\n"))
    }
}

pub struct Formats;

impl Extension for Formats {
    fn name(&self) -> &str {
        "RxSL formats"
    }

    fn formats(&self) -> Vec<Box<dyn Format>> {
        rxsl::formats()
    }
}
