# SPDX-License-Identifier: Apache-2.0
# The clangd pilot fixture: a small C++ project with deliberate E1 (same
# name), E7 (references) and E12 (declaration vs construction) cases.
#
# Configure it with CMAKE_EXPORT_COMPILE_COMMANDS=ON so that CMake itself
# produces the compilation database clangd consumes. The pilot never edits
# the generated file by hand.
cmake_minimum_required(VERSION 3.20)
project(riltra_clangd_fixture CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

add_library(geom STATIC
  src/point.cc
  src/overload.cc
  src/widget.cc
)
target_include_directories(geom PUBLIC include)

add_executable(demo
  src/main.cc
  src/use_point.cc
  src/use_state.cc
  src/use_overload.cc
  src/use_widget.cc
)
target_link_libraries(demo PRIVATE geom)
