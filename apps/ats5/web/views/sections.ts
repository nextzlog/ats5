import { fill, h } from "../dom.js";
import { t } from "../i18n.js";
import { contest } from "../state.js";
import type { Entry } from "../types.js";

export interface Slot {
  code: string;
  sel: HTMLSelectElement;
  city: HTMLInputElement;
  sync(): void;
}

export function buildSlots(changed: () => void): { box: HTMLElement; slots: Slot[] } {
  const c = contest();
  const slots: Slot[] = [];
  const box = h("div", { class: "grid" });
  for (const { code, limit } of c.codes) {
    for (let i = 0; i < limit; i++) {
      const index = slots.length;
      const id = "f-entries-" + index;
      const options = c.sections.filter((s) => s.code === code);
      const sel = h("select", { id, "aria-describedby": id + "-err" },
        h("option", { value: "" }, t("choose")),
        ...options.map((s) => h("option", { value: s.name }, s.name)));
      const city = h("input", { id: id + "-city", list: "cities-" + index, autocomplete: "off", placeholder: t("city_hint") });
      const list = h("datalist", { id: "cities-" + index });
      const cityField = h("div", { class: "field" }, h("label", { for: city.id }, t("city")), city, list);
      box.append(h("div", { class: "field wide", "data-field": "entries." + index },
        h("label", { for: id }, code, h("span", { class: "req" }, t("required"))), sel, cityField,
        h("span", { class: "error", id: id + "-err" })));
      const sync = (): void => {
        const s = c.sections.find((x) => x.name === sel.value);
        const cities = s && !s.absence ? c.cities[s.cities] ?? [] : [];
        cityField.classList.toggle("hidden", !cities.length);
        fill(list, ...cities.map((x) => h("option", { value: x.name })));
        if (cities.length && !cities.some((x) => x.name === city.value)) {
          const known = slots.map((x) => x.city.value).find((v) => v && cities.some((x) => x.name === v));
          city.value = known || "";
        }
        changed();
      };
      sel.addEventListener("change", sync);
      city.addEventListener("change", changed);
      slots.push({ code, sel, city, sync });
    }
  }
  return { box, slots };
}

export function chosen(slots: Slot[]): Entry[] {
  return slots.filter((s) => s.sel.value).map((s) => ({ sect: s.sel.value, city: s.city.value }));
}

export function assign(slots: Slot[], entries: Entry[]): void {
  const used = new Set<Slot>();
  for (const e of entries) {
    const s = contest().sections.find((x) => x.name === e.sect);
    const slot = slots.find((x) => !used.has(x) && s && x.code === s.code);
    if (!slot) continue;
    used.add(slot);
    slot.sel.value = e.sect;
    slot.sync();
    slot.city.value = e.city;
  }
}
