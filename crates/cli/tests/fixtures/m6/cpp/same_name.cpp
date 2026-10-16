// SPDX-License-Identifier: Apache-2.0
namespace A { int target() {
    return 1;
} }
namespace B { int target() {
    return 2;
} }
struct Obj { int (*target)(); };
int caller(Obj *obj) {
    return obj->target();
}
