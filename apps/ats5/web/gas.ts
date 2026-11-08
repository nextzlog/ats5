const CLIENT = "328148444857-pcp6epd21lqqestklbfmgl1cbksku469.apps.googleusercontent.com";
const KEY = "ats5.token";

export interface Saved {
  id: string;
  call: string;
  email: string;
  time: number;
  form: string;
  files: { name: string; data: string }[];
}

export interface Opened {
  rule: string;
  name: string;
  start: number;
  deadline: number;
  calls: string[];
  shared: { [name: string]: unknown };
}

interface Identity {
  accounts: {
    id: {
      initialize(config: { client_id: string; callback: (r: { credential: string }) => void; auto_select?: boolean }): void;
      renderButton(el: HTMLElement, options: object): void;
      disableAutoSelect(): void;
    };
  };
}

export function script(): string {
  return new URLSearchParams(location.search).get("s") ?? "";
}

function endpoint(): string {
  return "https://script.google.com/macros/s/" + encodeURIComponent(script()) + "/exec";
}

function claims(token: string): { email?: string; exp?: number } {
  try {
    const part = token.split(".")[1] ?? "";
    const text = atob(part.replace(/-/g, "+").replace(/_/g, "/"));
    return JSON.parse(new TextDecoder().decode(Uint8Array.from(text, (c) => c.charCodeAt(0)))) as { email?: string; exp?: number };
  } catch {
    return {};
  }
}

export function token(): string | null {
  try {
    const t = localStorage.getItem(KEY);
    if (!t) return null;
    const exp = claims(t).exp ?? 0;
    if (exp * 1000 < Date.now() + 60000) {
      localStorage.removeItem(KEY);
      return null;
    }
    return t;
  } catch {
    return null;
  }
}

export function email(): string | null {
  const t = token();
  return t ? (claims(t).email ?? "").toLowerCase() || null : null;
}

let library: Promise<Identity> | null = null;

function identity(): Promise<Identity> {
  library ??= new Promise((resolve, reject) => {
    const s = document.createElement("script");
    s.src = "https://accounts.google.com/gsi/client";
    s.onload = () => {
      const g = (globalThis as { google?: Identity }).google;
      if (!g) return reject(new Error("gsi"));
      g.accounts.id.initialize({
        client_id: CLIENT,
        callback: (r) => {
          try {
            localStorage.setItem(KEY, r.credential);
          } catch {
            return;
          }
          location.reload();
        },
      });
      resolve(g);
    };
    s.onerror = () => reject(new Error("gsi"));
    document.head.append(s);
  });
  return library;
}

export async function button(el: HTMLElement): Promise<void> {
  const g = await identity();
  g.accounts.id.renderButton(el, { type: "standard", theme: "outline", size: "large", text: "signin_with", locale: document.documentElement.lang });
}

export async function signOut(): Promise<void> {
  try {
    localStorage.removeItem(KEY);
  } catch {
    return;
  }
  const g = await identity().catch(() => null);
  g?.accounts.id.disableAutoSelect();
}

export async function call<T>(op: string, body: object = {}): Promise<T> {
  const res = await fetch(endpoint(), {
    method: "POST",
    headers: { "content-type": "text/plain;charset=utf-8" },
    body: JSON.stringify({ op, token: token(), ...body }),
  });
  const out = (await res.json()) as { ok: boolean; data?: T; code?: string };
  if (!out.ok) throw new Error(out.code ?? "server");
  return out.data as T;
}

export function base64(data: Uint8Array): string {
  let text = "";
  for (let i = 0; i < data.length; i += 0x8000) text += String.fromCharCode(...data.subarray(i, i + 0x8000));
  return btoa(text);
}

export function bytes(data: string): Uint8Array {
  return Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
}
