// SPDX-License-Identifier: Apache-2.0
// Self-authored M2 fixture.
#include <string>

class Worker {
public:
    int process(int x) {
        return run(x);
    }
};

int run(int x) {
    return x * 2;
}

int main() {
    Worker w;
    return w.process(21);
}
