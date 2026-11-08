local JST = 540

local HIGH = { [14000] = true, [21000] = true, [28000] = true, [50000] = true }
local LOW = { [1900] = true, [3500] = true, [7000] = true }

local SIN_HB = "個人部門 09:00-12:00"
local SIN_LB = "個人部門 16:00-20:00"
local SIN_DG = "個人部門 13:00-15:00"
local MUL_AB = "団体部門 アナログ"
local MUL_DG = "団体部門 デジタル"
local MIX_JS = "総合部門"

local function hour(it)
	return ats.hour(it.time or "", JST) or -1
end

local function analog(it)
	return ats.morse(it.mode or "") or ats.phone(it.mode or "")
end

local function high(it)
	local h = hour(it)
	return h >= 9 and h <= 11 and analog(it) and HIGH[it.khz or 0] == true
end

local function low(it)
	local h = hour(it)
	return h >= 16 and h <= 19 and analog(it) and LOW[it.khz or 0] == true
end

local function digital(it)
	local h = hour(it)
	return h >= 13 and h <= 14 and ats.digit(it.mode or "") and it.khz == 7000
end

local function digits(code, min, max)
	return #code >= min and #code <= max and code:match("^%d+$") ~= nil
end

local function place(it)
	return ats.city(it.code or "")
end

local function inside(it)
	local c = place(it)
	if not c then
		return false
	end
	local code = it.code
	if c.area == "1" then
		return digits(code, 4, 9)
	elseif c.area == "8" then
		return digits(code, 3, 3)
	else
		return code ~= "48" and not digits(code, 3, 9)
	end
end

local function outside(it)
	local c = place(it)
	return c ~= nil and c.area == "1" and digits(it.code, 4, 9)
end

local HOURS = {
	high = { high, "tHI?" },
	low = { low, "tLO?" },
	digital = { digital, "tDG?" },
	analog = { function(it) return high(it) or low(it) end, "tAN?" },
	total = { function(it) return high(it) or low(it) or digital(it) end, "tJS?" },
}

local MODES = {
	morse = { function(it) return ats.morse(it.mode or "") end, "MORSE?" },
	analog = { analog, "CW/PH?" },
	digital = { function(it) return ats.digit(it.mode or "") end, "DIGIT?" },
	any = { function(it) return analog(it) or ats.digit(it.mode or "") end, "AN/DG?" },
}

local function cities(within)
	local out = {}
	for _, c in ipairs(ats.cities()) do
		if (c.area == "1") == within and not digits(c.code, 2, 3) then
			table.insert(out, c.code)
		end
	end
	return out
end

local INSIDE = cities(true)
local OUTSIDE = cities(false)

local function mode_class(it)
	if ats.morse(it.mode or "") then
		return "1"
	elseif ats.phone(it.mode or "") then
		return "2"
	elseif ats.digit(it.mode or "") then
		return "3"
	end
	return ""
end

local function section(within, multi, kind, label, code, hours, mode, khz)
	local area = within and "1エリア内" or "1エリア外"
	local team = multi and "団体" or "個人"
	return {
		name = area .. " " .. team .. " " .. kind .. " " .. label .. "部門",
		code = code,
		cities = within and INSIDE or OUTSIDE,
		verify = function(it)
			if multi and (it.name == nil or it.name == "") then
				return nil, "MulOp?"
			end
			if within and not inside(it) then
				return nil, "内?"
			end
			if not within and not outside(it) then
				return nil, "外?"
			end
			if not HOURS[hours][1](it) then
				return nil, HOURS[hours][2]
			end
			if not MODES[mode][1](it) then
				return nil, MODES[mode][2]
			end
			if khz and it.khz ~= khz then
				return nil, "BAND?"
			end
			return 1
		end,
		unique = function(it)
			return { it.call or "", it.band or "", mode_class(it) }
		end,
		entity = function(it)
			local mult = { it.band or "", it.code or "" }
			if multi and it.name and it.name ~= "" then
				return { mult, { it.name } }
			end
			return { mult }
		end,
		result = function(s)
			local names = math.max(s.keys[2] or 1, 1)
			return math.ceil(s.score * s.keys[1] / names)
		end,
	}
end

local sections = {}

for _, code in ipairs({ SIN_HB, SIN_LB, SIN_DG, MUL_AB, MUL_DG, MIX_JS }) do
	table.insert(sections, { name = code .. " 不参加 (チェックログ)", code = code, absence = true })
end

local LOWS = { { "1.9MHz", 1900 }, { "3.5MHz", 3500 }, { "7MHz", 7000 }, { "1.9-7MHz", nil } }
local HIGHS = { { "14MHz", 14000 }, { "21MHz", 21000 }, { "28MHz", 28000 }, { "50MHz", 50000 }, { "14-50MHz", nil } }
local KINDS = { { "電信限定", "morse" }, { "電信電話", "analog" } }

for _, group in ipairs({ { LOWS, SIN_LB, "low" }, { HIGHS, SIN_HB, "high" } }) do
	for _, within in ipairs({ true, false }) do
		for _, kind in ipairs(KINDS) do
			for _, band in ipairs(group[1]) do
				table.insert(sections, section(within, false, kind[1], band[1], group[2], group[3], kind[2], band[2]))
			end
		end
	end
end

for _, within in ipairs({ true, false }) do
	for _, kind in ipairs(KINDS) do
		table.insert(sections, section(within, true, kind[1], "", MUL_AB, "analog", kind[2], nil))
	end
end

for _, spec in ipairs({
	{ false, "デジタル", SIN_DG, "digital", "digital" },
	{ true, "デジタル", MUL_DG, "digital", "digital" },
	{ false, "総合", MIX_JS, "total", "any" },
	{ true, "総合", MIX_JS, "total", "any" },
}) do
	for _, within in ipairs({ true, false }) do
		table.insert(sections, section(within, spec[1], spec[2], "", spec[3], spec[4], spec[5], nil))
	end
end

local function has(name, part)
	return string.find(name, part, 1, true) ~= nil
end

local function conflict(entries)
	local out = {}
	local multi, total, inside_any, inside_total = 0, false, false, false
	for _, name in ipairs(entries) do
		if has(name, "団体") then
			multi += 1
		end
		if has(name, "総合") then
			total = true
		end
		if has(name, "1エリア内") then
			inside_any = true
			if has(name, "総合") then
				inside_total = true
			end
		end
	end
	if multi > 0 and multi < #entries then
		table.insert(out, "個人部門と団体部門には、同時に参加できません。")
	end
	if #entries > 0 and not total then
		table.insert(out, "総合部門を選んでください。他の部門に参加する場合、総合部門は不参加にできません。")
	end
	if inside_any and total and not inside_total then
		table.insert(out, "1エリア内の部門に参加する場合は、総合部門も1エリア内を選んでください。")
	end
	return out
end

local function start(year)
	return ats.nth(year, 6, 4, 6)
end

return {
	name = "ALLJA1",
	host = "東大無線部",
	mail = "allja1.info@gmail.com",
	link = "https://ja1zlo.u-tokyo.org/allja1",
	help = [[
以下の場合は、提出できません。

- 記載漏れ
- 総合部門が不参加
- 個人部門と団体部門が混在

なお、未対応の交信記録を提出した場合は、運営に報告が必要です。
]],
	start = start,
	deadline = function(year)
		return ats.days(start(year), 21)
	end,
	conflict = conflict,
	sections = sections,
}
