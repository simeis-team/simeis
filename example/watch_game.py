PORT=8080
URL=f"http://0.0.0.0:{PORT}"

import os
import json
import time
import urllib.request

# TODO HTTP server that serves a graph of the data

INIT = False
HIST = {}

class SimeisError(Exception):
    pass

WIDTH=100
SCORE="█"
POTENTIAL="▒"
VOID=" "
def mkbar(n, maxs, width=50, mins=0, block="#"):
        if maxs == 0.0:
            perc = 0
        else:
            perc = n / maxs
        nblock = int(width * perc)
        nvoid = width - nblock
        return block * nblock + VOID * nvoid

def get(path):
    qry = f"{URL}/{path}"
    while True:
        try:
            reply = urllib.request.urlopen(qry)
            break
        except:
            os.system("clear")
            HIST = {}
            INIT=False
            print("DEAD SERVER")
            time.sleep(1)
            continue

    data = json.loads(reply.read().decode())
    err = data.pop("error")
    if err != "ok":
        raise SimeisError(err)

    return data

def get_info():
    return get("gamestats")

def get_resources():
    return get("resources")

def get_market():
    return get("market/prices")["prices"]

resources = get_resources()
while True:
    time.sleep(2)
    os.system("clear")
    market = get_market()
    max_res_len = max([len(k) for k in market.keys()])
    for (res, price) in market.items():
        relp = round((price / resources[res]["base-price"]) * 100, 2)
        price = round(price, 3)
        space = " "*(1 + max_res_len - len(res))
        print(f"{res}{space}{price} ({relp} %)")
    print("")

    info = get_info()
    if len(info) == 0:
        os.system("clear")
        HIST = {}
        print("No players on the server")
        continue

    for (_, p) in info.items():
        if p["lost"]:
            p["score"] = -1.0

    players = sorted(info.items(), key=lambda p: p[1]["score"] + p[1]["potential"], reverse=True)
    max_score = max([v["score"] + v["potential"] for v in info.values()])
    for (player, data) in players:
        if player not in HIST:
            HIST[player] = []

        if data["lost"]:
            print("Player {}:\tLOST".format(data["name"]))
            continue

        s = data["score"] + data["potential"]
        if data["age"] == 0:
            avg = 0.0
        else:
            avg = s / data["age"]
        HIST[player].append((s, avg))
        avg_lasts = max([n[1] for n in HIST[player][-30:]])

        score_bar = mkbar(data["score"], max_score, block=SCORE, width=WIDTH).strip(VOID)
        pot_bar = mkbar(data["potential"], max_score, block=POTENTIAL, width=WIDTH).strip(VOID)
        bar = score_bar + pot_bar
        nvoid = WIDTH - len(bar)
        bar += VOID * nvoid
        print("Player {}:\t{} {} (~{}/sec)\tpotential: {}".format(
            data["name"], bar, round(data["score"], 2),
            round(avg_lasts, 2),
            round(data["potential"], 2)
        ))
