#!/bin/sh

# Copyright (C) 2022-2025 The Nitrocli Developers
# SPDX-License-Identifier: GPL-3.0-or-later

set -e -u -o pipefail

# We support testing both /dev/null and pipe redirection of stdin and
# the first argument controls which one to use.
stdin=${1:-"pipe"}
instance=${2:-1}

if [ $instance -ge 5 ]; then
  # Print own process ID and then just wait for a while until we get
  # killed.
  echo $$
  sleep 60
else
  # Invoke the script recursively, doing an actual fork and not just an
  # exec in order to spawn a new process. Also redirect stdin to
  # simulate it not referring to a TTY directly.
  if [ $stdin != "pipe" ]; then
    sh $0 $stdin $((instance + 1)) < /dev/null
  else
    cat | sh $0 $stdin $((instance + 1))
  fi
fi
