PORT=8080
URL=f"http://0.0.0.0:{PORT}"

import os
import json
import time
import urllib.request

# TODO HTTP server that serves a graph of the data

HIST = {}

class SimeisError(Exception):
    pass

def get_info():
    qry = f"{URL}/gamestats"
    reply = urllib.request.urlopen(qry)
    data = json.loads(reply.read().decode())
    err = data.pop("error")
    if err != "ok":
        raise SimeisError(err)

    return data

width = 50
while True:
    time.sleep(2)
    os.system("clear")
    info = get_info()
    for (_, p) in info.items():
        if p["lost"]:
            p["score"] = -1.0
    players = sorted(info.items(), key=lambda p: p[1]["score"], reverse=True)
    max_score = max([v["score"] for v in info.values()])
    for (player, data) in players:
        if player not in HIST:
            HIST[player] = []

        if data["lost"]:
            print("Player {}:\tLOST".format(data["name"]))
            continue

        if data["age"] == 0:
            avg = 0.0
        else:
            avg = data["score"] / data["age"]
        HIST[player].append((data["score"], avg))
        avg_lasts = max([n[1] for n in HIST[player][-30:]])

        if max_score == 0.0:
            score_perc = 0
        else:
            score_perc = data["score"] / max_score
        nblock = int(width * score_perc)
        nvoid = width - nblock
        bar = "#" * nblock + " " * nvoid
        print("Player {}:\t{} {} {}".format(data["name"], bar, data["score"], avg_lasts))
