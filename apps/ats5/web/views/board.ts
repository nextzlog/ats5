import { api } from "../api.js";
import { fill, h, page, table } from "../dom.js";
import { t } from "../i18n.js";
import { state } from "../state.js";
import type { Board } from "../types.js";

type Filter = { area: string; band: string; sect: string };

export function areaOf(call: string): string {
  const [base = "", port] = call.split("/");
  if (port && /^\d$/.test(port)) return port;
  const m = base.match(/(\d)(?!.*\d)/);
  return m?.[1] ?? "";
}

function render(data: Board, filter: Filter, controls: HTMLElement, body: HTMLElement, again: () => void): void {
  const sel = (key: keyof Filter, label: string, options: string[]) => {
    const select = h("select", {}, h("option", { value: "" }, t("all")), ...options.map((o) => h("option", { value: o, selected: filter[key] === o }, o)));
    select.addEventListener("change", () => { filter[key] = select.value; again(); });
    return h("label", { class: "field" }, h("span", { class: "small muted" }, label), select);
  };
  const bands = [...new Set(data.reports.flatMap((r) => Object.keys(r.bands)))].sort((a, b) => parseFloat(a) - parseFloat(b));
  const areas = [...new Set([...data.reports.map((r) => r.area), ...data.standings.flatMap((s) => s.places.map((p) => areaOf(p.call)))])].filter(Boolean).sort();
  fill(controls, sel("area", t("area"), areas), sel("band", t("band"), bands), sel("sect", t("section"), data.standings.map((s) => s.name)));
  const reports = data.reports.filter((r) => !filter.area || r.area === filter.area)
    .map((r) => ({ ...r, key: filter.band ? (r.bands[filter.band] ?? 0) : r.score }))
    .filter((r) => !filter.band || r.key > 0)
    .sort((a, b) => b.key - a.key);
  const standings = data.standings.filter((s) => !filter.sect || s.name === filter.sect);
  const reported = reports.length
    ? table(["#", t("call"), t("class"), { label: t("qso"), num: true }, { label: t("multi"), num: true }, { label: filter.band || t("total"), num: true }, t("updated")],
      reports.map((r, i) => h("tr", {}, h("td", {}, i + 1), h("td", { class: "mono" }, r.call),
        h("td", { class: "small" }, [r.class.ops, r.class.power, r.class.mode, r.class.bands].filter(Boolean).join(" ")),
        h("td", { class: "num" }, filter.band ? (r.bands[filter.band] ?? 0) : r.qso), h("td", { class: "num" }, r.mult),
        h("td", { class: "num" }, r.key.toLocaleString()), h("td", { class: "small muted" }, new Date(r.time * 1000).toLocaleTimeString()))))
    : h("p", { class: "muted" }, "—");
  fill(body,
    h("section", { class: "card" }, h("header", {}, h("h2", {}, t("reported")), h("span", { class: "live" }, t("live"))), reported),
    ...standings.map((s) => h("section", { class: "card" }, h("h2", {}, t("tabulated") + " · " + s.name),
      table([t("rank"), t("call"), { label: t("score"), num: true }, { label: t("total"), num: true }, t("city")],
        s.places.filter((p) => !filter.area || areaOf(p.call) === filter.area).map((p) => h("tr", {}, h("td", {}, p.rank),
          h("td", { class: "mono" }, p.call), h("td", { class: "num" }, p.score), h("td", { class: "num" }, p.total.toLocaleString()), h("td", {}, p.city)))))));
}

export async function board(): Promise<void> {
  const filter: Filter = { area: "", band: "", sect: "" };
  const body = h("div");
  const controls = h("div", { class: "row" });
  let data: Board | null = null;
  const again = () => { if (data) render(data, filter, controls, body, again); };
  const load = async () => { data = (await api<Board>("/board")).data; again(); };
  page(t("board"), t("board_lead"), controls, h("div", { style: "height:12px" }), body);
  await load();
  state.stream?.close();
  const stream = new EventSource("/api/board/events");
  state.stream = stream;
  let pending: number | undefined;
  stream.addEventListener("update", () => {
    clearTimeout(pending);
    pending = window.setTimeout(() => void load(), 500);
  });
}
