const CONTESTS: [string, string][] = [
  ["ja1", "ALLJA1"],
  ["uec", "電通大コンテスト"],
  ["vus", "電通大VUSコンテスト"],
  ["qrp", "QRPコンテスト"],
  ["oqp", "大学社団QSOパーティ"],
  ["rtc", "リアルタイムコンテスト"],
  ["1am", "1エリアAMコンテスト"],
];

interface Files {
  mkdir(path: string): void;
  readFile(path: string): Uint8Array;
  writeFile(path: string, data: string | Uint8Array): void;
  mount(kind: unknown, options: object, path: string): void;
  syncfs(populate: boolean, done: (error: unknown) => void): void;
}

interface Module {
  FS: Files;
  IDBFS: unknown;
  HEAPU8: Uint8Array;
  _ats5_alloc(len: number): number;
  _ats5_free(ptr: number, len: number): void;
  _ats5_open(ptr: number, len: number): number;
  _ats5_request(m: number, ml: number, u: number, ul: number, k: number, kl: number, b: number, bl: number): number;
}

interface Answer {
  status: number;
  headers: Headers;
  body: Uint8Array;
}

function put(m: Module, bytes: Uint8Array): [number, number] {
  const ptr = m._ats5_alloc(bytes.length);
  m.HEAPU8.set(bytes, ptr);
  return [ptr, bytes.length];
}

function text(m: Module, value: string): [number, number] {
  return put(m, new TextEncoder().encode(value));
}

function answer(m: Module, ptr: number): Answer {
  const view = new DataView(m.HEAPU8.buffer);
  const len = view.getUint32(ptr, true);
  const status = view.getUint32(ptr + 4, true);
  const headLen = view.getUint32(ptr + 8, true);
  const head = new TextDecoder().decode(m.HEAPU8.slice(ptr + 12, ptr + 12 + headLen));
  const body = m.HEAPU8.slice(ptr + 12 + headLen, ptr + 4 + len);
  m._ats5_free(ptr, len + 4);
  const headers = new Headers();
  for (const line of head.split("\n")) {
    const at = line.indexOf(": ");
    if (at > 0) headers.append(line.slice(0, at), line.slice(at + 2));
  }
  return { status, headers, body };
}

function sync(m: Module, populate: boolean): Promise<void> {
  return new Promise((resolve, reject) => m.FS.syncfs(populate, (e) => (e ? reject(e) : resolve())));
}

function json(status: number, value: unknown): Response {
  return new Response(JSON.stringify(value), { status, headers: { "content-type": "application/json" } });
}

function guide(): void {
  const main = document.querySelector("#content");
  if (!main) return;
  const title = document.createElement("h1");
  title.textContent = "ATS-5";
  const lead = document.createElement("p");
  lead.className = "lead";
  lead.textContent = "コンテストの書類提出ページの URL は、各コンテストの主催者にお問い合わせください。";
  const link = document.createElement("a");
  link.href = "https://github.com/nextzlog/ats5";
  link.textContent = "nextzlog/ats5";
  main.replaceChildren(title, lead, link);
}

function fail(text: string): void {
  const main = document.querySelector("#content");
  if (main) main.textContent = text;
}

