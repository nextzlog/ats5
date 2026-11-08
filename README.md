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
- verifies the uploaded logs according to the contest rules implemented in Luau.
- reads the logs in the browser with WebAssembly and shows the score before submission.
- explains every form error next to the field, including invalid combinations of sections.
- keeps every resubmission and flags suspicious ones for the organizers.
- protects the submission view and the administration page with Google OAuth.
- provides a live scoreboard of online scores from [N1MM+](https://n1mmwp.hamdocs.com) and other loggers.
- supports many log formats including [zLog](https://zlog.org), CTESTWIN, HAMLOG, ADIF, Cabrillo, JARL, N1MM+, and QXML.
- supports many contests including [UEC](https://www.ja1zgp.com/uectest_public_info), [ALLJA1](https://ja1zlo.u-tokyo.org/allja1), [REAL-TIME](https://ja1zlo.u-tokyo.org/rt/rt1.html), [VUS](https://www.ja1zgp.com/uectest-vus_public_info), [QRP](https://www2.jaqrp.org/contests/), [AREA-1 AM](https://6mnet.jp/mannaka/blog), etc.

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
export ATS5_CONTEST=ja1
exec ./ats5
EOS

sh ats5.sh
```

Then, point your browser to http://localhost:9000 and verify that ATS-5 is running.

## Google Apps Script

ATS-5 also runs without any server, on the Google account of the organizer.
The submissions are stored in a folder of the Google Drive of the organizer, and nobody else can access them.
Both the participants and the organizers sign in with Google accounts in this mode.

1. Open https://script.google.com with the Google account of the organizer and create a new project.
2. Replace the content of `Code.gs` with [`gas/Code.js`](gas/Code.js).
3. Enable "Show appsscript.json manifest file in editor" in the project settings, and replace the content of `appsscript.json` with [`gas/appsscript.json`](gas/appsscript.json).
4. Run the function `setup` once and allow the requested access.
5. Choose "Deploy", "New deployment", and "Web app", and then deploy it.
6. Copy the deployment ID, and then open `https://nextzlog.github.io/ats5/?s=` followed by the ID.
7. Sign in as the organizer, and then choose the contest rules on the administration page.

The page is the submission page for the participants.
The organizer tabulates the submissions on the administration page of the same page and publishes the results there.
Receipts are sent from the Gmail address of the organizer, up to the daily quota of the Google account.

## Configuration

ATS-5 is configured with environment variables.
Write them into `ats5.sh` as follows, and then follow the instructions below.

```sh
export TZ=Asia/Tokyo
export ATS5_URL=https://allja1.org
export ATS5_DATA=data
export ATS5_CONTEST=ja1
export ATS5_PERIOD=2026-06-27..2026-07-18
export ATS5_ADMINS=organizer@example.com
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

### OAuth

Create an OAuth client of the web application type in the Google Cloud console.
Register `https://allja1.org/auth/callback` as the redirect URI, and set the variables as follows:

```sh
export ATS5_GOOGLE_ID=$id
export ATS5_GOOGLE_SECRET=$secret
export ATS5_ADMINS=organizer@example.com,another@example.com
```

Participants submit their logs without signing in.
They sign in with the Google account of the submitted address to view their submissions.
Organizers listed in `ATS5_ADMINS` sign in and open the administration page from the menu.
Each item of `ATS5_ADMINS` is either an address or a domain, such as `example.org`, which admits every account of the domain.
Nobody can sign in without these settings.

### Email

Configure the variables as follows:

```sh
export ATS5_MAIL_HOST=$host
export ATS5_MAIL_PORT=465
export ATS5_MAIL_USER=$user
export ATS5_MAIL_PASS=$pass
```

Port 465 uses TLS and the other ports use STARTTLS.
Alternatively, connect Gmail from the email page of the administration page, which sends mail from the Gmail address of the organizer through the Gmail API.
This requires the OAuth settings below, and Google may show a warning for unverified apps when the organizer authorizes it.
Without `ATS5_MAIL_HOST` or Gmail, mail is not sent and is only recorded in `data/mail.log`.
Send a test mail from the administration page.

### Regulation

Configure the variables as follows:

```sh
export ATS5_CONTEST=ja1
export ATS5_PERIOD=2026-06-27..2026-07-18
export ATS5_RULES=/path/to/contest.lua
```

Of course, you can specify different rules by writing Luau files and listing them in `ATS5_RULES`.
See [`ja1.lua`](rules/lua/ja1.lua) for example.
The submission period follows the contest rules by default, and `ATS5_PERIOD` overrides it.

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

Contest rules are written in [Luau](https://luau.org) under [`rules/lua`](rules/lua) and built into the executable.

|file|contest|
|---|---|
|[`1am.lua`](rules/lua/1am.lua)|[AREA-1 AM](https://6mnet.jp/mannaka/blog)|
|[`ja1.lua`](rules/lua/ja1.lua)|[ALLJA1](https://ja1zlo.u-tokyo.org/allja1)|
|[`oqp.lua`](rules/lua/oqp.lua)|[UNIV ORG QSO PARTY](https://ja1zlo.u-tokyo.org/univ_onair_together)|
|[`qrp.lua`](rules/lua/qrp.lua)|[QRP](https://www2.jaqrp.org/contests/)|
|[`rtc.lua`](rules/lua/rtc.lua)|[REAL-TIME](https://ja1zlo.u-tokyo.org/rt)|
|[`uec.lua`](rules/lua/uec.lua)|[UEC](https://www.ja1zgp.com/uectest_public_info)|
|[`vus.lua`](rules/lua/vus.lua)|[VUS](https://www.ja1zgp.com/uectest-vus_public_info)|

Set `ATS5_CONTEST` to the file name without `.lua`, which is `ja1` by default.
Some rules use the lists of the operating places by [JA6YCU](http://ja6ycu.in.coocan.jp/ZLOG/), which are stored under [`rules/dat`](rules/dat) and built into the executable.
The common parts of the rules are provided by [`prelude.lua`](crates/ats5-lua/src/prelude.lua).

A rule file returns a table of the contest name, dates, sections, and a function that explains invalid combinations of sections.
Each section verifies a QSO, identifies duplicates, extracts multipliers, and calculates the score.

## Online Score

ATS-5 accepts online scores in the `<dynamicresults>` XML format of N1MM+.
Set `ATS5_BOARD=true` to show the scoreboard page, and point the online score setting of the logger to `https://allja1.org/board`.
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
