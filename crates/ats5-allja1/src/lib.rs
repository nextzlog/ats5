use ats5_core::Extension;
use ats5_japan as ja;
use rxsl::local::CityItem;
use rxsl::ruler::{Absence, Contest, Element, Message, Program, Section, Summary};
use rxsl::{Date, Item, Weekday};

const B19: u64 = 1900;
const B35: u64 = 3500;
const B70: u64 = 7000;
const B14: u64 = 14000;
const B21: u64 = 21000;
const B28: u64 = 28000;
const B50: u64 = 50000;

const HIGH: [u64; 4] = [B14, B21, B28, B50];
const LOW: [u64; 3] = [B19, B35, B70];

const SIN_HB: &str = "個人部門 09:00-12:00";
const SIN_LB: &str = "個人部門 16:00-20:00";
const SIN_DG: &str = "個人部門 13:00-15:00";
const MUL_AB: &str = "団体部門 アナログ";
const MUL_DG: &str = "団体部門 デジタル";
const MIX_JS: &str = "総合部門";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Morse,
    Analog,
    Digital,
    Any,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hours {
    High,
    Low,
    Digital,
    Analog,
    Total,
}

struct Rule {
    name: String,
    code: &'static str,
    inside: bool,
    multi: bool,
    hours: Hours,
    mode: Mode,
    band: Option<u64>,
}

fn high(it: &Item) -> bool {
    let hour = ja::jst_hour(it).unwrap_or(99);
    (9..=11).contains(&hour) && analog(it) && ja::khz(it).is_some_and(|b| HIGH.contains(&b))
}

fn low(it: &Item) -> bool {
    let hour = ja::jst_hour(it).unwrap_or(99);
    (16..=19).contains(&hour) && analog(it) && ja::khz(it).is_some_and(|b| LOW.contains(&b))
}

fn digital(it: &Item) -> bool {
    let hour = ja::jst_hour(it).unwrap_or(99);
    (13..=14).contains(&hour) && ja::digit(it) && ja::khz(it) == Some(B70)
}

fn analog(it: &Item) -> bool {
    ja::morse(it) || ja::phone(it)
}

fn exists(it: &Item) -> bool {
    ja::city(it).is_some()
}

fn area(it: &Item, area: &str) -> bool {
    ja::city(it).and_then(ja::call_area).as_deref() == Some(area)
}

fn inside(it: &Item) -> bool {
    let code = ja::code(it);
    if area(it, "1") {
        exists(it) && ja::is_city(code)
    } else if area(it, "8") {
        exists(it) && ja::is_subprefecture(code)
    } else {
        exists(it) && ja::is_prefecture(code)
    }
}

fn outside(it: &Item) -> bool {
    exists(it) && area(it, "1") && ja::is_city(ja::code(it))
}

impl Rule {
    fn check(&self, it: &Item) -> Result<(), &'static str> {
        if self.multi && ja::name(it).is_none() {
            return Err("MulOp?");
        }
        if self.inside && !inside(it) {
            return Err("内?");
        }
        if !self.inside && !outside(it) {
            return Err("外?");
        }
        let hours = match self.hours {
            Hours::High => high(it),
            Hours::Low => low(it),
            Hours::Digital => digital(it),
            Hours::Analog => high(it) || low(it),
            Hours::Total => high(it) || low(it) || digital(it),
        };
        if !hours {
            return Err(match self.hours {
                Hours::High => "tHI?",
                Hours::Low => "tLO?",
                Hours::Digital => "tDG?",
                Hours::Analog => "tAN?",
                Hours::Total => "tJS?",
            });
        }
        let (ok, label) = match self.mode {
            Mode::Morse => (ja::morse(it), "MORSE?"),
            Mode::Analog => (analog(it), "CW/PH?"),
            Mode::Digital => (ja::digit(it), "DIGIT?"),
            Mode::Any => (analog(it) || ja::digit(it), "AN/DG?"),
        };
        if !ok {
            return Err(label);
        }
        match self.band {
            Some(b) if ja::khz(it) != Some(b) => Err("BAND?"),
            _ => Ok(()),
        }
    }
}

impl Section for Rule {
    fn name(&self) -> &str {
        &self.name
    }

    fn code(&self) -> &str {
        self.code
    }

    fn city_list(&self) -> Vec<CityItem> {
        let list: Vec<&CityItem> = if self.inside {
            ja::cities_in("1").collect()
        } else {
            ja::cities_out("1").collect()
        };
        list.into_iter()
            .filter(|p| !ja::digits(p.code(), 2, 3))
            .cloned()
            .collect()
    }

    fn verify(&self, item: &Item) -> Message {
        match self.check(item) {
            Ok(()) => Message::success(item.clone(), 1),
            Err(text) => Message::failure(item.clone(), text),
        }
    }

    fn unique(&self, item: &Item) -> Element {
        let mode = if ja::morse(item) {
            "1"
        } else if ja::phone(item) {
            "2"
        } else if ja::digit(item) {
            "3"
        } else {
            ""
        };
        let call = ja::call(item).unwrap_or_default();
        let band = ja::band(item).unwrap_or_default();
        Element::list([call.into(), band.into(), mode.into()])
    }

    fn entity(&self, item: &Item) -> Element {
        let band = ja::band(item).unwrap_or_default();
        let mult = Element::list([band.into(), ja::code(item).into()]);
        let name = if self.multi { ja::name(item) } else { None };
        Element::list([mult, name.into()])
    }

