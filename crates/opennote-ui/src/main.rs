use gpui::*;
use gpui_component::{
    button::*,
    sidebar::{Sidebar, SidebarMenu},
    *,
};

pub struct Main;

impl Render for Main {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let sidebar = Sidebar::new(Side::Left).child(SidebarMenu::new());

        v_flex().id("workspace-sidebar").h_full().child(sidebar)
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
