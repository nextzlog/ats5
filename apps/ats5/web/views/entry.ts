import { upload } from "../api.js";
import { $, field, fill, h, notice, page, toast, value } from "../dom.js";
import { t } from "../i18n.js";
import { contest } from "../state.js";
import type { Accepted, Amend, Failure, Problems, Unboxed } from "../types.js";
import { DONE } from "./done.js";
import { ManualTable } from "./manual.js";
import { check, show } from "./preview.js";
import type { Attached } from "./preview.js";
import { assign, buildSlots, chosen } from "./sections.js";

export { done } from "./done.js";

interface Kept {
  id: number;
  file: string;
  keep: boolean;
}

const STATION = ["call", "name", "post", "addr", "mail", "note"] as const;

function stationFields(): HTMLElement {
  return h("div", { class: "grid" },
    field("station.call", t("call"), { required: true, mono: true, hint: t("call_hint") }),
    field("station.name", t("name"), { required: true, hint: t("name_hint"), auto: "name" }),
    field("station.post", t("post"), { required: true, hint: t("post_hint"), auto: "postal-code", inputmode: "numeric" }),
    field("station.addr", t("addr"), { required: true, hint: t("addr_hint"), auto: "street-address", wide: true }),
    field("station.mail", t("mail"), { required: true, type: "email", hint: t("mail_hint"), auto: "email", wide: true }),
    field("station.note", t("note"), { textarea: true, hint: t("note_hint"), wide: true }));
}

function card(step: string, title: string, ...kids: (Node | null)[]): HTMLElement {
  return h("section", { class: "card" }, h("header", {}, h("h2", {}, h("span", { class: "step" }, step), title), kids[0] ?? null), ...kids.slice(1));
}

