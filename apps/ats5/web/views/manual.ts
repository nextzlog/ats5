import { h } from "../dom.js";
import { t } from "../i18n.js";
import { contest } from "../state.js";

const BANDS = ["1.9MHz", "3.5MHz", "7MHz", "10MHz", "14MHz", "18MHz", "21MHz", "24MHz", "28MHz", "50MHz", "144MHz", "430MHz", "1.2GHz", "2.4GHz", "5.6GHz", "10GHz"];

const MODES = ["CW", "SSB", "FM", "AM", "RTTY", "FT4", "FT8"];

export interface Manual {
  time: string;
  call: string;
  band: string;
  mode: string;
  sent: string;
  rcvd: string;
}

type Cells = { [K in keyof Manual]: HTMLInputElement | HTMLSelectElement };

export function zone(): string {
  const o = contest().offset;
  if (o === 540) return "JST";
  if (o === 0) return "UTC";
  const a = Math.abs(o);
  const hh = String(Math.floor(a / 60)).padStart(2, "0");
  const mm = String(a % 60).padStart(2, "0");
  return `UTC${o < 0 ? "-" : "+"}${hh}:${mm}`;
}

export class ManualTable {
  readonly element: HTMLElement;
  private readonly body = h("tbody");
  private readonly rows: (Cells | null)[] = [];

  constructor() {
    const head = [t("time_jst") + " (" + zone() + ")", t("call"), t("band"), t("mode"), t("sent"), t("rcvd"), ""];
    this.element = h("div", { class: "table-wrap hidden manual" },
      h("table", {}, h("thead", {}, h("tr", {}, ...head.map((x) => h("th", {}, x)))), this.body));
  }

  add(): void {
    const index = this.rows.length;
    const cells: Cells = {
      time: h("input", { type: "datetime-local", "aria-label": t("time_jst") }),
      call: h("input", { class: "mono", "aria-label": t("call") }),
      band: h("select", { "aria-label": t("band") }, ...BANDS.map((b) => h("option", {}, b))),
      mode: h("select", { "aria-label": t("mode") }, ...MODES.map((m) => h("option", {}, m))),
      sent: h("input", { "aria-label": t("sent") }),
      rcvd: h("input", { "aria-label": t("rcvd") }),
    };
    const remove = h("button", { class: "small ghost", type: "button" }, t("remove"));
    const tr = h("tr", { "data-field": "manual." + index }, ...Object.values(cells).map((c) => h("td", {}, c)), h("td", {}, remove));
    remove.addEventListener("click", () => { tr.remove(); this.rows[index] = null; });
    this.rows.push(cells);
    this.body.append(tr);
    this.element.classList.remove("hidden");
  }

  values(): (Manual | null)[] {
    return this.rows.map((c) => c && {
      time: c.time.value, call: c.call.value, band: c.band.value,
      mode: c.mode.value, sent: c.sent.value, rcvd: c.rcvd.value,
    });
  }
}
