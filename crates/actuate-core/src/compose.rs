use std::{
    any::TypeId,
    cell::{RefCell, UnsafeCell},
    mem,
};

use crate::{prelude::*, Memoize, ScopeData};

/// A composable function.
///
/// For a dynamically-typed composable, see [`DynCompose`].
pub trait Compose: Data {
    fn compose(cx: Scope<Self>) -> impl Compose;

    #[cfg(feature = "tracing")]
    #[doc(hidden)]
    fn name() -> std::borrow::Cow<'static, str> {
        std::any::type_name::<Self>().into()
    }
}

impl Compose for () {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_empty.set(true);
    }
}

impl<C: Compose> Compose for &C {
    fn compose(cx: Scope<Self>) -> impl Compose {
        unsafe {
            (**cx.me()).any_compose(&cx);
        }
    }
}

impl<C: Compose> Compose for Option<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_container.set(true);

        let state_cell: &RefCell<Option<ScopeData>> = use_ref(&cx, || RefCell::new(None));
        let mut state_cell = state_cell.borrow_mut();

        if let Some(content) = &*cx.me() {
            if let Some(state) = &*state_cell {
                state.is_parent_changed.set(cx.is_parent_changed.get());
                unsafe {
                    content.any_compose(state);
                }
            } else {
                let mut state = ScopeData::default();
                state.contexts = cx.contexts.clone();
                *state_cell = Some(state);
                unsafe {
                    content.any_compose(&*state_cell.as_ref().unwrap());
                }
            }
        } else {
            *state_cell = None;
        }
    }
}

pub fn from_iter<'a, I, C>(iter: I, f: impl Fn(I::Item) -> C + 'a) -> FromIter<'a, I, I::Item, C>
where
    I: IntoIterator + Clone + Data,
    I::Item: Clone + Data,
    C: Compose,
{
    FromIter {
        iter,
        f: Box::new(f),
    }
}

pub struct FromIter<'a, I, Item, C> {
    iter: I,
    f: Box<dyn Fn(Item) -> C + 'a>,
}

unsafe impl<I, Item, C> Data for FromIter<'_, I, Item, C>
where
    I: Data,
    Item: Data,
    C: Data,
{
    type Id = FromIter<'static, I::Id, Item::Id, C::Id>;
}

impl<I, Item, C> Compose for FromIter<'_, I, Item, C>
where
    I: IntoIterator<Item = Item> + Clone + Data,
    Item: Clone + Data,
    C: Compose,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_container.set(true);

        let states = use_ref(&cx, || RefCell::new(Vec::new()));
        let mut states = states.borrow_mut();

        let items: Vec<_> = cx.me().iter.clone().into_iter().collect();
        if items.len() >= states.len() {
            for _ in states.len()..items.len() {
                states.push(ScopeData::default());
            }
        } else {
            for _ in items.len()..states.len() {
                states.pop();
            }
        }

        for (item, state) in items.into_iter().zip(&*states) {
            *state.contexts.borrow_mut() = cx.contexts.borrow().clone();
            state.is_parent_changed.set(cx.is_parent_changed.get());

            unsafe { (cx.me().f)(item).any_compose(state) }
        }
    }
}

#[derive(Data)]
pub struct Memo<T, C> {
    dependency: T,
    content: C,
}

impl<T, C> Memo<T, C> {
    pub fn new(dependency: impl Memoize<Value = T>, content: C) -> Self {
        Self {
            dependency: dependency.memoized(),
            content,
        }
    }
}

impl<T, C> Compose for Memo<T, C>
where
    T: Clone + Data + PartialEq + 'static,
    C: Compose,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        let last = use_ref(&cx, RefCell::default);
        let mut last = last.borrow_mut();
        if let Some(last) = &mut *last {
            if cx.me().dependency != *last {
                *last = cx.me().dependency.clone();
                cx.is_parent_changed.set(true);
            }
        } else {
            *last = Some(cx.me().dependency.clone());
            cx.is_parent_changed.set(true);
        }

        Ref::map(cx.me(), |me| &me.content)
    }

    #[cfg(feature = "tracing")]
    fn name() -> std::borrow::Cow<'static, str> {
        format!("Memo<{}>", C::name()).into()
    }
}

/// Dynamically-typed composable.
pub struct DynCompose<'a> {
    compose: UnsafeCell<Option<Box<dyn AnyCompose + 'a>>>,
}

impl<'a> DynCompose<'a> {
    pub fn new(content: impl Compose + 'a) -> Self {
        Self {
            compose: UnsafeCell::new(Some(Box::new(content))),
        }
    }
}

struct DynComposeState {
    compose: Box<dyn AnyCompose>,
    data_id: TypeId,
}

