# SPDX-License-Identifier: Apache-2.0
"""Self-authored M2 fixture."""

import os
from helpers import run


class Worker:
    def process(self, items):
        return run(items)


def main():
    w = Worker()
    print(w.process([1, 2]))


if __name__ == "__main__":
    main()
