// SPDX-License-Identifier: Apache-2.0
// Overloads, a same-named overload set in a second namespace, a genuinely
// ambiguous overload set, and one template.
#pragma once

#include "geom/point.h"

namespace geom {

double scale(const Point& p, double factor);
double scale(const Point& p, int factor);

template <typename T>
T clampValue(T v, T lo, T hi) {
  return v < lo ? lo : (v > hi ? hi : v);
}

}  // namespace geom

// Same function name in a different namespace: identity must not merge.
namespace other {
double scale(const geom::Point& p, double factor);
}  // namespace other

// A deliberately ambiguous overload set: `pick('c')` converts a `char` to
// `long` and to `double` with the same rank, so no candidate wins.
namespace amb {
void pick(long);
void pick(double);
}  // namespace amb
