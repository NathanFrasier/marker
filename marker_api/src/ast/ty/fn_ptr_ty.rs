use crate::ast::{impl_callable_data_trait, CommonCallableData};

use super::CommonTyData;

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct FnPtrTy<'ast> {
    data: CommonTyData<'ast>,
    callable_data: CommonCallableData<'ast>,
    // FIXME: Add `for<'a>` bound
}

#[cfg(feature = "driver-api")]
impl<'ast> FnPtrTy<'ast> {
    pub fn new(data: CommonTyData<'ast>, callable_data: CommonCallableData<'ast>) -> Self {
        Self { data, callable_data }
    }
}

super::impl_ty_data!(FnPtrTy<'ast>, FnPtr);
impl_callable_data_trait!(FnPtrTy<'ast>);
