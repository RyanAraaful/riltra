# SPDX-License-Identifier: Apache-2.0
def caller():
    def target():
        return 1
    return target()
