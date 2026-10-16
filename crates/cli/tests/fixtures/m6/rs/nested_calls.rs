// SPDX-License-Identifier: Apache-2.0
fn target() -> i32 {
    1
}
fn outer(value: i32) -> i32 {
    value
}
fn caller() -> i32 {
    outer(target())
}