async function start(): Promise<void> {
  const gas = await import("./gas.js");
  const id = gas.script();
  if (!id) {
    guide();
    return;
  }
  let opened: import("./gas.js").Opened;
  try {
    opened = await gas.call("open");
  } catch (e) {
    fail("ATS-5: " + String(e));
    return;
  }
  let me: { email: string; admin: boolean } | null = null;
  if (gas.token()) {
    me = await gas.call<{ email: string; admin: boolean }>("whoami").catch(async () => {
      await gas.signOut();
      return null;
    });
  }
  const admin = me?.admin ?? false;
  const rule = CONTESTS.some(([c]) => c === opened.rule) ? opened.rule : "";
  if (!rule && !admin) {
    if (me) await gas.signOut();
    fail("この書類提出ページは準備中です。");
    return;
  }
  const loader = (await import("./ats5-site.js" as string)) as { default: (options: object) => Promise<Module> };
  const m = await loader.default({});
  m.FS.mkdir("/data");
  m.FS.mount(m.IDBFS, {}, "/data");
  await sync(m, true);
  const dir = id.replace(/[^A-Za-z0-9_-]/g, "").slice(0, 80);
  const active = rule || CONTESTS[0][0];
  const started = answer(m, m._ats5_open(...text(m, active + " " + dir)));
  if (started.status !== 200) {
    fail(new TextDecoder().decode(started.body));
    return;
  }
  const local = (method: string, path: string, kind = "", body = new Uint8Array()): Answer => {
    const args = [...text(m, method), ...text(m, path), ...text(m, kind), ...put(m, body)] as [number, number, number, number, number, number, number, number];
    return answer(m, m._ats5_request(...args));
  };
  const parse = (a: Answer): { [k: string]: unknown } => JSON.parse(new TextDecoder().decode(a.body)) as { [k: string]: unknown };
  local("POST", "/api/visitor", "application/json", new TextEncoder().encode(JSON.stringify({ email: me?.email ?? null, admin })));
  const info = parse(local("GET", "/api/contest"));
  const offset = Number(info.offset ?? 0);
  const day = (value: string, plus: number): number => {
    const [y, mo, da] = value.split("-").map(Number);
    return Date.UTC(y ?? 1970, (mo ?? 1) - 1, (da ?? 1) + plus) - offset * 1000;
  };
  const begin = day(String(info.start), 0);
  const until = day(String(info.deadline), 1);
  if (admin && rule && (opened.start !== begin || opened.deadline !== until || opened.name !== info.name)) {
    await gas.call("setup", { start: begin, deadline: until, name: info.name }).catch(() => undefined);
  }
  const store = "/data/" + dir + "/imported.json";
  const seen = new Set<string>();
  try {
    for (const x of JSON.parse(new TextDecoder().decode(m.FS.readFile(store))) as string[]) seen.add(x);
  } catch {
    seen.clear();
  }
  const remember = async (doc: string): Promise<void> => {
    seen.add(doc);
    m.FS.writeFile(store, JSON.stringify([...seen]));
    await sync(m, false);
  };
  const multipart = async (fd: FormData): Promise<[string, Uint8Array]> => {
    const req = new Request("http://ats5/", { method: "POST", body: fd });
    return [req.headers.get("content-type") ?? "", new Uint8Array(await req.arrayBuffer())];
  };
  if (me && rule) {
    let after = 0;
    for (;;) {
      const page = await gas.call<import("./gas.js").Saved[]>(admin ? "list" : "mine", { after }).catch(() => []);
      for (const r of page) {
        after = Math.max(after, r.time);
        if (seen.has(r.id)) continue;
        const form = JSON.parse(r.form || "{}") as { [k: string]: unknown };
        form.time = Math.floor(r.time / 1000);
        const fd = new FormData();
        fd.append("form", JSON.stringify(form));
        for (const f of r.files) fd.append("file", new Blob([gas.bytes(f.data)]), f.name);
        local("POST", "/api/import", ...(await multipart(fd)));
        await remember(r.id);
      }
      if (page.length < 20) break;
    }
  }
  const shared = (name: string): Response => {
    const value = opened.shared[name] ?? (name === "results" ? { open: false } : []);
    if (name === "calls") {
      const known = new Map(((value as { call: string; warning: string | null }[]) ?? []).map((c) => [c.call, c]));
      for (const c of opened.calls) if (!known.has(c)) known.set(c, { call: c, warning: null });
      return json(200, [...known.values()].sort((a, b) => a.call.localeCompare(b.call)));
    }
    return json(200, value);
  };
  const entry = async (req: Request): Promise<Response> => {
    if (!me) return json(401, { code: "login" });
    const fd = await req.formData();
    const form = JSON.parse(String(fd.get("form") ?? "{}")) as { station?: { mail?: string } };
    form.station = { ...(form.station ?? {}), mail: me.email };
    fd.set("form", JSON.stringify(form));
    const files: { name: string; data: string }[] = [];
    let size = 0;
    for (const [name, value] of fd.entries()) {
      if (name !== "file" || typeof value === "string") continue;
      const data = new Uint8Array(await value.arrayBuffer());
      size += data.length;
      files.push({ name: value.name, data: gas.base64(data) });
    }
    if (size > 20 * 1024 * 1024) return json(413, { code: "large" });
    const out = local("POST", "/api/entry", ...(await multipart(fd)));
    if (out.status !== 200) return new Response(out.body, { status: out.status, headers: out.headers });
    const done = parse(out);
    try {
      const entries = (done.entries as { sect: string; city: string }[] | undefined) ?? [];
      const sections = entries.map((x) => x.sect + (x.city ? " (運用地 " + x.city + ")" : ""));
      const saved = await gas.call<{ id: string; mailed: boolean }>("submit", { call: done.call, form: JSON.stringify(form), files, sections });
      await remember(saved.id);
      done.mail = saved.mailed;
    } catch (e) {
      const code = e instanceof Error ? e.message : "server";
      return json(code === "login" ? 401 : 403, { code: ["closed", "owner", "login"].includes(code) ? code : "server" });
    }
    await sync(m, false);
    return json(200, done);
  };
  (globalThis as { ats5Site?: unknown }).ats5Site = {
    contests: CONTESTS,
    rule: () => rule,
    choose: async (next: string): Promise<void> => {
      await gas.call("setup", { rule: next, start: 0, deadline: 0 });
      location.reload();
    },
    publish: async (): Promise<void> => {
      const out: { [name: string]: unknown } = {};
      for (const name of ["results", "calls", "notes"]) out[name] = parse(local("GET", "/api/" + name));
      await gas.call("publish", { shared: out });
    },
    login: (el: HTMLElement): Promise<void> => gas.button(el),
    signOut: async (): Promise<void> => {
      await gas.signOut();
      location.reload();
    },
  };
  let queue = Promise.resolve();
  const transport = async (input: string, init?: RequestInit): Promise<Response> => {
    const req = new Request(new URL(input, location.href), init);
    const url = new URL(req.url);
    const path = url.pathname;
    if (req.method === "POST" && path === "/api/entry") return entry(req);
    if (!admin && req.method === "GET" && ["/api/results", "/api/calls", "/api/notes"].includes(path)) return shared(path.slice(5));
    const body = new Uint8Array(await req.arrayBuffer());
    const call = async (): Promise<Response> => {
      const out = local(req.method, path + url.search, req.headers.get("content-type") ?? "", body);
      if (req.method !== "GET") await sync(m, false);
      return new Response(out.body.length ? out.body : null, { status: out.status, headers: out.headers });
    };
    const result = queue.then(call);
    queue = result.then(() => undefined, () => undefined);
    return result;
  };
  (globalThis as { ats5Fetch?: unknown }).ats5Fetch = transport;
  await import("./app.js");
}

void start();
