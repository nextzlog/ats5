#![allow(clippy::missing_safety_doc)]

use ats5_core::{Builder, Formats, Registry};
use rxsl::ruler::{Message, Program};
use serde::{Deserialize, Serialize};

pub fn register(b: &mut Builder) {
    b.extension(0, || Formats);
    b.extension(10, || ats5_allja1::Rules);
}

pub fn registry() -> Registry {
    let mut b = Builder::default();
    register(&mut b);
    b.build()
}

#[derive(Deserialize)]
pub struct Request {
    pub contest: String,
    pub sections: Vec<String>,
}

#[derive(Serialize)]
pub struct Contact {
    pub time: String,
    pub call: String,
    pub band: String,
    pub mode: String,
}

#[derive(Serialize)]
pub struct Preview {
    pub sect: String,
    pub valid: usize,
    pub invalid: usize,
    pub score: i32,
    pub total: i32,
    pub rejected: Vec<Contact>,
}

#[derive(Serialize)]
pub struct Report {
    pub items: usize,
    pub error: Option<String>,
    pub sections: Vec<Preview>,
    pub conflict: bool,
    pub reasons: Vec<String>,
}

fn contact(m: &Message) -> Contact {
    let i = m.item();
    let a = |k: &str| i.attrs.qxsl(k).unwrap_or_default().to_string();
    let time = rxsl::Time::parse_iso(&a("time"))
        .map(|t| t.at_offset(rxsl::value::local_offset()))
        .map(|t| format!("{:02}/{:02} {:02}:{:02}", t.month, t.day, t.hour, t.minute))
        .unwrap_or_default();
    Contact {
        time,
        call: i.rcvd.qxsl("call").unwrap_or_default().into(),
        band: rxsl::Band::parse(&a("band"))
            .map(|b| b.label())
            .unwrap_or_default(),
        mode: a("mode"),
    }
}

pub fn check(files: &[&[u8]], req: &Request) -> Report {
    let registry = registry();
    let mut items = Vec::new();
    let mut error = None;
    for data in files {
        match rxsl::sheet::unpack(data) {
            Ok(list) => items.extend(list),
            Err(e) => error = Some(e.to_string()),
        }
    }
    let contest = registry.contests().find(|c| c.name() == req.contest);
    let chosen: Vec<&dyn rxsl::ruler::Section> = contest
        .map(|c| {
            let all = req.sections.iter().filter_map(|n| c.section(n));
            all.filter(|s| !s.is_absence()).collect()
        })
        .unwrap_or_default();
    let conflict = contest.is_some_and(|c| c.conflict(&chosen));
    let reasons = match (contest, conflict) {
        (Some(c), true) => c.explain(&chosen),
        _ => Vec::new(),
    };
    let sections = contest
        .map(|c: &dyn Program| {
            req.sections
                .iter()
                .filter_map(|n| c.section(n))
                .filter(|s| !s.is_absence())
                .map(|s| {
                    let sum = s.summarize(&items);
                    Preview {
                        sect: s.name().into(),
                        valid: sum.accepted().len(),
                        invalid: sum.rejected().len(),
                        score: sum.score(),
                        total: sum.total(),
                        rejected: sum.rejected().iter().take(100).map(contact).collect(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    Report {
        items: items.len(),
        error,
        sections,
        conflict,
        reasons,
    }
}

fn split(data: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut rest = data;
    while rest.len() >= 4 {
        let n = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
        let end = (4 + n).min(rest.len());
        out.push(&rest[4..end]);
        rest = &rest[end..];
    }
    out
}

#[unsafe(no_mangle)]
pub extern "C" fn ats5_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ats5_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(unsafe { Vec::from_raw_parts(ptr, 0, len.max(1)) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ats5_clock(unix: f64) {
    rxsl::value::set_clock(Some(unix as i64));
}

#[unsafe(no_mangle)]
pub extern "C" fn ats5_offset(minutes: i32) {
    rxsl::value::set_local_offset(Some(minutes));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ats5_check(
    files: *const u8,
    files_len: usize,
    req: *const u8,
    req_len: usize,
) -> *mut u8 {
    let files = unsafe { std::slice::from_raw_parts(files, files_len) };
    let req = unsafe { std::slice::from_raw_parts(req, req_len) };
    let report = match serde_json::from_slice::<Request>(req) {
        Ok(r) => check(&split(files), &r),
        Err(e) => Report {
            items: 0,
            error: Some(e.to_string()),
            sections: Vec::new(),
            conflict: false,
            reasons: Vec::new(),
        },
    };
    let json = serde_json::to_vec(&report).unwrap_or_default();
    let mut out = (json.len() as u32).to_le_bytes().to_vec();
    out.extend(json);
    let ptr = ats5_alloc(out.len());
    unsafe { std::ptr::copy_nonoverlapping(out.as_ptr(), ptr, out.len()) };
    ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_scores() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../ats5-allja1/tests/data/allja1.qxml"
        );
        let data = std::fs::read(path).unwrap();
        let req = Request {
            contest: "ALLJA1".into(),
            sections: vec!["1エリア内 個人 総合 部門".into()],
        };
        let report = check(&[&data], &req);
        assert_eq!(report.items, 1000);
        assert_eq!(report.sections[0].score, 429);
        assert_eq!(report.sections[0].total, 139425);
    }

    #[test]
    fn splits_frames() {
        let mut data = 3u32.to_le_bytes().to_vec();
        data.extend(b"abc");
        data.extend(1u32.to_le_bytes());
        data.extend(b"d");
        assert_eq!(split(&data), vec![&b"abc"[..], &b"d"[..]]);
    }
}
