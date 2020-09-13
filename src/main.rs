use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use druid::{AppLauncher, ExtEventSink, LocalizedString, WindowDesc};

mod constants;
mod model;
mod persist;
mod spawner;
mod ui;

use crate::constants::RUN_RESPONSES;
use crate::model::{RunRequest, RunResponse};
use crate::persist::load_entries;
use crate::spawner::Spawner;
use crate::ui::{app_data::AppData, ui_builder};

const WINDOW_TITLE: LocalizedString<AppData> = LocalizedString::new("Top Spin");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(debug_assertions))]
    {
        use daemonize::Daemonize;
        use std::fs::File;

        let stdout = File::create("/tmp/topspin_stdout")?;
        let stderr = File::create("/tmp/topspin_stderr")?;

        let daemonize = Daemonize::new()
            .pid_file("/tmp/topspin.pid")
            .chown_pid_file(true)
            .stdout(stdout)
            .stderr(stderr)
            .privileged_action(|| "Executed before drop privileges");

        daemonize.start()
    }?;

    let mut rt = Runtime::new()?;
    rt.block_on(run())
}

async fn run() -> Result<(), Box<dyn ::std::error::Error>> {
    let persisted = load_entries().await.unwrap_or(None);
    let (req_tx, req_rx) = mpsc::channel::<RunRequest>(32);
    let (mut spawner, res_rx) = Spawner::new(req_rx);

    let mut req_tx_exit = req_tx.clone();
    // create the initial app state
    let initial_state = if let Some(commands) = persisted {
        AppData::from_commands(commands, req_tx)
    } else {
        AppData::new(req_tx)
    };

    tokio::task::spawn_blocking(move || {
        // describe the main window
        let main_window = WindowDesc::new(ui_builder)
            .title(WINDOW_TITLE)
            .window_size((800.0, 600.0));

        let launcher = AppLauncher::with_window(main_window);
        let event_sink = launcher.get_external_handle();
        tokio::spawn(async move {
            event_bridge(res_rx, event_sink)
                .await
                .unwrap_or_else(|err| {
                    log::error!("Event error: {}", err);
                    std::process::exit(1);
                });
        });
        // start the application
        launcher
            .use_simple_logger()
            .launch(initial_state)
            .unwrap_or_else(|err| {
                log::error!("Launch error: {}", err);
                std::process::exit(1);
            });

        tokio::spawn(async move {
            req_tx_exit
                .send(RunRequest::Stop)
                .await
                .expect("Cannot send message to stop spawner");
        });
    });

    spawner.run().await?;

    Ok(())
}

async fn event_bridge(
    mut chan: mpsc::Receiver<RunResponse>,
    sink: ExtEventSink,
) -> Result<(), Box<dyn ::std::error::Error>> {
    while let Some(res) = chan.recv().await {
        sink.submit_command(RUN_RESPONSES, res, None)?;
    }

    Ok(())
}
