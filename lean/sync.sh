#!/usr/bin/env bash
set -euo pipefail
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo"
[[ $(git branch --show-current) == lean ]] || { echo 'Run this from branch lean.' >&2; exit 1; }
[[ -z $(git status --porcelain) ]] || { echo 'Commit or stash your changes first.' >&2; exit 1; }
git fetch upstream
git merge-base --is-ancestor master upstream/master || {
    echo 'master has diverged from upstream/master; stopping without changing branches.' >&2
    exit 1
}
git switch master
git merge --ff-only upstream/master
git switch lean
git config --local rerere.enabled true
if ! git rebase master; then
    echo 'Rebase stopped. Resolve conflicts and run git rebase --continue, or git rebase --abort.' >&2
    exit 1
fi
