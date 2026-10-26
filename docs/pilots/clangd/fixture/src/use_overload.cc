// SPDX-License-Identifier: Apache-2.0
// Overload resolution, a same-named overload set in a second namespace, and
// a template instantiation.
#include "geom/overload.h"

namespace demo {

double apply(const geom::Point& p) {
  double d = geom::scale(p, 2.0);
  int i = geom::scale(p, 2);
  double o = other::scale(p, 2.0);
  double t = geom::clampValue(3.5, 0.0, 1.0);
  return d + i + o + t;
}

}  // namespace demo
