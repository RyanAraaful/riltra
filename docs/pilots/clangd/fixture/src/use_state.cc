// SPDX-License-Identifier: Apache-2.0
// E7, second translation unit: the same namespace-scope variable, and the
// non-const overload of the member.
#include "geom/point.h"

namespace demo {

double readBudget() {
  geom::Point p;
  p.budget() = 2.5;
  return geom::kBudget;
}

}  // namespace demo
