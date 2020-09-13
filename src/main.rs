use tokio::sync::mpsc;

use druid::{
    AppLauncher, Data, Env, ExtEventSink, Lens, LocalizedString, Selector, Widget, WidgetExt,
    WindowDesc,
};

mod constants;
mod model;
mod persist;
mod spawner;
mod ui;

use crate::constants::RUN_RESPONSES;
use crate::model::{RunRequest, RunResponse};
use crate::spawner::Spawner;
use crate::ui::{
    app_data::{new_app_data, AppData},
    ui_builder,
};

const WINDOW_TITLE: LocalizedString<AppData> = LocalizedString::new("Top Spin");

#[tokio::main]
async fn main() -> Result<(), Box<dyn ::std::error::Error>> {
    let (req_tx, req_rx) = mpsc::channel::<RunRequest>(32);
    let (mut spawner, res_rx) = Spawner::new(req_rx);

    // create the initial app state
    let initial_state = new_app_data(req_tx);

    tokio::task::spawn_blocking(move || {
        // describe the main window
        let main_window = WindowDesc::new(ui_builder)
            .title(WINDOW_TITLE)
            .window_size((800.0, 600.0));

        let launcher = AppLauncher::with_window(main_window);
        let event_sink = launcher.get_external_handle();
        tokio::spawn(async move {
            event_bridge(res_rx, event_sink).await.expect("Crash");
        });
        // start the application
        launcher
            .use_simple_logger()
            .launch(initial_state)
            .expect("Failed to launch application");
    });

    spawner.run().await?;

    Ok(())
}

async fn event_bridge(
    mut chan: mpsc::Receiver<RunResponse>,
    sink: ExtEventSink,
) -> Result<(), Box<dyn ::std::error::Error>> {
    while let Some(res) = chan.recv().await {
        eprintln!("{:?}", res);
        sink.submit_command(RUN_RESPONSES, res, None)?;
    }

    Ok(())
}
