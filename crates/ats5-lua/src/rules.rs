use mlua::{Function, Lua, LuaOptions, StdLib, Table, Value};
use rxsl::local::{cities, CityItem};
use rxsl::ruler::{Contest, Element, Message, Program, Section, Summary};
use rxsl::{Date, Item};

use crate::host;

const REMOVED: &[&str] = &[
    "print",
    "loadstring",
    "load",
    "require",
    "dofile",
    "loadfile",
    "getfenv",
    "setfenv",
    "collectgarbage",
    "gcinfo",
    "newproxy",
];

const MEMORY: usize = 256 * 1024 * 1024;

const RANKS: usize = 8;

const PRELUDE: &str = include_str!("prelude.lua");

include!(concat!(env!("OUT_DIR"), "/rules.rs"));

pub fn builtin() -> &'static [(&'static str, &'static str)] {
    BUILTIN
}

fn bundled() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        for (url, bytes) in DATS {
            host::fallback(url, bytes);
        }
    });
}

fn text(e: mlua::Error) -> String {
    if std::env::var("ATS5_LUA_DEBUG").is_ok() {
        eprintln!("{e:?}");
    }
    match e {
        mlua::Error::CallbackError { cause, .. } => text((*cause).clone()),
        mlua::Error::RuntimeError(m) => m.lines().next().unwrap_or("").to_string(),
        e => e.to_string().lines().next().unwrap_or("").to_string(),
    }
}

fn sandbox() -> mlua::Result<Lua> {
    let libs = StdLib::STRING | StdLib::TABLE | StdLib::MATH | StdLib::UTF8 | StdLib::BIT;
    let lua = Lua::new_with(libs, LuaOptions::default())?;
    let g = lua.globals();
    for name in REMOVED {
        g.raw_set(*name, Value::Nil)?;
    }
    let math: Table = g.get("math")?;
    math.raw_set("random", Value::Nil)?;
    math.raw_set("randomseed", Value::Nil)?;
    lua.set_memory_limit(MEMORY)?;
    host::install(&lua)?;
    Ok(lua)
}

fn element(v: Value) -> Element {
    match v {
        Value::Nil => Element::Null,
        Value::Boolean(b) => Element::Value(b.to_string()),
        Value::Integer(n) => Element::Value(n.to_string()),
        Value::Number(n) => Element::Value(n.to_string()),
        Value::String(s) => Element::Value(s.to_string_lossy()),
        Value::Table(t) => {
            let n = t.raw_len();
            Element::list((1..=n).map(|i| element(t.raw_get(i).unwrap_or(Value::Nil))))
        }
        other => Element::Value(format!("{other:?}")),
    }
}

struct LuaSection {
    lua: Lua,
    name: String,
    code: String,
    absence: bool,
    cities: Vec<CityItem>,
    verify: Option<Function>,
    unique: Option<Function>,
    entity: Option<Function>,
    result: Option<Function>,
    award: Option<Function>,
}

impl LuaSection {
    fn call(&self, f: &Function, item: &Item) -> mlua::Result<Value> {
        f.call(host::item(&self.lua, item)?)
    }

    fn band_code(item: &Item) -> (Element, Element) {
        let band = item.attrs.qxsl("band").map(String::from);
        let code = item.rcvd.qxsl("code").map(String::from);
        (band.into(), code.into())
    }
}

impl Section for LuaSection {
    fn name(&self) -> &str {
        &self.name
    }

    fn code(&self) -> &str {
        &self.code
    }

    fn is_absence(&self) -> bool {
        self.absence
    }

    fn verify(&self, item: &Item) -> Message {
        let Some(f) = self.verify.as_ref().filter(|_| !self.absence) else {
            return Message::failure(item.clone(), "N/A");
        };
        let out: mlua::Result<mlua::MultiValue> = f.call(host::item(&self.lua, item));
        match out {
            Err(e) => Message::failure(item.clone(), text(e)),
            Ok(values) => {
                let mut values = values.into_iter();
                match (values.next(), values.next()) {
                    (Some(Value::Integer(n)), _) => Message::success(item.clone(), n as i32),
                    (Some(Value::Number(n)), _) => Message::success(item.clone(), n as i32),
                    (_, Some(reason)) => {
                        Message::failure(item.clone(), element(reason).get(0).text())
                    }
                    _ => Message::failure(item.clone(), "invalid"),
                }
            }
        }
    }

    fn unique(&self, item: &Item) -> Element {
        match &self.unique {
            Some(f) => self.call(f, item).map(element).unwrap_or(Element::Null),
            None => {
                let call = item.rcvd.qxsl("call").map(String::from);
                let (band, _) = Self::band_code(item);
                Element::list([call.into(), band])
            }
        }
    }

    fn entity(&self, item: &Item) -> Element {
        match &self.entity {
            Some(f) => self.call(f, item).map(element).unwrap_or(Element::Null),
            None => {
                let (band, code) = Self::band_code(item);
                Element::list([Element::list([band, code])])
            }
        }
    }

    fn result(&self, summary: &Summary) -> i32 {
        let keys: Vec<usize> = (0..RANKS).map(|r| summary.keys(r).len()).collect();
        let Some(f) = &self.result else {
            return summary.score() * keys[0] as i32;
        };
        let run = || -> mlua::Result<i32> {
            let t = self.lua.create_table()?;
            t.set("score", summary.score())?;
            t.set("keys", self.lua.create_sequence_from(keys.iter().copied())?)?;
            t.set("accepted", summary.accepted().len())?;
            f.call(t)
        };
        run().unwrap_or(0)
    }

    fn city_list(&self) -> Vec<CityItem> {
        self.cities.clone()
    }

