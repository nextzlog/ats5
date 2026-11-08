export interface Reply<T> {
  ok: boolean;
  status: number;
  data: T | null;
}

interface Options {
  method?: string;
  json?: unknown;
}

export async function api<T>(path: string, opts: Options = {}): Promise<Reply<T>> {
  const headers: Record<string, string> = { "x-ats5": "1" };
  let body: string | undefined;
  if (opts.json !== undefined) {
    headers["content-type"] = "application/json";
    body = JSON.stringify(opts.json);
  }
  const res = await fetch("/api" + path, { method: opts.method, body, headers, credentials: "same-origin" });
  const type = res.headers.get("content-type") || "";
  const data = type.includes("json") ? ((await res.json()) as T) : null;
  return { ok: res.ok, status: res.status, data };
}

export async function upload<T>(path: string, body: FormData | ArrayBuffer): Promise<Reply<T>> {
  const res = await fetch("/api" + path, { method: "POST", headers: { "x-ats5": "1" }, body, credentials: "same-origin" });
  const type = res.headers.get("content-type") || "";
  const data = type.includes("json") ? ((await res.json()) as T) : null;
  return { ok: res.ok, status: res.status, data };
}

export function loginUrl(): string {
  return "/login?next=" + encodeURIComponent("/" + location.hash);
}
