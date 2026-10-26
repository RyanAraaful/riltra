// SPDX-License-Identifier: Apache-2.0
#include "geom/widget.h"

namespace ui {

Widget::Widget(const Config& config, int extra) : seed_(config.seed + extra) {}

int Widget::seed() const { return seed_; }

}  // namespace ui
