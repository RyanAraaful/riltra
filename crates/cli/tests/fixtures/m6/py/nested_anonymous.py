# SPDX-License-Identifier: Apache-2.0
def target():
    return 1
def caller():
    return lambda: target()
