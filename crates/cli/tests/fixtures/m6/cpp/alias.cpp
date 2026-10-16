// SPDX-License-Identifier: Apache-2.0
int target() {
    return 1;
}
int caller() {
    auto alias = target;
    return alias();
}
