use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension, Row};
use rxsl::table::qxml::Qxml;
use rxsl::{Format, Item};
use serde::{Deserialize, Serialize};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS submission (
    id INTEGER PRIMARY KEY,
    call TEXT NOT NULL,
    name TEXT NOT NULL,
    post TEXT NOT NULL,
    addr TEXT NOT NULL,
    mail TEXT NOT NULL,
    note TEXT NOT NULL,
    uuid TEXT NOT NULL,
    time INTEGER NOT NULL,
    peer TEXT NOT NULL,
    active INTEGER NOT NULL,
    flags TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS submission_call ON submission(call);
CREATE TABLE IF NOT EXISTS archive (
    id INTEGER PRIMARY KEY,
    submission INTEGER NOT NULL,
    file TEXT NOT NULL,
    data BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS archive_submission ON archive(submission);
CREATE TABLE IF NOT EXISTS entry (
    submission INTEGER NOT NULL,
    sect TEXT NOT NULL,
    city TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS entry_submission ON entry(submission);
CREATE TABLE IF NOT EXISTS message (
    call TEXT NOT NULL,
    dest TEXT NOT NULL,
    item BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS message_call ON message(call);
CREATE INDEX IF NOT EXISTS message_dest ON message(dest, call);
CREATE TABLE IF NOT EXISTS ranking (
    call TEXT NOT NULL,
    sect TEXT NOT NULL,
    city TEXT NOT NULL,
    score INTEGER NOT NULL,
    total INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS ranking_call ON ranking(call);
CREATE TABLE IF NOT EXISTS report (
    call TEXT PRIMARY KEY,
    data TEXT NOT NULL,
    time INTEGER NOT NULL
);
";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Station {
    pub call: String,
    pub name: String,
    pub post: String,
    pub addr: String,
    pub mail: String,
    pub note: String,
    pub uuid: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Submission {
    pub id: i64,
    pub station: Station,
    pub time: i64,
    pub peer: String,
    pub active: bool,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub sect: String,
    pub city: String,
}

#[derive(Debug, Clone)]
pub struct Archive {
    pub id: i64,
    pub file: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Ranking {
    pub call: String,
    pub sect: String,
    pub city: String,
    pub score: i64,
    pub total: i64,
}

pub struct Store {
    conn: Mutex<Connection>,
}

pub type Result<T> = rusqlite::Result<T>;

fn submission(row: &Row) -> Result<Submission> {
    let flags: String = row.get(11)?;
    Ok(Submission {
        id: row.get(0)?,
        station: Station {
            call: row.get(1)?,
            name: row.get(2)?,
            post: row.get(3)?,
            addr: row.get(4)?,
            mail: row.get(5)?,
            note: row.get(6)?,
            uuid: row.get(7)?,
        },
        time: row.get(8)?,
        peer: row.get(9)?,
        active: row.get(10)?,
        flags: flags
            .split(',')
            .filter(|f| !f.is_empty())
            .map(String::from)
            .collect(),
    })
}

const COLUMNS: &str = "id, call, name, post, addr, mail, note, uuid, time, peer, active, flags";

pub fn encode_item(item: &Item) -> Vec<u8> {
    Qxml.encode(std::slice::from_ref(item)).unwrap_or_default()
}

pub fn decode_item(bytes: &[u8]) -> Option<Item> {
    Qxml.decode(bytes).ok()?.into_iter().next()
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    pub fn memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    fn with<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut conn)
    }

    pub fn submit(
        &self,
        sub: &Submission,
        entries: &[Entry],
        files: &[(String, Vec<u8>)],
    ) -> Result<i64> {
        self.with(|c| {
            let tx = c.transaction()?;
            let s = &sub.station;
            tx.execute("UPDATE submission SET active = 0 WHERE call = ?1", [&s.call])?;
            tx.execute(
                "INSERT INTO submission (call, name, post, addr, mail, note, uuid, time, peer, active, flags)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10)",
                params![
                    s.call,
                    s.name,
                    s.post,
                    s.addr,
                    s.mail,
                    s.note,
                    s.uuid,
                    sub.time,
                    sub.peer,
                    sub.flags.join(","),
                ],
            )?;
            let id = tx.last_insert_rowid();
            for e in entries {
                tx.execute(
                    "INSERT INTO entry (submission, sect, city) VALUES (?1, ?2, ?3)",
                    params![id, e.sect, e.city],
                )?;
            }
            for (file, data) in files {
                tx.execute(
                    "INSERT INTO archive (submission, file, data) VALUES (?1, ?2, ?3)",
                    params![id, file, data],
                )?;
            }
            tx.commit()?;
            Ok(id)
        })
    }

    pub fn active(&self, call: &str) -> Result<Option<Submission>> {
        self.with(|c| {
            let sql = format!("SELECT {COLUMNS} FROM submission WHERE call = ?1 AND active = 1");
            c.query_row(&sql, [call], submission).optional()
        })
    }

    pub fn submission(&self, id: i64) -> Result<Option<Submission>> {
        self.with(|c| {
            let sql = format!("SELECT {COLUMNS} FROM submission WHERE id = ?1");
            c.query_row(&sql, [id], submission).optional()
        })
    }

    pub fn by_uuid(&self, uuid: &str) -> Result<Option<Submission>> {
        self.with(|c| {
            let sql = format!("SELECT {COLUMNS} FROM submission WHERE uuid = ?1 AND active = 1");
            c.query_row(&sql, [uuid], submission).optional()
        })
    }

    pub fn actives(&self) -> Result<Vec<Submission>> {
        self.with(|c| {
            let sql = format!("SELECT {COLUMNS} FROM submission WHERE active = 1 ORDER BY time");
            let mut stmt = c.prepare(&sql)?;
            let rows = stmt.query_map([], submission)?;
            rows.collect()
        })
    }

    pub fn history(&self, call: &str) -> Result<Vec<Submission>> {
        self.with(|c| {
            let sql = format!("SELECT {COLUMNS} FROM submission WHERE call = ?1 ORDER BY id DESC");
            let mut stmt = c.prepare(&sql)?;
            let rows = stmt.query_map([call], submission)?;
            rows.collect()
        })
    }

    pub fn entries(&self, id: i64) -> Result<Vec<Entry>> {
        self.with(|c| {
            let mut stmt = c.prepare("SELECT sect, city FROM entry WHERE submission = ?1")?;
            let rows = stmt.query_map([id], |r| {
                Ok(Entry {
                    sect: r.get(0)?,
                    city: r.get(1)?,
                })
            })?;
            rows.collect()
        })
    }

    pub fn archives(&self, id: i64) -> Result<Vec<Archive>> {
        self.with(|c| {
            let mut stmt =
                c.prepare("SELECT id, file, data FROM archive WHERE submission = ?1 ORDER BY id")?;
            let rows = stmt.query_map([id], |r| {
                Ok(Archive {
                    id: r.get(0)?,
                    file: r.get(1)?,
                    data: r.get(2)?,
                })
            })?;
            rows.collect()
        })
    }

    pub fn archive(&self, id: i64) -> Result<Option<(i64, Archive)>> {
        self.with(|c| {
            c.query_row(
                "SELECT submission, id, file, data FROM archive WHERE id = ?1",
                [id],
                |r| {
                    let archive = Archive {
                        id: r.get(1)?,
                        file: r.get(2)?,
                        data: r.get(3)?,
                    };
                    Ok((r.get(0)?, archive))
                },
            )
            .optional()
        })
    }

    pub fn activate(&self, id: i64) -> Result<Option<String>> {
        self.with(|c| {
            let tx = c.transaction()?;
            let call: Option<String> = tx
                .query_row("SELECT call FROM submission WHERE id = ?1", [id], |r| {
                    r.get(0)
                })
                .optional()?;
            if let Some(call) = &call {
                tx.execute("UPDATE submission SET active = 0 WHERE call = ?1", [call])?;
                tx.execute("UPDATE submission SET active = 1 WHERE id = ?1", [id])?;
            }
            tx.commit()?;
            Ok(call)
        })
    }

    pub fn set_flags(&self, id: i64, flags: &[String]) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE submission SET flags = ?2 WHERE id = ?1",
                params![id, flags.join(",")],
            )?;
            Ok(())
        })
    }

    pub fn drop_call(&self, call: &str) -> Result<()> {
        self.with(|c| {
            let tx = c.transaction()?;
            let ids = "SELECT id FROM submission WHERE call = ?1";
            tx.execute(
                &format!("DELETE FROM archive WHERE submission IN ({ids})"),
                [call],
            )?;
            tx.execute(
                &format!("DELETE FROM entry WHERE submission IN ({ids})"),
                [call],
            )?;
            tx.execute("DELETE FROM submission WHERE call = ?1", [call])?;
            tx.execute("DELETE FROM message WHERE call = ?1", [call])?;
            tx.execute("DELETE FROM ranking WHERE call = ?1", [call])?;
            tx.commit()
        })
    }

    pub fn clear(&self) -> Result<()> {
        self.with(|c| {
            c.execute_batch(
                "DELETE FROM archive; DELETE FROM entry; DELETE FROM submission;
                 DELETE FROM message; DELETE FROM ranking; DELETE FROM report;",
            )
        })
    }

    pub fn set_messages(&self, call: &str, items: &[Item]) -> Result<()> {
        self.with(|c| {
            let tx = c.transaction()?;
            tx.execute("DELETE FROM message WHERE call = ?1", [call])?;
            insert_messages(&tx, call, items)?;
            tx.commit()
        })
    }

    pub fn add_messages(&self, call: &str, items: &[Item]) -> Result<()> {
        self.with(|c| {
            let tx = c.transaction()?;
            insert_messages(&tx, call, items)?;
            tx.commit()
        })
    }

    pub fn drop_messages(&self, call: &str, items: &[Item]) -> Result<()> {
        self.with(|c| {
            let tx = c.transaction()?;
            for item in items {
                tx.execute(
                    "DELETE FROM message WHERE rowid = (SELECT rowid FROM message
                     WHERE call = ?1 AND item = ?2 LIMIT 1)",
                    params![call, encode_item(item)],
                )?;
            }
            tx.commit()
        })
    }

    pub fn messages(&self, call: &str) -> Result<Vec<Item>> {
        self.items(
            "SELECT item FROM message WHERE call = ?1 ORDER BY rowid",
            [call],
        )
    }

    pub fn replies(&self, call: &str, dest: &str) -> Result<Vec<Item>> {
        self.items(
            "SELECT item FROM message WHERE call = ?1 AND dest = ?2",
            [dest, call],
        )
    }

    fn items<P: rusqlite::Params>(&self, sql: &str, p: P) -> Result<Vec<Item>> {
        self.with(|c| {
            let mut stmt = c.prepare(sql)?;
            let rows = stmt.query_map(p, |r| r.get::<_, Vec<u8>>(0))?;
            let mut out = Vec::new();
            for row in rows {
                if let Some(item) = decode_item(&row?) {
                    out.push(item);
                }
            }
            Ok(out)
        })
    }

    pub fn set_rankings(&self, call: &str, rows: &[Ranking]) -> Result<()> {
        self.with(|c| {
            let tx = c.transaction()?;
            tx.execute("DELETE FROM ranking WHERE call = ?1", [call])?;
            for r in rows {
                tx.execute(
                    "INSERT INTO ranking (call, sect, city, score, total) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![r.call, r.sect, r.city, r.score, r.total],
                )?;
            }
            tx.commit()
        })
    }

    pub fn rankings(&self) -> Result<Vec<Ranking>> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT call, sect, city, score, total FROM ranking ORDER BY total DESC, call",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(Ranking {
                    call: r.get(0)?,
                    sect: r.get(1)?,
                    city: r.get(2)?,
                    score: r.get(3)?,
                    total: r.get(4)?,
                })
            })?;
            rows.collect()
        })
    }

    pub fn put_report(&self, call: &str, data: &str, time: i64) -> Result<()> {
        self.with(|c| {
            c.execute(
                "INSERT INTO report (call, data, time) VALUES (?1, ?2, ?3)
                 ON CONFLICT(call) DO UPDATE SET data = ?2, time = ?3",
                params![call, data, time],
            )?;
            Ok(())
        })
    }

    pub fn reports(&self) -> Result<Vec<(String, String, i64)>> {
        self.with(|c| {
            let mut stmt = c.prepare("SELECT call, data, time FROM report ORDER BY call")?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            rows.collect()
        })
    }
}

