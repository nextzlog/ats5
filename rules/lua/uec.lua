local SIN = "シングルバンド"
local ALL = "オールバンド"
local SWL = "SWL"

local SCORE = { UEC = 5, L = 4, I = 3, H = 2 }
local BANDS = { 1900, 3500, 7000, 14000, 21000, 28000, 50000 }
local LABELS = { "1.9MHz", "3.5MHz", "7MHz", "14MHz", "21MHz", "28MHz", "50MHz" }
local CITIES = ats.dat("http://ja6ycu.in.coocan.jp/ZLOG/uec.dat")

local function suffix(code)
	if string.sub(code, -3) == "UEC" then
		return "UEC"
	end
	local last = string.sub(code, -1)
	return SCORE[last] and last or ""
end

local function section(name, code, bands)
	return ats.section {
		name = name,
		code = code,
		bands = bands,
		modes = { "CW" },
		hours = ats.hours(17, 20),
		cities = CITIES,
		trim = function(code)
			local s = suffix(code)
			return string.sub(code, 1, #code - #s)
		end,
		points = function(it)
			return SCORE[suffix(ats.report(it.code))]
		end,
		award = function(n)
			return n >= 30 and 3 or n >= 11 and 2 or 1
		end,
	}
end

local sections = { ats.absence(ALL), ats.absence(SIN) }
for i, khz in ipairs(BANDS) do
	table.insert(sections, section(LABELS[i], SIN, { khz }))
end
table.insert(sections, section(ALL, ALL, BANDS))
table.insert(sections, section(SWL, ALL, BANDS))

return ats.program {
	name = "電通大コンテスト",
	host = "JA1ZGP",
	mail = "uectest-info@ja1zgp.com",
	link = "https://www.ja1zgp.com/uectest_public_info",
	help = "未対応の交信記録を提出した場合は、運営に報告が必要です。",
	start = ats.weekday(7, 3, "sat"),
	deadline = ats.on(8, 31),
	limit = { [ALL] = 1, [SIN] = 2 },
	sections = sections,
	conflict = function(entries)
		local out = {}
		local codes = {}
		local names = {}
		for _, e in ipairs(entries) do
			codes[e.code] = true
			table.insert(names, e.name)
		end
		if codes[ALL] and codes[SIN] then
			table.insert(out, "オールバンド部門とシングルバンド部門には、同時に参加できません。")
		end
		if not ats.unique_names(names) then
			table.insert(out, "同じシングルバンド部門を2つ選ぶことはできません。")
		end
		return out
	end,
}
