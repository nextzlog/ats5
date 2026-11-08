use rxsl::conv::{is_digit, is_morse, is_phone};
use rxsl::local::{areas, cities, CityItem};
use rxsl::value::JST;
use rxsl::{Item, QName, Time};

pub fn call_area(city: &CityItem) -> Option<String> {
    let pref = cities().by_name(&city.area())?;
    areas().by_code(pref.code()).map(CityItem::area)
}

pub fn cities_in(area: &str) -> impl Iterator<Item = &'static CityItem> + '_ {
    let all = cities().to_list().iter();
    all.filter(move |c| call_area(c).as_deref() == Some(area))
}

pub fn cities_out(area: &str) -> impl Iterator<Item = &'static CityItem> + '_ {
    let all = cities().to_list().iter();
    all.filter(move |c| call_area(c).as_deref() != Some(area))
}

pub fn digits(code: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_digit())
}

pub fn is_city(code: &str) -> bool {
    digits(code, 4, 9)
}

pub fn is_subprefecture(code: &str) -> bool {
    digits(code, 3, 3)
}

pub fn is_prefecture(code: &str) -> bool {
    code != "48" && !is_city(code) && !is_subprefecture(code)
}

fn q(local: &str) -> QName {
    QName::qxsl(local)
}

pub fn call(item: &Item) -> Option<&str> {
    item.rcvd
        .get(&q("call"))
        .or_else(|| item.attrs.get(&q("call")))
}

pub fn band(item: &Item) -> Option<&str> {
    item.attrs.get(&q("band"))
}

pub fn khz(item: &Item) -> Option<u64> {
    band(item)?.parse().ok()
}

pub fn mode(item: &Item) -> &str {
    item.attrs.get(&q("mode")).unwrap_or_default()
}

pub fn name(item: &Item) -> Option<&str> {
    item.attrs.get(&q("name")).filter(|n| !n.is_empty())
}

pub fn code(item: &Item) -> &str {
    item.rcvd.get(&q("code")).unwrap_or_default()
}

pub fn city(item: &Item) -> Option<&'static CityItem> {
    cities().by_code(code(item))
}

pub fn jst_hour(item: &Item) -> Option<u32> {
    let time = Time::parse_iso(item.attrs.get(&q("time"))?).ok()?;
    Some(time.at_offset(JST).hour)
}

pub fn morse(item: &Item) -> bool {
    is_morse(mode(item))
}

pub fn phone(item: &Item) -> bool {
    is_phone(mode(item))
}

pub fn digit(item: &Item) -> bool {
    is_digit(mode(item))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn areas_of_cities() {
        let area = |code| call_area(cities().by_code(code).unwrap());
        assert_eq!(area("10").as_deref(), Some("1"));
        assert_eq!(area("1002").as_deref(), Some("1"));
        assert_eq!(area("101").as_deref(), Some("8"));
    }

    #[test]
    fn code_kinds() {
        assert!(is_city("100110"));
        assert!(is_subprefecture("101"));
        assert!(is_prefecture("10"));
        assert!(!is_prefecture("48"));
    }
}
