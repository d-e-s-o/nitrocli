#!/usr/bin/python3 -B

# Copyright (C) 2019-2020 The Nitrocli Developers
# SPDX-License-Identifier: GPL-3.0-or-later

from argparse import (
  ArgumentParser,
  ArgumentTypeError,
)
from concurrent.futures import (
  ThreadPoolExecutor,
)
from json import (
  loads as jsonLoad,
)
from os import (
  stat,
)
from os.path import (
  join,
)
from subprocess import (
  check_call,
  check_output,
)
from sys import (
  argv,
  exit,
)
from tempfile import (
  TemporaryDirectory,
)

UNITS = {
  "byte": 1,
  "kib": 1024,
  "mib": 1024 * 1024,
}

def unit(string):
  """Create a unit."""
  if string in UNITS:
    return UNITS[string]
  else:
    raise ArgumentTypeError("Invalid unit: \"%s\"." % string)


def nitrocliPath(cwd):
  """Determine the path to the nitrocli release build binary."""
  out = check_output(["cargo", "metadata", "--format-version=1"], cwd=cwd)
  data = jsonLoad(out)
  return join(data["target_directory"], "release", "nitrocli")


def fileSize(path):
  """Determine the size of the file at the given path."""
  return stat(path).st_size


def repoRoot():
  """Retrieve the root directory of the current git repository."""
  out = check_output(["git", "rev-parse", "--show-toplevel"])
  return out.decode().strip()


def resolveCommit(commit):
  """Resolve a commit into a SHA1 hash."""
  out = check_output(["git", "rev-parse", "--verify", "%s^{commit}" % commit])
  return out.decode().strip()


def determineSizeAt(root, rev):
  """Determine the size of the nitrocli release build binary at the given git revision."""
  sha1 = resolveCommit(rev)
  with TemporaryDirectory() as cwd:
    check_call(["git", "clone", root, cwd])
    check_call(["git", "checkout", "--quiet", sha1], cwd=cwd)
    check_call(["cargo", "build", "--quiet", "--release"], cwd=cwd)

    ncli = nitrocliPath(cwd)
    check_call(["strip", ncli])
    return fileSize(ncli)


def setupArgumentParser():
  """Create and initialize an argument parser."""
  parser = ArgumentParser()
  parser.add_argument(
    "revs", metavar="REVS", nargs="+",
    help="The revisions at which to measure the release binary size.",
  )
  parser.add_argument(
    "-u", "--unit", default="byte", dest="unit", metavar="UNIT", type=unit,
    help="The unit in which to output the result (%s)." % "|".join(UNITS.keys()),
  )
  return parser


def main(args):
  """Determine the size of the nitrocli binary at given git revisions."""
  parser = setupArgumentParser()
  ns = parser.parse_args(args)
  root = repoRoot()
  futures = []
  executor = ThreadPoolExecutor()

  for rev in ns.revs:
    futures += [executor.submit(lambda r=rev: determineSizeAt(root, r))]

  executor.shutdown(wait=True)

  for future in futures:
    print(int(round(future.result() / ns.unit, 0)))

  return 0


if __name__ == "__main__":
  exit(main(argv[1:]))
