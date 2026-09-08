#!/usr/bin/env bash
# Forensic snapshot of idle-manager's on-disk session state.
# Usage: snapshot.sh <label>     e.g. snapshot.sh before-reboot
set -uo pipefail

LABEL="${1:?usage: snapshot.sh <label>}"
ROOT="$HOME/.local/state/idle-manager-forensics"
OUT="$ROOT/$LABEL"
PROFILES="$HOME/.local/share/idle-manager/profiles"
CONFIG="$HOME/.config/idle-manager"

mkdir -p "$OUT/jars" "$OUT/localstorage" && chmod -R 700 "$OUT"

{
  echo "label:      $LABEL"
  echo "taken:      $(date -Is)"
  echo "boot_id:    $(cat /proc/sys/kernel/random/boot_id)"
  echo "uptime:     $(uptime -p)"
  echo "app_pids:   $(pgrep -d, -f '[t]arget/release/idle-manager' || echo none)"
  echo "app_start:  $(ps -o lstart= -p "$(pgrep -f '[t]arget/release/idle-manager' | head -1)" 2>/dev/null || echo none)"
  echo "netproc:    $(pgrep -d, -f '[W]ebKitNetworkProcess' || echo none)"
  echo
  echo "--- mount holding the profiles ---"
  findmnt -T "$PROFILES" -o TARGET,SOURCE,FSTYPE,OPTIONS 2>/dev/null
  echo
  echo "--- autostart entries (would the app launch at boot with a different env?) ---"
  ls -la "$HOME/.config/autostart" 2>/dev/null || echo "no ~/.config/autostart"
  echo
  echo "--- hot journals / WAL left behind (evidence of an unclean exit) ---"
  find "$PROFILES" \( -name '*-journal' -o -name '*-wal' -o -name '*-shm' \) -printf '%s\t%p\n' 2>/dev/null | sort -k2
} > "$OUT/meta.txt" 2>&1

# stat: inode is the key field. A changed inode means the file was replaced,
# not modified — that tells a deletion apart from a rollback.
{
  printf '%-14s %-12s %10s %8s  %-25s %-25s %s\n' INODE LINKS SIZE BLOCKS MTIME CTIME PATH
  for f in "$CONFIG/sessions.toml" "$PROFILES"/*/data/cookies.sqlite \
           "$PROFILES"/*/state.toml "$PROFILES"/*/data/storage/*/*/LocalStorage/localstorage.sqlite3; do
    [ -e "$f" ] || continue
    stat -c '%-14i %-12h %10s %8b  %-25y %-25z %n' "$f" 2>/dev/null
  done
} > "$OUT/stat.txt" 2>&1

# Byte-level checksums of the files as they currently read.
find "$CONFIG" "$PROFILES" -type f \( -name 'sessions.toml' -o -name 'state.toml' \
     -o -name 'cookies.sqlite*' -o -name 'localstorage.sqlite3*' -o -name 'origin' \) \
     -exec sha256sum {} + 2>/dev/null | sort -k2 > "$OUT/sums.txt"

# Raw byte copies — the only way to prove a rollback rather than a delete.
for d in "$PROFILES"/*/; do
  id=$(basename "$d")
  [ -f "$d/data/cookies.sqlite" ] && cp -a "$d/data/cookies.sqlite" "$OUT/jars/$id.sqlite" 2>/dev/null
  for extra in "$d"/data/cookies.sqlite-journal "$d"/data/cookies.sqlite-wal; do
    [ -f "$extra" ] && cp -a "$extra" "$OUT/jars/$id.$(basename "$extra" | sed 's/.*sqlite//')" 2>/dev/null
  done
done
cp -a "$CONFIG/sessions.toml" "$OUT/sessions.toml" 2>/dev/null

# Parsed view. Cookie VALUES are live auth tokens, so they are hashed, never
# written in clear — a changed hash still proves the value rotated.
python3 - "$PROFILES" "$OUT/db.txt" <<'PY'
import sqlite3, glob, os, sys, hashlib, datetime
profiles, out = sys.argv[1], sys.argv[2]
GOOGLE_AUTH = {'SID','LSID','HSID','SSID','APISID','SAPISID','__Secure-1PSID',
               '__Secure-3PSID','__Host-GAPS','__Host-1PLSID','__Secure-1PSIDTS',
               'SIDCC','__Secure-1PSIDCC','ACCOUNT_CHOOSER'}
with open(out,'w') as f:
    for d in sorted(glob.glob(os.path.join(profiles,'session-*'))):
        p = os.path.join(d,'data','cookies.sqlite')
        name = os.path.basename(d)
        if not os.path.exists(p):
            f.write(f"== {name}: NO JAR FILE\n"); continue
        try:
            c = sqlite3.connect('file:%s?mode=ro' % p, uri=True)
            g = lambda q: list(c.execute(q))[0][0]
            rows = list(c.execute("select name,host,path,expiry,isSecure,isHttpOnly,sameSite,value from moz_cookies order by host,name,path"))
            auth = sum(1 for r in rows if r[0] in GOOGLE_AUTH)
            f.write(f"== {name} size={os.path.getsize(p)} pages={g('pragma page_count')} "
                    f"freelist={g('pragma freelist_count')} rows={len(rows)} "
                    f"max_rowid={g('select coalesce(max(id),0) from moz_cookies')} "
                    f"google_auth_rows={auth}\n")
            for n,h,pa,e,s,ho,ss,v in rows:
                vh = hashlib.sha256(v.encode('utf-8','replace')).hexdigest()[:16]
                exp = datetime.datetime.fromtimestamp(e).date() if e else 'SESSION'
                f.write(f"   {n:<24} {h:<26} {pa:<6} exp={exp} sec={s} http={ho} ss={ss} val={vh}\n")
        except Exception as ex:
            f.write(f"== {name}: ERROR {ex}\n")
PY

chmod -R go-rwx "$OUT"
echo "snapshot '$LABEL' written to $OUT"
