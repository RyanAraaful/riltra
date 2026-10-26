// SPDX-License-Identifier: Apache-2.0
// The deliberately ambiguous call is kept out of the build so that the rest
// of the fixture compiles cleanly; the pilot compiles this file on its own
// to capture the compiler's verdict as ground truth.
#include "geom/overload.h"

namespace demo {

void ambiguous() {
  amb::pick('c');
}

}  // namespace demo
