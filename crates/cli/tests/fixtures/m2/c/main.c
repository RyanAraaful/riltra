// SPDX-License-Identifier: Apache-2.0
// Self-authored M2 fixture.
#include "util.h"
#include <stdio.h>

int add(int a, int b) {
    return a + b;
}

int main(void) {
    printf("%d\n", add(1, 2));
    return 0;
}
