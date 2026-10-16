// SPDX-License-Identifier: Apache-2.0
int target(void) {
    return 1;
}
#define CALL_TARGET() target()
int caller(void) {
    return CALL_TARGET();
}
