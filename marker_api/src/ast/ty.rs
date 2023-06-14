use std::marker::PhantomData;

use super::{Span, SpanId};

// Primitive types
mod bool_ty;
mod fn_ty;
mod other_ty;
mod prim_ty;
mod ptr_ty;
mod sequence_ty;
mod trait_ty;
mod user_ty;
pub use bool_ty::*;
pub use fn_ty::*;
pub use other_ty::*;
pub use prim_ty::*;
pub use ptr_ty::*;
pub use sequence_ty::*;
pub use trait_ty::*;
pub use user_ty::*;
mod text_ty;
pub use text_ty::*;
mod num_ty;
pub use num_ty::*;
mod never_ty;
pub use never_ty::*;
// Sequence types
mod tuple_ty;
pub use tuple_ty::*;
mod array_ty;
pub use array_ty::*;
mod slice_ty;
pub use slice_ty::*;
// Function types
mod closure_ty;
pub use closure_ty::*;
// Pointer types
mod ref_ty;
pub use ref_ty::*;
mod raw_ptr_ty;
pub use raw_ptr_ty::*;
mod fn_ptr_ty;
pub use fn_ptr_ty::*;
// Trait types
mod trait_obj_ty;
pub use trait_obj_ty::*;
mod impl_trait_ty;
pub use impl_trait_ty::*;
// Syntactic types
mod inferred_ty;
pub use inferred_ty::*;
mod path_ty;
pub use path_ty::*;

pub trait SynTyData<'ast> {
    fn as_kind(&'ast self) -> SynTyKind<'ast>;

    /// The [`Span`] of the type, if it's written in the source code.
    fn span(&self) -> &Span<'ast>;
}

#[repr(C)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SynTyKind<'ast> {
    // ================================
    // Primitive types
    // ================================
    /// The `bool` type
    Bool(&'ast BoolTy<'ast>),
    /// A numeric type like [`u32`], [`i32`], [`f64`]
    Num(&'ast NumTy<'ast>),
    /// A textual type like [`char`] or [`str`]
    Text(&'ast TextTy<'ast>),
    /// The never type [`!`](prim@never)
    Never(&'ast NeverTy<'ast>),
    // ================================
    // Sequence types
    // ================================
    /// A tuple type like [`()`](prim@tuple), [`(T, U)`](prim@tuple)
    Tuple(&'ast TupleTy<'ast>),
    /// An array with a known size like: [`[T; N]`](prim@array)
    Array(&'ast ArrayTy<'ast>),
    /// A variable length slice like [`[T]`](prim@slice)
    Slice(&'ast SliceTy<'ast>),
    // ================================
    // Function types
    // ================================
    Closure(&'ast ClosureTy<'ast>),
    // ================================
    // Pointer types
    // ================================
    /// A reference like [`&T`](prim@reference) or [`&mut T`](prim@reference)
    Ref(&'ast RefTy<'ast>),
    /// A raw pointer like [`*const T`](prim@pointer) or [`*mut T`](prim@pointer)
    RawPtr(&'ast RawPtrTy<'ast>),
    /// A function pointer like [`fn (T) -> U`](prim@fn)
    FnPtr(&'ast FnPtrTy<'ast>),
    // ================================
    // Trait types
    // ================================
    /// A trait object like [`dyn Trait`](https://doc.rust-lang.org/stable/std/keyword.dyn.html)
    TraitObj(&'ast TraitObjTy<'ast>),
    /// An [`impl Trait`](https://doc.rust-lang.org/stable/std/keyword.impl.html) type like:
    ///
    /// ```
    /// trait Trait {}
    /// # impl Trait for () {}
    ///
    /// // argument position: anonymous type parameter
    /// fn foo(arg: impl Trait) {
    /// }
    ///
    /// // return position: abstract return type
    /// fn bar() -> impl Trait {
    /// }
    /// ```
    ///
    /// See: <https://doc.rust-lang.org/stable/reference/types/impl-trait.html>
    ImplTrait(&'ast ImplTraitTy<'ast>),
    // ================================
    // Syntactic types
    // ================================
    /// An inferred type
    Inferred(&'ast InferredTy<'ast>),
    Path(&'ast PathTy<'ast>),
}

impl<'ast> SynTyKind<'ast> {
    /// Returns `true` if this is a primitive type.
    #[must_use]
    pub fn is_primitive_ty(&self) -> bool {
        matches!(self, Self::Bool(..) | Self::Num(..) | Self::Text(..) | Self::Never(..))
    }

    /// Returns `true` if this is a sequence type.
    #[must_use]
    pub fn is_sequence_ty(&self) -> bool {
        matches!(self, Self::Tuple(..) | Self::Array(..) | Self::Slice(..))
    }

    /// Returns `true` if this is a function type.
    #[must_use]
    pub fn is_fn(&self) -> bool {
        matches!(self, Self::FnPtr(..) | Self::Closure(..))
    }

    /// Returns `true` if this is a pointer type.
    #[must_use]
    pub fn is_ptr_ty(&self) -> bool {
        matches!(self, Self::Ref(..) | Self::RawPtr(..) | Self::FnPtr(..))
    }

    /// Returns `true` if this is a trait type.
    #[must_use]
    pub fn is_trait_ty(&self) -> bool {
        matches!(self, Self::TraitObj(..) | Self::ImplTrait(..))
    }

