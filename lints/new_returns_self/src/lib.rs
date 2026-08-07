#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::def::Res;
use rustc_hir::{FnRetTy, ImplItemKind, QPath, TyKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::sym;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Requires that `fn new` return `Self` directly.
    ///
    /// ### Why is this bad?
    ///
    /// Returning `Option<Self>`, `Result<Self, _>`, `Box<Self>`, `Arc<Self>`,
    /// `impl Trait`, or no value at all from a `new` constructor hides the
    /// intent of the method and complicates call sites.
    ///
    /// ### Known problems
    ///
    /// None.
    ///
    /// ### Example
    ///
    /// ```rust
    /// struct Foo;
    ///
    /// impl Foo {
    ///     fn new() -> Self {
    ///         Self
    ///     }
    /// }
    /// ```
    pub NEW_RETURNS_SELF,
    Warn,
    "`new` must return `Self` directly"
}

impl<'tcx> LateLintPass<'tcx> for NewReturnsSelf {
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx rustc_hir::ImplItem<'tcx>) {
        if item.ident.name != sym::new {
            return;
        }

        let ImplItemKind::Fn(sig, _) = item.kind else {
            return;
        };

        if is_clean_self_return(&sig.decl.output) {
            return;
        }

        cx.emit_span_lint(
            NEW_RETURNS_SELF,
            sig.decl.output.span(),
            DiagDecorator(|diag| {
                diag.primary_message(
                    "`new` must return `Self` directly (no Option/Result/Box/Arc/impl)",
                );
            }),
        );
    }
}

fn is_clean_self_return(ret: &FnRetTy) -> bool {
    match ret {
        FnRetTy::DefaultReturn(_) => false,
        FnRetTy::Return(ty) => match &ty.kind {
            TyKind::Path(QPath::Resolved(None, path)) => {
                matches!(path.res, Res::SelfTyParam { .. } | Res::SelfTyAlias { .. })
            },
            _ => false,
        },
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
