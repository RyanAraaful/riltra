// SPDX-License-Identifier: Apache-2.0
#include "geom/overload.h"

namespace geom {

double scale(const Point& p, double factor) { return p.norm() * factor; }

double scale(const Point& p, int factor) { return p.norm() * factor; }

}  // namespace geom

namespace other {

double scale(const geom::Point& p, double factor) { return p.norm() + factor; }

}  // namespace other

namespace amb {

void pick(long) {}

void pick(double) {}

}  // namespace amb
