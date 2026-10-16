// SPDX-License-Identifier: Apache-2.0
fn caller() -> i32 {
    fn target() -> i32 {
        1
    }
    target()
}
