// SPDX-License-Identifier: Apache-2.0
int target(int (*target)()) {
    return target();
}
int caller() {
    return 0;
}