fn insert_messages(tx: &rusqlite::Transaction, call: &str, items: &[Item]) -> Result<()> {
    for item in items {
        let dest = item
            .rcvd
            .qxsl("call")
            .unwrap_or_default()
            .to_ascii_uppercase();
        tx.execute(
            "INSERT INTO message (call, dest, item) VALUES (?1, ?2, ?3)",
            params![call, dest, encode_item(item)],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rxsl::QName;

    fn station(call: &str, mail: &str) -> Submission {
        Submission {
            id: 0,
            station: Station {
                call: call.into(),
                mail: mail.into(),
                uuid: format!("{call}-uuid"),
                ..Station::default()
            },
            time: 1,
            peer: "127.0.0.1".into(),
            active: true,
            flags: Vec::new(),
        }
    }

    #[test]
    fn resubmission_keeps_history() {
        let store = Store::memory().unwrap();
        let entry = Entry {
            sect: "A".into(),
            city: "B".into(),
        };
        let first = store
            .submit(
                &station("JA1ZLO", "a@example.com"),
                std::slice::from_ref(&entry),
                &[],
            )
            .unwrap();
        let second = store
            .submit(
                &station("JA1ZLO", "b@example.com"),
                &[entry],
                &[("a.txt".into(), b"x".to_vec())],
            )
            .unwrap();
        assert_eq!(store.active("JA1ZLO").unwrap().unwrap().id, second);
        assert_eq!(store.history("JA1ZLO").unwrap().len(), 2);
        store.activate(first).unwrap();
        assert_eq!(store.active("JA1ZLO").unwrap().unwrap().id, first);
        assert_eq!(store.archives(second).unwrap().len(), 1);
        assert!(store.by_uuid("JA1ZLO-uuid").unwrap().is_some());
        store.drop_call("JA1ZLO").unwrap();
        assert!(store.history("JA1ZLO").unwrap().is_empty());
    }

    #[test]
    fn messages_round_trip() {
        let store = Store::memory().unwrap();
        let mut item = Item::default();
        item.attrs.set(QName::qxsl("time"), "2017-06-04T00:00:00Z");
        item.rcvd.set(QName::qxsl("call"), "JA1YAA");
        item.rcvd.set(QName::qxsl("code"), "100110");
        store
            .set_messages("JA1ZLO", &[item.clone(), item.clone()])
            .unwrap();
        assert_eq!(
            store.messages("JA1ZLO").unwrap(),
            vec![item.clone(), item.clone()]
        );
        assert_eq!(store.replies("JA1YAA", "JA1ZLO").unwrap().len(), 2);
        store.drop_messages("JA1ZLO", &[item.clone()]).unwrap();
        assert_eq!(store.messages("JA1ZLO").unwrap().len(), 1);
    }
}
