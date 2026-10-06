//! Template: a workbench — the shape of an editor, IDE, notes app or asset
//! browser.
//!
//! A resizable split: a file tree on the left; on the right, a tab strip of
//! open documents over a virtualised, line-numbered document view, with an
//! inspector pane. Command palette, status bar with the cursor position.
//!
//!     cargo run --example app_workbench
//!
//! Ferrite has no multi-line text editor yet (docs/ROADMAP.md); the view
//! here is read-only. Swap `document()` for your own content.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ferrite_design::prelude::*;
use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, KeyBinding, Render, SharedString, Subscription, Window, div,
    px, size,
};

/// Stand-in for your documents: a few deterministic lines per file.
fn document(path: &str) -> Vec<String> {
    let n = 40 + path.len() * 13;
    (0..n)
        .map(|i| match i % 9 {
            0 => format!("// {path} — section {}", i / 9 + 1),
            1 => "use ferrite_design::prelude::*;".into(),
            2 | 6 => String::new(),
            3 => format!("pub fn step_{i}(cx: &mut App) -> Result<()> {{"),
            4 => format!("    let frame = motion::frame() * {};", i % 7 + 1),
            5 => "}".into(),
            7 => format!("const LIMIT_{i}: usize = {};", i * 31 % 997),
            _ => format!("// note {i}: keep the grid at 8×16"),
        })
        .collect()
}

fn file_tree() -> Vec<TreeNode> {
    vec![
        tree_node("src", "src/").icon(Icon::Folder).children([
            tree_node("src/main.rs", "main.rs").icon(Icon::File),
            tree_node("src/app.rs", "app.rs").icon(Icon::File),
            tree_node("src/modules", "modules/").icon(Icon::Folder).children([
                tree_node("src/modules/editor.rs", "editor.rs").icon(Icon::File),
                tree_node("src/modules/search.rs", "search.rs").icon(Icon::File),
            ]),
        ]),
        tree_node("assets", "assets/").icon(Icon::Folder).child(tree_node("assets/icon.png", "icon.png").icon(Icon::File)),
        tree_node("Cargo.toml", "Cargo.toml").icon(Icon::File),
    ]
}

struct Workbench {
    open: Vec<SharedString>,
    active: usize,
    selected: Option<SharedString>,
    line: usize,
    palette: Entity<CommandPalette>,
    toaster: Entity<Toaster>,
    _appearance: Subscription,
}

impl Workbench {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let weak = cx.weak_entity();
        let open = move |path: &'static str| {
            let weak = weak.clone();
            move |_: &mut Window, cx: &mut App| {
                let _ = weak.update(cx, |this, cx| this.open_file(path.into(), cx));
            }
        };
        let palette = cx.new(|cx| {
            let mut p = CommandPalette::new(window, cx);
            p.set_commands(
                vec![
                    command("Open main.rs").group("Go to file").icon(Icon::File).on_run(open("src/main.rs")),
                    command("Open app.rs").group("Go to file").icon(Icon::File).on_run(open("src/app.rs")),
                    command("Open editor.rs").group("Go to file").icon(Icon::File).on_run(open("src/modules/editor.rs")),
                    command("Open Cargo.toml").group("Go to file").icon(Icon::File).on_run(open("Cargo.toml")),
                ],
                cx,
            );
            p
        });
        Self {
            open: vec!["src/main.rs".into()],
            active: 0,
            selected: Some("src/main.rs".into()),
            line: 0,
            palette,
            toaster: cx.new(|_| Toaster::new()),
            _appearance: theme::follow_system(window),
        }
    }

    fn open_file(&mut self, path: SharedString, cx: &mut Context<Self>) {
        // Folders end in a name without a dot; only files open.
        if !path.rsplit('/').next().is_some_and(|n| n.contains('.')) {
            return;
        }
        self.active = match self.open.iter().position(|p| *p == path) {
            Some(i) => i,
            None => {
                self.open.push(path.clone());
                self.open.len() - 1
            }
        };
        self.selected = Some(path);
        self.line = 0;
        cx.notify();
    }
}

impl Render for Workbench {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let path = self.open.get(self.active).cloned().unwrap_or_default();
        let lines = document(&path);
        let count = lines.len();
        let current = self.line;

        let files = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(hsla(p.surface))
            .child(div().h(px(28.)).px_3().flex().items_center().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child("EXPLORER"))
            .child(scroll_area("tree-scroll").flex_1().min_h_0().child(
                tree("files").nodes(file_tree()).expanded(["src", "src/modules"]).selected(self.selected.clone()).on_select(cx.listener(|this, id: &SharedString, _, cx| {
                    this.selected = Some(id.clone());
                    this.open_file(id.clone(), cx);
                })),
            ));

