pub trait Service { fn run(&self) -> u32; }
pub struct A; pub struct B;
impl Service for A { fn run(&self) -> u32 { 1 } }
impl Service for B { fn run(&self) -> u32 { 2 } }
pub fn dispatch(x: &dyn Service) -> u32 { x.run() }
pub fn direct(x: &A) -> u32 { Service::run(x) }
