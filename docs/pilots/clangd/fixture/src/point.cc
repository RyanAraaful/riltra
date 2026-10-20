// SPDX-License-Identifier: Apache-2.0
#include "geom/point.h"

namespace geom {

double Point::norm() const { return x * x + y * y; }

double Point::budget() const { return kBudget; }

double& Point::budget() { return kBudget; }

double norm(const Point& p) { return p.x; }

double kBudget = 1.5;

}  // namespace geom
