import { api, loginUrl } from "./api.js";
import { $, download, fill, h, here as path, internal, navigate, notice, page, site, SITE, toast } from "./dom.js";
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
  const links: [string, string][] = c?.user?.admin
    ? [["/admin", "admin"], ["/lists", "results"], ["/admin/calls", "calls"], ["/admin/entry", "entry"], ["/admin/email", "sendmail"]]
    : [["/entry", "entry"], ["/lists", "results"], ["/calls", "calls"], ["/forum", "voices"], ["/guide", "guide"]];
  if (c?.board) links.push(["/board", "board"]);
  if (c && !c.user?.admin) links.push(["mailto:" + c.mail, "contact"]);
  const here = path();
  const current = (href: string) => href === here || (href !== "/admin" && here.startsWith(href + "/"));
  fill($("#nav"), ...links.map(([href, key]) => h("a", { href, "aria-current": current(href) ? "page" : null }, t(key))));
  fill($("#brand"), "ATS-5");
  const people = c?.contributors || [];
  fill($("#contributors"), people.length ? "Contributed by " + people.join(", ") + "." : null);
  const user = c?.user;
  const toggle = () => { setLang(getLang() === "ja" ? "en" : "ja"); void route(); };
  if (SITE) {
    const out = h("button", { class: "small" }, t("logout"));
    out.addEventListener("click", () => void site()?.signOut());
    fill($("#nav"), ...links.filter(([href]) => href !== "/admin/email").map(([href, key]) => h("a", { href, "aria-current": current(href) ? "page" : null }, t(key))));
    fill($("#tools"),
      h("button", { class: "small ghost", onclick: toggle, "aria-label": "language" }, t("language")),
      user && !user.admin ? h("a", { class: "button small", href: "/mine", title: user.email }, t("mine")) : null,
      user ? out : h("a", { class: "button small", href: "/login" }, t("login")));
    document.title = (c ? c.name + " · " : "") + "ATS-5";
    return;
  }
  fill($("#tools"),
    h("button", { class: "small ghost", onclick: toggle, "aria-label": "language" }, t("language")),
    user ? h("a", { class: "button small", href: "/mine", title: user.email }, t("mine")) : null,
    user ? h("a", { class: "button small", href: "/logout" }, t("logout"))
      : c?.login ? h("a", { class: "button small", href: loginUrl() }, t("login")) : null,
  );
  document.title = (c ? c.name + " · " : "") + "ATS-5";
}

async function show(main: HTMLElement, parts: string[]): Promise<void> {
  switch (parts[0]) {
    case "": return home(main);
    case "entry": return entryPage(main);
    case "done": return done(main);
    case "lists": return results();
    case "calls": return calls();
    case "forum": return voices();
    case "guide": return guide();
    case "board": return board();
    case "proof": return proof(main, parts[1] || "");
    case "mine": return mine();
    case "admin":
      if (parts[1] === "proof") return adminStation(parts[2] || "");
      if (parts[1] === "amend") return adminAmend(main, parts[2] || "");
      if (parts[1] === "email") return adminMail();
      if (parts[1] === "entry") return entryPage(main, null, true);
      if (parts[1] === "calls") return calls();
      return admin();
    case "login": return signIn();
    case "login-failed": return page(t("login"), null, notice("bad", t("login_failed")));
    case "login-unavailable": return page(t("login"), null, notice("warn", t("login_unavailable")));
    default: return page("404", null, notice("warn", t("missing")));
  }
}

function signIn(): void {
  const go = h("div", { class: "gsi" });
  page(t("login"), null, h("p", { class: "lead" }, t("login_site")), go);
  site()?.login(go).catch((e: Error) => toast(t("login_failed") + " " + e.message));
}

async function route(): Promise<void> {
  const current = path();
  const parts = current.slice(1).split("/").map(decodeURIComponent);
  const main = $("#content");
  if (!main) return;
  if (!current.startsWith("/board")) {
    state.stream?.close();
    state.stream = null;
  }
  if (SITE) state.contest = (await api<Contest>("/contest")).data;
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
  window.addEventListener(SITE ? "hashchange" : "popstate", () => void route());
  let last = window.scrollY;
  window.addEventListener("scroll", () => {
    const y = window.scrollY;
    const header = $("header.top");
    if (header && !header.contains(document.activeElement)) header.classList.toggle("away", y > last && y > 80);
    last = y;
  }, { passive: true });
  document.addEventListener("click", (e) => {
    const a = (e.target as Element | null)?.closest("a");
    if (!a || e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    const url = new URL(a.href, location.href);
    if (url.origin !== location.origin || a.target || a.hasAttribute("download")) return;
    const target = a.getAttribute("href") ?? "";
    if (SITE && (target.startsWith("/admin/excel") || target.startsWith("/admin/table/"))) {
      e.preventDefault();
      void download(target);
      return;
    }
    if (!internal(url.pathname) || !target.startsWith("/")) return;
    e.preventDefault();
    navigate(target);
  });
  window.addEventListener("ats5:refresh", () => void route());
  await route();
}

void start();