impl<'a> Compose for DynCompose<'a> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_container.set(true);

        let cell: &UnsafeCell<Option<DynComposeState>> = use_ref(&cx, || UnsafeCell::new(None));
        let cell = unsafe { &mut *cell.get() };

        let inner = unsafe { &mut *cx.me().compose.get() };

        let child_state = use_ref(&cx, ScopeData::default);

        *child_state.contexts.borrow_mut() = cx.contexts.borrow().clone();
        child_state
            .is_parent_changed
            .set(cx.is_parent_changed.get());

        if let Some(any_compose) = inner.take() {
            let mut compose: Box<dyn AnyCompose> = unsafe { mem::transmute(any_compose) };

            if let Some(state) = cell {
                if state.data_id != compose.data_id() {
                    todo!()
                }

                let ptr = (*state.compose).as_ptr_mut();

                unsafe {
                    compose.reborrow(ptr);
                }
            } else {
                *cell = Some(DynComposeState {
                    data_id: compose.data_id(),
                    compose,
                })
            }
        }

        unsafe { cell.as_mut().unwrap().compose.any_compose(child_state) }
    }
}

macro_rules! impl_tuples {
    ($($t:tt : $idx:tt),*) => {
        unsafe impl<$($t: Data),*> Data for ($($t,)*) {
            type Id = ($($t::Id,)*);
        }

        impl<$($t: Compose),*> Compose for ($($t,)*) {
            fn compose(cx: Scope<Self>) -> impl Compose {
                cx.is_container.set(true);

                $(
                    let state = use_ref(&cx, || {
                        ScopeData::default()
                    });

                    *state.contexts.borrow_mut() = cx.contexts.borrow().clone();
                    state.is_parent_changed.set(cx.is_parent_changed.get());

                    unsafe { cx.me().$idx.any_compose(state) }
                )*
            }

            fn name() -> std::borrow::Cow<'static, str> {
                let mut s = String::from('(');

                $(s.push_str(&$t::name());)*

                s.push(')');
                s.into()
            }
        }
    };
}

impl_tuples!(T1:0);
impl_tuples!(T1:0, T2:1);
impl_tuples!(T1:0, T2:1, T3:2);
impl_tuples!(T1:0, T2:1, T3:2, T4:3);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4, T6:5);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4, T6:5, T7:6);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4, T6:5, T7:6, T8:7);

pub(crate) trait AnyCompose {
    fn data_id(&self) -> TypeId;

    fn as_ptr_mut(&mut self) -> *mut ();

    unsafe fn reborrow(&mut self, ptr: *mut ());

    unsafe fn any_compose(&self, state: &ScopeData);

    #[cfg(feature = "tracing")]
    fn name(&self) -> std::borrow::Cow<'static, str>;
}

impl<C> AnyCompose for C
where
    C: Compose + Data,
{
    fn data_id(&self) -> TypeId {
        TypeId::of::<C::Id>()
    }

    fn as_ptr_mut(&mut self) -> *mut () {
        self as *mut Self as *mut ()
    }

    unsafe fn reborrow(&mut self, ptr: *mut ()) {
        std::ptr::swap(self, ptr as _);
    }

    unsafe fn any_compose(&self, state: &ScopeData) {
        state.hook_idx.set(0);

        // Transmute the lifetime of `&Self`, `&ScopeData`, and the `Scope` containing both to the same`'a`.
        let cx: Scope<'_, C> = Scope {
            me: unsafe { mem::transmute(self) },
            state: unsafe { mem::transmute(state) },
        };
        let cx: Scope<'_, C> = unsafe { mem::transmute(cx) };

        let cell: &UnsafeCell<Option<Box<dyn AnyCompose>>> = use_ref(&cx, || UnsafeCell::new(None));
        let cell = unsafe { &mut *cell.get() };

        let child_state = use_ref(&cx, ScopeData::default);

        if cell.is_none()
            || cx.is_changed.take()
            || cx.is_parent_changed.get()
            || cx.is_container.get()
        {
            let child = C::compose(cx);

            cx.is_parent_changed.set(false);
            if cx.state.is_empty.take() {
                return;
            }

            #[cfg(feature = "tracing")]
            if !cx.is_container.get() {
                tracing::trace!("Compose::compose: {}", self.name());
            }

            *child_state.contexts.borrow_mut() = cx.contexts.borrow().clone();
            child_state.is_parent_changed.set(true);

            unsafe {
                if let Some(ref mut content) = cell {
                    child.reborrow((**content).as_ptr_mut());
                } else {
                    let boxed: Box<dyn AnyCompose> = Box::new(child);
                    *cell = Some(mem::transmute(boxed));
                }
            }
        } else {
            child_state.is_parent_changed.set(false);
        }

        let child = cell.as_mut().unwrap();
        (*child).any_compose(child_state);
    }

    #[cfg(feature = "tracing")]
    fn name(&self) -> std::borrow::Cow<'static, str> {
        C::name().into()
    }
}
