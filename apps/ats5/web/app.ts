import { api, loginUrl } from "./api.js";
import { $, fill, h, notice, page } from "./dom.js";
import { getLang, setLang, t } from "./i18n.js";
import { state } from "./state.js";
import type { Contest } from "./types.js";
import { admin, adminAmend, adminMail, adminStation } from "./views/admin.js";
import { board } from "./views/board.js";
import { done, entryPage } from "./views/entry.js";
import { home } from "./views/home.js";
import { calls, guide, results, voices } from "./views/lists.js";
import { mine, proof } from "./views/proof.js";

function chrome(): void {
  const c = state.contest;
  const links: [string, string][] = [["#/", "home"], ["#/entry", "entry"], ["#/results", "results"], ["#/board", "board"], ["#/calls", "calls"], ["#/voices", "voices"], ["#/guide", "guide"]];
  if (c?.user?.admin) links.push(["#/admin", "admin"]);
  const here = location.hash.split("/").slice(0, 2).join("/") || "#/";
  fill($("#nav"), ...links.map(([href, key]) => h("a", { href, "aria-current": here === href ? "page" : null }, t(key))));
  fill($("#brand"), "ATS-5");
  const people = c?.contributors || [];
  fill($("#contributors"), people.length ? "Contributed by " + people.join(", ") + "." : null);
  const user = c?.user;
  const toggle = () => { setLang(getLang() === "ja" ? "en" : "ja"); void route(); };
  fill($("#tools"),
    h("button", { class: "small ghost", onclick: toggle, "aria-label": "language" }, t("language")),
    user ? h("a", { class: "button small", href: "#/mine", title: user.email }, t("mine")) : null,
    user ? h("a", { class: "button small", href: "/logout" }, t("logout"))
      : c?.login ? h("a", { class: "button small", href: loginUrl() }, t("login")) : null,
  );
  document.title = (c ? c.name + " · " : "") + "ATS-5";
}

async function show(main: HTMLElement, parts: string[]): Promise<void> {
  switch (parts[0]) {
    case "entry": return entryPage(main);
    case "done": return done(main);
    case "results": return results();
    case "calls": return calls();
    case "voices": return voices();
    case "guide": return guide();
    case "board": return board();
    case "proof": return proof(main, parts[1] || "");
    case "mine": return mine();
    case "admin":
      if (parts[1] === "station") return adminStation(parts[2] || "");
      if (parts[1] === "amend") return adminAmend(main, parts[2] || "");
      if (parts[1] === "mail") return adminMail();
      return admin();
    case "login-failed": return page(t("login"), null, notice("bad", t("login_failed")));
    case "login-unavailable": return page(t("login"), null, notice("warn", t("login_unavailable")));
    default: return home(main);
  }
}

async function route(): Promise<void> {
  const hash = location.hash || "#/";
  const parts = hash.slice(2).split("/").map(decodeURIComponent);
  const main = $("#content");
  if (!main) return;
  if (!hash.startsWith("#/board")) {
    state.stream?.close();
    state.stream = null;
  }
  chrome();
  try {
    await show(main, parts);
  } catch (e) {
    page("ATS-5", null, notice("bad", t("e_server")));
    console.error(e);
  }
  main.focus({ preventScroll: true });
}

async function start(): Promise<void> {
  document.documentElement.lang = getLang();
  const { data } = await api<Contest>("/contest");
  state.contest = data;
  window.addEventListener("hashchange", () => { window.scrollTo(0, 0); void route(); });
  window.addEventListener("ats5:refresh", () => void route());
  await route();
}

void start();
