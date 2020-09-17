use std::future::Future;

use std::collections::HashMap;
use std::pin::Pin;
use std::process::{ExitStatus, Stdio};
use std::sync::{Arc, Mutex};

use std::task::{Context, Poll};

use log;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::process::{Child, ChildStderr, ChildStdout};
use tokio::sync::{
    mpsc::{self, error::SendError, Receiver, Sender},
    oneshot,
};

use crate::model::{ProgramId, ProgramMap, RunCommand, RunRequest, RunResponse, SpawnerInput};

type Unclaimed<T> = Arc<Mutex<Vec<T>>>;

trait Fatal<T, E>: Into<Result<T, E>> {
    const MESSAGE: &'static str;

    fn die_on_err(self) -> T {
        match self.into() {
            Ok(val) => val,
            Err(_) => {
                log::warn!("{}", Self::MESSAGE);
                std::process::exit(1);
            }
        }
    }
}

impl<T> Fatal<(), SendError<T>> for Result<(), SendError<T>> {
    const MESSAGE: &'static str = "Receiver (UI) is gone, killing the program";
}

pub struct Spawner<W> {
    requests_chan: Receiver<RunRequest>,
    sinks_chan: Receiver<(String, W)>,
    responses: Sender<RunResponse>,
    spawned: ProgramMap<oneshot::Sender<Kill>>,
    log_sinks: ProgramMap<Unclaimed<W>>,
    alias_to_id: HashMap<String, ProgramId>,
}

impl<W> Drop for Spawner<W> {
    fn drop(&mut self) {
        for tx in self.spawned.drain() {
            let _ = tx.send(Kill);
        }
        self.alias_to_id.clear();
    }
}