    fn award_limit(&self, scores: &[i32]) -> usize {
        let default = ((0.1 * scores.len() as f64).ceil() as usize).min(7);
        let Some(f) = &self.award else {
            return default;
        };
        let n: mlua::Result<usize> = f.call(scores.len());
        n.unwrap_or(default)
    }
}

trait Text {
    fn text(&self) -> String;
}

impl Text for Element {
    fn text(&self) -> String {
        match self {
            Element::Value(v) => v.clone(),
            Element::Null => "invalid".into(),
            Element::List(l) => l.iter().map(|e| e.text()).collect::<Vec<_>>().join(" "),
        }
    }
}

pub struct LuaContest {
    name: String,
    host: String,
    mail: String,
    link: String,
    help: String,
    sections: Vec<Box<dyn Section>>,
    start: Function,
    last: Option<Function>,
    deadline: Function,
    conflict: Option<Function>,
    limit: Option<Function>,
    always: bool,
}

fn section(lua: &Lua, t: Table) -> mlua::Result<LuaSection> {
    let name: String = t.get("name")?;
    let code: String = t.get("code")?;
    let absence = t.get::<Option<bool>>("absence")?.unwrap_or(false);
    let list: Option<Table> = t.get("cities")?;
    let mut items = Vec::new();
    if let Some(list) = list {
        for entry in list.sequence_values::<Value>() {
            match entry? {
                Value::String(code) => {
                    let code = code.to_string_lossy();
                    let city = cities().by_code(&code).cloned();
                    let missing = || mlua::Error::runtime(format!("{name}: unknown city {code}"));
                    items.push(city.ok_or_else(missing)?);
                }
                Value::Table(t) => {
                    let code: String = t.get("code")?;
                    let label: String = t.get("name")?;
                    let mut path: Vec<String> =
                        label.split_whitespace().map(String::from).collect();
                    path.push(code);
                    items.push(CityItem::new(path));
                }
                _ => return Err(mlua::Error::runtime(format!("{name}: malformed city"))),
            }
        }
    }
    let verify: Option<Function> = t.get("verify")?;
    if verify.is_none() && !absence {
        return Err(mlua::Error::runtime(format!("{name}: verify is missing")));
    }
    Ok(LuaSection {
        lua: lua.clone(),
        name,
        code,
        absence,
        cities: items,
        verify,
        unique: t.get("unique")?,
        entity: t.get("entity")?,
        result: t.get("result")?,
        award: t.get("award")?,
    })
}

pub fn load(name: &str, source: &str) -> Result<LuaContest, String> {
    bundled();
    let build = || -> mlua::Result<LuaContest> {
        let lua = sandbox()?;
        lua.load(PRELUDE).set_name("prelude").exec()?;
        let t: Table = lua.load(source).set_name(name).eval()?;
        let mut sections: Vec<Box<dyn Section>> = Vec::new();
        let list: Table = t.get("sections")?;
        for s in list.sequence_values::<Table>() {
            sections.push(Box::new(section(&lua, s?)?));
        }
        Ok(LuaContest {
            name: t.get("name")?,
            host: t.get::<Option<String>>("host")?.unwrap_or_default(),
            mail: t.get::<Option<String>>("mail")?.unwrap_or_default(),
            link: t.get::<Option<String>>("link")?.unwrap_or_default(),
            help: t.get::<Option<String>>("help")?.unwrap_or_default(),
            sections,
            start: t.get("start")?,
            last: t.get("final")?,
            deadline: t.get("deadline")?,
            conflict: t.get("conflict")?,
            limit: t.get("limit")?,
            always: t.get::<Option<bool>>("finish")?.unwrap_or(false),
        })
    };
    build().map_err(text)
}

impl LuaContest {
    fn date(&self, f: &Function, year: i32) -> Date {
        let d: mlua::Result<String> = f.call(year);
        d.and_then(|d| host::parse_date(&d))
            .unwrap_or(Date::new(year, 1, 1))
    }

    fn reasons(&self, entries: &[&dyn Section]) -> Vec<String> {
        let Some(f) = &self.conflict else {
            return Vec::new();
        };
        let names: Vec<String> = entries.iter().map(|s| s.name().to_string()).collect();
        let out: mlua::Result<Vec<String>> = f.call(names);
        out.unwrap_or_else(|e| vec![text(e)])
    }
}

impl Contest for LuaContest {
    fn name(&self) -> &str {
        &self.name
    }

    fn sections(&self) -> &[Box<dyn Section>] {
        &self.sections
    }
}

impl Program for LuaContest {
    fn host(&self) -> &str {
        &self.host
    }

    fn mail(&self) -> &str {
        &self.mail
    }

    fn link(&self) -> &str {
        &self.link
    }

    fn help(&self) -> &str {
        &self.help
    }

    fn start_day(&self, year: i32) -> Date {
        self.date(&self.start, year)
    }

    fn final_day(&self, year: i32) -> Date {
        match &self.last {
            Some(f) => self.date(f, year),
            None => self.start_day(year),
        }
    }

    fn dead_line(&self, year: i32) -> Date {
        self.date(&self.deadline, year)
    }

    fn limit_multiple_entry(&self, code: &str) -> usize {
        let n: Option<mlua::Result<usize>> = self.limit.as_ref().map(|f| f.call(code.to_string()));
        n.and_then(Result::ok).unwrap_or(1)
    }

    fn finish_at(&self, offset: i32) -> bool {
        if self.always {
            return true;
        }
        let t = rxsl::Time::now(offset);
        self.dead_line(self.year()) < Date::new(t.year, t.month, t.day)
    }

    fn conflict(&self, entries: &[&dyn Section]) -> bool {
        !self.reasons(entries).is_empty()
    }

    fn explain(&self, entries: &[&dyn Section]) -> Vec<String> {
        self.reasons(entries)
    }
}
