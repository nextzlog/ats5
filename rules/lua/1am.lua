local CITIES = ats.dat("http://ja6ycu.in.coocan.jp/ZLOG/ja1.dat")

local function section(name)
	return ats.section {
		name = name,
		bands = { 50000 },
		modes = { "AM" },
		hours = ats.hours(10, 13),
		cities = CITIES,
		points = function(it)
			local rcvd = it.rcvd.code or ""
			local sent = it.sent.code or ""
			return (#rcvd >= 4 or #sent >= 4) and 2 or 1
		end,
		unique = function(it)
			return { it.call or "" }
		end,
		entity = function(it)
			return { it.code or "" }
		end,
	}
end

return ats.program {
	name = "1エリアAMコンテスト",
	host = "まんなかくらぶ",
	mail = "1amtest-info@example.com",
	link = "https://6mnet.jp/mannaka/blog",
	help = "未対応の交信記録を提出した場合は、運営に報告が必要です。",
	start = ats.weekday(12, 4, "sun"),
	deadline = ats.on(1, 31),
	sections = {
		section("1エリア内固定局"),
		section("1エリア内移動局"),
		section("1エリア外局"),
		section("QRP"),
		section("SWL"),
	},
}
