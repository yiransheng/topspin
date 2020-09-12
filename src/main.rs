use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

use druid::{
    AppLauncher, Data, Env, ExtEventSink, Lens, LocalizedString, Selector, Widget, WidgetExt,
    WindowDesc,
};

mod model;
mod spawner;
mod ui;

use crate::ui::{
    app_data::{new_app_data, AppData},
    ui_builder,
};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;
const TEXT_BOX_WIDTH: f64 = 200.0;
const WINDOW_TITLE: LocalizedString<AppData> = LocalizedString::new("Hello World!");

const NEW_LINE: Selector<String> = Selector::new("line");

#[tokio::main]
async fn main() -> Result<(), Box<dyn ::std::error::Error>> {
    // describe the main window
    let main_window = WindowDesc::new(ui_builder)
        .title(WINDOW_TITLE)
        .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state = new_app_data();

    let launcher = AppLauncher::with_window(main_window);
    let _event_sink = launcher.get_external_handle();

    // start the application
    launcher
        .use_simple_logger()
        .launch(initial_state)
        .expect("Failed to launch application");

    Ok(())
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
