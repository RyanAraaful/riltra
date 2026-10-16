// SPDX-License-Identifier: Apache-2.0
class A { target() { return 1; } }
class B { target() { return 2; } }
function caller(obj: any) {
    return obj.target();
}
