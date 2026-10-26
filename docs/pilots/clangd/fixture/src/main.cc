// SPDX-License-Identifier: Apache-2.0
#include "geom/point.h"

int main() {
  geom::Point p;
  p.x = 1.0;
  p.y = 2.0;
  return p.norm() > 0.0 ? 0 : 1;
}
