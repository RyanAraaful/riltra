// SPDX-License-Identifier: Apache-2.0
fn target() -> i32 {
    1
}
fn caller() -> i32 {
    0
}
const CALLBACK: fn() -> i32 = || target();
