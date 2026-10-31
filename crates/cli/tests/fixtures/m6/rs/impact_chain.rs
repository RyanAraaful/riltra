// SPDX-License-Identifier: Apache-2.0
fn target() {}
fn middle() { target(); }
fn caller() { middle(); }
