#!/usr/bin/env bash

set -euo pipefail

cargo build --release

export PATH="$PWD/target/release:$PATH"
channel1=$(mktemp --dry-run)
mkfifo $channel1
channel2=$(mktemp --dry-run)
mkfifo $channel2

file=$(mktemp)

echo "step: acquire write-lock"
cat $channel1 | fcntl-tool write-lock $file &
sleep 0.2 # give time for locking

echo "step: test for write-lock"
fcntl-tool test-lock $file
fcntl-tool test-lock $file | grep -q ExclusiveWrite

echo "step: releasing write-lock"
echo > $channel1 # send enter/newline
sleep 0.2 # give time for releasing the lock

echo "step: test for unlocked"
fcntl-tool test-lock $file
fcntl-tool test-lock $file | grep -q Unlocked

echo "step: acquire read-lock #1"
cat $channel1 | fcntl-tool read-lock $file &
echo "step: acquire read-lock #2"
cat $channel2 | fcntl-tool read-lock $file &
sleep 0.2 # wait so that the tool is actually scheduled and can get the lock

echo "step: test for SharedLock"
fcntl-tool test-lock $file
fcntl-tool test-lock $file | grep -q SharedRead

echo > $channel1 # send enter/newline
echo > $channel2 # send enter/newline
sleep 0.2 # give time for releasing the locks

rm -f $channel1
rm -f $channel2
rm -f $file
