// SPDX-License-Identifier: Apache-2.0
// A class with a two-argument constructor: the E12 discriminator between a
// constructor call and a declaration that only looks like one.
#pragma once

namespace ui {

struct Config {
  int seed = 0;
};

class Widget {
 public:
  Widget(const Config& config, int extra);
  int seed() const;

 private:
  int seed_;
};

}  // namespace ui
