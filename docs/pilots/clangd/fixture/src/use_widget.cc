// SPDX-License-Identifier: Apache-2.0
// E12: a construction and a declaration that only looks like one.
//
// E12-a: `ui::Widget w(options, 3);` reads like a function declaration
// `w(Config, int)`. Only semantic analysis can tell that `options` is a
// VARIABLE, so this is a construction. The static_assert is the compiler's
// own verdict.
//
// E12-b: the most vexing parse in its pure form. `ui::Widget w2(ui::Widget);`
// has a TYPE as its argument, so `w2` really is a function declaration.
//
// The two static_asserts are the compiler's verdict, so the fixture carries
// its ground truth in the source.
#include "geom/widget.h"

#include <type_traits>

namespace demo {

void makeWidget() {
  ui::Config options{7};
  ui::Widget w(options, 3);
  static_assert(std::is_same_v<decltype(w), ui::Widget>, "w is a Widget object");
  (void)w.seed();
}

void makeAmbiguous() {
  ui::Widget w2(ui::Widget);
  static_assert(std::is_function_v<std::remove_reference_t<decltype(w2)>>,
                "w2 is a function declaration");
  (void)&w2;
}

}  // namespace demo
