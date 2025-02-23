import sys
import time
import json
import requests
import traceback
import urllib.parse

TESTS = []

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
        print(exc)
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

        if method == "GET":
            headers["Content-Type"] = "application/x-www-form-urlencoded"
            got = requests.get(url, headers=headers)
        elif method == "POST":
            headers["Content-Type"] = "application/json"
            got = requests.post(url, data=json.dumps(body), headers=headers)
        else:
            raise Exception("Test uses an unknown method", method)

        self.addtrace("Got result from server", got.status_code, got.text)
        assert got.status_code == expcode

        if expcode != 200:
            return got
        else:
            data = json.loads(got.text)
            self.addtrace("Decoded JSON data to", data)
            assert "error" in data.keys()
            return data

    def create_test_player(self):
        name = "TestPlayer_" + self.current_test.replace(" ", "_").lower()
        got = self.assert_ok("/newplayer", method="POST", body={"name": name})
        self.key = self.assert_got(got, "key", None)
        self.id = self.assert_got(got, "playerId", None)

    def buy_a_ship(self, retind=0):
        player = self.assert_ok(f"/player/{self.id}")
        got = self.assert_ok("/shipyard/list")
        shiplist = self.assert_got(got, "ships", None)
        assert len(shiplist) > 0
        for ship in shiplist:
            if ship["price"] <= player["money"]:
                self.assert_ok("/shipyard/buy/" + str(ship["id"]))
        after = self.assert_ok(f"/player/{self.id}")
        assert len(after["ships"]) > 0
        return after["ships"][retind]

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

        self.request("/newplayer", method="POST", body={}, expcode=400)
        got = self.assert_ok("/newplayer", method="POST", body={"name": "Testuser"})
        self.key = self.assert_got(got, "key", None)
        self.id = self.assert_got(got, "playerId", 11070862243173938738)

        pl2 = self.assert_ok("/newplayer", method="POST", body={"name": "Testuser2"})
        self.assert_error("/newplayer",
            method="POST", body={"name": "Testuser2"},
            errtype="PlayerAlreadyExists(\"Testuser2\")"
        )
        pl2_key = self.assert_got(pl2, "key", None)

        got = self.assert_ok(f"/player/{self.id}")
        self.assert_got(got, "money", 30000)

        got = self.assert_ok(f"/player/{self.id}", key=pl2["key"])
        self.assert_got(got, "money", None, negate=True)

    @functest
    def test_shipyard(self):
        self.create_test_player()

        got = self.assert_ok(f"/player/{self.id}")
        beforemoney = self.assert_got(got, "money", 30000)

        got = self.assert_ok("/shipyard/list")
        shiplist = self.assert_got(got, "ships", None)
        assert len(shiplist) == 3
        ships = {}
        for ship in shiplist:
            self.assert_got(ship, "id", None)
            self.assert_got(ship, "modules", [])
            self.assert_got(ship, "reactor_power", None)
            self.assert_got(ship, "cargo_capacity", None)
            self.assert_got(ship, "fuel_tank_capacity", None)
            self.assert_got(ship, "hull_decay_capacity", None)
            self.assert_got(ship, "price", None)
            ships[ship["id"]] = ship

        n = 0
        while n in ships.keys():
            n += 1
        self.assert_error(f"/shipyard/buy/{n}", errtype=f"ShipNotFound({n})")

        sid_cant = [id for id, ship in ships.items() if ship["price"] > beforemoney ][0]
        self.assert_error(f"/shipyard/buy/{sid_cant}",
            errtype="NotEnoughMoney(30000.0, {})".format(ships[sid_cant]["price"])
        )

        sid_can = [id for id, ship in ships.items() if ship["price"] <= beforemoney ][0]
        self.assert_ok(f"/shipyard/buy/{sid_can}")
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
            got = self.assert_ok(f"/crew/hire/{ctype}")
            crew_id = self.assert_got(got, "id", None)

            idled = self.assert_ok("/crew/idle")
            idled = self.assert_got(idled, "idle", None)
            assert len(idled) > 0
            assert idled[str(crew_id)] == { "member_type": ctype.capitalize(), "rank": 1 }
        self.assert_error(f"/crew/hire/notexist", errtype="InvalidArgument(\"crewtype\")")

    @functest
    def test_money(self):
        self.create_test_player()
        self.assert_ok(f"/crew/hire/pilot")
        before = self.assert_ok(f"/player/{self.id}")
        time.sleep(0.3)
        after = self.assert_ok(f"/player/{self.id}")
        assert before["money"] > after["money"]

    @functest
    def test_assign_crew(self):
        self.create_test_player()
        ship = self.buy_a_ship()

        pilot = self.assert_ok(f"/crew/hire/pilot")
        pilot2 = self.assert_ok(f"/crew/hire/pilot")
        operator = self.assert_ok(f"/crew/hire/operator")

        idle = self.assert_ok("/crew/idle")
        assert len(self.assert_got(idle, "idle", None)) == 3

        shipstatus = self.assert_ok("/ship/" + str(ship["id"]))
        stats = self.assert_got(shipstatus, "stats", None)
        speed = self.assert_got(stats, "speed", None)
        assert speed == 0

        self.assert_ok("/crew/assign/" + str(pilot["id"]) + "/" + str(ship["id"]))
        idle = self.assert_ok("/crew/idle")
        assert len(self.assert_got(idle, "idle", None)) == 2

        shipstatus = self.assert_ok("/ship/" + str(ship["id"]))
        self.assert_got(shipstatus, "crew", { str(pilot["id"]): { "member_type": "Pilot", "rank": 1 }})
        self.assert_got(shipstatus, "pilot", pilot["id"])
        stats = self.assert_got(shipstatus, "stats", None)
        speed_after = self.assert_got(stats, "speed", None)
        assert speed_after > 0

        self.assert_error(
            "/crew/assign/" + str(pilot2["id"]) + "/" + str(ship["id"]),
            errtype="ShipAlreadyHasPilot",
        )
        idle = self.assert_ok("/crew/idle")
        assert len(self.assert_got(idle, "idle", None)) == 2
        shipstatus_after = self.assert_ok("/ship/" + str(ship["id"]))
        assert shipstatus == shipstatus_after

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
    print(f"\nAll tests finished, {nok} OK, {nerrors} ERR\n")

    for (key, errdata) in t.saved_errors.items():
        side = (50 - len(key)) // 2
        print("*" * side, key, "*" * side )
        print(errdata)
        print("*" * 52)
        print("")

    sys.exit(nerrors > 0)