    /// Returns `true` if this is a syntactic type, meaning a type that is
    /// only used in syntax like [`TyKind::Inferred`].
    ///
    /// See [`TyKind::is_syntactic()`] to check if this type originates from
    /// a syntactic definition.
    #[must_use]
    pub fn is_inferred(&self) -> bool {
        matches!(self, Self::Inferred(..))
    }
}

impl<'ast> SynTyKind<'ast> {
    impl_syn_ty_data_fn!(span() -> &Span<'ast>);
}

/// Until [trait upcasting](https://github.com/rust-lang/rust/issues/65991) has been implemented
/// and stabilized we need this to call [`TyData`] functions for every [`TyKind`].
macro_rules! impl_syn_ty_data_fn {
    ($method:ident () -> $return_ty:ty) => {
        impl_syn_ty_data_fn!($method() -> $return_ty,
        Bool, Num, Text, Never, Tuple, Array, Slice, Closure, Ref,
        RawPtr, FnPtr, TraitObj, ImplTrait, Inferred, Path);
    };
    ($method:ident () -> $return_ty:ty $(, $item:ident)+) => {
        pub fn $method(&self) -> $return_ty {
            match self {
                $(SynTyKind::$item(data) => data.$method(),)*
            }
        }
    };
}

use impl_syn_ty_data_fn;

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "driver-api", visibility::make(pub))]
pub(crate) struct CommonSynTyData<'ast> {
    _lifetime: PhantomData<&'ast ()>,
    span: SpanId,
}

#[cfg(feature = "driver-api")]
impl<'ast> CommonSynTyData<'ast> {
    pub fn new_syntactic(span: SpanId) -> Self {
        Self {
            _lifetime: PhantomData,
            span,
        }
    }
}

macro_rules! impl_ty_data {
    ($self_ty:ty, $enum_name:ident) => {
        impl<'ast> From<&'ast $self_ty> for $crate::ast::ty::SynTyKind<'ast> {
            fn from(from: &'ast $self_ty) -> Self {
                $crate::ast::ty::SynTyKind::$enum_name(from)
            }
        }

        impl<'ast> $crate::ast::ty::SynTyData<'ast> for $self_ty {
            fn as_kind(&'ast self) -> $crate::ast::ty::SynTyKind<'ast> {
                self.into()
            }

            fn span(&self) -> &$crate::ast::Span<'ast> {
                $crate::context::with_cx(self, |cx| cx.get_span(self.data.span))
            }
        }
    };
}
use impl_ty_data;

/// The semantic representation of a type
#[repr(C)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone)]
pub enum SemTyKind<'ast> {
    // ================================
    // Primitive types
    // ================================
    /// The `bool` type
    Bool(&'ast SemBoolTy<'ast>),
    /// A numeric type like [`u32`], [`i32`], [`f64`]
    Num(&'ast SemNumTy<'ast>),
    /// A textual type like [`char`] or [`str`]
    Text(&'ast SemTextTy<'ast>),
    /// The never type [`!`](prim@never)
    Never(&'ast SemNeverTy<'ast>),
    // ================================
    // Sequence types
    // ================================
    /// A tuple type like [`()`](prim@tuple), [`(T, U)`](prim@tuple)
    Tuple(&'ast SemTupleTy<'ast>),
    /// An array with a known size like: [`[T; N]`](prim@array)
    Array(&'ast SemArrayTy<'ast>),
    /// A variable length slice like [`[T]`](prim@slice)
    Slice(&'ast SemSliceTy<'ast>),
    // ================================
    // Function types
    // ================================
    /// A [function item type](https://doc.rust-lang.org/reference/types/function-item.html)
    /// identifying a specific function and potentualy additional generics.
    FnTy(&'ast SemFnTy<'ast>),
    /// The semantic representation of a
    /// [closure type](https://doc.rust-lang.org/reference/types/closure.html).
    ClosureTy(&'ast SemClosureTy<'ast>),
    // ================================
    // Pointer types
    // ================================
    /// A reference like [`&T`](prim@reference) or [`&mut T`](prim@reference)
    Ref(&'ast SemRefTy<'ast>),
    /// A raw pointer like [`*const T`](prim@pointer) or [`*mut T`](prim@pointer)
    RawPtr(&'ast SemRawPtrTy<'ast>),
    /// The semantic representation of a function pointer, like [`fn (T) -> U`](prim@fn)
    FnPtr(&'ast SemFnPtrTy<'ast>),
    // ================================
    // Trait types
    // ================================
    /// A trait object like [`dyn Trait`](https://doc.rust-lang.org/stable/std/keyword.dyn.html)
    TraitObj(&'ast SemTraitObjTy<'ast>),
    // ================================
    // User defined types
    // ================================
    /// A user defined data type, identified by an [`TyDefId`](super::TyDefId)
    Adt(&'ast SemAdtTy<'ast>),
    /// A generic type defined by a generic parameter
    Generic(&'ast SemGenericTy<'ast>),
    /// A type alias. Note that simple type aliases will already be replaced in
    /// semantic types. This kind is mainly used for type aliases, where the concrete
    /// type is not yet known, for example in traits.
    Alias(&'ast SemAliasTy<'ast>),
    // ================================
    // Other types
    // ================================
    /// The placeholder type, signalling that the semantic type is still unstable
    /// and therefor not represented as part of the API.
    Unstable(&'ast SemUnstableTy<'ast>),
}
