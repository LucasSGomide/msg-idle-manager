#!/usr/bin/env bash
# Diff two snapshots and say which failure mode the evidence supports.
# Usage: compare.sh before-reboot after-reboot
set -uo pipefail
ROOT="$HOME/.local/state/idle-manager-forensics"
A="$ROOT/${1:?usage: compare.sh <before> <after>}"
B="$ROOT/${2:?usage: compare.sh <before> <after>}"
for d in "$A" "$B"; do [ -d "$d" ] || { echo "missing snapshot: $d"; exit 1; }; done

echo "############ 1. Did the app or the machine change identity?"
grep -E '^(label|taken|boot_id|app_start)' "$A/meta.txt" | sed 's/^/  BEFORE  /'
grep -E '^(label|taken|boot_id|app_start)' "$B/meta.txt" | sed 's/^/  AFTER   /'
echo "  (a changed boot_id confirms the machine actually rebooted between snapshots)"

echo
echo "############ 2. Were the jar files REPLACED, or modified in place?"
echo "  A changed inode = the file was deleted and recreated."
echo "  Same inode, fewer pages  = rolled back by its journal."
echo "  Same inode, same content = intact, and the loss is server-side."
paste <(grep cookies.sqlite "$A/stat.txt" | awk '{print $1"\t"$3"\t"$NF}') \
      <(grep cookies.sqlite "$B/stat.txt" | awk '{print $1"\t"$3}') 2>/dev/null |
  awk -F'\t' 'BEGIN{printf "  %-12s %-10s | %-12s %-10s  %-14s %s\n","INODE(B4)","SIZE","INODE(AFT)","SIZE","VERDICT","PROFILE"}
  { ino_a=$1; sz_a=$2; path=$3; ino_b=$4; sz_b=$5;
    v = (ino_a!=ino_b) ? "REPLACED" : (sz_a!=sz_b ? "TRUNCATED" : "same inode+size");
    n=split(path,p,"/"); prof=p[n-2];
    printf "  %-12s %-10s | %-12s %-10s  %-14s %s\n", ino_a, sz_a, ino_b, sz_b, v, prof }'

echo
echo "############ 3. Did the Google authorization survive?"
printf "  %-16s %-28s %s\n" PROFILE BEFORE AFTER
join -j1 \
  <(grep '^==' "$A/db.txt" | awk '{print $2"\t"$6" "$8}' | sort) \
  <(grep '^==' "$B/db.txt" | awk '{print $2"\t"$6" "$8}' | sort) \
  -t$'\t' 2>/dev/null | awk -F'\t' '{printf "  %-16s %-28s %s\n",$1,$2,$3}'

echo
echo "############ 4. Byte-level: are the copied jars identical?"
for f in "$A"/jars/*.sqlite; do
  n=$(basename "$f"); g="$B/jars/$n"
  if [ ! -f "$g" ]; then echo "  $n: MISSING AFTER"; continue; fi
  if cmp -s "$f" "$g"; then echo "  $n: byte-identical"
  else echo "  $n: DIFFERS ($(stat -c%s "$f") -> $(stat -c%s "$g") bytes)"; fi
done

echo
echo "############ 5. Which specific cookies disappeared?"
for n in $(grep '^==' "$A/db.txt" | awk '{print $2}'); do
  lost=$(LC_ALL=C comm -23 \
    <(awk -v p="$n" '$0 ~ "^== "p" " {f=1;next} /^== /{f=0} f{print $1" "$2}' "$A/db.txt" | LC_ALL=C sort -u) \
    <(awk -v p="$n" '$0 ~ "^== "p" " {f=1;next} /^== /{f=0} f{print $1" "$2}' "$B/db.txt" | LC_ALL=C sort -u))
  if [ -n "$lost" ]; then
    echo "  --- $n lost:"; echo "$lost" | sed 's/^/      /'
  else
    echo "  --- $n: nothing lost"
  fi
done

echo
echo "############ 6. Hot journals present after the reboot?"
sed -n '/hot journals/,$p' "$B/meta.txt" | grep -E 'cookies|no ' || echo "  none for the cookie jars"
