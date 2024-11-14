use crate::{prelude::*, RendererContext};
use actuate_core::MapCompose;
use taffy::{FlexDirection, Style};

pub struct Flex<C> {
    style: Style,
    content: C,
}

impl<C> Flex<C> {
    pub fn new(style: Style, content: C) -> Self {
        Self { style, content }
    }

    pub fn column(content: C) -> Self {
        Self::new(
            Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            content,
        )
    }

    pub fn row(content: C) -> Self {
        Self::new(
            Style {
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
            content,
        )
    }
}

unsafe impl<C: Data> Data for Flex<C> {
    type Id = Flex<C::Id>;
}

impl<C: Compose> Compose for Flex<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let renderer_cx = use_context::<RendererContext>(&cx);
        use_ref(&cx, || {
            let id = renderer_cx
                .taffy
                .borrow_mut()
                .new_leaf(cx.me().style.clone())
                .unwrap();
            renderer_cx
                .taffy
                .borrow_mut()
                .add_child(*renderer_cx.parent_key.borrow(), id)
                .unwrap();
            *renderer_cx.parent_key.borrow_mut() = id;
        });

        unsafe { MapCompose::new(Ref::map(cx.me(), |me| &me.content)) }
    }
}