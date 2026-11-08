import { api, loginUrl } from "../api.js";
import { fill, filterBox, h, notice, page, refresh, table, toast } from "../dom.js";
import { t } from "../i18n.js";
import { contest } from "../state.js";
import type { AdminStation, Amend, Failure, Past, StationDetail } from "../types.js";
import { entryPage } from "./entry.js";
import { proofView } from "./proof.js";

function failed(title: string, data: unknown): void {
  const code = (data as Failure | null)?.code || "server";
  page(title, null, notice("bad", t("e_" + code)));
}

function stationRow(s: AdminStation): HTMLTableRowElement {
  return h("tr", { class: s.flags.length ? "flag" : null },
    h("td", { class: "mono" }, h("a", { href: "#/admin/station/" + encodeURIComponent(s.call) }, s.call)),
    h("td", {}, s.name), h("td", { class: "small" }, s.mail),
    h("td", { class: "small" }, new Date(s.time * 1000).toLocaleString()), h("td", { class: "num" }, s.history),
    h("td", {}, ...s.flags.map((f) => h("span", { class: "pill warn" }, t("f_" + f))), s.scoreless ? h("span", { class: "pill bad" }, t("scoreless")) : null));
}

async function clearAll(): Promise<void> {
  const v = prompt(t("confirm_clear"));
  if (v === null) return;
  const r = await api("/admin/clear", { method: "POST", json: { confirm: v } });
  toast(r.ok ? t("done") : t("e_request"));
  refresh();
}

async function recalc(): Promise<void> {
  await api("/admin/recalc", { method: "POST" });
  toast(t("done"));
  refresh();
}

export async function admin(): Promise<void> {
  const { status, data } = await api<AdminStation[]>("/admin/stations");
  if (status === 401) {
    location.href = loginUrl();
    return;
  }
  if (status !== 200 || !data) return failed(t("admin"), data);
  let q = "";
  const body = h("div");
  const render = (): void => {
    fill(body, table([t("call"), t("name"), t("mail"), t("time"), t("history"), t("flags")], data.filter((s) => s.call.includes(q)).map(stationRow)));
  };
  const kpi = (label: string, value: number) => h("div", { class: "kpi" }, h("div", { class: "label" }, label), h("div", { class: "value" }, value));
  page(t("admin"), contest().name,
    h("div", { class: "kpis" },
      kpi(t("stations"), data.length),
      kpi(t("flagged"), data.filter((s) => s.flags.length).length),
      kpi(t("scoreless"), data.filter((s) => s.scoreless).length)),
    h("div", { class: "row", style: "margin-bottom:16px" },
      h("a", { class: "button primary", href: "#/entry" }, "+ " + t("entry")),
      h("button", { onclick: () => void recalc() }, t("recalc")),
      h("a", { class: "button", href: "/api/admin/csv" }, t("csv")),
      h("a", { class: "button", href: "#/admin/mail" }, t("sendmail")),
      h("button", { class: "danger", onclick: () => void clearAll() }, t("clear"))),
    filterBox((v) => { q = v; render(); }), h("div", { style: "height:12px" }), body);
  render();
}

function historyRow(x: Past): HTMLTableRowElement {
  const s = x.submission;
  const revert = async () => { await api("/admin/revert/" + s.id, { method: "POST" }); toast(t("done")); refresh(); };
  const ack = async () => { await api("/admin/flags/" + s.id, { method: "POST", json: { flags: [] } }); refresh(); };
  return h("tr", { class: s.active ? "award" : s.flags.length ? "flag" : null },
    h("td", { class: "small" }, new Date(s.time * 1000).toLocaleString()), h("td", { class: "small" }, s.station.mail), h("td", { class: "small mono" }, s.peer),
    h("td", { class: "small" }, x.entries.map((e) => e.sect).join(" / ")),
    h("td", {}, ...x.files.map((f) => h("a", { class: "button small", href: "/api/admin/file/" + f.id }, f.file || "manual"))),
    h("td", {}, ...s.flags.map((f) => h("span", { class: "pill warn" }, t("f_" + f)))),
    h("td", {}, s.active ? h("span", { class: "pill ok" }, t("current")) : h("button", { class: "small", onclick: () => void revert() }, t("revert")),
      s.flags.length ? h("button", { class: "small ghost", onclick: () => void ack() }, t("ack")) : null));
}

export async function adminStation(call: string): Promise<void> {
  const { status, data } = await api<StationDetail>("/admin/station/" + encodeURIComponent(call));
  if (status !== 200 || !data) return failed(call, data);
  const remove = async () => {
    if (!confirm(t("confirm_delete"))) return;
    await api("/admin/station/" + encodeURIComponent(call), { method: "DELETE" });
    location.hash = "#/admin";
  };
  page(call, null,
    h("div", { class: "row", style: "margin-bottom:16px" },
      h("a", { class: "button", href: "#/admin" }, "← " + t("back")),
      h("button", { class: "danger", onclick: () => void remove() }, t("delete"))),
    h("section", { class: "card" }, h("h2", {}, t("history")), table([t("time"), t("mail"), t("peer"), t("section"), t("files"), t("flags"), ""], data.history.map(historyRow))),
    ...(data.current ? proofView(data.current, true) : []));
}

export async function adminAmend(main: HTMLElement, call: string): Promise<void> {
  const { status, data } = await api<Amend>("/admin/form/" + encodeURIComponent(call));
  if (status !== 200 || !data) return failed(call, data);
  entryPage(main, data);
}

export function adminMail(): void {
  const c = contest();
  const to = h("input", { id: "m-to", value: c.mail, type: "text" });
  const subject = h("input", { id: "m-sub" });
  const body = h("textarea", { id: "m-body", rows: 10 }, "\n\n---\n" + c.host + "\n" + c.name + "\n" + c.link + "\n" + c.mail);
  const send = h("button", { class: "primary" }, t("send"));
  send.addEventListener("click", async () => {
    send.disabled = true;
    const r = await api<{ mock?: boolean } & Failure>("/admin/mail", { method: "POST", json: { to: to.value, subject: subject.value, body: body.value } });
    send.disabled = false;
    toast(r.ok ? t("sent_ok") + (r.data?.mock ? " (mock)" : "") : t("e_" + (r.data?.code || "server")));
  });
  page(t("sendmail"), null, c.mock ? notice("warn", t("mock")) : null,
    h("section", { class: "card" }, h("div", { class: "grid" },
      h("div", { class: "field wide" }, h("label", { for: "m-to" }, t("to")), to),
      h("div", { class: "field wide" }, h("label", { for: "m-sub" }, t("subject")), subject),
      h("div", { class: "field wide" }, h("label", { for: "m-body" }, t("body")), body)),
      h("div", { class: "row", style: "margin-top:12px" }, send)));
}
