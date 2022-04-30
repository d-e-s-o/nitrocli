#!/usr/bin/env python3

# Copyright (C) 2020-2022 The Nitrocli Developers
# SPDX-License-Identifier: GPL-3.0-or-later

from argparse import (
  ArgumentParser,
)
from enum import (
  Enum,
)
from os import (
  environ,
)
from sys import (
  argv,
  exit,
)


def main(args):
  """The extension's main function."""
  parser = ArgumentParser()
  parser.add_argument(dest="env", action="store", default=None)
  parser.add_argument("--nitrocli", action="store", default=None)
  parser.add_argument("--model", action="store", default=None)
  # We deliberately store the argument to this option as a string
  # because we can differentiate between None and a valid value, in
  # order to verify that it really was supplied.
  parser.add_argument("--verbosity", action="store", default=None)

  namespace = parser.parse_args(args[1:])
  try:
    print(environ[f"{namespace.env}"])
  except KeyError:
    return 1

  return 0


if __name__ == "__main__":
  exit(main(argv))
