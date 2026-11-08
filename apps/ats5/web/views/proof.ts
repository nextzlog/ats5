import { api, loginUrl } from "../api.js";
import { fill, h, notice, page, table } from "../dom.js";
import type { Kid } from "../dom.js";
import { t } from "../i18n.js";
import { contest } from "../state.js";
import type { Mine, Proof, Row, Sheet } from "../types.js";

function bandCounts(map: Record<string, number>): HTMLElement {
  const entries = Object.entries(map);
  if (!entries.length) return h("span", { class: "muted" }, "0");
  return h("div", { class: "row" }, ...entries.map(([b, n]) => h("span", { class: "pill" }, b + " " + n)));
}

function contactRow(r: Row, bad: boolean, admin: boolean): HTMLTableRowElement {
  const verdict = bad
    ? h("span", { class: "pill bad" }, admin && r.text ? r.text : t("invalid"))
    : h("span", { class: "pill ok" }, t("valid"));
  return h("tr", { class: bad ? "rejected" : null },
    h("td", { class: "mono" }, r.time), h("td", { class: "mono" }, r.call), h("td", {}, r.band), h("td", {}, r.mode),
    h("td", { class: "mono" }, r.rcvd), h("td", { class: "mono" }, r.sent), h("td", {}, r.name),
    h("td", {}, r.sign ? "✓" : "-"), h("td", {}, verdict));
}

function sheetView(s: Sheet, admin: boolean): HTMLElement {
  if (s.absence) return h("section", { class: "card" }, h("h3", {}, s.sect));
  const rows = [...s.accepted.map((r) => contactRow(r, false, admin)), ...s.rejected.map((r) => contactRow(r, true, admin))];
  return h("section", { class: "card" },
    h("header", {}, h("h2", {}, s.sect), h("span", { class: "muted" }, s.city || "")),
    admin ? h("div", { class: "kpis" },
      h("div", { class: "kpi" }, h("div", { class: "label" }, t("score")), h("div", { class: "value" }, s.score ?? 0)),
      h("div", { class: "kpi" }, h("div", { class: "label" }, t("total")), h("div", { class: "value" }, (s.total ?? 0).toLocaleString()))) : null,
    h("h3", {}, t("by_band")),
    h("div", { class: "grid" },
      h("div", {}, h("div", { class: "muted small" }, t("valid")), bandCounts(s.valid)),
      h("div", {}, h("div", { class: "muted small" }, t("invalid")), bandCounts(s.invalid))),
    h("details", { style: "margin-top:12px" }, h("summary", {}, t("contacts") + " (" + rows.length + ")"),
      table([t("time"), t("call"), t("band"), t("mode"), t("rcvd"), t("sent"), "OP", t("sign"), admin ? t("reason") : ""], rows)));
}

export function proofView(p: Proof, admin: boolean): Kid[] {
  const st = p.submission.station;
  const files = p.files.map((f) => h("li", { class: f.error !== undefined ? "bad" : null },
    h("span", { class: "name" }, f.file || "manual"),
    f.error !== undefined ? h("span", { class: "pill bad" }, t("unreadable")) : h("span", { class: "pill ok" }, (f.items ?? 0) + " " + t("qsos")),
    admin ? h("a", { class: "button small", href: "/api/admin/file/" + f.id }, t("download")) : null,
    admin && f.error ? h("pre", { class: "small" }, f.error) : null));
  return [
    h("section", { class: "card" },
      h("header", {}, h("h2", { class: "mono" }, st.call), admin ? h("a", { class: "button small", href: "#/admin/amend/" + encodeURIComponent(st.call) }, t("amend")) : null),
      h("p", {}, st.name, h("br"), "〒" + st.post + " " + st.addr, h("br"), st.mail),
      st.note ? h("blockquote", { class: "voice" }, st.note) : null),
    h("section", { class: "card" }, h("h2", {}, t("files")), h("ul", { class: "files" }, ...files)),
    ...p.sheets.map((s) => sheetView(s, admin)),
  ];
}

export async function proof(main: HTMLElement, call: string): Promise<void> {
  const c = contest();
  fill(main, h("p", { class: "muted" }, t("checking")));
  const { status, data } = await api<Proof>("/proof/" + encodeURIComponent(call));
  if (status === 401) {
    page(t("proof_title"), null, notice("info", h("p", {}, t("proof_login")),
      c.login ? h("a", { class: "button primary", href: loginUrl() }, t("login")) : h("p", {}, t("login_unavailable"))));
    return;
  }
  if (status === 403) {
    page(t("proof_title"), null, notice("warn", t("proof_owner"), " ", h("a", { href: "/logout" }, t("logout"))));
    return;
  }
  if (!data?.submission) {
    page(t("proof_title"), null, notice("warn", t("e_server")));
    return;
  }
  page(t("proof_title"), null, ...proofView(data, c.user?.admin ?? false));
}

export async function mine(): Promise<void> {
  const { status, data } = await api<Mine>("/mine");
  if (status === 401 || !data) {
    location.hash = "#/proof/";
    return;
  }
  const links = data.calls.map((c) => h("a", { class: "button mono", href: "#/proof/" + encodeURIComponent(c) }, c));
  page(t("my_calls"), data.user.email, links.length ? h("div", { class: "actions", style: "justify-content:flex-start" }, ...links) : notice("info", t("none")));
}
