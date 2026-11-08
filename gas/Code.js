const CLIENT = "328148444857-pcp6epd21lqqestklbfmgl1cbksku469.apps.googleusercontent.com";
const PAGE = 20;

function props() {
  return PropertiesService.getScriptProperties();
}

function folder() {
  const id = props().getProperty("folder");
  if (id) return DriveApp.getFolderById(id);
  const made = DriveApp.createFolder("ATS-5");
  props().setProperty("folder", made.getId());
  return made;
}

function child(name) {
  const root = folder();
  const found = root.getFoldersByName(name);
  return found.hasNext() ? found.next() : root.createFolder(name);
}

function load(name, fallback) {
  const files = folder().getFilesByName(name);
  return files.hasNext() ? JSON.parse(files.next().getBlob().getDataAsString("UTF-8")) : fallback;
}

function save(name, value) {
  const text = JSON.stringify(value);
  const files = folder().getFilesByName(name);
  if (files.hasNext()) files.next().setContent(text);
  else folder().createFile(name, text, "application/json");
}

function owner() {
  return Session.getEffectiveUser().getEmail().toLowerCase();
}

function settings() {
  return load("settings.json", { rule: "", name: "", start: 0, deadline: 0, admins: [] });
}

function verify(token) {
  if (!token) throw new Error("login");
  const res = UrlFetchApp.fetch("https://oauth2.googleapis.com/tokeninfo?id_token=" + encodeURIComponent(token), { muteHttpExceptions: true });
  if (res.getResponseCode() !== 200) throw new Error("login");
  const info = JSON.parse(res.getContentText());
  if (info.aud !== CLIENT || String(info.email_verified) !== "true") throw new Error("login");
  return String(info.email).toLowerCase();
}

function admin(email) {
  const s = settings();
  const domain = email.split("@")[1] || "";
  return email === owner() || s.admins.some((a) => a.toLowerCase() === email || a.toLowerCase() === domain);
}

function reply(value) {
  return ContentService.createTextOutput(JSON.stringify(value)).setMimeType(ContentService.MimeType.JSON);
}

function open() {
  const s = settings();
  const calls = Object.keys(load("calls.json", {})).sort();
  return { rule: s.rule, name: s.name, start: s.start, deadline: s.deadline, calls, shared: load("public.json", {}) };
}

function doGet() {
  return reply({ ok: true, data: open() });
}

function submit(email, body) {
  const now = Date.now();
  const s = settings();
  if (!s.rule || now < s.start || now >= s.deadline) throw new Error("closed");
  const call = String(body.call || "").toUpperCase();
  if (!/^[A-Z0-9/]+$/.test(call)) throw new Error("request");
  const lock = LockService.getScriptLock();
  lock.waitLock(30000);
  try {
    const calls = load("calls.json", {});
    if (calls[call] && calls[call] !== email) throw new Error("owner");
    calls[call] = email;
    save("calls.json", calls);
  } finally {
    lock.releaseLock();
  }
  const logs = child("logs");
  const stamp = Utilities.formatDate(new Date(now), "UTC", "yyyyMMdd'T'HHmmss'Z'");
  const files = (body.files || []).map((f, i) => {
    const blob = Utilities.newBlob(Utilities.base64Decode(f.data), "application/octet-stream", call + "-" + stamp + "-" + i + "-" + f.name);
    return { name: f.name, id: logs.createFile(blob).getId() };
  });
  const id = call + "-" + stamp;
  const record = { id, call, email, time: now, form: body.form, files };
  child("submissions").createFile(id + ".json", JSON.stringify(record), "application/json");
  let mailed = false;
  try {
    const sections = (body.sections || []).map((x) => "  " + x).join("\n");
    const text = call + " 様\n\nこの度は" + s.name + "に参加いただきありがとうございます。\n以下の部門にて書類を受理いたしました。\n\n" + sections + "\n\n受付番号 " + id + "\n\n今後とも当コンテストをよろしくお願いいたします。\n" + s.name + "\n";
    MailApp.sendEmail({ to: email, subject: s.name + "書類提出の完了", body: text, name: s.name });
    mailed = true;
  } catch (err) {
    mailed = false;
  }
  return { id, time: now, mailed };
}

function records(filter, after) {
  const out = [];
  const it = child("submissions").getFiles();
  while (it.hasNext()) {
    const r = JSON.parse(it.next().getBlob().getDataAsString("UTF-8"));
    if (filter(r) && r.time > after) out.push(r);
  }
  out.sort((a, b) => a.time - b.time || (a.id < b.id ? -1 : 1));
  return out.slice(0, PAGE).map((r) => ({
    ...r,
    files: r.files.map((f) => ({ name: f.name, data: Utilities.base64Encode(DriveApp.getFileById(f.id).getBlob().getBytes()) })),
  }));
}

function handle(body) {
  if (body.op === "open") return open();
  const email = verify(body.token);
  const boss = admin(email);
  switch (body.op) {
    case "whoami": return { email, admin: boss };
    case "submit": return submit(email, body);
    case "mine": return records((r) => r.email === email, Number(body.after || 0));
  }
  if (!boss) throw new Error("admin");
  switch (body.op) {
    case "list": return records(() => true, Number(body.after || 0));
    case "setup": {
      const s = settings();
      for (const k of ["rule", "name", "start", "deadline", "admins"]) if (body[k] !== undefined) s[k] = body[k];
      save("settings.json", s);
      return s;
    }
    case "publish": {
      save("public.json", body.shared || {});
      return true;
    }
  }
  throw new Error("request");
}

function doPost(e) {
  try {
    return reply({ ok: true, data: handle(JSON.parse(e.postData.contents)) });
  } catch (err) {
    return reply({ ok: false, code: String(err && err.message ? err.message : err) });
  }
}

function setup() {
  folder();
  child("logs");
  child("submissions");
  return owner();
}
