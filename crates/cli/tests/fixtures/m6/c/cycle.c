// SPDX-License-Identifier: Apache-2.0
int caller(void);
int target(void) {
    return caller();
}
int caller(void) {
    return target();
}
