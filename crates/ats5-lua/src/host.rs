use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use mlua::{Lua, Result, Table, Value};
use rxsl::local::{areas, cities, CityItem};
use rxsl::{Date, Item, QName, Time, Weekday};

fn store() -> &'static Mutex<HashMap<String, String>> {
    static DAT: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    DAT.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn fallback(url: &str, bytes: &[u8]) {
    let mut map = store().lock().unwrap_or_else(|e| e.into_inner());
    if !map.contains_key(url) {
        map.insert(url.to_string(), rxsl::text::decode_sjis(bytes));
    }
}

pub fn preload(url: &str, bytes: &[u8]) {
    let text = rxsl::text::decode_sjis(bytes);
    store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(url.to_string(), text);
}

#[cfg(not(target_family = "wasm"))]
fn download(url: &str) -> std::result::Result<Vec<u8>, String> {
    if url.starts_with("http://") || url.starts_with("https://") {
        let mut res = ureq::get(url).call().map_err(|e| format!("{url}: {e}"))?;
        res.body_mut()
            .read_to_vec()
            .map_err(|e| format!("{url}: {e}"))
    } else {
        std::fs::read(url).map_err(|e| format!("{url}: {e}"))
    }
}

#[cfg(target_family = "wasm")]
fn download(url: &str) -> std::result::Result<Vec<u8>, String> {
    Err(format!("{url}: not preloaded"))
}

fn fetch(url: &str) -> std::result::Result<String, String> {
    if let Some(text) = store().lock().unwrap_or_else(|e| e.into_inner()).get(url) {
        return Ok(text.clone());
    }
    let bytes = download(url)?;
    preload(url, &bytes);
    Ok(rxsl::text::decode_sjis(&bytes))
}

pub fn dat(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter(|l| !l.starts_with("end of file"))
        .filter_map(|l| {
            let (code, name) = l.trim().split_once(char::is_whitespace)?;
            let code = Some(code).filter(|c| c.bytes().all(|b| b.is_ascii_alphanumeric()))?;
            Some((code.to_string(), name.trim().to_string()))
        })
        .filter(|(_, n)| !n.is_empty())
        .collect()
}

fn call_area(city: &CityItem) -> Option<String> {
    let pref = cities().by_name(&city.area())?;
    areas().by_code(pref.code()).map(CityItem::area)
}

fn city(lua: &Lua, c: &CityItem) -> Result<Table> {
    let t = lua.create_table()?;
    t.set("code", c.code())?;
    t.set("name", c.name())?;
    t.set("pref", c.area())?;
    t.set("area", call_area(c))?;
    Ok(t)
}

fn date(text: &str) -> Result<Date> {
    let mut p = text.split('-').map(|x| x.parse::<i64>().ok());
    match (p.next().flatten(), p.next().flatten(), p.next().flatten()) {
        (Some(y), Some(m), Some(d)) => Ok(Date::new(y as i32, m as u32, d as u32)),
        _ => Err(mlua::Error::runtime(format!("malformed date: {text}"))),
    }
}

fn weekday(n: i64) -> Result<Weekday> {
    use Weekday::*;
    Ok(match n {
        1 => Monday,
        2 => Tuesday,
        3 => Wednesday,
        4 => Thursday,
        5 => Friday,
        6 => Saturday,
        7 => Sunday,
        _ => return Err(mlua::Error::runtime(format!("weekday must be 1 to 7: {n}"))),
    })
}

pub fn install(lua: &Lua) -> Result<()> {
    let ats = lua.create_table()?;
    ats.set(
        "city",
        lua.create_function(|lua, code: String| match cities().by_code(&code) {
            Some(c) => Ok(Value::Table(city(lua, c)?)),
            None => Ok(Value::Nil),
        })?,
    )?;
    ats.set(
        "cities",
        lua.create_function(|lua, ()| {
            let all = lua.create_table()?;
            for (i, c) in cities().to_list().iter().enumerate() {
                all.raw_set(i + 1, city(lua, c)?)?;
            }
            Ok(all)
        })?,
    )?;
    ats.set(
        "hour",
        lua.create_function(|_, (time, offset): (String, i32)| {
            Ok(Time::parse_iso(&time)
                .ok()
                .map(|t| t.at_offset(offset).hour))
        })?,
    )?;
    ats.set(
        "morse",
        lua.create_function(|_, mode: String| Ok(rxsl::conv::is_morse(&mode)))?,
    )?;
    ats.set(
        "phone",
        lua.create_function(|_, mode: String| Ok(rxsl::conv::is_phone(&mode)))?,
    )?;
    ats.set(
        "digit",
        lua.create_function(|_, mode: String| Ok(rxsl::conv::is_digit(&mode)))?,
    )?;
    ats.set(
        "call",
        lua.create_function(|_, text: String| {
            let call = rxsl::draft::Call::new(&text);
            Ok((call.valid(), call.strip().to_string()))
        })?,
    )?;
    ats.set(
        "nth",
        lua.create_function(|_, (y, m, n, w): (i32, u32, u32, i64)| {
            Ok(Date::nth_weekday(y, m, n, weekday(w)?).to_string())
        })?,
    )?;
    ats.set(
        "date",
        lua.create_function(|_, (y, m, d): (i32, u32, u32)| Ok(Date::new(y, m, d).to_string()))?,
    )?;
    ats.set(
        "days",
        lua.create_function(|_, (d, n): (String, i64)| Ok(date(&d)?.plus_days(n).to_string()))?,
    )?;
    ats.set(
        "months",
        lua.create_function(|_, (d, n): (String, u32)| Ok(date(&d)?.plus_months(n).to_string()))?,
    )?;
    ats.set(
        "dat",
        lua.create_function(|lua, url: String| {
            let text = fetch(&url).map_err(mlua::Error::runtime)?;
            let list = lua.create_table()?;
            for (i, (code, name)) in dat(&text).into_iter().enumerate() {
                let t = lua.create_table()?;
                t.set("code", code)?;
                t.set("name", name)?;
                list.raw_set(i + 1, t)?;
            }
            Ok(list)
        })?,
    )?;
    ats.set(
        "local_date",
        lua.create_function(|_, (time, offset): (String, i32)| {
            Ok(Time::parse_iso(&time).ok().map(|t| {
                let t = t.at_offset(offset);
                format!("{:04}-{:02}-{:02}", t.year, t.month, t.day)
            }))
        })?,
    )?;
    lua.globals().set("ats", ats)?;
    Ok(())
}

pub fn parse_date(text: &str) -> Result<Date> {
    date(text)
}

fn node(lua: &Lua, n: &rxsl::Node) -> Result<Table> {
    let t = lua.create_table()?;
    for (k, v) in n.iter() {
        if k.space == rxsl::QXSL {
            t.set(k.local.as_str(), v)?;
        }
    }
    Ok(t)
}

pub fn item(lua: &Lua, item: &Item) -> Result<Table> {
    let t = node(lua, &item.attrs)?;
    t.set("rcvd", node(lua, &item.rcvd)?)?;
    t.set("sent", node(lua, &item.sent)?)?;
    let q = |k: &str| item.attrs.get(&QName::qxsl(k));
    t.set("call", item.rcvd.qxsl("call").or_else(|| q("call")))?;
    t.set("khz", q("band").and_then(|b| b.parse::<i64>().ok()))?;
    t.set("code", item.rcvd.qxsl("code"))?;
    t.set_readonly(true);
    Ok(t)
}
