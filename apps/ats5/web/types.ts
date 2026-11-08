export interface City {
  code: string;
  name: string;
}

export interface SectionInfo {
  name: string;
  code: string;
  absence: boolean;
  cities: number;
}

export interface User {
  email: string;
  admin: boolean;
}

export interface Format {
  name: string;
  extensions: string[];
}

export interface Contest {
  name: string;
  host: string;
  mail: string;
  link: string;
  help: string;
  start: string;
  final: string;
  deadline: string;
  accept: boolean;
  finish: boolean;
  offset: number;
  sections: SectionInfo[];
  codes: { code: string; limit: number }[];
  cities: City[][];
  formats: Format[];
  sheets: Format[];
  login: boolean;
  board: boolean;
  user: User | null;
  media: string | null;
  contributors: string[];
}

export interface Station {
  call: string;
  name: string;
  post: string;
  addr: string;
  mail: string;
  note: string;
  uuid?: string;
}

export interface Entry {
  sect: string;
  city: string;
}

export interface Problems {
  fields: Record<string, string>;
  conflicts: string[];
  files?: unknown[];
}

export interface Failure {
  code?: string;
  problems?: Problems;
}

export interface Accepted {
  call: string;
  uuid: string;
  items: number;
  entries: Entry[];
  mail: boolean;
  unreadable: string[];
  contact: string;
}

export interface Submission {
  id: number;
  station: Station;
  time: number;
  peer: string;
  active: boolean;
  flags: string[];
}

export interface Row {
  time: string;
  call: string;
  band: string;
  mode: string;
  rcvd: string;
  sent: string;
  name: string;
  score: number;
  sign: boolean;
  text: string;
}

export interface Sheet {
  sect: string;
  city: string;
  absence: boolean;
  valid: Record<string, number>;
  invalid: Record<string, number>;
  accepted: Row[];
  rejected: Row[];
  score?: number;
  total?: number;
}

export interface FileInfo {
  id: number;
  file: string;
  items?: number;
  error?: string;
}

export interface Proof {
  submission: Submission;
  files: FileInfo[];
  sheets: Sheet[];
}

export interface Place {
  rank: number;
  call: string;
  city: string;
  score: number;
  total: number;
  award: boolean;
}

export interface Standing {
  name: string;
  code: string;
  places: Place[];
}

export interface Results {
  open: boolean;
  standings: Standing[];
  checklogs: string[];
}

export interface Report {
  call: string;
  area: string;
  contest: string;
  class: Record<string, string>;
  club: string;
  score: number;
  qso: number;
  mult: number;
  bands: Record<string, number>;
  time: number;
}

export interface Board {
  reports: Report[];
  standings: Standing[];
}

export interface Note {
  call: string;
  note: string;
}

export interface CallInfo {
  call: string;
  warning: string | null;
}

export interface Mine {
  user: User;
  calls: string[];
}

export interface AdminStation {
  id: number;
  call: string;
  name: string;
  mail: string;
  time: number;
  flags: string[];
  history: number;
  entries: Entry[];
  scoreless: boolean;
  absent: boolean;
}

export interface Past {
  submission: Submission;
  files: { id: number; file: string }[];
  entries: Entry[];
}

export interface StationDetail {
  current: Proof | null;
  history: Past[];
}

export interface Amend {
  station: Station;
  entries: Entry[];
  files: { id: number; file: string }[];
}

export interface Unboxed {
  call: string;
  name: string;
  post: string;
  addr: string;
  mail: string;
  note: string;
  sect: string | null;
  code: string | null;
  city: string | null;
}

export interface Trial {
  files: { file: string; items?: number; error?: string }[];
  sections: Preview[];
  conflict: boolean;
  reasons: string[];
}

export interface Contact {
  time: string;
  call: string;
  band: string;
  mode: string;
}

export interface Preview {
  sect: string;
  valid: number;
  invalid: number;
  score: number;
  total: number;
  rejected: Contact[];
}

export interface Readout {
  items: number;
  error: string | null;
}

export interface CheckReport {
  sections: Preview[];
  conflict: boolean;
  reasons: string[];
}
