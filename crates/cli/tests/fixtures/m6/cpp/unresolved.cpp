// SPDX-License-Identifier: Apache-2.0
int target() {
    return 1;
}
int caller() {
    return 0;
}
struct Holder { int value = target(); };
