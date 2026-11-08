local CRAFT = "自作機"
local OTHER = "一般"
local CLASSES = { [CRAFT] = "H", [OTHER] = "G" }
local MODES = { "CW", "PH", "AM", "FM", "SSB" }
local BANDS = { { "1.9MHz", 1900 }, { "3.5MHz", 3500 }, { "7MHz", 7000 }, { "14MHz", 14000 }, { "21MHz", 21000 }, { "28MHz", 28000 }, { "50MHz", 50000 } }
local CITIES = ats.dat("http://ja6ycu.in.coocan.jp/ZLOG/xpo.dat")

local function section(name, bands)
	return ats.section {
		name = name,
		bands = bands,
		modes = MODES,
		hours = ats.hours(13, 21),
		cities = CITIES,
		trim = function(code)
			return (string.gsub(code, "[Pp]$", ""))
		end,
		check = function(it)
			if string.match(ats.report(it.code), "[Pp]$") then
				return true
			end
			return false, "P?"
		end,
		unique = function(it)
			return { it.call or "", it.band or "", tostring(ats.morse(it.mode or "")) }
		end,
		award = function()
			return 1
		end,
	}
end

local sections = { ats.absence("部門", "チェックログ (CHECKLOG)") }
for _, kind in ipairs({ CRAFT, OTHER }) do
	local all = {}
	for _, b in ipairs(BANDS) do
		local short = CLASSES[kind] .. string.gsub(string.gsub(b[1], "%.", ""), "MHz", "")
		table.insert(sections, section(short .. " " .. b[1] .. "バンド " .. kind, { b[2] }))
		table.insert(all, b[2])
	end
	table.insert(sections, section(CLASSES[kind] .. "A オールバンド " .. kind, all))
end

return ats.program {
	name = "QRPコンテスト",
	host = "JARL QRP CLUB",
	mail = "contest2@jaqrp.net",
	link = "https://www2.jaqrp.org/contests/",
	help = [[
参加は1種目のみです。
交換するナンバーは、RST、運用場所を示す都府県・地域等ナンバー、アルファベットのPです。
種目「自作機」で参加する場合は、補助資料を別途、運営に送ってください。
]],
	start = ats.on(11, 3),
	deadline = ats.on(11, 17),
	sections = sections,
	conflict = function(entries)
		if #entries > 1 then
			return { "参加は1種目のみです。" }
		end
		return {}
	end,
}
