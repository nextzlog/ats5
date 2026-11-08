use ats5_lua::{builtin, load, preload, LuaContest};
use rxsl::ruler::{Contest, Program, Section};
use rxsl::{Item, QName};

const DAT: &str = "header\n101        宗谷\n1001       札幌\n100110     文京\nend of file\n";

fn contest(file: &str) -> LuaContest {
    for name in ["uec", "ja1", "xpo", "acag"] {
        preload(
            &format!("http://ja6ycu.in.coocan.jp/ZLOG/{name}.dat"),
            DAT.as_bytes(),
        );
    }
    let (name, source) = builtin().iter().find(|(n, _)| *n == file).unwrap();
    load(name, source).unwrap()
}

fn qso(time: &str, khz: u64, mode: &str, rcvd: &str, sent: &str) -> Item {
    let mut it = Item::default();
    it.attrs.set(QName::qxsl("time"), time);
    it.attrs.set(QName::qxsl("band"), khz.to_string());
    it.attrs.set(QName::qxsl("mode"), mode);
    it.rcvd.set(QName::qxsl("call"), "JA1ZLO");
    it.rcvd.set(QName::qxsl("code"), rcvd);
    it.sent.set(QName::qxsl("code"), sent);
    it
}

fn score(s: &dyn Section, it: &Item) -> i32 {
    let m = s.verify(it);
    assert!(m.is_success(), "{}: {}", s.name(), m.text());
    m.score()
}

fn rejected(s: &dyn Section, it: &Item) -> bool {
    s.verify(it).is_failure()
}

fn pick<'a>(c: &'a LuaContest, names: &[&str]) -> Vec<&'a dyn Section> {
    names.iter().map(|n| c.section(n).expect(n)).collect()
}

#[test]
fn every_rule_loads() {
    for (name, _) in builtin() {
        let c = contest(name);
        assert!(!c.sections().is_empty(), "{name}");
        assert!(
            c.start_day(2026) <= c.dead_line(2026) || *name == "1am.lua",
            "{name}"
        );
    }
}

#[test]
fn uec() {
    let c = contest("uec.lua");
    assert_eq!(c.sections().len(), 11);
    assert_eq!(c.start_day(2026).to_string(), "2026-07-18");
    assert_eq!(c.dead_line(2026).to_string(), "2026-08-31");
    assert_eq!(c.limit_multiple_entry("シングルバンド"), 2);
    assert_eq!(c.limit_multiple_entry("オールバンド"), 1);
    let s = c.section("7MHz").unwrap();
    assert_eq!(
        score(
            s,
            &qso(
                "2026-07-18T09:00:00Z",
                7000,
                "CW",
                "599101UEC",
                "599100110L"
            )
        ),
        5
    );
    assert_eq!(
        score(s, &qso("2026-07-18T09:00:00Z", 7000, "CW", "599101H", "")),
        2
    );
    assert!(rejected(
        s,
        &qso("2026-07-18T07:00:00Z", 7000, "CW", "599101H", "")
    ));
    assert!(rejected(
        s,
        &qso("2026-07-18T09:00:00Z", 7000, "SSB", "599101H", "")
    ));
    assert!(rejected(
        s,
        &qso("2026-07-18T09:00:00Z", 7000, "CW", "599999H", "")
    ));
    assert!(rejected(
        s,
        &qso("2026-07-18T09:00:00Z", 14000, "CW", "599101H", "")
    ));
    assert!(c.explain(&pick(&c, &["7MHz", "オールバンド"])).len() == 1);
    assert!(c.explain(&pick(&c, &["7MHz", "7MHz"])).len() == 1);
    assert!(c.explain(&pick(&c, &["7MHz", "14MHz"])).is_empty());
    assert_eq!(s.award_limit(&[0; 30]), 3);
    assert_eq!(s.award_limit(&[0; 11]), 2);
    assert_eq!(s.award_limit(&[0; 10]), 1);
}

#[test]
fn area_one_am() {
    let c = contest("1am.lua");
    assert_eq!(c.sections().len(), 5);
    assert_eq!(c.start_day(2026).to_string(), "2026-12-27");
    let s = c.section("1エリア内固定局").unwrap();
    assert_eq!(
        score(s, &qso("2026-12-27T02:00:00Z", 50000, "AM", "1001", "101")),
        2
    );
    assert_eq!(
        score(s, &qso("2026-12-27T02:00:00Z", 50000, "AM", "101", "101")),
        1
    );
    assert!(rejected(
        s,
        &qso("2026-12-27T02:00:00Z", 50000, "FM", "101", "101")
    ));
    assert!(rejected(
        s,
        &qso("2026-12-27T05:00:00Z", 50000, "AM", "101", "101")
    ));
}

