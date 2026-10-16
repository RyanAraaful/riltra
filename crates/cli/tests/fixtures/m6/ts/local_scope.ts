// SPDX-License-Identifier: Apache-2.0
function caller() {
    function target() {
        return 1;
    }
    return target();
}
