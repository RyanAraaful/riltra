# SPDX-License-Identifier: Apache-2.0
def target():
    return caller()
def caller():
    return target()