        let mut strip = tabs("open-files").selected(self.active).on_select(cx.listener(|this, i: &usize, _, cx| {
            this.active = *i;
            this.selected = this.open.get(*i).cloned();
            cx.notify();
        }));
        for f in &self.open {
            strip = strip.tab(f.rsplit('/').next().unwrap_or(f).to_string());
        }

        let weak = cx.weak_entity();
        let code = virtual_list(("code", self.active), count, move |range, _, cx| {
            let p = palette(cx);
            range
                .map(|i| {
                    let weak = weak.clone();
                    let here = i == current;
                    div()
                        .id(("ln", i))
                        .h(px(20.))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .body(text::SM)
                        .when(here, |el| el.bg(hsla(p.raised)))
                        .child(div().w(px(48.)).flex_none().pr_2().flex().justify_end().text_color(hsla(if here { p.accent_text } else { p.fg_faint })).child(format!("{}", i + 1)))
                        .child(div().flex_1().min_w_0().whitespace_nowrap().text_color(hsla(p.fg)).child(lines[i].clone()))
                        .on_click(move |_, _, cx| {
                            let _ = weak.update(cx, |this, cx| {
                                this.line = i;
                                cx.notify();
                            });
                        })
                        .into_any_element()
                })
                .collect()
        })
        .size_full();

        let inspector = div()
            .w(px(260.))
            .flex_none()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .bg(hsla(p.surface))
            .border_l_1()
            .border_color(hsla(p.line))
            .child(div().display(Scale::X1, window).text_color(hsla(p.fg_dim)).child("[ INSPECTOR ]"))
            .child(property_list().key_width(px(72.)).row("path", path.clone()).row("lines", format!("{count}")).row("line", format!("{}", current + 1)).row_with("status", tag("saved").success()))
            .child(rule(Some("outline"), window, cx))
            .child(div().flex().flex_col().children((0..count).step_by(9).take(6).map(|i| div().body(text::XS).text_color(hsla(p.fg_dim)).child(format!("§{}  line {}", i / 9 + 1, i + 1)))));

        let editor = div()
            .flex()
            .flex_col()
            .size_full()
            .child(strip)
            .child(toolbar().child(breadcrumb("crumbs").crumbs(path.split('/').map(str::to_string))).spacer().child(Button::new("save").label("Save").small().primary().on_click({
                let toaster = self.toaster.clone();
                let name = path.clone();
                move |_, _, cx| toaster.update(cx, |t, cx| {
                    t.push(toast("Saved").success().message(name.clone()), cx);
                })
            })))
            .child(div().flex().flex_row().flex_1().min_h_0().child(div().flex_1().min_w_0().child(unroll_in("doc", &path, code))).child(inspector));

        window_frame().child(power_on_in(
            "power",
            div()
                .flex()
                .flex_col()
                .size_full()
                .bg(hsla(p.bg))
                .text_color(hsla(p.fg))
                .body(text::BASE)
                .on_action(cx.listener(|this, _: &TogglePalette, window, cx| this.palette.update(cx, |p, cx| p.toggle(window, cx))))
                .child(self.palette.clone())
                .child(self.toaster.clone())
                .child(title_bar("Workbench"))
                .child(split("main").initial(0.22).min(px(180.)).flex_1().min_h_0().first(files).second(editor))
                .child(status_bar().left("READY").left(path.to_string()).right(format!("LN {}", current + 1)).right("UTF-8")),
        ))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        ferrite_design::init(Appearance::Dark, cx);
        // FERRITE_SCHEME=harbor|mono|phosphor|… starts in another color scheme.
        if let Some(scheme) = std::env::var("FERRITE_SCHEME").ok().and_then(|k| ferrite_design::schemes::by_key(&k)) {
            ferrite_design::theme::set_scheme(scheme, cx);
        }
        cx.bind_keys([KeyBinding::new("ctrl-shift-p", TogglePalette, None), KeyBinding::new("ctrl-p", TogglePalette, None)]);
        let options = chrome::window_options("Workbench", size(px(1280.), px(820.)), cx);
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            chrome::power_off_on_close(window, cx);
            cx.new(|cx| Workbench::new(window, cx))
        })
        .expect("failed to open the window");
        cx.activate(true);
    });
}
