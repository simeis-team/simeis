import sys
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

        errdata = "Error occured on test " + self.current_test + "\n\n"
        tb_str = traceback.format_exception(
            exc,
            value=exc,
            tb=exc.__traceback__,
        )
        for line in tb_str:
            if (" assert " in line) or ("in test_" in line):
                errdata += line.strip() + "\n"

        errdata += "\nTrace of the test:\n"
        errdata += ("=" * 10) + " TRACE " + ("=" * 10) + "\n"
        for line in self.trace:
            errdata += line + "\n"
        errdata += ("=" * 10) + "  END  " + ("=" * 10)
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

    def all_tests(self):
        self.test_ping()
        self.test_create_player()

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

        got = self.assert_ok("/player/" + str(self.id))
        self.assert_got(got, "money", 10000)

        got = self.assert_ok("/player/" + str(self.id), key=1234123423)
        self.assert_got(got, "money", None, negate=True)

        self.assert_ok("/newplayer", method="POST", body={"name": "Testuser2"})
        self.assert_error("/newplayer",
            method="POST", body={"name": "Testuser2"},
            errtype="PlayerAlreadyExists(\"Testuser2\")"
        )

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
