// SPDX-License-Identifier: Apache-2.0
// Self-authored M2 fixture: CUDA is explicitly unsupported (experimental).
__global__ void add(int *a, int *b, int *c) {
    int i = threadIdx.x;
    c[i] = a[i] + b[i];
}
