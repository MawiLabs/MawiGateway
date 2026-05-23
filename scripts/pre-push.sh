#!/usr/bin/env bash
# Pre-push gate for develop / main — closes #110.
#
# Without this hook, partial-bundle pushes have repeatedly broken
# develop HEAD: a fresh clone in those windows fails `cargo check`
# and crashes a fresh DB at runtime. See #110 for the three commits
# that explicitly fixed this within 24h.
#
# Install (one-time, per checkout):
#
#     git config core.hooksPath .githooks
#
# …or symlink directly:
#
#     ln -s ../../scripts/pre-push.sh .git/hooks/pre-push
#
# To skip in an emergency (NOT for normal use):
#
#     git push --no-verify
#
# This script is a no-op for branches other than develop / main, so
# WIP feature branches are unaffected.

set -euo pipefail

REMOTE="${1:-origin}"

# Read the refs being pushed (hook contract: STDIN gives lines of
# "<local_ref> <local_sha> <remote_ref> <remote_sha>")
PROTECTED=0
while IFS=' ' read -r local_ref local_sha remote_ref remote_sha; do
  case "$remote_ref" in
    refs/heads/develop|refs/heads/main)
      PROTECTED=1
      ;;
  esac
done

if [ "$PROTECTED" -eq 0 ]; then
  exit 0
fi

echo "[pre-push] develop/main detected — running gate checks…"

cd "$(git rev-parse --show-toplevel)/backend"

echo "[pre-push] cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "[pre-push] cargo check --workspace --all-targets"
cargo check --workspace --all-targets

echo "[pre-push] cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings || {
  echo "[pre-push] clippy failed — fix the lints or push to a feature branch and let CI handle it"
  exit 1
}

echo "[pre-push] OK"
