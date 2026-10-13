// SPDX-License-Identifier: Apache-2.0
// Self-authored M2 fixture: qualified C++ definitions.
#include "widget.h"

void PatchMatch::Run() {
    Start();
}

void PatchMatch::Problem::Print() const {
    Show();
}

int add(int a, int b) {
    return a + b;
}
