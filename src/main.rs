use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

use druid::widget::prelude::*;
use druid::widget::{Align, Flex, Label, TextBox};
use druid::{
    AppLauncher, Data, Env, ExtEventSink, Lens, LocalizedString, Selector, Widget, WidgetExt,
    WindowDesc,
};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;
const TEXT_BOX_WIDTH: f64 = 200.0;
const WINDOW_TITLE: LocalizedString<HelloState> = LocalizedString::new("Hello World!");

const NEW_LINE: Selector<String> = Selector::new("line");

#[derive(Clone, Data, Lens)]
struct HelloState {
    name: String,
}

struct HandleCmd<T> {
    child: T,
}

impl<T> HandleCmd<T> {
    fn new(child: T) -> Self {
        Self { child }
    }
}

impl<W> Widget<HelloState> for HandleCmd<W>
where
    W: Widget<HelloState>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut HelloState, env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(NEW_LINE) => {
                data.name = cmd.get_unchecked(NEW_LINE).clone();
                ctx.request_paint();
            }
            _ => self.child.event(ctx, event, data, env),
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &HelloState,
        env: &Env,
    ) {
        self.child.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &HelloState, data: &HelloState, env: &Env) {
        self.child.update(ctx, old_data, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &HelloState,
        env: &Env,
    ) -> Size {
        self.child.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &HelloState, env: &Env) {
        self.child.paint(ctx, data, env);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn ::std::error::Error>> {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget)
        .title(WINDOW_TITLE)
        .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state = HelloState {
        name: "World".into(),
    };

    let launcher = AppLauncher::with_window(main_window);
    let event_sink = launcher.get_external_handle();

    tokio::spawn(async {
        run_command(event_sink).await.unwrap();
    });

    // start the application
    launcher
        .use_simple_logger()
        .launch(initial_state)
        .expect("Failed to launch application");

    Ok(())
}

fn build_root_widget() -> impl Widget<HelloState> {
    // a label that will determine its text based on the current app data.
    let label = Label::new(|data: &HelloState, _env: &Env| format!("Hello {}!", data.name));
    // a textbox that modifies `name`.
    let textbox = TextBox::new()
        .with_placeholder("Who are we greeting?")
        .fix_width(TEXT_BOX_WIDTH)
        .lens(HelloState::name);

    // arrange the two widgets vertically, with some padding
    let layout = Flex::column()
        .with_child(label)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(textbox);

    // center the two widgets in the available space
    let inner = Align::centered(layout);
    HandleCmd::new(inner)
}

async fn run_command(sink: ExtEventSink) -> Result<(), Box<dyn ::std::error::Error>> {
    let mut child = tokio::process::Command::new("nc")
        .arg("-l")
        .arg("6700")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();

    let stdout = child.stdout.take().unwrap();

    let mut reader = BufReader::new(stdout).lines();

    // Ensure the child process is spawned in the runtime so it can
    // make progress on its own while we await for any output.
    tokio::spawn(async {
        let status = child.await.expect("child process encountered an error");
        println!("child status was: {}", status);
    });

    while let Some(line) = reader.next_line().await? {
        sink.submit_command(NEW_LINE, line, None)?;
    }

    Ok(())
}
