// SPDX-License-Identifier: Apache-2.0
struct Obj;
impl Obj { fn target(&self) -> i32 { 1 } }
fn caller(obj: &Obj) -> i32 {
    obj.target()
}
