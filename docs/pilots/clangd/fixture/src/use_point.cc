// SPDX-License-Identifier: Apache-2.0
// E1: the free function `geom::norm` and the method `Point::norm` share a
// name. E7: the namespace-scope variable `geom::kBudget`.
#include "geom/point.h"

namespace demo {

double area(const geom::Point& p) {
  double freeNorm = geom::norm(p);
  double methodNorm = p.norm();
  geom::kBudget += 1.0;
  return freeNorm + methodNorm;
}

}  // namespace demo
