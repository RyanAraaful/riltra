// SPDX-License-Identifier: Apache-2.0
const fn target() -> i32 {
    1
}
fn caller() -> i32 {
    0
}
const VALUE: i32 = target();
