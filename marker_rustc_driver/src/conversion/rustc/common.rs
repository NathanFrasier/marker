use std::mem::{size_of, transmute};

use marker_api::{
    ast::{
        BodyId, CrateId, ExprId, FieldId, GenericId, ItemId, LetStmtId, Span, SpanId, StmtIdInner, SymbolId, TyDefId,
        VarId, VariantId,
    },
    diagnostic::{Applicability, EmissionNode},
    lint::Level,
};
use rustc_hir as hir;

use crate::conversion::common::{BodyIdLayout, DefIdInfo, DefIdLayout, HirIdLayout};
use crate::transmute_id;

use super::RustcConverter;

macro_rules! impl_into_def_id_for {
    ($id:ty) => {
        impl From<$id> for DefIdInfo {
            fn from(value: $id) -> Self {
                let layout = transmute_id!($id as DefIdLayout = value);
                DefIdInfo {
                    index: layout.index,
                    krate: layout.krate,
                }
            }
        }
    };
}

impl_into_def_id_for!(GenericId);
impl_into_def_id_for!(ItemId);
impl_into_def_id_for!(TyDefId);
impl_into_def_id_for!(VariantId);

pub struct HirIdInfo {
    pub owner: u32,
    pub index: u32,
}

macro_rules! impl_into_hir_id_for {
    ($id:ty) => {
        impl From<$id> for HirIdInfo {
            fn from(value: $id) -> Self {
                let layout = transmute_id!($id as HirIdLayout = value);
                HirIdInfo {
                    owner: layout.owner,
                    index: layout.index,
                }
            }
        }
    };
}

impl_into_hir_id_for!(ExprId);
impl_into_hir_id_for!(VarId);
impl_into_hir_id_for!(LetStmtId);
impl_into_hir_id_for!(FieldId);

#[derive(Debug, Clone, Copy)]
pub struct SpanSourceInfo {
    pub rustc_span_cx: rustc_span::hygiene::SyntaxContext,
    pub rustc_start_offset: usize,
}

impl<'ast, 'tcx> RustcConverter<'ast, 'tcx> {
    #[must_use]
    pub fn to_crate_num(&self, api_id: CrateId) -> hir::def_id::CrateNum {
        assert_eq!(size_of::<CrateId>(), 4);
        hir::def_id::CrateNum::from_u32(api_id.data())
    }

    #[must_use]
    pub fn to_item_id(&self, api_id: ItemId) -> hir::ItemId {
        let layout = transmute_id!(ItemId as DefIdLayout = api_id);
        hir::ItemId {
            owner_id: hir::OwnerId {
                def_id: hir::def_id::LocalDefId {
                    local_def_index: hir::def_id::DefIndex::from_u32(layout.index),
                },
            },
        }
    }

    #[must_use]
    pub fn to_body_id(&self, api_id: BodyId) -> hir::BodyId {
        let layout = transmute_id!(BodyId as BodyIdLayout = api_id);
        hir::BodyId {
            hir_id: hir::HirId {
                owner: hir::OwnerId {
                    def_id: hir::def_id::LocalDefId {
                        local_def_index: hir::def_id::DefIndex::from_u32(layout.owner),
                    },
                },
                local_id: hir::hir_id::ItemLocalId::from_u32(layout.index),
            },
        }
    }

    #[must_use]
    pub fn to_symbol(&self, api_id: SymbolId) -> rustc_span::Symbol {
        assert_eq!(size_of::<SymbolId>(), 4);
        assert_eq!(size_of::<rustc_span::Symbol>(), 4);
        // FIXME: `rustc_span::Symbol` currently has no public constructor for the
        // index value and no `#[repr(C)]` attribute. Therefore, this conversion is
        // unsound. This requires changes in rustc.
        unsafe { transmute(api_id) }
    }

    #[must_use]
    pub fn to_span_from_id(&self, api_id: SpanId) -> rustc_span::Span {
        assert_eq!(
            size_of::<SpanId>(),
            size_of::<rustc_span::Span>(),
            "the size of `Span` or `SpanId` has changed"
        );
        // # Safety
        // The site was validated with the `assert` above, the layout is provided by rustc
        unsafe { transmute(api_id) }
    }

