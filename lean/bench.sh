#!/usr/bin/env bash
# Sample CPU and memory of every process running BINARY (matched by /proc/PID/exe) over SECONDS.
# Usage: lean/bench.sh BINARY SECONDS LABEL   (appends one line to lean/bench-results.tsv)
set -euo pipefail
bin=$(readlink -f "$1") secs=$2 label=$3
out=$(dirname "$0")/bench-results.tsv
hz=$(getconf CLK_TCK)

pids() { local e; for e in /proc/[0-9]*/exe; do [ "$(readlink "$e" 2>/dev/null)" = "$bin" ] && { e=${e#/proc/}; echo "${e%/exe}"; }; done; }
cpu_ticks() { # sum utime+stime (fields 14,15) across pids; comm may contain spaces, so cut after ')'
  local t=0 p
  for p in $(pids); do
    [ -r /proc/$p/stat ] || continue
    t=$((t + $(sed 's/.*) //' /proc/$p/stat | awk '{print $12 + $13}')))
  done
  echo $t
}
mem() { # total RSS kB, PSS kB, threads, process count
  local rss=0 pss=0 thr=0 n=0 p
  for p in $(pids); do
    [ -r /proc/$p/smaps_rollup ] || continue
    rss=$((rss + $(awk '/^Rss:/{print $2}' /proc/$p/smaps_rollup)))
    pss=$((pss + $(awk '/^Pss:/{print $2}' /proc/$p/smaps_rollup)))
    thr=$((thr + $(awk '/^Threads:/{print $2}' /proc/$p/status)))
    n=$((n + 1))
  done
  echo "$rss $pss $thr $n"
}

[ -n "$(pids)" ] || { echo "no process runs $bin" >&2; exit 1; }
t0=$(cpu_ticks); s0=$(date +%s.%N)
peak_rss=0
for _ in $(seq "$secs"); do
  sleep 1
  read -r rss _ _ _ <<<"$(mem)"; [ "$rss" -gt "$peak_rss" ] && peak_rss=$rss
done
t1=$(cpu_ticks); s1=$(date +%s.%N)
read -r rss pss thr n <<<"$(mem)"
cpu=$(awk -v d=$((t1 - t0)) -v hz="$hz" -v w="$(awk -v a="$s0" -v b="$s1" 'BEGIN{print b-a}')" 'BEGIN{printf "%.2f", 100*d/hz/w}')
cpus=$(awk -v d=$((t1 - t0)) -v hz="$hz" 'BEGIN{printf "%.2f", d/hz}')
[ -f "$out" ] || printf 'time\tlabel\tsecs\tcpu_pct_avg\tcpu_sec\trss_mb\tpss_mb\tpeak_rss_mb\tthreads\tprocs\n' >"$out"
printf '%s\t%s\t%s\t%s\t%s\t%d\t%d\t%d\t%s\t%s\n' "$(date +%T)" "$label" "$secs" "$cpu" "$cpus" \
  $((rss / 1024)) $((pss / 1024)) $((peak_rss / 1024)) "$thr" "$n" | tee -a "$out"
