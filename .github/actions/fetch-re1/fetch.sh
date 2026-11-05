#!/usr/bin/env bash
# Put the RE1 models research/re1-models.tsv pins in $DEST, each checked
# against its sha256 and byte count, downloading only what $STORE (the restored
# cache) lacks or holds wrongly.
#
# PINS      the pin list (research/re1-models.tsv)
# STORE     the directory the cache holds
# DEST      the directory the caller wants the files in
# FILES     space-separated names to place in DEST; empty places every file
# FILL_ALL  "true" makes every file of the list good in STORE, not only the ones
#           asked for, so that the cache entry the job then saves is the whole
#           set whichever job saves it; otherwise only the files asked for are
#           made good, since a job that saves nothing has no use for the rest
set -euo pipefail

pins=${PINS:?PINS is not set}
store=${STORE:?STORE is not set}
dest=${DEST:?DEST is not set}
wanted=${FILES:-}
fill_all=${FILL_ALL:-}

sha() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1
}

repo=
commit=
dir=
names=()
sums=()
sizes=()
# `|| [ -n "$kind" ]`: a last row with no newline after it is still a row.
while IFS=$'\t' read -r kind a b c || [ -n "$kind" ]; do
  case "$kind" in
    source) repo=$a commit=$b dir=$c ;;
    file) names+=("$a"); sums+=("$b"); sizes+=("$c") ;;
  esac
done <"$pins"

if [[ ! "$commit" =~ ^[0-9a-f]{40}$ || -z "$repo" || -z "$dir" ]]; then
  echo "::error::$pins has no usable source row (repository, 40-digit commit, directory)"
  exit 1
fi
if [ "${#names[@]}" -eq 0 ]; then
  echo "::error::$pins lists no file"
  exit 1
fi
for i in "${!names[@]}"; do
  if [[ ! "${names[$i]}" =~ ^[A-Za-z0-9._-]+$ || ! "${sums[$i]}" =~ ^[0-9a-f]{64}$ || ! "${sizes[$i]}" =~ ^[0-9]+$ ]]; then
    echo "::error::$pins: the row of '${names[$i]}' is malformed"
    exit 1
  fi
done

# A file is good when its size and its sha256 are the pin's.
good() {
  [ -f "$1" ] && [ "$(wc -c <"$1" | tr -d ' ')" = "${sizes[$2]}" ] && [ "$(sha "$1")" = "${sums[$2]}" ]
}

# A 404 is final for a host; anything else is tried three times. Only a transfer
# that has stopped (under 1 kB/s for 60 s) is abandoned: a slow one that keeps
# moving is left to finish, and a crawl is ended by the step's own bound. The
# download that took 6.9 minutes on one run (#1380) moved its first file at about
# 25 kB/s, which no rate floor above that could have let through.
download() {
  local code attempt
  for attempt in 1 2 3; do
    code=$(curl -sSL --connect-timeout 20 --max-time 600 --speed-limit 1024 --speed-time 60 -o "$2" -w '%{http_code}' "$1") || code=000
    [ "$code" = 200 ] && return 0
    [ "$code" = 404 ] && return 1
    sleep $((attempt * 5))
  done
  return 1
}

if [ -z "$wanted" ]; then
  wanted=${names[*]}
fi
order=()
queued=" "
for name in $wanted; do
  known=
  for i in "${!names[@]}"; do
    if [ "${names[$i]}" = "$name" ]; then known=$i; fi
  done
  if [ -z "$known" ]; then
    echo "::error::$name is not in $pins"
    exit 1
  fi
  case "$queued" in *" $known "*) ;; *) order+=("$known"); queued="$queued$known " ;; esac
done
if [ "$fill_all" = true ]; then
  for i in "${!names[@]}"; do
    case "$queued" in *" $i "*) ;; *) order+=("$i"); queued="$queued$i " ;; esac
  done
fi

mkdir -p "$store" "$dest"
for i in "${order[@]}"; do
  name=${names[$i]}
  file=$store/$name
  if good "$file" "$i"; then
    echo "$name: in the cache and matches its pin"
    continue
  fi
  rm -f "$file" "$file.part"
  fetched=
  for host in raw.githubusercontent.com media.githubusercontent.com/media; do
    if download "https://$host/$repo/$commit/$dir/$name" "$file.part" && good "$file.part" "$i"; then
      mv "$file.part" "$file"
      echo "$name: fetched from ${host%%/*}"
      fetched=1
      break
    fi
    echo "::warning::$name is not at $host as pinned (missing, or its size or sha256 differs)"
    rm -f "$file.part"
  done
  if [ -z "$fetched" ]; then
    echo "::error::$name matches its pin in $pins from neither host"
    exit 1
  fi
done

for name in $wanted; do
  for i in "${!names[@]}"; do
    if [ "${names[$i]}" = "$name" ]; then
      cp "$store/$name" "$dest/$name"
      if ! good "$dest/$name" "$i"; then
        echo "::error::$dest/$name does not match its pin after the copy"
        exit 1
      fi
    fi
  done
done
echo "placed in $dest: $wanted"
