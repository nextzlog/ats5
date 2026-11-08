import { contest, state } from "./state.js";
import type { Rules } from "./state.js";
import type { CheckReport } from "./types.js";

export function frame(buffers: ArrayBuffer[]): Uint8Array {
  const total = buffers.reduce((n, b) => n + 4 + b.byteLength, 0);
  const out = new Uint8Array(total);
  const view = new DataView(out.buffer);
  let at = 0;
  for (const b of buffers) {
    view.setUint32(at, b.byteLength, true);
    out.set(new Uint8Array(b), at + 4);
    at += 4 + b.byteLength;
  }
  return out;
}

export async function loadWasm(): Promise<Rules | false> {
  if (state.wasm !== null) return state.wasm;
  try {
    if (!state.wasmBytes) {
      const res = await fetch("/ats5.wasm");
      if (!res.ok) throw new Error("no wasm");
      state.wasmBytes = await res.arrayBuffer();
    }
    const { instance } = await WebAssembly.instantiate(state.wasmBytes, {});
    const rules = instance.exports as unknown as Rules;
    rules.ats5_offset(contest().offset);
    rules.ats5_clock(Date.now() / 1000);
    state.wasm = rules;
  } catch {
    state.wasm = false;
  }
  return state.wasm;
}

export async function wasmCheck(buffers: ArrayBuffer[], sections: string[]): Promise<CheckReport | null> {
  const w = await loadWasm();
  if (!w) return null;
  try {
    const files = frame(buffers);
    const req = new TextEncoder().encode(JSON.stringify({ contest: contest().name, sections }));
    const fp = w.ats5_alloc(files.length);
    new Uint8Array(w.memory.buffer, fp, files.length).set(files);
    const rp = w.ats5_alloc(req.length);
    new Uint8Array(w.memory.buffer, rp, req.length).set(req);
    const out = w.ats5_check(fp, files.length, rp, req.length);
    const len = new DataView(w.memory.buffer).getUint32(out, true);
    const json = new TextDecoder().decode(new Uint8Array(w.memory.buffer, out + 4, len));
    w.ats5_free(fp, files.length);
    w.ats5_free(rp, req.length);
    w.ats5_free(out, len + 4);
    return JSON.parse(json) as CheckReport;
  } catch {
    state.wasm = null;
    return null;
  }
}
