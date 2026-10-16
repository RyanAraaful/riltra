# SPDX-License-Identifier: Apache-2.0
def target():
    return 1
def caller():
    ref = target
    return ref
