import { upload } from "../api.js";
import { readLog } from "../check.js";
import { fill, h, notice, SITE } from "../dom.js";
import type { Kid } from "../dom.js";
import { t } from "../i18n.js";
import type { CheckReport, Entry, Preview, Trial } from "../types.js";

export interface Attached {
  file: File;
  buffer: ArrayBuffer;
  sheet?: boolean;
  items?: number;
  error?: string | null;
}

export async function check(files: Attached[], entries: Entry[]): Promise<{ report: CheckReport; fallback: boolean }> {
  let fallback = false;
  for (const f of files) {
    const r = await readLog(f.buffer);
    if (r === null) {
      fallback = true;
      continue;
    }
    const bad = r.error !== null && !r.items;
    f.items = bad ? undefined : r.items;
    f.error = bad ? r.error : null;
  }
  const fd = new FormData();
  fd.append("form", JSON.stringify({ station: {}, entries }));
  files.filter((f) => !f.error).forEach((f) => fd.append("file", f.file, f.file.name));
  const data = (await upload<Trial>("/trial", fd)).data ?? { files: [], sections: [], conflict: false, reasons: [] };
  if (fallback) {
    const good = files.filter((f) => !f.error);
    data.files.forEach((x, i) => {
      const f = good[i];
      if (f) {
        f.items = x.items;
        f.error = x.error ?? null;
      }
    });
  }
  return { report: { sections: data.sections, conflict: data.conflict, reasons: data.reasons }, fallback: fallback && !SITE };
}

function section(s: Preview): HTMLElement {
  const item = (label: string, value: string | number) => h("div", { class: "item" }, h("div", { class: "label" }, label), h("div", { class: "num" }, value));
  const rejected = s.rejected.length
    ? h("details", {}, h("summary", {}, t("invalid_list")),
      h("div", { class: "table-wrap" }, h("table", {}, h("tbody", {}, ...s.rejected.map((c) =>
        h("tr", {}, h("td", { class: "mono" }, c.time), h("td", { class: "mono" }, c.call), h("td", {}, c.band), h("td", {}, c.mode)))))))
    : null;
  return h("div", { class: "score" }, h("h3", {}, s.sect),
    h("div", { class: "row" }, item(t("valid"), s.valid), item(t("invalid"), s.invalid), item(t("total"), s.total.toLocaleString())),
    rejected);
}

export function show(box: HTMLElement, report: CheckReport, fallback: boolean): void {
  const blocks: Kid[] = [];
  if (fallback) blocks.push(h("p", { class: "muted small" }, t("browser_off")));
  if (report.conflict) blocks.push(notice("bad", h("strong", {}, t("e_conflict")), h("ul", {}, ...report.reasons.map((r) => h("li", {}, r)))));
  blocks.push(...report.sections.map(section));
  if (!report.sections.length && !report.conflict) blocks.push(h("p", { class: "muted small" }, t("preview_empty")));
  fill(box, ...blocks);
}
