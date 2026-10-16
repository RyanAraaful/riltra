// SPDX-License-Identifier: Apache-2.0
fn target() -> i32 {
    caller()
}
fn caller() -> i32 {
    target()
}
