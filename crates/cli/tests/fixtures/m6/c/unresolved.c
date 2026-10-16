// SPDX-License-Identifier: Apache-2.0
int target(void) {
    return 1;
}
int caller(void) {
    return 0;
}
int (*global_ref)(void) = target;
