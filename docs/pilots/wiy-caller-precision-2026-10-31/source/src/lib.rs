pub mod a { pub fn target() -> i32 { 1 } }
pub mod b { pub fn target() -> i32 { 2 } }
pub fn direct() -> i32 { a::target() }
pub fn alias() -> i32 { let f = a::target; f() }
pub fn reference() { let _f = a::target; }
pub fn other() -> i32 { b::target() }