    fn result(&self, summary: &Summary) -> i32 {
        let mults = summary.keys(0).len() as i32;
        let names = summary.keys(1).len().max(1) as i32;
        (summary.score() * mults + names - 1) / names
    }
}

pub struct Allja1 {
    sections: Vec<Box<dyn Section>>,
}

impl Default for Allja1 {
    fn default() -> Self {
        Allja1 {
            sections: sections(),
        }
    }
}

fn label(inside: bool, multi: bool, kind: &str, band: &str) -> String {
    let area = if inside {
        "1エリア内"
    } else {
        "1エリア外"
    };
    let team = if multi { "団体" } else { "個人" };
    format!("{area} {team} {kind} {band}部門")
}

fn sections() -> Vec<Box<dyn Section>> {
    let mut all: Vec<Box<dyn Section>> = Vec::new();
    for code in [SIN_HB, SIN_LB, SIN_DG, MUL_AB, MUL_DG, MIX_JS] {
        let name = format!("{code} 不参加 (チェックログ)");
        all.push(Box::new(Absence::new(name, code)));
    }
    let mut add = |inside, multi, kind, label_band: &str, code, hours, mode, band| {
        all.push(Box::new(Rule {
            name: label(inside, multi, kind, label_band),
            code,
            inside,
            multi,
            hours,
            mode,
            band,
        }));
    };
    let lows = [
        ("1.9MHz", Some(B19)),
        ("3.5MHz", Some(B35)),
        ("7MHz", Some(B70)),
        ("1.9-7MHz", None),
    ];
    let highs = [
        ("14MHz", Some(B14)),
        ("21MHz", Some(B21)),
        ("28MHz", Some(B28)),
        ("50MHz", Some(B50)),
        ("14-50MHz", None),
    ];
    let kinds = [("電信限定", Mode::Morse), ("電信電話", Mode::Analog)];
    for inside in [true, false] {
        for (kind, mode) in kinds {
            for (text, band) in lows {
                add(inside, false, kind, text, SIN_LB, Hours::Low, mode, band);
            }
        }
    }
    for inside in [true, false] {
        for (kind, mode) in kinds {
            for (text, band) in highs {
                add(inside, false, kind, text, SIN_HB, Hours::High, mode, band);
            }
        }
    }
    for inside in [true, false] {
        for (kind, mode) in kinds {
            add(inside, true, kind, "", MUL_AB, Hours::Analog, mode, None);
        }
    }
    for inside in [true, false] {
        add(
            inside,
            false,
            "デジタル",
            "",
            SIN_DG,
            Hours::Digital,
            Mode::Digital,
            None,
        );
    }
    for inside in [true, false] {
        add(
            inside,
            true,
            "デジタル",
            "",
            MUL_DG,
            Hours::Digital,
            Mode::Digital,
            None,
        );
    }
    for inside in [true, false] {
        add(
            inside,
            false,
            "総合",
            "",
            MIX_JS,
            Hours::Total,
            Mode::Any,
            None,
        );
    }
    for inside in [true, false] {
        add(
            inside,
            true,
            "総合",
            "",
            MIX_JS,
            Hours::Total,
            Mode::Any,
            None,
        );
    }
    all
}

impl Contest for Allja1 {
    fn name(&self) -> &str {
        "ALLJA1"
    }

    fn sections(&self) -> &[Box<dyn Section>] {
        &self.sections
    }
}

impl Program for Allja1 {
    fn host(&self) -> &str {
        "東大無線部"
    }

    fn mail(&self) -> &str {
        "allja1.info@gmail.com"
    }

    fn link(&self) -> &str {
        "https://ja1zlo.u-tokyo.org/allja1"
    }

    fn help(&self) -> &str {
        include_str!("help.md")
    }

    fn start_day(&self, year: i32) -> Date {
        Date::nth_weekday(year, 6, 4, Weekday::Saturday)
    }

    fn final_day(&self, year: i32) -> Date {
        self.start_day(year)
    }

    fn dead_line(&self, year: i32) -> Date {
        self.start_day(year).plus_days(21)
    }

    fn conflict(&self, entries: &[&dyn Section]) -> bool {
        !problems(entries).is_empty()
    }

    fn explain(&self, entries: &[&dyn Section]) -> Vec<String> {
        problems(entries).into_iter().map(String::from).collect()
    }
}

fn problems(entries: &[&dyn Section]) -> Vec<&'static str> {
    let has = |s: &dyn Section, p: &str| s.name().contains(p);
    let mut out = Vec::new();
    let multi = entries.iter().filter(|s| has(**s, "団体")).count();
    if multi > 0 && multi < entries.len() {
        out.push("個人部門と団体部門には、同時に参加できません。");
    }
    let total = entries.iter().any(|s| has(*s, "総合"));
    if !entries.is_empty() && !total {
        out.push(
            "総合部門を選んでください。他の部門に参加する場合、総合部門は不参加にできません。",
        );
    }
    let inside = entries.iter().any(|s| has(*s, "1エリア内"));
    let inside_total = entries
        .iter()
        .any(|s| has(*s, "1エリア内") && has(*s, "総合"));
    if inside && total && !inside_total {
        out.push("1エリア内の部門に参加する場合は、総合部門も1エリア内を選んでください。");
    }
    out
}

pub struct Rules;

impl Extension for Rules {
    fn name(&self) -> &str {
        "ALLJA1"
    }

    fn contests(&self) -> Vec<Box<dyn Program>> {
        vec![Box::new(Allja1::default())]
    }
}
