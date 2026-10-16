// SPDX-License-Identifier: Apache-2.0
fn target() -> i32 {
    1
}
fn caller() -> i32 {
    let _ref = target;
    0
}