export function entryPage(main: HTMLElement, amend: Amend | null = null): void {
  const c = contest();
  if (!c.accept && !c.user?.admin) {
    page(t("entry_title"), null, notice("warn", t("closed_notice")));
    return;
  }
  const files: Attached[] = [];
  const kept: Kept[] = (amend?.files ?? []).map((f) => ({ ...f, keep: true }));
  let ticket = 0;
  const errors = h("div", { id: "errors" });
  const list = h("ul", { class: "files", "aria-live": "polite" });
  const box = h("p", { class: "muted small", id: "preview" }, t("preview_empty"));
  const manual = new ManualTable();
  const { box: sectionBox, slots } = buildSlots(() => void preview());

  const picker = h("input", { type: "file", multiple: true, class: "hidden", "aria-hidden": "true", tabindex: "-1" });
  const choose = h("button", { type: "button", class: "primary" }, t("choose_files"));
  const drop = h("div", { class: "drop" }, choose, h("span", { id: "drop-sub" }, t("drop_sub")));
  choose.addEventListener("click", (e) => { e.stopPropagation(); picker.click(); });
  drop.addEventListener("click", () => picker.click());
  drop.addEventListener("dragover", (e) => { e.preventDefault(); drop.classList.add("over"); });
  drop.addEventListener("dragleave", () => drop.classList.remove("over"));
  drop.addEventListener("drop", (e) => {
    e.preventDefault();
    drop.classList.remove("over");
    void add([...(e.dataTransfer?.files ?? [])]);
  });
  picker.addEventListener("change", () => { void add([...(picker.files ?? [])]); picker.value = ""; });

  const addQso = h("button", { class: "small", type: "button" }, "+ " + t("add_qso"));
  addQso.addEventListener("click", () => manual.add());
  const trap = h("div", { class: "trap", "aria-hidden": "true" }, h("label", {}, "website", h("input", { name: "website", tabindex: "-1", autocomplete: "off" })));
  const form = h("form", { novalidate: true, id: "entry-form" },
    errors,
    card("1", t("s_files"), null, drop, picker, list, h("div", { "data-field": "files" }, h("span", { class: "error" }))),
    card("2", t("s_station"), null, stationFields(), trap),
    card("3", t("s_sections"), null, h("div", { "data-field": "entries" }, h("span", { class: "error" })), sectionBox),
    card("4", t("s_manual"), addQso, manual.element));
  const submit = h("button", { class: "primary big", type: "submit", form: "entry-form" }, t("submit"));
  const aside = h("div", { class: "card", "aria-live": "polite" }, h("h2", {}, t("preview")), box);
  const title = amend ? t("amend") + " · " + amend.station.call : t("entry_title");
  fill(main, h("h1", {}, title), h("p", { class: "lead" }, t("entry_lead")), c.mock ? notice("warn", t("mock")) : null,
    h("div", { class: "layout" }, form, h("div", { class: "aside" }, aside, h("div", { class: "actions" }, submit))));

  if (amend) {
    for (const k of STATION) {
      const el = $<HTMLInputElement>("#f-station-" + k);
      if (el) el.value = amend.station[k] ?? "";
    }
    assign(slots, amend.entries);
  }
  slots.forEach((s) => s.sync());
  renderFiles();

  function renderFiles(): void {
    const keptItems = kept.map((f) => {
      const box = h("input", { type: "checkbox", checked: f.keep, style: "width:auto;min-height:0" });
      box.addEventListener("change", () => { f.keep = box.checked; });
      return h("li", {}, h("label", { class: "row" }, box, h("span", { class: "name" }, f.file || "manual")), h("span", { class: "pill" }, t("keep_files")));
    });
    const newItems = files.map((f, i) => {
      const remove = h("button", { class: "small ghost", type: "button" }, t("remove"));
      remove.addEventListener("click", () => { files.splice(i, 1); renderFiles(); void preview(); });
      const state = f.error ? h("span", { class: "pill bad" }, t("unreadable"))
        : f.items !== undefined ? h("span", { class: "pill ok" }, f.items + " " + t("qsos"))
          : h("span", { class: "spinner", "aria-label": t("checking") });
      return h("li", { class: f.error ? "bad" : null, "data-field": "file." + f.file.name },
        h("span", { class: "name", title: f.file.name }, f.file.name), f.sheet ? h("span", { class: "pill accent" }, "sheet") : null, state, remove);
    });
    fill(list, ...keptItems, ...newItems);
  }

  async function add(list: File[]): Promise<void> {
    for (const file of list) {
      const entry: Attached = { file, buffer: await file.arrayBuffer() };
      files.push(entry);
      renderFiles();
      const { ok, data } = await upload<Unboxed>("/unbox", entry.buffer);
      if (ok && data) {
        entry.sheet = true;
        applySheet(data);
      }
    }
    await preview();
  }

  function applySheet(s: Unboxed): void {
    for (const k of STATION) {
      const el = $<HTMLInputElement>("#f-station-" + k);
      const v = s[k];
      if (el && v && !el.value) el.value = v;
    }
    const slot = slots.find((x) => x.code === s.code);
    if (slot && s.sect) {
      slot.sel.value = s.sect;
      slot.sync();
      if (s.city) slot.city.value = s.city;
    }
    toast(t("sheet_filled"));
  }

  async function preview(): Promise<void> {
    const mine = ++ticket;
    if (!files.length) {
      fill(box, t("preview_empty"));
      return;
    }
    fill(box, h("span", { class: "spinner" }), " ", t("checking"));
    const { report, fallback } = await check(files, chosen(slots));
    if (mine !== ticket) return;
    renderFiles();
    show(box, report, fallback);
  }

  function confirmDialog(): Promise<boolean> {
    return new Promise((resolve) => {
      const ok = h("button", { class: "primary" }, t("submit"));
      const back = h("button", {}, t("back"));
      const dialog = h("dialog", { "aria-labelledby": "confirm-title" },
        h("h2", { id: "confirm-title" }, t("confirm_title")),
        h("p", { class: "mono" }, value("#f-station-call").toUpperCase()),
        h("ul", {}, ...chosen(slots).map((e) => h("li", {}, e.sect, e.city ? " · " + e.city : ""))),
        h("p", { class: "small" }, files.length + " " + t("files") + " · " + t("confirm_submit")),
        h("div", { class: "row" }, ok, back));
      ok.addEventListener("click", () => dialog.close("ok"));
      back.addEventListener("click", () => dialog.close("no"));
      dialog.addEventListener("close", () => { resolve(dialog.returnValue === "ok"); dialog.remove(); });
      document.body.append(dialog);
      dialog.showModal();
    });
  }

  function clearErrors(): void {
    form.querySelectorAll(".invalid").forEach((el) => el.classList.remove("invalid"));
    form.querySelectorAll("[aria-invalid]").forEach((el) => el.removeAttribute("aria-invalid"));
    form.querySelectorAll(".error").forEach((el) => { el.textContent = ""; });
    fill(errors);
  }

  function mark(key: string, message: string): HTMLElement | null {
    const box = form.querySelector<HTMLElement>(`[data-field="${CSS.escape(key)}"]`) ?? form.querySelector<HTMLElement>(`[data-field="files"]`);
    if (!box) return null;
    box.classList.add("invalid");
    box.querySelectorAll("input,select,textarea").forEach((el) => el.setAttribute("aria-invalid", "true"));
    const err = box.querySelector(".error");
    if (err) err.textContent = message;
    return box;
  }

  function showProblems(p: Problems): void {
    clearErrors();
    const all = [...form.querySelectorAll("[data-field]")];
    const found = Object.entries(p.fields).map(([key, code]) => {
      const msg = t("e_" + code);
      const box = mark(key, msg);
      const label = box?.querySelector("label")?.firstChild?.textContent || key.replace(/^file\./, "");
      return { box, label, msg };
    });
    const position = (b: Element | null) => (b ? all.indexOf(b) : all.length);
    found.sort((a, b) => position(a.box) - position(b.box));
    const items = found.map(({ box, label, msg }) => {
      const target = box?.querySelector<HTMLElement>("input,select,textarea,button") ?? box;
      const go = h("a", { href: "#" }, label);
      go.addEventListener("click", (e) => { e.preventDefault(); target?.focus(); box?.scrollIntoView({ block: "center", behavior: "smooth" }); });
      return h("li", {}, go, ": ", msg);
    });
    items.push(...p.conflicts.map((r) => h("li", {}, r)));
    const heading = h("h2", { tabindex: "-1" }, t("fix_title"));
    fill(errors, notice("bad", heading, h("ul", {}, ...items)));
    heading.focus();
  }

  function payload(): string {
    const station = Object.fromEntries(STATION.map((k) => [k, value("#f-station-" + k)]));
    const website = form.querySelector<HTMLInputElement>('[name="website"]')?.value ?? "";
    const keep = kept.filter((f) => f.keep).map((f) => f.id);
    return JSON.stringify({ station, entries: chosen(slots), manual: manual.values().filter(Boolean), keep, website });
  }

  form.addEventListener("submit", (e) => {
    e.preventDefault();
    void send();
  });

  async function send(): Promise<void> {
    clearErrors();
    const missing = Object.fromEntries(slots.flatMap((s, i) => s.sel.value ? [] : [["entries." + i, "choose"]]));
    if (Object.keys(missing).length) {
      showProblems({ fields: missing, conflicts: [] });
      return;
    }
    if (!amend && !(await confirmDialog())) return;
    submit.disabled = true;
    fill(submit, h("span", { class: "spinner" }), t("submitting"));
    const fd = new FormData();
    fd.append("form", payload());
    files.forEach((f) => fd.append("file", f.file, f.file.name));
    try {
      const { ok, data } = await upload<Accepted & Failure>(amend ? "/admin/entry" : "/entry", fd);
      if (ok && data) {
        sessionStorage.setItem(DONE, JSON.stringify(data));
        location.hash = amend ? "#/admin/station/" + data.call : "#/done";
        return;
      }
      if (data?.problems) showProblems(data.problems);
      else fill(errors, notice("bad", t("e_" + (data?.code || "server"))));
    } catch {
      fill(errors, notice("bad", t("e_server")));
    } finally {
      submit.disabled = false;
      fill(submit, t("submit"));
    }
  }
}
