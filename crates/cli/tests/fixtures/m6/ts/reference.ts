// SPDX-License-Identifier: Apache-2.0
function target() {
    return 1;
}
function caller() {
    const ref = target;
    return ref;
}
