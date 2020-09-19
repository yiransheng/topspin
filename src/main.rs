use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use druid::{AppLauncher, ExtEventSink, LocalizedString, WindowDesc};
use structopt::StructOpt;

mod constants;
mod log_client;
mod log_server;
mod model;
mod persist;
mod spawner;
mod ui;

use crate::constants::RUN_RESPONSES;
use crate::log_client::run_log_client;
use crate::log_server::run_log_server;
use crate::model::{RunRequest, RunResponse};
use crate::persist::load_entries;
use crate::spawner::Spawner;
use crate::ui::{app_data::AppData, ui_builder};

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

#[derive(Debug, StructOpt)]
struct Opt {
    // If set run in client mode.
    #[structopt(short, long)]
    connect: Option<String>,
}

const WINDOW_TITLE: LocalizedString<AppData> = LocalizedString::new("Top Spin");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    if let Some(alias) = opt.connect {
        run_log_client(&alias)?;
        return Ok(());
    }

    // do not daemonize in debug mode.
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
    let (mut req_tx, req_rx) = mpsc::channel::<RunRequest>(32);
    let (sink_tx, sink_rx) = mpsc::channel::<(String, _)>(32);
    let (mut spawner, res_rx) = Spawner::new(req_rx, sink_rx);

    // create the initial app state
    let initial_state = if let Some(commands) = persisted {
        AppData::from_commands(commands, req_tx.clone())
    } else {
        AppData::new(req_tx.clone())
    };

    tokio::task::spawn_blocking(move || {
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
            req_tx
                .send(RunRequest::Stop)
                .await
                .expect("Cannot send message to stop spawner");
        });
    });

    tokio::spawn(async move {
        let _ = run_log_server(sink_tx).await;
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
