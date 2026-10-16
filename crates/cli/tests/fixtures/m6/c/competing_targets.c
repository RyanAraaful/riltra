// SPDX-License-Identifier: Apache-2.0
struct A { int (*target)(void); };
struct B { int (*target)(void); };
int caller(struct A *obj) {
    return obj->target();
}
