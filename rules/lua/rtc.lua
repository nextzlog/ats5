local JST = 540
local BANDS = { [3500] = true, [7000] = true, [14000] = true, [21000] = true, [28000] = true, [50000] = true }

local function prefecture(code)
	return code:match("^%d%d$") ~= nil
end

local CITIES = {}
for _, c in ipairs(ats.cities()) do
	if prefecture(c.code) then
		table.insert(CITIES, c.code)
	end
end

local function area(it)
	local c = ats.city(it.code or "")
	if not c then
		return false
	end
	if c.area == "8" then
		return it.code:match("^%d%d%d$") ~= nil
	end
	return it.code ~= "48" and not it.code:match("^%d%d%d+$")
end

local MODES = {
	["電信限定"] = function(m) return ats.morse(m) end,
	["電話限定"] = function(m) return ats.phone(m) end,
	["電信電話"] = function(m) return ats.morse(m) or ats.phone(m) end,
}

local function section(team, kind)
	return {
		name = team .. " " .. kind .. " 部門",
		code = kind,
		cities = CITIES,
		verify = function(it)
			if team == "団体" and (it.name == nil or it.name == "") then
				return nil, "MulOp?"
			end
			if not BANDS[it.khz or 0] then
				return nil, "band?"
			end
			if not area(it) then
				return nil, "area?"
			end
			if not MODES[kind](it.mode or "") then
				return nil, "mode?"
			end
			return it.sign and 2 or 1
		end,
		unique = function(it)
			return { it.call or "", it.band or "" }
		end,
		entity = function(it)
			return { { it.band or "", it.code or "" } }
		end,
		result = function(s)
			return s.score * s.keys[1]
		end,
	}
end

local sections = {}
for _, team in ipairs({ "個人", "団体" }) do
	for _, kind in ipairs({ "電信限定", "電話限定", "電信電話" }) do
		table.insert(sections, section(team, kind))
	end
end

local start = ats.weekday(7, 4, "mon")

return {
	name = "リアルタイムコンテスト",
	host = "東大無線部",
	mail = "allja1.info@gmail.com",
	link = "https://ja1zlo.u-tokyo.org/rt",
	help = [[
以下の場合は、提出できません。

- 記載漏れ
- 個人部門と団体部門が混在

[zLogの設定方法はこちら](https://nextzlog.dev/real)
]],
	start = start,
	final = function(year)
		return ats.months(start(year), 5)
	end,
	deadline = function(year)
		return ats.months(start(year), 5)
	end,
	finish = true,
	sections = sections,
	conflict = function(entries)
		local single, multi = false, false
		for _, name in ipairs(entries) do
			single = single or string.find(name, "個人", 1, true) ~= nil
			multi = multi or string.find(name, "団体", 1, true) ~= nil
		end
		if single and multi then
			return { "個人部門と団体部門には、同時に参加できません。" }
		end
		return {}
	end,
}
