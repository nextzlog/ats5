use ats5_lua::{builtin, load, LuaContest};
use rxsl::ruler::{Contest, Program, Section};
use rxsl::table::{ctxt::CTxt, jarl::Jarl, zall::ZAll, zdos::ZDos};
use rxsl::value::JST;
use rxsl::Format;

fn format(kind: &str) -> Box<dyn Format> {
    match kind {
        "jarl" => Box::new(Jarl::jst()),
        "zall" => Box::new(ZAll { offset: JST }),
        "zdos" => Box::new(ZDos {
            offset: JST,
            ..ZDos::default()
        }),
        "ctxt" => Box::new(CTxt {
            offset: JST,
            ..CTxt::default()
        }),
        _ => rxsl::formats()
            .into_iter()
            .find(|f| f.kind() == kind)
            .expect(kind),
    }
}

#[test]
fn scores_match_qxsl() {
    let contest = allja1();
    let expected = include_str!("data/allja1.test");
    let mut checked = 0;
    for line in expected.lines().filter(|l| !l.trim().is_empty()) {
        let cols: Vec<&str> = line.split(',').map(str::trim).collect();
        let section = contest.section(cols[0]).expect(cols[0]);
        let score: u64 = cols[1].parse().unwrap();
        let total: u64 = cols[2].parse().unwrap();
        for kind in cols[3].split(':') {
            let format = format(kind);
            let path = format!("{}/tests/data/allja1.{kind}", env!("CARGO_MANIFEST_DIR"));
            let bytes = std::fs::read(&path).unwrap();
            let items = format.read(&bytes).unwrap();
            let summary = section.summarize(&items);
            let got = (summary.score() as u64, summary.total() as u64);
            assert_eq!(got, (score, total), "{} {kind}", cols[0]);
            checked += 1;
        }
    }
    assert_eq!(checked, 528);
}

#[test]
fn section_count() {
    assert_eq!(allja1().sections().len(), 6 + 48);
}

#[test]
fn schedule() {
    let contest = allja1();
    assert_eq!(contest.start_day(2026).to_string(), "2026-06-27");
    assert_eq!(contest.dead_line(2026).to_string(), "2026-07-18");
}

#[test]
fn conflicts() {
    let contest = allja1();
    let pick = |names: &[&str]| -> Vec<&dyn Section> {
        names.iter().map(|n| contest.section(n).unwrap()).collect()
    };
    let ok = pick(&[
        "1エリア内 個人 電信限定 7MHz部門",
        "1エリア内 個人 総合 部門",
    ]);
    assert!(!contest.conflict(&ok));
    let no_total = pick(&["1エリア内 個人 電信限定 7MHz部門"]);
    assert!(contest.conflict(&no_total));
    let mixed = pick(&["1エリア内 団体 総合 部門", "1エリア内 個人 総合 部門"]);
    assert!(contest.conflict(&mixed));
    assert!(!contest.conflict(&[]));
    assert_eq!(contest.explain(&no_total).len(), 1);
    assert!(contest.explain(&no_total)[0].contains("総合部門"));
    assert!(contest.explain(&ok).is_empty());
}

fn allja1() -> LuaContest {
    let (name, source) = builtin().iter().find(|(n, _)| *n == "ja1.lua").unwrap();
    load(name, source).unwrap()
}
