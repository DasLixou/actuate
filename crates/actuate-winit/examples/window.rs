use actuate_core::prelude::*;
use actuate_winit::Window;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::FmtSubscriber;
use winit::window::WindowAttributes;

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        Window::new(
            WindowAttributes::default(),
            move |_window, event| {
                dbg!(event);
            },
            (),
        )
    }
}

fn main() {
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(LevelFilter::TRACE)
            .finish(),
    )
    .unwrap();

    actuate_winit::run(App);
}
