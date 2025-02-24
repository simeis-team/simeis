import os
import sys
import math
import time
import json
import requests
import traceback
import urllib.parse

TESTS = []

FAILFAST=os.getenv("CI") is None

def functest(f):
    assert f.__name__.startswith("test_")
    name = f.__name__.removeprefix("test_").replace("_", " ").capitalize()
    global TESTS
    TESTS.append(f.__name__)
    def decorated(tester, *args, **kwargs):
        tester.current_test = name
        tester.trace = []
        f(tester, *args, **kwargs)
        tester.disp_ok()
    return decorated

class Tester:
    def __init__(self, host, port):
        self.host = host
        self.port = port
        self.key = None
        self.current_test = "No Test"
        self.trace = []
        self.saved_errors = {}
        self.indent = 30

    def addtrace(self, *args):
        self.trace.append(" ".join([ str(v) for v in args ]))

    def disp_error(self, exc):
        print("!!! Test", self.current_test, " "*(self.indent - len(self.current_test)), "ERR")

        errdata = "Trace of the test:\n"
        errdata += ("=" * 10) + " TRACE " + ("=" * 10) + "\n"
        for line in self.trace:
            errdata += line + "\n"
        errdata += ("=" * 10) + "  END  " + ("=" * 10) + "\n"
        tb_str = traceback.format_exception(
            exc,
            value=exc,
            tb=exc.__traceback__,
        )
        for line in tb_str:
            if (" assert " in line) or ("in test_" in line):
                errdata += line.strip() + "\n"
        self.saved_errors[self.current_test] = errdata

    def disp_ok(self):
        print("  * Test", self.current_test, " "*(self.indent - len(self.current_test)), "OK")

    def request(self, endpoint, method="GET", body={}, expcode=200, **kwargs):
        self.addtrace(method, endpoint, "with body", body, "expected code", expcode)
        if "key" not in kwargs:
            if self.key is not None:
                kwargs["key"] = self.key

        if "headers" not in kwargs:
            headers = {}

        url = f"http://{self.host}:{self.port}/{endpoint}"
        qry = urllib.parse.urlencode(kwargs)
        if len(qry) > 0:
            self.addtrace("Query args", kwargs)
            url += "?" + qry

        try:
            if method == "GET":
                headers["Content-Type"] = "application/x-www-form-urlencoded"
                got = requests.get(url, headers=headers)
            elif method == "POST":
                headers["Content-Type"] = "application/json"
                got = requests.post(url, data=json.dumps(body), headers=headers)
            else:
                raise Exception("Test uses an unknown method", method)
        except requests.exceptions.ConnectionError:
            print("")
            print("Server panicked")
            print("===== TRACE =====")
            print("\n".join(self.trace))
            print("=================")
            print("Server panicked")
            print("")
            sys.exit(1)

        self.addtrace("Got result from server", got.status_code, got.text)
        assert got.status_code == expcode

        if expcode != 200:
            return got
        else:
            data = json.loads(got.text)
            self.addtrace("Decoded JSON data to", data)
            assert "error" in data.keys()
            return data

    def create_test_player(self, name=None):
        if name is None:
            name = "TestPlayer_" + self.current_test.replace(" ", "_").lower()
        got = self.assert_ok("/player/new", method="POST", body={"name": name})
        self.key = self.assert_got(got, "key", None)
        self.id = self.assert_got(got, "playerId", None)
        player = self.assert_ok(f"/player/{self.id}")
        self.station = list(self.assert_got(player, "stations", None).keys())[0]
        return player

    def buy_a_ship(self, retind=0):
        player = self.assert_ok(f"/player/{self.id}")
        got = self.assert_ok(f"/station/{self.station}/shipyard/list")
        shiplist = self.assert_got(got, "ships", None)
        assert len(shiplist) > 0
        for ship in shiplist:
            if ship["price"] <= player["money"]:
                self.assert_ok(f"/station/{self.station}/shipyard/buy/" + str(ship["id"]))
        after = self.assert_ok(f"/player/{self.id}")
        assert len(after["ships"]) > 0
        ship = after["ships"][retind]
        return self.assert_got(ship, "id", None)

    def setup_crew(self, shipid):
        pilot = self.assert_ok(f"/station/{self.station}/crew/hire/pilot")
        pilotid = self.assert_got(pilot, "id", None)
        self.assert_ok(f"/station/{self.station}/crew/assign/{pilotid}/{shipid}/0")

    def assert_ok(self, endpoint, **kwargs):
        got = self.request(endpoint, **kwargs)
        self.addtrace("Expect this data to be OK")
        assert got["error"] == "ok"
        return got

    def assert_error(self, endpoint, errtype=None, **kwargs):
        got = self.request(endpoint, **kwargs)
        self.addtrace("Expect this data to be ERR")
        assert "type" in got
        if errtype is None:
            self.addtrace("Expect this data to have any error")
            assert got["error"] != "ok"
        else:
            self.addtrace("Expect this data to have an error", errtype, "got", got["type"])
            assert got["type"] == errtype
        return got

    def assert_got(self, data, key, val, negate=False):
        if not negate:
            self.addtrace("Expect this data to have the key", key)
            assert key in data.keys()
        else:
            self.addtrace("Expect this data to NOT have the key", key)
            assert key not in data.keys()
            return None

        if val is not None:
            self.addtrace(f"Expect data[{key}] to have value {val}")
            assert data[key] == val
        return data[key]

    @functest
    def test_ping(self):
        got = self.assert_ok("/ping")
        self.assert_got(got, "ping", "pong")

    @functest
    def test_create_player(self):
        self.assert_error("/player/53", errtype="NoPlayerKey")
        self.assert_error("/player/53", errtype="PlayerNotFound(53)", key=12341234)

        self.request("/player/new", method="POST", body={}, expcode=400)
        got = self.assert_ok("/player/new", method="POST", body={"name": "Testuser"})
        self.key = self.assert_got(got, "key", None)
        self.id = self.assert_got(got, "playerId", 52238)

        pl2 = self.assert_ok("/player/new", method="POST", body={"name": "Testuser2"})
        pl2id = self.assert_got(pl2, "playerId", None)
        self.assert_error("/player/new",
            method="POST", body={"name": "Testuser2"},
            errtype="PlayerAlreadyExists(\"Testuser2\")"
        )
        pl2_key = self.assert_got(pl2, "key", None)

        pl1 = self.assert_ok(f"/player/{self.id}")
        self.assert_got(pl1, "money", 30000)

        got = self.assert_ok(f"/player/{self.id}", key=pl2["key"])
        self.assert_got(got, "money", None, negate=True)

        pl2 = self.assert_ok(f"/player/{pl2id}", key=pl2["key"])
        self.assert_got(pl2, "money", self.assert_got(pl1, "money", None))

    @functest
    def test_shipyard(self):
        self.create_test_player()

        got = self.assert_ok(f"/player/{self.id}")
        beforemoney = self.assert_got(got, "money", None)

        got = self.assert_ok(f"/station/{self.station}/shipyard/list")
        shiplist = self.assert_got(got, "ships", None)
        assert len(shiplist) == 3
        ships = {}
        for ship in shiplist:
            self.assert_got(ship, "id", None)
            self.assert_got(ship, "modules", {})
            self.assert_got(ship, "reactor_power", None)
            self.assert_got(ship, "cargo_capacity", None)
            self.assert_got(ship, "fuel_tank_capacity", None)
            self.assert_got(ship, "hull_decay_capacity", None)
            self.assert_got(ship, "price", None)
            ships[ship["id"]] = ship

        n = 0
        while n in ships.keys():
            n += 1
        self.assert_error(f"/station/{self.station}/shipyard/buy/{n}", errtype=f"ShipNotFound({n})")

        sid_cant = [id for id, ship in ships.items() if ship["price"] > beforemoney ][0]
        self.assert_error(f"/station/{self.station}/shipyard/buy/{sid_cant}",
            errtype="NotEnoughMoney(30000.0, {})".format(ships[sid_cant]["price"])
        )

        sid_can = [id for id, ship in ships.items() if ship["price"] <= beforemoney ][0]
        self.assert_ok(f"/station/{self.station}/shipyard/buy/{sid_can}")
        after = self.assert_ok(f"/player/{self.id}")
        aftermoney = self.assert_got(after, "money", None)
        ships = self.assert_got(after, "ships", None)
        assert len(ships) > 0
        assert ships[0]["id"] == sid_can
        assert aftermoney < beforemoney

        # TODO (#22)    Sell ship with given ID
        # TODO (#22)   Assert ship no longer in possession

    @functest
    def test_hire_crew(self):
        self.create_test_player()
        for ctype in ["pilot", "operator", "trader", "soldier"]:
            got = self.assert_ok(f"/station/{self.station}/crew/hire/{ctype}")
            crew_id = self.assert_got(got, "id", None)

            idled = self.assert_ok(f"/station/{self.station}/crew/idle")
            idled = self.assert_got(idled, "idle", None)
            assert len(idled) > 0
            assert idled[str(crew_id)] == { "member_type": ctype.capitalize(), "rank": 1 }
        self.assert_error(f"/station/{self.station}/crew/hire/notexist", errtype="InvalidArgument(\"crewtype\")")

    @functest
    def test_money(self):
        self.create_test_player()
        self.assert_ok(f"/station/{self.station}/crew/hire/pilot")
        before = self.assert_ok(f"/player/{self.id}")
        time.sleep(0.3)
        after = self.assert_ok(f"/player/{self.id}")
        assert before["money"] > after["money"]

    @functest
    def test_assign_crew(self):
        self.create_test_player()
        shipid = self.buy_a_ship()

        pilot = self.assert_ok(f"/station/{self.station}/crew/hire/pilot")
        pilot2 = self.assert_ok(f"/station/{self.station}/crew/hire/pilot")
        operator = self.assert_ok(f"/station/{self.station}/crew/hire/operator")
        operator2 = self.assert_ok(f"/station/{self.station}/crew/hire/operator")

        idle = self.assert_ok(f"/station/{self.station}/crew/idle")
        assert len(self.assert_got(idle, "idle", None)) == 4

        shipstatus = self.assert_ok(f"/ship/{shipid}")
        stats = self.assert_got(shipstatus, "stats", None)
        speed = self.assert_got(stats, "speed", None)
        assert speed == 0

        self.assert_ok(f"/station/{self.station}/crew/assign/" + str(pilot["id"]) + f"/{shipid}/0")
        idle = self.assert_ok(f"/station/{self.station}/crew/idle")
        assert len(self.assert_got(idle, "idle", None)) == 3

        shipstatus = self.assert_ok(f"/ship/{shipid}")
        self.assert_got(shipstatus, "crew", { str(pilot["id"]): { "member_type": "Pilot", "rank": 1 }})
        self.assert_got(shipstatus, "pilot", pilot["id"])
        stats = self.assert_got(shipstatus, "stats", None)
        speed_after = self.assert_got(stats, "speed", None)
        assert speed_after > 0

        self.assert_error(
            f"/station/{self.station}/crew/assign/" + str(pilot2["id"]) + f"/{shipid}/0",
            errtype="CrewNotNeeded",
        )
        idle = self.assert_ok(f"/station/{self.station}/crew/idle")
        assert len(self.assert_got(idle, "idle", None)) == 3
        shipstatus_after = self.assert_ok(f"/ship/{shipid}")
        assert shipstatus == shipstatus_after

        operatorid = self.assert_got(operator, "id", None)
        self.assert_error(f"/station/{self.station}/crew/assign/{operatorid}/{shipid}/0", errtype="WrongCrewType(Pilot)")

        got = self.assert_ok(f"/station/{self.station}/shop/modules/{shipid}/buy/miner")
        modid = self.assert_got(got, "id", 1)
        self.assert_ok(f"/station/{self.station}/crew/assign/{operatorid}/{shipid}/1")
        self.assert_error(f"/station/{self.station}/crew/assign/{operatorid}/{shipid}/1", errtype=f"CrewMemberNotIdle({operatorid})")
        self.assert_error(f"/station/{self.station}/crew/assign/{operator2["id"]}/{shipid}/1", errtype="CrewNotNeeded")

    @functest
    def test_travel(self):
        self.create_test_player()
        ship_id = self.buy_a_ship()
        self.setup_crew(ship_id)

        ship = self.assert_ok(f"/ship/{ship_id}")
        shippos = ship["position"]

        self.assert_error(f"/ship/{ship_id}/travelcost",
            method="POST",
            body=dict(destination=shippos),
            errtype="NullDistance",
        )

        close = self.assert_ok(f"/ship/{ship_id}/travelcost",
            method="POST",
            body=dict(destination=(shippos[0]+1, shippos[1]+1, shippos[2]+1))
        )
        dist = self.assert_got(close, "distance", None)
        assert dist is not None
        assert dist > 0.0

        far = self.assert_ok(f"/ship/{ship_id}/travelcost",
            method="POST",
            body=dict(destination=(shippos[0]+2, shippos[1]+2, shippos[2]+2))
        )

        self.assert_got(far, "distance", 2 * self.assert_got(close, "distance", None))
        self.assert_got(far, "duration", 2 * self.assert_got(close, "duration", None))
        self.assert_got(far, "fuel_consumption", 2 * self.assert_got(close, "fuel_consumption", None))
        self.assert_got(far, "hull_usage", 2 * self.assert_got(close, "hull_usage", None))
        self.assert_got(close, "direction", self.assert_got(far, "direction", None))

        cost = self.assert_ok(f"/ship/{ship_id}/travelcost",
            method="POST",
            body=dict(destination=(shippos[0]+1, shippos[1]+1, shippos[2]+1))
        )
        nadd = int(0.5 / cost["duration"]) + 1
        before = self.assert_ok(f"/ship/{ship_id}")
        beforepos = self.assert_got(before, "position", None)
        self.assert_got(before, "state", "Idle")
        cost = self.assert_ok(f"/ship/{ship_id}/navigate",
            method="POST",
            body=dict(destination=(shippos[0]+nadd, shippos[1]+nadd, shippos[2]+nadd))
        )
        time.sleep(0.2)
        during = self.assert_ok(f"/ship/{ship_id}")
        assert self.assert_got(during, "state", None) != "Idle"
        pos = self.assert_got(during, "position", None)
        assert (pos[0] > shippos[0]) and (pos[1] > shippos[1]) and (pos[2] > shippos[2])
        assert (pos[0] < shippos[0]+nadd) and (pos[1] < shippos[1]+nadd) and (pos[2] < shippos[2]+nadd)
        time.sleep(cost["duration"])

        after = self.assert_ok(f"/ship/{ship_id}")
        self.assert_got(after, "state", "Idle")
        afterpos = self.assert_got(after, "position", None)
        assert self.assert_got(after, "fuel_tank", None) < self.assert_got(before, "fuel_tank", None)
        assert self.assert_got(after, "hull_decay", None) > self.assert_got(before, "hull_decay", None)

        self.addtrace(
            "nadd = ", nadd,
            "diff coord = ",
            afterpos[0] - beforepos[0],
            afterpos[1] - beforepos[1],
            afterpos[2] - beforepos[2],
        )
        assert (afterpos[0] - beforepos[0]) == nadd
        assert (afterpos[1] - beforepos[1]) == nadd
        assert (afterpos[2] - beforepos[2]) == nadd

        cost = self.assert_ok(f"/ship/{ship_id}/navigate",
            method="POST",
            body=dict(destination=(shippos[0], shippos[1], shippos[2]))
        )
        time.sleep(cost["duration"])

        back = self.assert_ok(f"/ship/{ship_id}")
        self.addtrace("start", afterpos, "now", back["position"])
        self.addtrace("want", shippos, "got", back["position"])
        pos = self.assert_got(back, "position", shippos)
        assert self.assert_got(back, "fuel_tank", None) < self.assert_got(after, "fuel_tank", None)
        assert self.assert_got(back, "hull_decay", None) > self.assert_got(after, "hull_decay", None)
        assert self.assert_got(back, "fuel_tank", None) < self.assert_got(before, "fuel_tank", None)
        assert self.assert_got(back, "hull_decay", None) > self.assert_got(before, "hull_decay", None)

    @functest
    def test_scan(self):
        self.create_test_player()

        scan = self.assert_ok(f"/station/{self.station}/scan")

        planets = self.assert_got(scan, "planets", None)
        assert len(planets) > 0

        stations = self.assert_got(scan, "stations", None)
        assert len(stations) > 0
        assert any([sta["id"] == int(self.station) for sta in stations])

    @functest
    def test_extract(self):
        self.create_test_player("test-rich-extract")
        shipid = self.buy_a_ship()
        self.setup_crew(shipid)

        got = self.assert_ok(f"/station/{self.station}/shop/modules/{shipid}/buy/miner")
        modid = self.assert_got(got, "id", 1)
        operator = self.assert_ok(f"/station/{self.station}/crew/hire/operator")
        opid = self.assert_got(operator, "id", None)
        self.assert_ok(f"/station/{self.station}/crew/assign/{opid}/{shipid}/{modid}")

        got = self.assert_ok(f"/station/{self.station}/shop/modules/{shipid}/buy/gassucker")
        modid = self.assert_got(got, "id", 2)
        operator = self.assert_ok(f"/station/{self.station}/crew/hire/operator")
        opid = self.assert_got(operator, "id", None)
        self.assert_ok(f"/station/{self.station}/crew/assign/{opid}/{shipid}/{modid}")
        self.addtrace("Got ship all set up")

        scan = self.assert_ok(f"/station/{self.station}/scan")
        ship = self.assert_ok(f"/ship/{shipid}")
        distances = []
        for planet in self.assert_got(scan, "planets", None):
            distances.append((planet, compute_distance(planet["position"], ship["position"])))

        best = sorted(distances, key=lambda f: f[1])[0][0]
        self.addtrace("Traveling to", best)

        cost = self.assert_ok(
            f"/ship/{shipid}/navigate",
            method="POST",
            body=dict(destination=best["position"])
        )
        time.sleep(cost["duration"] + 0.2)

        ship = self.assert_ok(f"/ship/{shipid}")
        self.assert_got(ship, "state", "Idle")
        self.addtrace("Ship arrived")
        got = self.assert_ok(f"/ship/{shipid}/extraction/start")
        assert ("Stone" in got) or ("Helium" in got)
        time.sleep(0.5)
        before = self.assert_ok(f"/ship/{shipid}")
        cargob = self.assert_got(before, "cargo", None)
        time.sleep(0.5)
        after = self.assert_ok(f"/ship/{shipid}")
        cargoa = self.assert_got(after, "cargo", None)
        assert cargob["usage"] < cargoa["usage"]

        player = self.assert_ok(f"/player/{self.id}")
        stationid = list(player["stations"].keys())[0]
        self.assert_error(
            f"/ship/{shipid}/navigate",
            method="POST",
            body=dict(destination=player["stations"][stationid]),
            errtype="ShipNotIdle",
        )
        self.assert_ok(f"/ship/{shipid}/extraction/stop")

    # Uses environment set up by previous test
    @functest
    def test_unload_cargo(self):
        player = self.assert_ok(f"/player/{self.id}")
        stationid = list(player["stations"].keys())[0]
        ship = player["ships"][0]
        shipid = ship["id"]

        cost = self.assert_ok(
            f"/ship/{shipid}/navigate",
            method="POST",
            body=dict(destination=player["stations"][stationid])
        )
        time.sleep(cost["duration"] + 0.2)

        resname = list(ship["cargo"]["resources"].keys())[0]
        resamnt = ship["cargo"]["resources"][resname]
        resname = resname.lower()
        self.assert_error(f"/ship/{shipid}/unload/{resname}/{resamnt}", errtype="CargoFull")

        initprice = self.assert_ok(f"/station/{stationid}/shop/cargo/price")
        initprice = self.assert_got(initprice, "price", 1.0)
        self.assert_ok(f"/station/{stationid}/shop/cargo/buy/2000")
        afterprice = self.assert_ok(f"/station/{stationid}/shop/cargo/price")
        afterprice = self.assert_got(afterprice, "price", None)
        assert afterprice > initprice

        cargobefore = self.assert_ok(f"/station/{stationid}/cargo")
        usagebefore = self.assert_got(cargobefore, "usage", 0.0)
        got = self.assert_ok(f"/ship/{shipid}/unload/{resname}/{resamnt}")
        self.assert_got(got, "unloaded", resamnt)
        cargoafter = self.assert_ok(f"/station/{stationid}/cargo")
        usageafter = self.assert_got(cargoafter, "usage", None)
        assert usageafter > usagebefore

        shipafter = self.assert_ok(f"/ship/{shipid}")
        shipcargo = self.assert_got(shipafter, "cargo", None)
        assert shipcargo["usage"] == 0

def compute_distance(a, b):
    sum = 0
    sum += (b[0] - a[0]) ** 2
    sum += (b[1] - a[1]) ** 2
    sum += (b[2] - a[2]) ** 2
    return math.sqrt(sum)

if __name__ == "__main__":
    t = Tester(sys.argv[1], sys.argv[2])
    nok = 0
    nerrors = 0
    for test in TESTS:
        try:
            getattr(t, test)()
            nok += 1
        except AssertionError as exc:
            t.disp_error(exc)
            nerrors += 1
            if FAILFAST:
                break;
    if FAILFAST and nerrors > 0:
        print("")
        print("")
        print("FAIL FAST")
        print("")
    else:
        print(f"\nAll tests finished, {nok} OK, {nerrors} ERR\n")

    for (key, errdata) in t.saved_errors.items():
        side = (50 - len(key)) // 2
        print("*" * side, key, "*" * side )
        print(errdata)
        print("*" * 52)
        print("")

    sys.exit(nerrors > 0)
