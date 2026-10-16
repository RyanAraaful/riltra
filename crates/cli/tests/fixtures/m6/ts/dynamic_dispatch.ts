// SPDX-License-Identifier: Apache-2.0
function target() {
    return 1;
}
function caller(obj: any) {
    return obj.target();
}
