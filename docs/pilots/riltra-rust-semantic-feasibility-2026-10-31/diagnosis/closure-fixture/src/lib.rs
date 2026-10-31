pub fn callee(x: u32) -> Option<u32> {
    Some(x)
}

pub fn direct_caller(x: u32) -> Option<u32> {
    callee(x)
}

pub fn closure_caller(x: u32) -> Option<u32> {
    None.or_else(|| callee(x))
}
