# SPDX-License-Identifier: Apache-2.0
"""Self-authored M0 demo fixture (no external source)."""


def hello() -> str:
    return "hello"


if __name__ == "__main__":
    print(hello())
