local BANDS = { 135, 475, 1900, 3500, 7000, 10000, 14000, 18000, 21000, 24000, 28000, 50000, 144000, 430000, 1200000, 2400000, 5600000, 10000000 }
local POINTS = {
	CW = 3, A1A = 3, F2A = 3,
	PH = 3, AM = 3, FM = 3, SSB = 3, LSB = 3, USB = 3, DSB = 3,
	FT8 = 1, FT4 = 1, JT65 = 1,
	RTTY = 3, SSTV = 3,
}
local MODES = {}
for mode in pairs(POINTS) do
	table.insert(MODES, mode)
end
table.sort(MODES)

local function day(it)
	return ats.local_date(it.time or "", 540) or ""
end

local function section(name, result)
	return ats.section {
		name = name,
		code = name,
		bands = BANDS,
		modes = MODES,
		points = function(it)
			return POINTS[string.upper(it.mode or "")]
		end,
		unique = function(it)
			return { day(it), it.call or "", it.band or "", string.upper(it.mode or "") }
		end,
		entity = function(it)
			return { day(it) }
		end,
		result = result,
		award = function()
			return 1
		end,
	}
end

return ats.program {
	name = "大学社団QSOパーティ",
	host = "JA1ZLO",
	mail = "allja1.info@gmail.com",
	link = "https://ja1zlo.u-tokyo.org/univ_onair_together",
	help = "未対応の交信記録を提出した場合は、運営に報告が必要です。",
	start = ats.on(11, 1),
	final = ats.on(11, 30),
	deadline = ats.on(1, 31, 1),
	sections = {
		section("局数部門", function(s)
			return s.score
		end),
		section("運用日数部門", function(s)
			return s.keys[1]
		end),
		section("総合部門"),
	},
}
