#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_errors::DiagDecorator;
use rustc_hir::attrs::AttributeKind;
use rustc_hir::{Attribute, Item, ItemKind, VariantData};
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Requires every hand-written `pub` struct in a module named `api` to be
    /// `#[non_exhaustive]`.
    ///
    /// ### Why is this bad?
    ///
    /// `#![deny(clippy::exhaustive_structs)]` is disabled for `api` modules
    /// because `#[utoipa::path]` generates public unit structs (`__path_*`).
    /// This lint restores the invariant for structs written by hand, while
    /// macro-expanded items are ignored.
    ///
    /// ### Known problems
    ///
    /// Recognizes the module by the literal name `api` only.
    ///
    /// ### Example
    ///
    /// ```rust
    /// mod api {
    ///     pub struct Views {
    ///         pub count: u32,
    ///     }
    /// }
    /// ```
    pub NON_EXHAUSTIVE_API_STRUCTS,
    Deny,
    "hand-written `pub` structs in `api` modules must be `#[non_exhaustive]`"
}

impl<'tcx> LateLintPass<'tcx> for NonExhaustiveApiStructs {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let ItemKind::Struct(_, _, variant_data) = &item.kind else {
            return;
        };

        if !cx.tcx.visibility(item.owner_id.def_id).is_public() {
            return;
        }

        // `#[utoipa::path]` expands to `pub struct __path_*;`: not hand-written.
        if item.span.from_expansion() {
            return;
        }

        // `#[non_exhaustive]` is a built-in attribute and is stored as
        // `Attribute::Parsed(AttributeKind::NonExhaustive)`, so path-based
        // lookups (`get_attrs_by_path`, `has_name`) do not see it.
        let is_non_exhaustive = cx.tcx.hir_attrs(item.hir_id()).iter().any(
            |attr| matches!(attr, Attribute::Parsed(AttributeKind::NonExhaustive(_))),
        );
        if is_non_exhaustive {
            return;
        }

        if !is_exhaustive(cx, variant_data) {
            return;
        }

        // Restrict to modules under `api` where `clippy::exhaustive_structs`
        // is allowed off.
        if !is_under_api(cx, item) {
            return;
        }

        cx.emit_span_lint(
            NON_EXHAUSTIVE_API_STRUCTS,
            item.span,
            DiagDecorator(|diag| {
                diag.primary_message("`pub` struct in `api` module must be `#[non_exhaustive]`");
            }),
        );
    }
}

fn is_under_api(cx: &LateContext<'_>, item: &Item<'_>) -> bool {
    cx.tcx
        .def_path_str(item.owner_id.def_id)
        .split("::")
        .any(|segment| segment == "api")
}

fn is_exhaustive(cx: &LateContext<'_>, variant_data: &VariantData) -> bool {
    match variant_data {
        VariantData::Unit(..) => true,
        VariantData::Tuple(fields, ..) | VariantData::Struct { fields, .. } => {
            fields.iter().all(|field| cx.tcx.visibility(field.def_id).is_public())
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
