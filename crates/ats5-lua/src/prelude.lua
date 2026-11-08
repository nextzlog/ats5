local WEEKDAYS = { mon = 1, tue = 2, wed = 3, thu = 4, fri = 5, sat = 6, sun = 7 }

local function set(list)
	local out = {}
	for _, v in ipairs(list or {}) do
		out[v] = true
	end
	return out
end

local function upper(text)
	return string.upper(text or "")
end

function ats.hours(from, to)
	local out = {}
	for h = from, to - 1 do
		table.insert(out, h)
	end
	return out
end

function ats.on(month, day, shift)
	return function(year)
		return ats.date(year + (shift or 0), month, day)
	end
end

function ats.weekday(month, nth, day)
	return function(year)
		return ats.nth(year, month, nth, WEEKDAYS[day])
	end
end

function ats.report(code)
	local text = code or ""
	return (string.gsub(text, "^599?", ""))
end

function ats.section(spec)
	local bands = spec.bands and set(spec.bands)
	local modes = spec.modes and {}
	for _, m in ipairs(spec.modes or {}) do
		modes[upper(m)] = true
	end
	local hours = spec.hours and set(spec.hours)
	local zone = spec.zone or 540
	local cities = nil
	if spec.cities then
		cities = {}
		for _, c in ipairs(spec.cities) do
			cities[type(c) == "table" and c.code or c] = true
		end
	end
	local trim = spec.trim or function(code)
		return code
	end
	local function city(it)
		return trim(ats.report(it.code))
	end
	local points = spec.points or function()
		return 1
	end
	return {
		name = spec.name,
		code = spec.code or "部門",
		cities = spec.cities,
		city = city,
		verify = function(it)
			if bands and not bands[it.khz or 0] then
				return nil, tostring(it.band)
			end
			if modes and not modes[upper(it.mode)] then
				return nil, tostring(it.mode)
			end
			if hours and not hours[ats.hour(it.time or "", zone) or -1] then
				return nil, tostring(it.time)
			end
			if cities and not cities[city(it)] then
				return nil, tostring(it.code)
			end
			if spec.check then
				local ok, why = spec.check(it)
				if not ok then
					return nil, why or "invalid"
				end
			end
			return points(it)
		end,
		unique = spec.unique or function(it)
			return { it.call or "", it.band or "" }
		end,
		entity = spec.entity or function(it)
			return { { it.band or "", city(it) } }
		end,
		result = spec.result or function(s)
			return s.score > 0 and s.score * s.keys[1] or 0
		end,
		award = spec.award,
	}
end

function ats.absence(code, name)
	return { name = name or (code .. " 不参加 または交信記録のみ提出"), code = code, absence = true }
end

function ats.program(spec)
	local start = spec.start
	local limit = spec.limit
	return {
		name = spec.name,
		host = spec.host,
		mail = spec.mail,
		link = spec.link,
		help = spec.help,
		start = start,
		final = spec.final,
		deadline = spec.deadline or function(year)
			return ats.months(start(year), 1)
		end,
		sections = spec.sections,
		conflict = spec.conflict and function(names)
			local found = {}
			for _, name in ipairs(names) do
				for _, sec in ipairs(spec.sections) do
					if sec.name == name then
						table.insert(found, sec)
						break
					end
				end
			end
			return spec.conflict(found)
		end,
		finish = spec.finish,
		limit = type(limit) == "table" and function(code)
			return limit[code] or 1
		end or limit,
	}
end

function ats.unique_names(entries)
	local seen = {}
	for _, name in ipairs(entries) do
		if seen[name] then
			return false
		end
		seen[name] = true
	end
	return true
end
