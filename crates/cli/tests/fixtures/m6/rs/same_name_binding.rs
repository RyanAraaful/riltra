// SPDX-License-Identifier: Apache-2.0
fn target(target: fn() -> i32) -> i32 {
    target()
}
fn caller() -> i32 {
    0
}
