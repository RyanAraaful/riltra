# SPDX-License-Identifier: Apache-2.0
def target():
    return 1
def caller():
    return 0
callback = lambda: target()
