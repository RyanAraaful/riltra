// SPDX-License-Identifier: Apache-2.0
int target(int (*target)(void)) {
    return target();
}
int caller(void) {
    return 0;
}
