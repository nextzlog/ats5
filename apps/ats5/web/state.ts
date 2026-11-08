import type { Contest } from "./types.js";

export interface Rules {
  memory: WebAssembly.Memory;
  ats5_alloc(len: number): number;
  ats5_free(ptr: number, len: number): void;
  ats5_offset(minutes: number): void;
  ats5_clock(unix: number): void;
  ats5_check(files: number, filesLen: number): number;
}

interface State {
  contest: Contest | null;
  wasm: Rules | false | null;
  wasmBytes: ArrayBuffer | null;
  stream: EventSource | null;
}

export const state: State = { contest: null, wasm: null, wasmBytes: null, stream: null };

export function contest(): Contest {
  if (!state.contest) throw new Error("contest is not loaded");
  return state.contest;
}
