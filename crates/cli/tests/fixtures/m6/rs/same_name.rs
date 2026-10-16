// SPDX-License-Identifier: Apache-2.0
struct A;
struct B;
impl A { fn target(&self) -> i32 { 1 } }
impl B { fn target(&self) -> i32 { 2 } }
fn caller(obj: &A) -> i32 {
    obj.target()
}
