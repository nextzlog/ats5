ATS-5: Amateur-Radio Contest Administration System
====

![image](https://img.shields.io/badge/Rust-2021-red.svg)
![image](https://img.shields.io/badge/WebAssembly-1.0-orange.svg)
![image](https://img.shields.io/badge/SQLite-3-blueviolet.svg)
![image](https://img.shields.io/badge/TypeScript-5.6-blue.svg)
![image](https://img.shields.io/badge/license-MIT-darkblue.svg)
![badge](https://github.com/nextzlog/ats5/actions/workflows/build.yaml/badge.svg)

ATS-5 is an Automatic Acceptance & Tabulation System for Amateur-Radio Contests, based on [RxSL](https://github.com/autodyne/rxsl).
ATS-5 is the successor of [ATS-4](https://github.com/nextzlog/ats4) and runs as a single executable.

## Features

- provides a web interface for contest-log acceptance, on phones and desktops, in Japanese and English.
- verifies the uploaded logs according to the contest rules implemented in Rust.
- shows the score in the browser before submission, with the same rules compiled into WebAssembly.
- explains every form error next to the field, including invalid combinations of sections.
- keeps every resubmission and flags suspicious ones for the organizers.
- protects the submission view and the administration page with Google sign-in.
- provides a live scoreboard of online scores from [N1MM+](https://n1mmwp.hamdocs.com) and other loggers.
- supports many log formats including [zLog](https://zlog.org), CTESTWIN, HAMLOG, ADIF, Cabrillo, JARL, N1MM+, and QXML.
- supports contests including [ALLJA1](https://ja1zlo.u-tokyo.org/allja1).

## Quick Start

Executables are available for Linux, macOS, and Windows.
Paste the entire following script into the terminal and run it.

```sh
curl -L -o ats5 https://github.com/nextzlog/ats5/releases/download/latest/ats5-linux-x86_64
chmod +x ats5

echo -n 'enter mail hostname: '
read host

echo -n 'enter mail username: '
read user

echo -n 'enter mail password: '
read pass

cat << EOS > ats5.sh
export TZ=Asia/Tokyo
export ATS5_MAIL_HOST=$host
export ATS5_MAIL_USER=$user
export ATS5_MAIL_PASS=$pass
export ATS5_CONTEST=ALLJA1
exec ./ats5
EOS

sh ats5.sh
```

Then, point your browser to http://localhost:9000 and verify that ATS-5 is running.

## Configuration

ATS-5 is configured with environment variables.
Write them into `ats5.sh` as follows, and then follow the instructions below.

```sh
export TZ=Asia/Tokyo
export ATS5_URL=https://allja1.org
export ATS5_DATA=data
export ATS5_CONTEST=ALLJA1
export ATS5_PERIOD=2026-06-27..2026-07-18
export ATS5_ADMINS=allja1.info@gmail.com
export ATS5_GOOGLE_ID=$id
export ATS5_GOOGLE_SECRET=$secret
export ATS5_MAIL_HOST=$host
export ATS5_MAIL_USER=$user
export ATS5_MAIL_PASS=$pass
exec ./ats5
```

### Proxy

Run ATS-5 behind a reverse proxy that provides HTTPS, such as nginx, and set the variables as follows:

```sh
export ATS5_URL=https://allja1.org
export ATS5_PROXY=true
```

Make sure that the proxy forwards WebSocket connections to `/agent/` for real-time contests.
ATS-5 listens on `0.0.0.0:9000` by default, which can be changed with `ATS5_BIND`.

### Sign-in

Create an OAuth client of the web application type in the Google Cloud console.
Register `https://allja1.org/auth/callback` as the redirect URI, and set the variables as follows:

```sh
export ATS5_GOOGLE_ID=$id
export ATS5_GOOGLE_SECRET=$secret
export ATS5_ADMINS=allja1.info@gmail.com,another@example.com
```

Participants submit their logs without signing in.
They sign in with the Google account of the submitted address to view their submissions.
Organizers listed in `ATS5_ADMINS` sign in and open the administration page from the menu.
Nobody can sign in without these settings.

### Email

Configure the variables as follows:

```sh
export ATS5_MAIL_HOST=$host
export ATS5_MAIL_PORT=465
export ATS5_MAIL_USER=$user
export ATS5_MAIL_PASS=$pass
export ATS5_MAIL_MOCK=false
```

Port 465 uses TLS and the other ports use STARTTLS.
Without `ATS5_MAIL_HOST`, mail is written to `data/mail.log` instead of being sent.
Send a test mail from the administration page.

### Regulation

Configure the variables as follows:

```sh
export ATS5_CONTEST=ALLJA1
export ATS5_PERIOD=2026-06-27..2026-07-18
```

The submission period follows the contest rules by default.
`ATS5_PERIOD` overrides it without rebuilding ATS-5.
See [`ats5-allja1`](crates/ats5-allja1/src/lib.rs) for example.

### Guide

Configure the variables as follows:

```sh
export ATS5_MEDIA=https://www.youtube.com/embed/Yb6QY7BI4kA
```

The video is embedded into the guide page.

### Run

Finally, start ATS-5 as follows:

```sh
$ sh ats5.sh
```

The database, the secret key, and the mail log are stored in `ATS5_DATA`.

### Stop

Stop ATS-5 by pressing `Ctrl+C`.

### Update

Download the latest executable as follows:

```sh
$ curl -L -o ats5 https://github.com/nextzlog/ats5/releases/download/latest/ats5-linux-x86_64
```

## Development Mode

You can start ATS-5 in development mode as follows:

```sh
$ ATS5_ADMINS=admin@example.com cargo run --example dev
```

Then, access http://localhost:9000/dev/login?email=admin@example.com to sign in without Google.
You can test the scoring algorithm by attaching QSO data to the web form.
Run the tests and check the TypeScript code as follows:

```sh
$ cargo test
$ npx -p typescript tsc -p apps/ats5/web
```

The web pages are written in TypeScript under [`apps/ats5/web`](apps/ats5/web) and compiled into the executable by `cargo build`.

## Contest Rules

Each contest is implemented as a crate that implements `rxsl::ruler::Program`.
Register the crate in [`ats5-wasm`](crates/ats5-wasm/src/lib.rs) so that both the server and the browser use the same rules.

|crate|contest|
|---|---|
|[`ats5-allja1`](crates/ats5-allja1)|[ALLJA1](https://ja1zlo.u-tokyo.org/allja1)|

## Online Score

ATS-5 accepts online scores in the `<dynamicresults>` XML format of N1MM+.
Point the online score setting of the logger to `https://allja1.org/board`.
The scoreboard page shows the reported scores and the scores tabulated from logs, updated live.

## Stream API

ATS-5 provides the streaming API for the [REAL-TIME CONTEST](http://ja1zlo.u-tokyo.org/rt/rt1.html), compatible with ATS-4.

### Registration

Contest participants will register their account information with ATS-5 in advance.
ATS-5 returns a security key (UUID) by sending a `GET` request to `http://localhost:8873?id=<UUID>`.
Clients may retrieve the key by listening on the 8873 port and access `/agent/<UUID>`.

### Upstream

When the contest starts, the client always connects to the server via WebSocket.
Each time a participant contacts another participant on air, the client sends the difference in the QSO records to the server.
Messages from the clients to the server must follow the format below.

|position|field                 |
|--------|----------------------|
|1st byte|number of QSOs deleted|
|sequence|header of the QSO data|
|sequence|QSO entities to delete|
|sequence|QSO entities to append|

The second and subsequent bytes of the messages are formatted as a single electronic log file.
The format must be supported by the [RxSL](https://github.com/autodyne/rxsl) library.

### Downstream

The server receives the QSO records, scores it, wait a few seconds, and then notifies all clients of the score update.
The delay can be changed with `ATS5_RTC_DELAY`.
JSON messages from the server to the clients are formatted as follows:

```JSON
{
  "14MHz": [
    {"call": "JA1ZLO", "score": 200, "total": 2200},
    {"call": "JA1YWX", "score": 100, "total": 2100}
  ]
}
```

## Contribution

Feel free to make issues at [nextzlog/todo](https://github.com/nextzlog/todo).
Follow [@nextzlog](https://twitter.com/nextzlog) on Twitter.

## License

### Author

[無線部開発班](https://nextzlog.dev)

- JG1VPP
- JJ2ULU
- JH1GEB
- JE6MDL
- JO4EFC
- JJ1IBY
- JS2FVO

### Clauses

[MIT License](LICENSE)
