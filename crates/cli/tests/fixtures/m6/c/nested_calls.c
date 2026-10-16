// SPDX-License-Identifier: Apache-2.0
int target(void) {
    return 1;
}
int outer(int value) {
    return value;
}
int caller(void) {
    return outer(target());
}
