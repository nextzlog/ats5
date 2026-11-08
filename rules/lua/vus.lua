local GEN = "総合"
local RD1 = "第1ラウンド"
local RD2 = "第2ラウンド"

local SAB = "シングルオペ オールバンド"
local SJR = "シングルオペ こども"
local SNC = "シングルオペ ニューカマー"
local MAB = "マルチオペ オールバンド"
local SWL = "SWL"
local SOP = "シングルオペ"
local VHF = "シングルオペ V/UHF"
local SHF = "シングルオペ SHF"

local B144, B430, B1_2, B2_4, B5_6, B10 = 144000, 430000, 1200000, 2400000, 5600000, 10000000
local BVHF = { B144, B430 }
local BSHF = { B1_2, B2_4, B5_6, B10 }
local BALL = { B144, B430, B1_2, B2_4, B5_6, B10 }
local LABEL = { [B144] = "144MHz", [B430] = "430MHz", [B1_2] = "1.2GHz", [B2_4] = "2.4GHz", [B5_6] = "5.6GHz", [B10] = "10GHz" }
local SCORE_BAND = { [B144] = 1, [B430] = 1, [B1_2] = 1, [B2_4] = 10, [B5_6] = 10, [B10] = 15 }
local SCORE_MODE = { CW = 2, AM = 1, FM = 1, SSB = 1 }
local CITIES = ats.dat("http://ja6ycu.in.coocan.jp/ZLOG/acag.dat")

local function contains(list, value)
	for _, v in ipairs(list) do
		if v == value then
			return true
		end
	end
	return false
end

local function section(name, code, bands)
	return ats.section {
		name = name,
		code = code,
		bands = bands,
		modes = { "CW", "AM", "FM", "SSB" },
		hours = ats.hours(9, 15),
		cities = CITIES,
		check = function(it)
			local hour = ats.hour(it.time or "", 540) or -1
			if hour < 11 and contains(BSHF, it.khz) then
				return true
			end
			if hour > 11 and contains(BVHF, it.khz) then
				return true
			end
			return false, tostring(it.band) .. " @ " .. tostring(it.time)
		end,
		points = function(it)
			return SCORE_BAND[it.khz] * SCORE_MODE[string.upper(it.mode)]
		end,
		unique = function(it)
			local mode = string.gsub(string.upper(it.mode or ""), "AM", "SSB")
			return { it.call or "", it.band or "", mode }
		end,
		award = function(n)
			return n >= 30 and 3 or n >= 11 and 2 or 1
		end,
	}
end

local sections = { ats.absence(GEN), ats.absence(RD1), ats.absence(RD2) }
for _, area in ipairs({ 1, 2, 3, 4, 5, 6, 7, 8, 9, 0 }) do
	local prefix = area .. "エリア "
	for _, name in ipairs({ SAB, SJR, SNC, MAB, SWL }) do
		table.insert(sections, section(prefix .. name, GEN, BALL))
	end
	for _, band in ipairs(BSHF) do
		table.insert(sections, section(prefix .. SOP .. " " .. LABEL[band], RD1, { band }))
	end
	table.insert(sections, section(prefix .. SHF, RD1, BSHF))
	for _, band in ipairs(BVHF) do
		table.insert(sections, section(prefix .. SOP .. " " .. LABEL[band], RD2, { band }))
	end
	table.insert(sections, section(prefix .. VHF, RD2, BVHF))
end

return ats.program {
	name = "電通大VUSコンテスト",
	host = "JA1ZGP",
	mail = "uectest-info@ja1zgp.com",
	link = "https://www.ja1zgp.com/uectest-vus_public_info",
	help = "未対応の交信記録を提出した場合は、運営に報告が必要です。",
	start = ats.weekday(5, 1, "mon"),
	deadline = function(year)
		return ats.days(ats.nth(year, 5, 1, 1), 14)
	end,
	sections = sections,
	conflict = function(entries)
		local out = {}
		local codes, vhf, shf = {}, false, false
		for _, e in ipairs(entries) do
			codes[e.code] = true
			vhf = vhf or string.find(e.name, VHF, 1, true) ~= nil
			shf = shf or string.find(e.name, SHF, 1, true) ~= nil
		end
		if codes[GEN] and (codes[RD1] or codes[RD2]) then
			table.insert(out, "総合部門とラウンド部門には、同時に参加できません。")
		end
		if vhf and shf then
			table.insert(out, "V/UHF部門とSHF部門には、同時に参加できません。")
		end
		return out
	end,
}
