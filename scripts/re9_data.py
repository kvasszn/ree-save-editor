import json
import mmh3
base = "/home/nikola/programming/mhtame/outputs/re_chunk_000"
achievements = "natives/stm/leveldesign/achievement/achievementdatacatalogapp.user.3.json"

f = open(f"{base}/{achievements}", 'r')
rsz = json.load(f)["rsz"]["rsz"][0]

for achievement in rsz["_Data"]:
    print(achievement["_BonusID"], mmh3.hash(achievement["_BonusID"].strip(), 0xffffffff) & 0xffffffff)


bonus_path = "natives/stm/message/gamesystem/bonus.msg.23.json"
f = open(f"{base}/{bonus_path}", 'r')
bonusmsg = json.load(f)["entries"]
for entry in bonusmsg:
    name = entry["name"].removesuffix("_Title").removesuffix("_Description")
    name_hash = mmh3.hash(name.lower(), 0xffffffff) & 0xffffffff
    print(f"{name_hash}")
    name_hash = mmh3.hash(f"Bonus:{name.lower()}", 0xffffffff) & 0xffffffff
    print(f"{name_hash}")
