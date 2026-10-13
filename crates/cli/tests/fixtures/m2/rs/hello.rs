// SPDX-License-Identifier: Apache-2.0
// Self-authored M2 fixture.
use crate::helpers::run;

pub struct Worker;

impl Worker {
    pub fn process(&self, items: &[i32]) -> i32 {
        run(items)
    }
}

pub fn main() {
    let w = Worker;
    println!("{}", w.process(&[1, 2]));
}
