// SPDX-License-Identifier: Apache-2.0
int target() {
    return 1;
}
int outer(int value) {
    return value;
}
int caller() {
    return outer(target());
}
