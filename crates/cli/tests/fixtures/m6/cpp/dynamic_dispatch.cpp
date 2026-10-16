// SPDX-License-Identifier: Apache-2.0
struct Obj { virtual int target() = 0; };
int target() {
    return 1;
}
int caller(Obj *obj) {
    return obj->target();
}
