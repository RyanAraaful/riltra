// SPDX-License-Identifier: Apache-2.0
struct Obj { int (*target)(void); };
int target(void) {
    return 1;
}
int caller(struct Obj *obj) {
    return obj->target();
}
