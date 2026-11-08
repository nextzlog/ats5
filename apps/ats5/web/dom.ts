import { t } from "./i18n.js";

export type Kid = Node | string | number | null | undefined | false | Kid[];

type Value = string | number | boolean | null | undefined | EventListener | ((e: never) => unknown);

export type Attrs = Record<string, Value>;

export function $<T extends Element = HTMLElement>(sel: string, root: ParentNode = document): T | null {
  return root.querySelector<T>(sel);
}

function flat(kids: Kid[]): (Node | string)[] {
  const out: (Node | string)[] = [];
  for (const kid of kids) {
    if (kid === null || kid === undefined || kid === false) continue;
    if (Array.isArray(kid)) out.push(...flat(kid));
    else out.push(kid instanceof Node ? kid : String(kid));
  }
  return out;
}

export function h<K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Attrs = {}, ...kids: Kid[]): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === null || v === undefined || v === false) continue;
    if (typeof v === "function") el.addEventListener(k.slice(2), v as EventListener);
    else if (k === "html") el.innerHTML = String(v);
    else if (v === true) el.setAttribute(k, "");
    else el.setAttribute(k, String(v));
  }
  el.append(...flat(kids));
  return el;
}

export function fill<T extends Element>(el: T | null, ...kids: Kid[]): T | null {
  el?.replaceChildren(...flat(kids));
  return el;
}

export function toast(text: string): void {
  const el = h("div", { class: "toast", role: "status" }, text);
  document.body.append(el);
  setTimeout(() => el.remove(), 2600);
}

export function page(title: string, lead: string | null, ...body: Kid[]): void {
  fill($("#content"), h("h1", {}, title), lead ? h("p", { class: "lead" }, lead) : null, ...body);
}

export function notice(kind: "info" | "ok" | "warn" | "bad", ...kids: Kid[]): HTMLDivElement {
  return h("div", { class: "notice " + kind, role: kind === "bad" ? "alert" : "status" }, ...kids);
}

interface FieldOptions {
  textarea?: boolean;
  type?: string;
  auto?: string;
  mono?: boolean;
  inputmode?: string;
  required?: boolean;
  hint?: string;
  wide?: boolean;
}

export function field(name: string, label: string, opts: FieldOptions = {}): HTMLDivElement {
  const id = "f-" + name.replace(/\W/g, "-");
  const described = id + "-hint " + id + "-err";
  const input = opts.textarea
    ? h("textarea", { id, name, "aria-describedby": described, rows: 3 })
    : h("input", { id, name, type: opts.type || "text", autocomplete: opts.auto || "off", class: opts.mono ? "mono" : null, inputmode: opts.inputmode || null, "aria-describedby": described, required: opts.required || null });
  return h("div", { class: "field" + (opts.wide ? " wide" : ""), "data-field": name },
    h("label", { for: id }, label, opts.required ? h("span", { class: "req" }, t("required")) : null),
    input,
    opts.hint ? h("span", { class: "hint", id: id + "-hint" }, opts.hint) : null,
    h("span", { class: "error", id: id + "-err" }));
}

export type Head = string | { label: string; num: boolean };

export function table(head: Head[], rows: Kid[]): HTMLDivElement {
  const cell = (x: Head) => typeof x === "string" ? h("th", {}, x) : h("th", { class: x.num ? "num" : null }, x.label);
  return h("div", { class: "table-wrap" }, h("table", {},
    h("thead", {}, h("tr", {}, ...head.map(cell))),
    h("tbody", {}, ...rows)));
}

export function filterBox(onInput: (value: string) => void): HTMLInputElement {
  const input = h("input", { type: "search", placeholder: t("search"), "aria-label": t("search"), class: "mono", style: "max-width:280px" });
  input.addEventListener("input", () => onInput(input.value.toUpperCase()));
  return input;
}

export function value(sel: string): string {
  return $<HTMLInputElement>(sel)?.value || "";
}

export function refresh(): void {
  window.dispatchEvent(new Event("ats5:refresh"));
}
