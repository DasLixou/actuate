use crate::prelude::*;
use std::marker::PhantomData;

/// Composable data.
///
/// This trait should be derived with `#[derive(Data)]`.
pub unsafe trait Data: Sized {
    type Id: 'static;

    unsafe fn reborrow(self, ptr: *mut ()) {
        let x = ptr as *mut Self;
        *x = self;
    }
}

unsafe impl Data for () {
    type Id = ();
}

// TODO
unsafe impl Data for i32 {
    type Id = i32;
}

unsafe impl Data for String {
    type Id = Self;
}

unsafe impl Data for &str {
    type Id = &'static str;
}

unsafe impl<T: Data> Data for Vec<T> {
    type Id = Vec<T::Id>;
}

unsafe impl<T: ?Sized + Data> Data for &T {
    type Id = &'static T::Id;
}

unsafe impl<T: Data> Data for Option<T> {
    type Id = Option<T::Id>;
}

unsafe impl<T: Data + ?Sized> Data for Ref<'_, T> {
    type Id = PhantomData<Ref<'static, T::Id>>;
}

unsafe impl<T: Data + ?Sized> Data for Map<'_, T> {
    type Id = PhantomData<Map<'static, T::Id>>;
}

unsafe impl<T: Data> Data for Mut<'_, T> {
    type Id = PhantomData<Mut<'static, T::Id>>;
}

unsafe impl Data for DynCompose<'_> {
    type Id = PhantomData<DynCompose<'static>>;
}

pub struct FieldWrap<T>(pub T);

#[doc(hidden)]
pub unsafe trait StateField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: 'static> StateField for FieldWrap<&T> {}

#[doc(hidden)]
pub unsafe trait FnField<Marker> {
    fn check(&self) {
        let _ = self;
    }
}

macro_rules! impl_data_for_fns {
    ($($t:tt),*) => {
        unsafe impl<$($t,)* F: FnMut($($t,)*)> FnField<fn($($t,)*)> for &FieldWrap<F> {}
    }
}

impl_data_for_fns!();
impl_data_for_fns!(T1);
impl_data_for_fns!(T1, T2);
impl_data_for_fns!(T1, T2, T3);
impl_data_for_fns!(T1, T2, T3, T4);
impl_data_for_fns!(T1, T2, T3, T4, T5);
impl_data_for_fns!(T1, T2, T3, T4, T5, T6);
impl_data_for_fns!(T1, T2, T3, T4, T5, T6, T7);
impl_data_for_fns!(T1, T2, T3, T4, T5, T6, T7, T8);

#[doc(hidden)]
pub unsafe trait DataField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: Data> DataField for &FieldWrap<T> {}

#[doc(hidden)]
pub unsafe trait StaticField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: 'static> StaticField for &&FieldWrap<T> {}
