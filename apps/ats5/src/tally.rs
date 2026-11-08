use rxsl::ruler::{Message, Program, Section};
use rxsl::{Item, QName, Time};
use serde::Serialize;

use crate::store::{Archive, Ranking, Store, Submission};

pub type Result<T> = std::result::Result<T, String>;

fn db<T>(r: rusqlite::Result<T>) -> Result<T> {
    r.map_err(|e| e.to_string())
}

pub fn decode(data: &[u8]) -> Result<Vec<Item>> {
    let items = rxsl::sheet::unpack(data).map_err(|e| e.to_string())?;
    if items.is_empty() {
        return Err("交信記録が空です".into());
    }
    Ok(items)
}

pub fn items_of(archives: &[Archive]) -> Vec<Item> {
    archives
        .iter()
        .filter_map(|a| decode(&a.data).ok())
        .flatten()
        .collect()
}

pub fn signed(store: &Store, call: &str) -> Result<Vec<Item>> {
    let mut items = db(store.messages(call))?;
    for item in &mut items {
        let Some(dest) = item.rcvd.qxsl("call").map(str::to_ascii_uppercase) else {
            continue;
        };
        let replies = db(store.replies(call, &dest))?;
        if let Some(r) = replies.iter().find(|r| rxsl::conv::cross_check(item, r)) {
            if let Some(t) = r.attrs.qxsl("time") {
                item.attrs.set(QName::qxsl("sign"), t);
            }
        }
    }
    Ok(items)
}

pub fn update(store: &Store, contest: &dyn Program, call: &str) -> Result<()> {
    let Some(sub) = db(store.active(call))? else {
        return db(store.set_rankings(call, &[]));
    };
    let items = signed(store, call)?;
    let mut rows = Vec::new();
    for entry in db(store.entries(sub.id))? {
        let Some(section) = contest.section(&entry.sect) else {
            continue;
        };
        let summary = section.summarize(&items);
        rows.push(Ranking {
            call: call.into(),
            sect: entry.sect,
            city: entry.city,
            score: summary.score() as i64,
            total: summary.total() as i64,
        });
    }
    db(store.set_rankings(call, &rows))
}

pub fn refresh(store: &Store, contest: &dyn Program, call: &str) -> Result<()> {
    match db(store.active(call))? {
        Some(sub) => {
            let items = items_of(&db(store.archives(sub.id))?);
            db(store.set_messages(call, &items))?;
        }
        None => db(store.set_messages(call, &[]))?,
    }
    update(store, contest, call)
}

pub fn refresh_all(store: &Store, contest: &dyn Program) -> Result<()> {
    let actives = db(store.actives())?;
    for sub in &actives {
        let items = items_of(&db(store.archives(sub.id))?);
        db(store.set_messages(&sub.station.call, &items))?;
    }
    for sub in &actives {
        update(store, contest, &sub.station.call)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct Place {
    pub rank: usize,
    pub call: String,
    pub city: String,
    pub score: i64,
    pub total: i64,
    pub award: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Standing {
    pub name: String,
    pub code: String,
    pub places: Vec<Place>,
}

pub fn standings(store: &Store, contest: &dyn Program) -> Result<Vec<Standing>> {
    let all = db(store.rankings())?;
    let mut out = Vec::new();
    for section in contest.sections().iter().filter(|s| !s.is_absence()) {
        let rows: Vec<&Ranking> = all.iter().filter(|r| r.sect == section.name()).collect();
        if rows.is_empty() {
            continue;
        }
        let totals: Vec<i32> = rows.iter().map(|r| r.total as i32).collect();
        let limit = section.award_limit(&totals);
        let places = rows
            .iter()
            .map(|r| {
                let rank = rows.iter().filter(|o| o.total > r.total).count();
                Place {
                    rank: rank + 1,
                    call: r.call.clone(),
                    city: r.city.clone(),
                    score: r.score,
                    total: r.total,
                    award: rank < limit,
                }
            })
            .collect();
        out.push(Standing {
            name: section.name().into(),
            code: section.code().into(),
            places,
        });
    }
    Ok(out)
}

pub fn absent_only(store: &Store, contest: &dyn Program, sub: &Submission) -> Result<bool> {
    let entries = db(store.entries(sub.id))?;
    Ok(entries.iter().all(|e| {
        contest
            .section(&e.sect)
            .is_none_or(|s: &dyn Section| s.is_absence())
    }))
}

pub fn scoreless(store: &Store, contest: &dyn Program, call: &str) -> Result<bool> {
    let rankings = db(store.rankings())?;
    Ok(rankings.iter().filter(|r| r.call == call).any(|r| {
        contest
            .section(&r.sect)
            .is_some_and(|s| !s.is_absence() && r.score == 0)
    }))
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub time: String,
    pub call: String,
    pub band: String,
    pub mode: String,
    pub rcvd: String,
    pub sent: String,
    pub name: String,
    pub score: i32,
    pub sign: bool,
    pub text: String,
}

fn row(m: &Message) -> Row {
    let i = m.item();
    let a = |k: &str| i.attrs.qxsl(k).unwrap_or_default().to_string();
    let time = Time::parse_iso(&a("time"))
        .map(|t| {
            let t = t.at_offset(rxsl::value::local_offset());
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}",
                t.year, t.month, t.day, t.hour, t.minute
            )
        })
        .unwrap_or_default();
    Row {
        time,
        call: i.rcvd.qxsl("call").unwrap_or_default().into(),
        band: rxsl::Band::parse(&a("band"))
            .map(|b| b.label())
            .unwrap_or_default(),
        mode: a("mode"),
        rcvd: i.rcvd.qxsl("code").unwrap_or_default().into(),
        sent: i.sent.qxsl("code").unwrap_or_default().into(),
        name: a("name"),
        score: m.score(),
        sign: i.attrs.qxsl("sign").is_some(),
        text: m.text().into(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Sheet {
    pub sect: String,
    pub city: String,
    pub absence: bool,
    pub score: i32,
    pub total: i32,
    pub accepted: Vec<Row>,
    pub rejected: Vec<Row>,
}

pub fn sheets(store: &Store, contest: &dyn Program, sub: &Submission) -> Result<Vec<Sheet>> {
    let items = if sub.active {
        signed(store, &sub.station.call)?
    } else {
        items_of(&db(store.archives(sub.id))?)
    };
    let mut out = Vec::new();
    for entry in db(store.entries(sub.id))? {
        let Some(section) = contest.section(&entry.sect) else {
            continue;
        };
        let summary = section.summarize(&items);
        out.push(Sheet {
            sect: entry.sect,
            city: entry.city,
            absence: section.is_absence(),
            score: summary.score(),
            total: summary.total(),
            accepted: summary.accepted().iter().map(row).collect(),
            rejected: summary.rejected().iter().map(row).collect(),
        });
    }
    Ok(out)
}
