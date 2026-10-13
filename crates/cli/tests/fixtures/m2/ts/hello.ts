// SPDX-License-Identifier: Apache-2.0
// Self-authored M2 fixture.
import { run } from "./helpers";

export interface Job {
  id: number;
}

export class Worker {
  process(items: number[]): number {
    return run(items);
  }
}

export function main(): void {
  const w = new Worker();
  console.log(w.process([1, 2]));
}

main();
