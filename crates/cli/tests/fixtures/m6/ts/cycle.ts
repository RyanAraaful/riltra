// SPDX-License-Identifier: Apache-2.0
function target() {
    return caller();
}
function caller() {
    return target();
}
