// SPDX-License-Identifier: Apache-2.0
function target() {
    return 1;
}
function outer(value: number) {
    return value;
}
function caller() {
    return outer(target());
}
