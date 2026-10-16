// SPDX-License-Identifier: Apache-2.0
int caller(void);
int target() {
    return caller();
}
int caller() {
    return target();
}
