# SPDX-License-Identifier: Apache-2.0
class A:
    def target(self):
        return 1

class B:
    def target(self):
        return 2

def caller(obj):
    return obj.target()
