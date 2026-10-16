// SPDX-License-Identifier: Apache-2.0
static int target(void) {
    return 1;
}
int caller(void) {
    return target();
}
