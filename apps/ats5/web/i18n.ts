const T = {
  ja: {
    skip: "本文へ移動", home: "ホーム", entry: "書類提出", results: "集計結果", board: "得点板",
    calls: "全参加局", voices: "皆様の声", guide: "提出方法", admin: "管理", mine: "提出内容",
    login: "ログイン", logout: "ログアウト", language: "English",
    held: "開催", deadline: "締切", rules: "規約を閲覧する", submit_now: "書類を提出する", see_results: "結果を閲覧する",
    open: "受付中", closed: "提出期間外", finished: "集計結果を公開中",
    closed_notice: "提出期間外です。",
    entry_title: "書類提出", entry_lead: "参加した部門をすべて選び、交信記録を添付してください。再提出すると、以前に提出した書類は無効になります。",
    s_files: "交信記録", s_station: "参加局の情報", s_sections: "参加部門", s_manual: "交信の手入力（任意）",
    drop_title: "ファイルをここにドロップ", drop_sub: "複数のファイルをまとめてドロップできます。サマリーシートの場合は各欄を自動で入力します。",
    qsos: "件の交信", unreadable: "読み取れません", unreadable_ok: "読み取れません（このまま提出でき、運営で確認します）", done_unreadable: "次のファイルは交信記録として読み取れませんでした。", done_unreadable_hint: "運営で確認いたしますが、ロガーから書き出したファイルで再提出いただくと確実です。ご不明な点は運営にお問い合わせください。お問い合わせ先：", f_unreadable: "読み取れない交信記録", remove: "削除", sheet_filled: "サマリーシートの内容を入力しました。確認してください。",
    call: "呼出符号", name: "名前", post: "郵便番号", addr: "住所", mail: "メール", note: "ご感想",
    call_hint: "例：JA1ZLO、JA1ZLO/1", name_hint: "この名称で賞を発行します。", post_hint: "例：113-0033",
    addr_hint: "この宛先に賞を発送します。", mail_hint: "受理通知メールが届きます。後で提出内容を確認するには、このアドレスの Google アカウントでログインします。",
    note_hint: "呼出符号とともに公表します（任意）。", required: "必須",
    section: "部門", choose: "選択してください", e_choose: "部門を選んでください。参加していない部門は「不参加」を選んでください。", choose_files: "ログファイルを選ぶ", confirm_title: "この内容で提出しますか", city: "運用地", city_hint: "地名を入力すると候補が表示されます", not_entered: "不参加",
    add_qso: "交信を追加", time_jst: "日時", band: "周波数帯", mode: "モード", sent: "送信ナンバー", rcvd: "受信ナンバー",
    preview: "提出前の確認", preview_empty: "ログを添付し、部門を選ぶと、ここに得点が表示されます。",
    valid: "有効", invalid: "無効", score: "得点", total: "総得点", multi: "マルチ",
    invalid_list: "無効な交信を表示", checking: "確認しています…", browser_off: "ブラウザでの確認ができないため、サーバーで確認しました。",
    submit: "提出する", submitting: "提出しています…", confirm_submit: "以前に提出した書類がある場合、その書類は無効になります。",
    fix_title: "提出できませんでした。次の項目を確認してください。",
    e_call: "呼出符号の形式が正しくありません（英数字と / - のみ）。", e_required: "入力してください。",
    e_mail: "メールアドレスの形式が正しくありません。", e_sect: "部門を選び直してください。", e_city: "運用地を候補の中から選んでください。",
    e_conflict: "部門の組み合わせが規約に合いません。", e_qso: "この交信の入力に不足があります（日時・呼出符号・周波数帯・モード・受信ナンバー）。",
    e_unreadable: "このファイルは交信記録として読み取れません。形式を確認してください。", e_nolog: "交信記録のファイルを添付するか、交信を手入力してください。",
    e_busy: "短時間に送信が集中しています。しばらく待ってから再度お試しください。", e_closed: "提出期間外です。",
    e_server: "サーバーでエラーが発生しました。時間をおいて再度お試しください。", e_request: "送信内容を読み取れませんでした。",
    done_title: "提出完了", done_lead: "下記の内容で受理しました。交信記録が正しく読み込まれたかご確認ください。訂正が必要な場合は期間内に限り再提出できます。受理通知メールもお送りしました。",
    done_proof: "提出内容を確認する", done_mail_failed: "メールの送信に失敗しました。運営にお問い合わせください。",
    proof_title: "提出内容", proof_login: "提出内容の確認には、提出時のメールアドレスの Google アカウントでログインしてください。",
    proof_owner: "このアカウントでは、この呼出符号の提出内容を確認できません。提出時のメールアドレスでログインしてください。",
    files: "添付ファイル", by_band: "周波数帯ごとの件数", contacts: "交信の一覧", sign: "照合", reason: "理由",
    results_closed: "集計結果は、提出期間の終了後に公開します。", checklog: "交信記録のみ提出", rank: "順位", award: "入賞",
    search: "呼出符号で絞り込み", warning: "注意", w_scoreless: "得点のない部門があります。内容をご確認のうえ、必要に応じて再提出してください。",
    w_absent: "どの部門にも参加していないか、交信記録のみの提出です。", no_voice: "まだ投稿はありません。",
    formats: "対応する交信記録の形式", sheets: "対応するサマリーシートの形式", extensions: "拡張子",
    board_lead: "ロガーから送られた得点と、ログから集計した得点を、リアルタイムで表示します。",
    reported: "自己申告の得点", tabulated: "ログからの集計", area: "地域", all: "すべて", updated: "更新",
    live: "ライブ", class: "部門", qso: "交信", n1mm: "N1MM+ などのオンラインスコアの送信先",
    my_calls: "提出済みの書類", none: "このアカウントで提出された書類はありません。",
    login_failed: "ログインできませんでした。もう一度お試しください。", login_unavailable: "ログインが設定されていません。運営にお問い合わせください。",
    stations: "提出局", flagged: "要確認", scoreless: "無得点", recalc: "得点を再計算する", csv: "データを CSV で保存する", clear: "全データを消去する",
    flags: "目印", f_mail: "前回と異なるメールアドレス", f_burst: "締切直前に再提出を繰り返しています", history: "提出履歴",
    revert: "この提出に戻す", current: "現在の提出", amend: "修正", delete: "削除", download: "保存する", ack: "確認済みにする",
    sendmail: "連絡送信", to: "宛先", subject: "件名", body: "本文", send: "送信する", sent_ok: "送信しました",
    confirm_delete: "この局の提出をすべて削除します。元に戻せません。", confirm_clear: "全データを消去します。確認のためコンテスト名を入力してください。",
    keep_files: "以前の添付ファイルを残す", done: "完了しました", back: "戻る", time: "日時", peer: "送信元", open_admin: "管理画面",
    welcome: "書類受付システム", mailer: "メールの送信方法", via_gmail: "Gmail", via_smtp: "SMTP", via_none: "未設定（送信しない）", gmail_link: "Gmail と連携する", gmail_unlink: "連携を解除する", gmail_hint: "主催者の Google アカウントで許可すると、その Gmail アドレスから受理通知などを送ります。許可の画面で「確認されていないアプリ」と表示された場合は、内容を確認して続行してください。", gmail_off: "Gmail と連携するには、OAuth の設定が必要です。", login_google: "Google でログイン", login_site: "書類の提出と確認には、Google アカウントでのログインが必要です。提出時のメールアドレスには、このアカウントのアドレスを使います。", publish: "集計結果を公開する", rule_choice: "規約", rule_none: "規約を選んでください", rule_confirm: "規約を変更しますか。提出期間も規約に合わせて変わります。", rule_hint: "この書類提出ページで使う規約です。規約を選ぶと、書類の受付が始まります。", published: "公開しました。", publish_hint: "集計結果、全参加局、皆様の声を、参加者が見られるように公開します。書類を修正したときは、再度公開してください。", e_large: "交信記録が大きすぎます。テキスト形式（Cabrillo など）で書き出して提出してください。", e_login: "提出するには、Google アカウントでログインしてください。", contact: "運営に連絡", missing: "ページが見つかりません。", zlog: "zLog で快適なコンテスト運用を。",
  },
  en: {
    skip: "Skip to content", home: "Home", entry: "Submit", results: "Results", board: "Scoreboard",
    calls: "Stations", voices: "Comments", guide: "How to submit", admin: "Admin", mine: "My submission",
    login: "Sign in", logout: "Sign out", language: "日本語",
    held: "Held", deadline: "Deadline", rules: "Contest rules", submit_now: "Submit your log", see_results: "See results",
    open: "Open", closed: "Closed", finished: "Results published",
    closed_notice: "The submission period is over.",
    entry_title: "Submit your log", entry_lead: "Choose every section you entered and attach your log. Resubmitting replaces your earlier submission.",
    s_files: "Logs", s_station: "Station", s_sections: "Sections", s_manual: "Manual entry (optional)",
    drop_title: "Drop files here", drop_sub: "Multiple files allowed. A summary sheet fills in the form automatically.",
    qsos: "QSOs", unreadable: "Unreadable", unreadable_ok: "Unreadable (you can still submit; we will check it)", done_unreadable: "These files could not be read as logs.", done_unreadable_hint: "We will check them, but resubmitting a file exported from your logger is the most reliable fix. Contact the organizer:", f_unreadable: "Unreadable log", remove: "Remove", sheet_filled: "Filled in from the summary sheet. Please check.",
    call: "Call sign", name: "Name", post: "Postal code", addr: "Address", mail: "Email", note: "Comments",
    call_hint: "e.g. JA1ZLO or JA1ZLO/1", name_hint: "Used on certificates", post_hint: "e.g. 113-0033",
    addr_hint: "Where awards are sent", mail_hint: "We send the receipt here. To view your submission later, sign in with the Google account for this address.",
    note_hint: "Published with your call sign (optional)", required: "Required",
    section: "Section", choose: "Choose", e_choose: "Choose a section, or \"Not entered\" if you did not enter it.", choose_files: "Choose log files", confirm_title: "Submit this?", city: "Location", city_hint: "Type a place name to see suggestions", not_entered: "Not entered",
    add_qso: "Add QSO", time_jst: "Date and time", band: "Band", mode: "Mode", sent: "Sent", rcvd: "Received",
    preview: "Before you submit", preview_empty: "Attach a log and choose sections to see your score here.",
    valid: "Valid", invalid: "Invalid", score: "Points", total: "Total score", multi: "Mults",
    invalid_list: "Show invalid QSOs", checking: "Checking…", browser_off: "Checked on the server because the browser check is unavailable.",
    submit: "Submit", submitting: "Submitting…", confirm_submit: "Submit this log? Any earlier submission will be replaced.",
    fix_title: "Please fix the following.",
    e_call: "Invalid call sign (letters, digits, / and - only).", e_required: "This field is required.",
    e_mail: "Invalid email address.", e_sect: "Choose the section again.", e_city: "Choose a location from the suggestions.",
    e_conflict: "This combination of sections is not allowed.", e_qso: "This QSO is incomplete (time, call, band, mode, received).",
    e_unreadable: "This file cannot be read as a log.", e_nolog: "Attach a log or enter QSOs by hand.",
    e_busy: "Too many submissions. Please wait and try again.", e_closed: "The submission period is over.",
    e_server: "A server error occurred. Please try again later.", e_request: "The request could not be read.",
    done_title: "Submission received", done_lead: "Your log has been accepted and a receipt sent by email. Please check that your log was read correctly; you can resubmit until the deadline.",
    done_proof: "View your submission", done_mail_failed: "The receipt could not be sent. Please contact the organizer.",
    proof_title: "Submission", proof_login: "Sign in with the Google account for the email address you used to submit.",
    proof_owner: "This account cannot view this station's submission. Sign in with the address you used to submit.",
    files: "Files", by_band: "QSOs by band", contacts: "QSOs", sign: "Matched", reason: "Reason",
    results_closed: "Results are published after the submission period.", checklog: "Check log", rank: "Rank", award: "Award",
    search: "Filter by call sign", warning: "Note", w_scoreless: "One or more sections have no score. Please check and resubmit if needed.",
    w_absent: "Not entered in any section, or a check log.", no_voice: "No comments yet.",
    formats: "Supported log formats", sheets: "Supported summary sheets", extensions: "Extensions",
    board_lead: "Scores reported by loggers and scores tabulated from logs, updated live.",
    reported: "Reported scores", tabulated: "Tabulated from logs", area: "Area", all: "All", updated: "Updated",
    live: "Live", class: "Section", qso: "QSOs", n1mm: "Online score endpoint for N1MM+ and others",
    my_calls: "Your submissions", none: "No submission is linked to this account.",
    login_failed: "Sign-in failed. Please try again.", login_unavailable: "Sign-in is not configured. Please contact the organizer.",
    stations: "Stations", flagged: "Flagged", scoreless: "No score", recalc: "Recalculate", csv: "Download CSV", clear: "Erase all data",
    flags: "Flags", f_mail: "Different email from last time", f_burst: "Repeated resubmissions near the deadline", history: "History",
    revert: "Restore this", current: "Current", amend: "Edit", delete: "Delete", download: "Download", ack: "Mark as checked",
    sendmail: "Send mail", to: "To", subject: "Subject", body: "Body", send: "Send", sent_ok: "Sent",
    confirm_delete: "Delete all submissions from this station? This cannot be undone.", confirm_clear: "Erase all data. Type the contest name to confirm.",
    keep_files: "Keep earlier files", done: "Done", back: "Back", time: "Time", peer: "From", open_admin: "Admin",
    welcome: "Log submission system", mailer: "Mail delivery", via_gmail: "Gmail", via_smtp: "SMTP", via_none: "Not configured (not sent)", gmail_link: "Connect Gmail", gmail_unlink: "Disconnect", gmail_hint: "Authorize with the organizer's Google account to send receipts from that Gmail address. If Google shows an unverified-app warning, review it and continue.", gmail_off: "OAuth must be configured to connect Gmail.", login_google: "Sign in with Google", login_site: "Sign in with a Google account to submit and review logs. The address of this account is used as your email address.", publish: "Publish results", rule_choice: "Rules", rule_none: "Choose the rules", rule_confirm: "Change the rules? The submission period follows the rules.", rule_hint: "The contest rules used on this submission page. Submissions open once the rules are chosen.", published: "Published.", publish_hint: "Publish the results, the list of stations, and the comments for participants. Publish again after changing any submission.", e_large: "The log is too large. Export it as text (such as Cabrillo) and submit again.", e_login: "Sign in with a Google account to submit.", contact: "Contact", missing: "Page not found.", zlog: "Enjoy contesting with zLog.",
  },
};

export type Lang = keyof typeof T;

export type Key = keyof typeof T.ja;

let lang: Lang = (() => {
  try {
    const saved = localStorage.getItem("ats5-lang");
    if (saved === "ja" || saved === "en") return saved;
    return navigator.language.startsWith("ja") ? "ja" : "en";
  } catch {
    return "ja";
  }
})();

export function t(key: Key | string): string {
  const table: Record<string, string> = T[lang];
  const base: Record<string, string> = T.ja;
  return table[key] || base[key] || key;
}

export function getLang(): Lang {
  return lang;
}

export function setLang(next: Lang): void {
  lang = next;
  try { localStorage.setItem("ats5-lang", lang); } catch {}
  document.documentElement.lang = lang;
}
