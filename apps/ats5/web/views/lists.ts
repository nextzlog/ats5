import { api } from "../api.js";
import { fill, filterBox, h, notice, page, table } from "../dom.js";
import { t } from "../i18n.js";
import { contest } from "../state.js";
import type { CallInfo, Format, Note, Results } from "../types.js";

export function link(call: string): Node | string {
  return contest().user?.admin ? h("a", { href: "/admin/proof/" + encodeURIComponent(call) }, call) : call;
}

export async function results(): Promise<void> {
  const { data } = await api<Results>("/results");
  if (!data?.open) {
    page(t("results"), null, notice("info", t("results_closed")));
    return;
  }
  let q = "";
  let tab = 0;
  const body = h("div");
  const tabs = h("div", { class: "tabs", role: "group" });
  const pick = (i: number) => () => { tab = i; render(); };
  const render = (): void => {
    fill(tabs, ...data.standings.map((s, i) => h("button", { class: "small", "aria-pressed": String(i === tab), onclick: pick(i) }, s.name)),
      data.checklogs.length ? h("button", { class: "small", "aria-pressed": String(tab === -1), onclick: pick(-1) }, t("checklog")) : null);
    if (tab === -1) {
      fill(body, table([t("call")], data.checklogs.filter((c) => c.includes(q)).map((c) => h("tr", {}, h("td", { class: "mono" }, link(c))))));
      return;
    }
    const s = data.standings[tab];
    if (!s) {
      fill(body);
      return;
    }
    fill(body, h("h2", {}, s.name), table([t("rank"), t("call"), { label: t("score"), num: true }, { label: t("total"), num: true }, t("city"), t("award")],
      s.places.filter((p) => p.call.includes(q)).map((p) => h("tr", { class: p.award ? "award" : null },
        h("td", {}, p.rank), h("td", { class: "mono" }, link(p.call)), h("td", { class: "num" }, p.score),
        h("td", { class: "num" }, p.total.toLocaleString()), h("td", {}, p.city), h("td", {}, p.award ? "🏆" : "")))));
  };
  page(t("results"), null, h("div", { class: "spread" }, tabs, filterBox((v) => { q = v; render(); })), body);
  render();
}

export async function calls(): Promise<void> {
  const data = (await api<CallInfo[]>("/calls")).data || [];
  let q = "";
  const body = h("div");
  const render = (): void => {
    fill(body, table(["#", t("call"), t("warning")], data.filter((c) => c.call.includes(q)).map((c, i) =>
      h("tr", { class: c.warning ? "flag" : null }, h("td", {}, i + 1), h("td", { class: "mono" }, link(c.call)), h("td", {}, c.warning ? t("w_" + c.warning) : "-")))));
  };
  page(t("calls"), data.length + " " + t("stations"), filterBox((v) => { q = v; render(); }), h("div", { style: "height:12px" }), body);
  render();
}

export async function voices(): Promise<void> {
  const data = (await api<Note[]>("/notes")).data || [];
  page(t("voices"), null, data.length
    ? h("div", { class: "voices" }, ...data.map((v) => h("blockquote", { class: "voice" }, v.note, h("footer", {}, v.call))))
    : notice("info", t("no_voice")));
}

function formats(title: string, list: Format[]): HTMLElement {
  return h("section", { class: "card" }, h("h2", {}, title),
    table([t("name"), t("extensions")], list.map((f) => h("tr", {}, h("td", {}, f.name), h("td", { class: "mono" }, f.extensions.map((e) => "." + e).join(" "))))));
}

export function guide(): void {
  const c = contest();
  page(t("guide"), null,
    h("section", { class: "card help-body", html: c.help }),
    c.media ? h("div", { class: "video" }, h("iframe", { src: c.media, title: "video", allowfullscreen: true, loading: "lazy" })) : null,
    formats(t("formats"), c.formats),
    formats(t("sheets"), c.sheets),
    h("section", { class: "card" }, h("h2", {}, t("n1mm")), h("p", { class: "mono" }, location.origin + "/board")));
}
