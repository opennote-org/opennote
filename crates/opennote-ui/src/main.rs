use gpui::*;
use gpui_component::{button::*, sidebar::Sidebar, *};

pub struct Main;

impl Render for Main {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child(Sidebar::new(Side::Left).child(Tree::new()))
    }
}

pub struct Tree {
    is_collapsed: bool,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            is_collapsed: false,
        }
    }
}

impl Element for Tree {
    
}

impl IntoElement for Tree {
    fn into_element(self) -> Self::Element {
    }
}

impl Collapsible for Tree {
    fn collapsed(self, collapsed: bool) -> Self {
        Self {
            is_collapsed: collapsed,
        }
    }

    fn is_collapsed(&self) -> bool {
        self.is_collapsed
    }
}

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Main);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
