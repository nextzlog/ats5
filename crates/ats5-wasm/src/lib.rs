#![allow(clippy::missing_safety_doc)]

use serde::Serialize;

#[derive(Serialize)]
pub struct Report {
    pub items: usize,
    pub error: Option<String>,
}

pub fn check(files: &[&[u8]]) -> Report {
    let mut items = 0;
    let mut error = None;
    for data in files {
        match rxsl::sheet::unpack(data) {
            Ok(list) => items += list.len(),
            Err(e) => error = Some(e.to_string()),
        }
    }
    Report { items, error }
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
pub unsafe extern "C" fn ats5_check(files: *const u8, len: usize) -> *mut u8 {
    let files = unsafe { std::slice::from_raw_parts(files, len) };
    let json = serde_json::to_vec(&check(&split(files))).unwrap_or_default();
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
    fn reads_logs() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../ats5-lua/tests/data/allja1.qxml"
        );
        let data = std::fs::read(path).unwrap();
        let report = check(&[&data]);
        assert_eq!(report.items, 1000);
        assert!(check(&[b"garbage"]).error.is_some());
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
