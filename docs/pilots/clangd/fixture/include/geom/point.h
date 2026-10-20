// SPDX-License-Identifier: Apache-2.0
// Point: a type with a method whose name also exists as a free function
// (the E1 "same name" case) and with a member/namespace-scope state that
// other translation units read (the E7 "reference" case).
#pragma once

namespace geom {

struct Point {
  double x = 0.0;
  double y = 0.0;

  // Method identity candidate: `Point::norm` vs the free `geom::norm`.
  double norm() const;

  // Overloaded member: const and non-const.
  double budget() const;
  double& budget();
};

// Same name as the method above, different entity.
double norm(const Point& p);

// Namespace-scope state: the E7 reference target.
extern double kBudget;

}  // namespace geom