#[test]
fn qrp() {
    let c = contest("qrp.lua");
    assert_eq!(c.sections().len(), 17);
    assert_eq!(c.start_day(2026).to_string(), "2026-11-03");
    assert_eq!(c.dead_line(2026).to_string(), "2026-11-17");
    assert!(c.section("H19 1.9MHzバンド 自作機").is_some());
    assert!(c.section("GA オールバンド 一般").is_some());
    let s = c.section("G7 7MHzバンド 一般").unwrap();
    assert_eq!(
        score(s, &qso("2026-11-03T05:00:00Z", 7000, "CW", "599101P", "")),
        1
    );
    assert!(rejected(
        s,
        &qso("2026-11-03T05:00:00Z", 7000, "CW", "599101M", "")
    ));
    assert!(rejected(
        s,
        &qso("2026-11-03T03:00:00Z", 7000, "CW", "599101P", "")
    ));
    assert!(rejected(
        s,
        &qso("2026-11-03T12:00:00Z", 7000, "CW", "599101P", "")
    ));
    let cw = qso("2026-11-03T05:00:00Z", 7000, "CW", "599101P", "");
    let ph = qso("2026-11-03T05:01:00Z", 7000, "SSB", "59101P", "");
    let sum = s.summarize(&[cw.clone(), ph, cw]);
    assert_eq!((sum.accepted().len(), sum.rejected().len()), (2, 1));
    assert_eq!(s.award_limit(&[0; 50]), 1);
    let two = pick(&c, &["G7 7MHzバンド 一般", "G14 14MHzバンド 一般"]);
    assert_eq!(c.explain(&two).len(), 1);
}

#[test]
fn oqp() {
    let c = contest("oqp.lua");
    assert_eq!(c.sections().len(), 3);
    assert_eq!(c.dead_line(2026).to_string(), "2027-01-31");
    let items = [
        qso("2026-11-01T01:00:00Z", 7000, "CW", "x", ""),
        qso("2026-11-01T02:00:00Z", 7000, "FT8", "x", ""),
        qso("2026-11-02T01:00:00Z", 7000, "CW", "x", ""),
    ];
    let calls = c.section("局数部門").unwrap().summarize(&items);
    assert_eq!(calls.total(), 7);
    let days = c.section("運用日数部門").unwrap().summarize(&items);
    assert_eq!(days.total(), 2);
    assert!(c
        .explain(&pick(&c, &["局数部門", "運用日数部門"]))
        .is_empty());
}

#[test]
fn real_time() {
    let c = contest("rtc.lua");
    assert_eq!(c.sections().len(), 6);
    assert!(c.finish());
    let s = c.section("個人 電信限定 部門").unwrap();
    let mut it = qso("2026-07-27T05:00:00Z", 7000, "CW", "10", "");
    assert_eq!(score(s, &it), 1);
    it.attrs.set(QName::qxsl("sign"), "2026-07-27T05:00:30Z");
    assert_eq!(score(s, &it), 2);
    assert!(rejected(
        s,
        &qso("2026-07-27T05:00:00Z", 7000, "CW", "1001", "")
    ));
    assert!(rejected(
        s,
        &qso("2026-07-27T05:00:00Z", 1900, "CW", "10", "")
    ));
    assert!(rejected(c.section("団体 電信限定 部門").unwrap(), &it));
    assert_eq!(
        c.explain(&pick(&c, &["個人 電信限定 部門", "団体 電話限定 部門"]))
            .len(),
        1
    );
}

#[test]
fn vus() {
    let c = contest("vus.lua");
    assert_eq!(c.sections().len(), 3 + 10 * 13);
    assert_eq!(c.start_day(2026).to_string(), "2026-05-04");
    assert_eq!(c.dead_line(2026).to_string(), "2026-05-18");
    let s = c.section("1エリア シングルオペ オールバンド").unwrap();
    assert_eq!(
        score(s, &qso("2026-05-04T01:00:00Z", 2400000, "CW", "100110", "")),
        20
    );
    assert_eq!(
        score(s, &qso("2026-05-04T04:00:00Z", 144000, "SSB", "100110", "")),
        1
    );
    assert!(rejected(
        s,
        &qso("2026-05-04T01:00:00Z", 144000, "SSB", "100110", "")
    ));
    assert!(rejected(
        s,
        &qso("2026-05-04T04:00:00Z", 2400000, "SSB", "100110", "")
    ));
    let gen = "1エリア シングルオペ オールバンド";
    let rd1 = "1エリア シングルオペ SHF";
    let rd2 = "1エリア シングルオペ V/UHF";
    assert_eq!(c.explain(&pick(&c, &[gen, rd1])).len(), 1);
    assert_eq!(c.explain(&pick(&c, &[rd1, rd2])).len(), 1);
    assert!(c
        .explain(&pick(&c, &[rd1, "1エリア シングルオペ 144MHz"]))
        .is_empty());
}
