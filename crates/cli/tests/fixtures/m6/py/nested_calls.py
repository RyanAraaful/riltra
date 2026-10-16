# SPDX-License-Identifier: Apache-2.0
def target():
    return 1
def outer(value):
    return value
def caller():
    return outer(target())
