import { api } from "../api.js";
import { $, fill, h, notice } from "../dom.js";
import { t } from "../i18n.js";
import { contest } from "../state.js";
import type { Note } from "../types.js";

export function home(main: HTMLElement): void {
  const c = contest();
  const status = c.finish ? ["accent", t("finished")] : c.accept ? ["ok", t("open")] : ["warn", t("closed")];
  fill(main,
    h("section", { class: "card hero" },
      h("h1", {}, c.name),
      h("p", { class: "lead" }, t("welcome") + " · " + c.host),
      h("div", { class: "dates" },
        h("span", { class: "pill " + status[0] }, status[1]),
        h("span", { class: "pill" }, t("held") + " " + c.start),
        h("span", { class: "pill" }, t("deadline") + " " + c.deadline)),
      h("div", { class: "actions" },
        c.accept ? h("a", { class: "button primary big", href: "#/entry" }, t("submit_now")) : null,
        h("a", { class: "button big", href: "#/results" }, t("see_results")),
        h("a", { class: "button big", href: c.link, rel: "noopener" }, t("rules")))),
    c.mock ? notice("warn", t("mock")) : null,
    h("div", { id: "voice" }),
    h("p", { class: "promo" }, h("a", { href: "https://zlog.org", rel: "noopener" }, t("zlog"))),
  );
  void api<Note[]>("/notes").then(({ data }) => {
    if (!data || !data.length) return;
    const v = data[Math.floor(Math.random() * data.length)];
    if (v) fill($("#voice"), h("blockquote", { class: "voice" }, v.note, h("footer", {}, v.call)));
  });
}