    #[must_use]
    pub fn to_def_id(&self, api_id: impl Into<DefIdInfo>) -> hir::def_id::DefId {
        let info: DefIdInfo = api_id.into();
        hir::def_id::DefId {
            index: hir::def_id::DefIndex::from_u32(info.index),
            krate: hir::def_id::CrateNum::from_u32(info.krate),
        }
    }

    #[must_use]
    pub fn try_to_hir_id_from_emission_node(&self, node: EmissionNode) -> Option<hir::HirId> {
        let def_id = match node {
            EmissionNode::Expr(id) => return Some(self.to_hir_id(id)),
            EmissionNode::Item(id) => self.to_def_id(id),
            EmissionNode::Stmt(stmt_id) => match stmt_id.data() {
                StmtIdInner::Expr(id) => return Some(self.to_hir_id(id)),
                StmtIdInner::Item(id) => self.to_def_id(id),
                StmtIdInner::LetStmt(id) => return Some(self.to_hir_id(id)),
            },
            EmissionNode::Field(id) => return Some(self.to_hir_id(id)),
            EmissionNode::Variant(id) => self.to_def_id(id),
            _ => unreachable!(),
        };

        def_id
            .as_local()
            .map(|id| self.rustc_cx.hir().local_def_id_to_hir_id(id))
    }

    #[must_use]
    pub fn to_hir_id(&self, api_id: impl Into<HirIdInfo>) -> hir::HirId {
        let info: HirIdInfo = api_id.into();
        hir::HirId {
            owner: hir::OwnerId {
                def_id: hir::def_id::LocalDefId {
                    local_def_index: hir::def_id::DefIndex::from_u32(info.owner),
                },
            },
            local_id: hir::hir_id::ItemLocalId::from_u32(info.index),
        }
    }

    #[must_use]
    pub fn to_lint_level(&self, api_level: Level) -> rustc_lint::Level {
        Self::static_to_lint_level(api_level)
    }

    /// This not being a method taking `&self` is a small hack, to allow the
    /// creation of `&'static Lint` instances before the start of the `'ast`
    /// lifetime, required by the [`RustcConverter`].
    ///
    /// When possible, please use [`RustcConverter::to_lint_level`] instead.
    #[must_use]
    pub fn static_to_lint_level(api_level: Level) -> rustc_lint::Level {
        match api_level {
            Level::Allow => rustc_lint::Level::Allow,
            Level::Warn => rustc_lint::Level::Warn,
            Level::Deny => rustc_lint::Level::Deny,
            Level::Forbid => rustc_lint::Level::Forbid,
            _ => unreachable!(),
        }
    }

    #[must_use]
    pub(crate) fn to_applicability(&self, app: Applicability) -> rustc_errors::Applicability {
        match app {
            Applicability::MachineApplicable => rustc_errors::Applicability::MachineApplicable,
            Applicability::MaybeIncorrect => rustc_errors::Applicability::MaybeIncorrect,
            Applicability::HasPlaceholders => rustc_errors::Applicability::HasPlaceholders,
            Applicability::Unspecified => rustc_errors::Applicability::Unspecified,
            _ => unreachable!(),
        }
    }

    #[must_use]
    pub fn to_span(&self, api_span: &Span<'ast>) -> rustc_span::Span {
        let (_, src_info) = self
            .storage
            .get_span_src_info(api_span.source())
            .expect("all driver created `SpanSources` have a matching info");

        #[expect(clippy::cast_possible_truncation, reason = "`u32` is set by rustc and will be fine")]
        let lo = rustc_span::BytePos((api_span.start() + src_info.rustc_start_offset) as u32);
        #[expect(clippy::cast_possible_truncation, reason = "`u32` is set by rustc and will be fine")]
        let hi = rustc_span::BytePos((api_span.end() + src_info.rustc_start_offset) as u32);
        rustc_span::Span::new(lo, hi, src_info.rustc_span_cx, None)
    }
}
