// SPDX-License-Identifier: Apache-2.0
// Self-authored M2 fixture.
import { run } from "./helpers.js";

export class Worker {
  process(items) {
    return run(items);
  }
}

export function main() {
  const w = new Worker();
  console.log(w.process([1, 2]));
}

main();
