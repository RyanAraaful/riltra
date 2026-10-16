// SPDX-License-Identifier: Apache-2.0
function target(target: () => number) {
    return target();
}
function caller() {
    return 0;
}