impl<W: 'static + Send + AsyncWrite + std::marker::Unpin> Spawner<W> {
    pub fn new(
        requests: Receiver<RunRequest>,
        sinks: Receiver<(String, W)>,
    ) -> (Self, Receiver<RunResponse>) {
        let (tx, rx) = mpsc::channel(32);
        (
            Self {
                requests_chan: requests,
                sinks_chan: sinks,
                responses: tx,
                spawned: ProgramMap::new(),
                log_sinks: ProgramMap::new(),
                alias_to_id: HashMap::new(),
            },
            rx,
        )
    }

    pub async fn run(&mut self) -> tokio::io::Result<()> {
        loop {
            let input = tokio::select! {
                input = self.requests_chan.recv() => {
                    if let Some(input) = input {
                        input.into()
                    } else {
                        continue
                    }
                }
                input = self.sinks_chan.recv() => {
                    if let Some(input) = input {
                        input.into()
                    } else {
                        continue
                    }
                }
            };
            match input {
                SpawnerInput::RunRequest(RunRequest::Run(cmd)) => {
                    let id = cmd.id;
                    let sink = Arc::new(Mutex::new(Vec::new()));
                    let alias = cmd.alias.clone();
                    let kill_chan = run_command(cmd, self.responses.clone(), sink.clone());
                    match kill_chan {
                        Ok(kill_chan) => {
                            let _ = self.spawned.insert(id, kill_chan);
                            let _ = self.alias_to_id.insert(alias, id);
                            let _ = self.log_sinks.insert(id, sink);
                        }
                        Err(err) => {
                            tokio::spawn({
                                let mut resp = self.responses.clone();
                                async move {
                                    resp.send(RunResponse::IoError(id, err)).await.die_on_err();
                                }
                            });
                        }
                    }
                }
                SpawnerInput::RunRequest(RunRequest::Kill(id)) => {
                    if let Some(tx) = self.spawned.remove(id) {
                        let _ = tx.send(Kill);
                    }
                }
                SpawnerInput::RunRequest(RunRequest::Stop) => break,
                SpawnerInput::Sink(alias, sink) => {
                    if let Some(program_id) = self.alias_to_id.get(alias.trim()) {
                        if let Some(sinks) = self.log_sinks.get(*program_id) {
                            log::info!("Command {} is running, streaming logs...", &alias);
                            let mut sinks = sinks.lock().unwrap();
                            sinks.push(sink);
                            drop(sinks);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

struct LogForward {
    stdout: ChildStdout,
    stderr: ChildStderr,
}

impl LogForward {
    async fn run<W: AsyncWrite + std::marker::Unpin>(
        &mut self,
        new_sinks: Unclaimed<W>,
    ) -> tokio::io::Result<()> {
        let mut out_buf: [u8; 1024] = [0; 1024];
        let mut err_buf: [u8; 1024] = [0; 1024];

        let mut sinks: Vec<Option<W>> = vec![];

        const STDOUT_TAG: [u8; 1] = [1];
        const STDERR_TAG: [u8; 1] = [2];

        let mut buf: &[u8];
        let mut tag: &[u8];

        loop {
            tokio::select! {
                len = self.stdout.read(&mut out_buf) => {
                    let len = len?;
                    tag = &STDOUT_TAG;
                    buf = &out_buf[0..len];
                }
                len = self.stderr.read(&mut err_buf) => {
                    let len = len?;
                    tag = &STDERR_TAG;
                    buf = &err_buf[0..len];
                }
            };
            {
                let mut new_sinks = new_sinks.lock().expect("mutex error");
                sinks.extend(new_sinks.drain(..).map(Some));
            }
            for sink in sinks.iter_mut() {
                let mut sink_taken = sink.take().unwrap();
                if LogForward::write_frame(&mut sink_taken, tag, buf)
                    .await
                    .is_err()
                {
                    drop(sink_taken);
                    break;
                }
                *sink = Some(sink_taken);
            }
            sinks.retain(Option::is_some);
        }
    }

    async fn write_frame<W: AsyncWrite + std::marker::Unpin>(
        out: &mut W,
        tag: &[u8],
        contents: &[u8],
    ) -> tokio::io::Result<()> {
        let len = contents.len();
        out.write(tag).await?;
        out.write_u64(len as u64).await?;
        out.write(contents).await?;
        Ok(())
    }
}

fn run_command<W: 'static + AsyncWrite + Send + std::marker::Unpin>(
    cmd: RunCommand,
    mut resp: Sender<RunResponse>,
    log_sinks: Unclaimed<W>,
) -> Result<oneshot::Sender<Kill>, ::tokio::io::Error> {
    let RunCommand {
        name,
        args,
        id,
        working_dir,
        ..
    } = cmd;
    let mut command = tokio::process::Command::new(name);
    for arg in args.into_iter() {
        command.arg(arg);
    }
    if let Some(dir) = working_dir {
        command.current_dir(dir);
    }
    let mut child: Child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let pid = child.id();
    log::info!("PID: {}", pid);

    let mut resp_1 = resp.clone();
    tokio::spawn(async move {
        resp_1
            .send(RunResponse::Started(id, pid))
            .await
            .die_on_err();
    });

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let mut log_forwarder = LogForward { stdout, stderr };
    tokio::spawn(async move {
        if let Err(err) = log_forwarder.run(log_sinks).await {
            log::error!("Error forwarding logs: {}", err);
        }
    });

    let (tx, rx) = oneshot::channel();
    let child = KillableChild {
        killed: false,
        kill_chan: rx,
        child,
    };

    let _join_handle = tokio::spawn(async move {
        match child.await {
            Ok(status) => {
                resp.send(RunResponse::Exited(id, status))
                    .await
                    .die_on_err();
            }
            Err(err) => {
                resp.send(RunResponse::IoError(id, err)).await.die_on_err();
            }
        }
    });

    Ok(tx)
}

#[derive(Copy, Clone)]
struct Kill;

struct KillableChild {
    killed: bool,
    kill_chan: oneshot::Receiver<Kill>,
    child: Child,
}

impl KillableChild {
    fn kill(&mut self) -> std::io::Result<()> {
        if self.killed {
            return Ok(());
        }
        let success = unsafe { libc::kill(self.child.id() as i32, libc::SIGTERM) };
        if success != 0 {
            // Reads from errno
            return Err(std::io::Error::last_os_error());
        } else {
            Ok(())
        }
    }
}

impl Future for KillableChild {
    type Output = ::tokio::io::Result<ExitStatus>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.killed {
            Pin::new(&mut self.child).poll(cx)
        } else if let Poll::Ready(_) = Pin::new(&mut self.kill_chan).poll(cx) {
            log::info!("Killing {}", self.child.id());
            match self.kill() {
                Ok(_) => {}
                Err(err) => return Poll::Ready(Err(err)),
            }
            self.killed = true;
            Pin::new(&mut self.child).poll(cx)
        } else {
            Pin::new(&mut self.child).poll(cx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::channel;

    use crate::model::program_id;

    #[tokio::test(threaded_scheduler)]
    async fn test_run_then_kill() {
        let (mut tx, rx) = channel(128);
        let (_, rx_unused) = channel::<(String, Vec<u8>)>(128);
        let (mut spawner, _) = Spawner::new(rx, rx_unused);

        tokio::spawn(async move {
            spawner.run().await.unwrap();
        });

        tx.send(RunRequest::Run(RunCommand {
            id: program_id(0),
            alias: "cat".to_string(),
            name: "cat".to_string(),
            args: vec![],
            working_dir: None,
        }))
        .await
        .unwrap();

        tx.send(RunRequest::Kill(program_id(0))).await.unwrap();
    }
}
