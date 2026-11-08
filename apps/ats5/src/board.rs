use std::collections::BTreeMap;

use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Count {
    pub kind: String,
    pub band: String,
    pub mode: String,
    pub value: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub call: String,
    pub contest: String,
    pub ops: String,
    pub club: String,
    pub soft: String,
    pub class: BTreeMap<String, String>,
    pub qth: BTreeMap<String, String>,
    pub breakdown: Vec<Count>,
    pub score: i64,
    pub timestamp: String,
}

impl Report {
    pub fn total(&self, kind: &str) -> i64 {
        let all = self
            .breakdown
            .iter()
            .filter(|c| c.kind == kind && c.band.eq_ignore_ascii_case("total"));
        all.map(|c| c.value).max().unwrap_or(0)
    }
}

pub fn parse(xml: &str) -> Result<Report, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut report = Report::default();
    let mut path: Vec<String> = Vec::new();
    let mut attrs: Vec<(String, String)> = Vec::new();
    let mut root = false;
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_lowercase();
                root |= name == "dynamicresults";
                attrs = e
                    .attributes()
                    .flatten()
                    .map(|a| {
                        let k = String::from_utf8_lossy(a.key.local_name().as_ref()).to_lowercase();
                        let v = a
                            .normalized_value(quick_xml::XmlVersion::default())
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        (k, v)
                    })
                    .collect();
                if name == "class" {
                    report.class.extend(attrs.iter().cloned());
                }
                path.push(name);
            }
            Event::Empty(e) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_lowercase();
                if name == "class" {
                    for a in e.attributes().flatten() {
                        let k = String::from_utf8_lossy(a.key.local_name().as_ref()).to_lowercase();
                        let v = a
                            .normalized_value(quick_xml::XmlVersion::default())
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        report.class.insert(k, v);
                    }
                }
            }
            Event::Text(t) => {
                let raw = t.decode().map(|d| d.to_string()).unwrap_or_default();
                let text = quick_xml::escape::unescape(&raw)
                    .map(|u| u.trim().to_string())
                    .unwrap_or_default();
                let leaf = path.last().cloned().unwrap_or_default();
                let parent = path.iter().rev().nth(1).cloned().unwrap_or_default();
                match (parent.as_str(), leaf.as_str()) {
                    ("dynamicresults", "call") => report.call = text.to_uppercase(),
                    ("dynamicresults", "contest") => report.contest = text,
                    ("dynamicresults", "ops") => report.ops = text,
                    ("dynamicresults", "club") => report.club = text,
                    ("dynamicresults", "soft") => report.soft = text,
                    ("dynamicresults", "timestamp") => report.timestamp = text,
                    ("dynamicresults", "score") => report.score = text.parse().unwrap_or(0),
                    ("qth", k) => {
                        report.qth.insert(k.to_string(), text);
                    }
                    ("breakdown", k) => {
                        let get = |n: &str| {
                            attrs
                                .iter()
                                .find(|(a, _)| a == n)
                                .map(|(_, v)| v.clone())
                                .unwrap_or_default()
                        };
                        report.breakdown.push(Count {
                            kind: k.to_string(),
                            band: get("band"),
                            mode: get("mode"),
                            value: text.parse().unwrap_or(0),
                        });
                    }
                    _ => (),
                }
            }
            Event::End(_) => {
                path.pop();
            }
            Event::Eof => break,
            _ => (),
        }
    }
    if !root {
        return Err("<dynamicresults> is not found".into());
    }
    if !rxsl::draft::Call::new(&report.call).valid() {
        return Err(format!("invalid call sign: {}", report.call));
    }
    Ok(report)
}

pub fn area(call: &str) -> String {
    let base = call.split('/').next().unwrap_or_default();
    let port = call
        .split('/')
        .nth(1)
        .filter(|p| p.len() == 1 && p.chars().all(|c| c.is_ascii_digit()));
    if let Some(p) = port {
        return p.to_string();
    }
    let digit = base.chars().rev().find(|c| c.is_ascii_digit());
    digit.map(|c| c.to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<dynamicresults>
  <contest>ALLJA1</contest>
  <call>ja1zlo</call>
  <ops>JA1ZLO</ops>
  <class power="HIGH" assisted="NONASSISTED" transmitter="ONE" ops="MULTI-OP" bands="ALL" mode="MIXED" overlay="N/A"></class>
  <club>UTokyo</club>
  <soft>N1MM Logger+</soft>
  <version>1.0.10000</version>
  <qth><cqzone>25</cqzone><iaruzone>45</iaruzone><stprvoth>10</stprvoth><grid6>PM95</grid6></qth>
  <breakdown>
    <qso band="total" mode="ALL">120</qso>
    <point band="total" mode="ALL">120</point>
    <mult band="total" mode="ALL" type="ALL">40</mult>
    <qso band="7" mode="CW">80</qso>
    <qso band="14" mode="CW">40</qso>
  </breakdown>
  <score>4800</score>
  <timestamp>2026-06-27 01:23:45</timestamp>
</dynamicresults>"#;

    #[test]
    fn parses_n1mm_scores() {
        let r = parse(SAMPLE).unwrap();
        assert_eq!(r.call, "JA1ZLO");
        assert_eq!(r.contest, "ALLJA1");
        assert_eq!(r.score, 4800);
        assert_eq!(r.total("qso"), 120);
        assert_eq!(r.total("mult"), 40);
        assert_eq!(r.class.get("ops").map(String::as_str), Some("MULTI-OP"));
        assert_eq!(r.qth.get("cqzone").map(String::as_str), Some("25"));
        assert_eq!(r.breakdown.len(), 5);
    }

    #[test]
    fn rejects_other_documents() {
        assert!(parse("<html></html>").is_err());
        assert!(parse("<dynamicresults><call>bad call</call></dynamicresults>").is_err());
    }

    #[test]
    fn areas() {
        assert_eq!(area("JA1ZLO"), "1");
        assert_eq!(area("JA1ZLO/3"), "3");
        assert_eq!(area("7K1ABC"), "1");
        assert_eq!(area("JR8YLM/P"), "8");
    }
}
