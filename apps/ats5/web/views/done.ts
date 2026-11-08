import { fill, h, notice } from "../dom.js";
import { t } from "../i18n.js";
import type { Accepted } from "../types.js";

export const DONE = "ats5-done";

function read(): Partial<Accepted> {
  try {
    return JSON.parse(sessionStorage.getItem(DONE) || "{}") as Partial<Accepted>;
  } catch {
    return {};
  }
}

export function done(main: HTMLElement): void {
  const data = read();
  if (data.uuid) {
    fetch("http://localhost:8873?id=" + encodeURIComponent(data.uuid), { mode: "no-cors" }).catch(() => undefined);
  }
  const call = data.call || "";
  fill(main,
    notice("ok", h("h2", {}, t("done_title")), h("p", {}, t("done_lead"))),
    data.mail === false ? notice("warn", t("done_mail_failed")) : null,
    data.unreadable?.length
      ? notice("warn", h("strong", {}, t("done_unreadable")), h("ul", {}, ...data.unreadable.map((f) => h("li", { class: "mono" }, f))),
        h("p", {}, t("done_unreadable_hint"), " ", h("a", { href: "mailto:" + (data.contact ?? "") }, data.contact ?? "")))
      : null,
    h("section", { class: "card" },
      h("h2", { class: "mono" }, call),
      h("p", {}, (data.items ?? 0) + " " + t("qsos")),
      h("ul", {}, ...(data.entries || []).map((e) => h("li", {}, e.sect, e.city ? " · " + e.city : ""))),
      h("div", { class: "actions", style: "justify-content:flex-start" },
        h("a", { class: "button primary", href: "/proof/" + encodeURIComponent(call) }, t("done_proof")),
        h("a", { class: "button", href: "/" }, t("home")))));
}
