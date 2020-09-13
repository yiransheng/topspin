use std::future::Future;

use std::pin::Pin;
use std::process::{ExitStatus, Stdio};

use std::task::{Context, Poll};

use log;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::{
    mpsc::{self, error::SendError, Receiver, Sender},
    oneshot,
};

use crate::model::{ProgramMap, RunCommand, RunRequest, RunResponse};

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

pub struct Spawner {
    requests: Receiver<RunRequest>,
    responses: Sender<RunResponse>,
    spawned: ProgramMap<oneshot::Sender<Kill>>,
}

#[derive(Copy, Clone)]
struct Kill;

struct KillableChild {
    killed: bool,
    kill_chan: oneshot::Receiver<Kill>,
    child: Child,
}

impl Future for KillableChild {
    type Output = ::tokio::io::Result<ExitStatus>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.killed {
            Pin::new(&mut self.child).poll(cx)
        } else if let Poll::Ready(_) = Pin::new(&mut self.kill_chan).poll(cx) {
            match self.child.kill() {
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

impl Spawner {
    pub fn new(requests: Receiver<RunRequest>) -> (Self, Receiver<RunResponse>) {
        let (tx, rx) = mpsc::channel(32);
        (
            Self {
                requests,
                responses: tx,
                spawned: ProgramMap::new(),
            },
            rx,
        )
    }

    pub async fn run(&mut self) -> Result<(), ::tokio::io::Error> {
        while let Some(req) = self.requests.recv().await {
            match req {
                RunRequest::Run(cmd) => {
                    let id = cmd.id;
                    let kill_chan = run_command(cmd, self.responses.clone());
                    match kill_chan {
                        Ok(kill_chan) => {
                            let _ = self.spawned.insert(id, kill_chan);
                        }
                        Err(err) => {
                            let mut resp = self.responses.clone();
                            tokio::spawn(async move {
                                resp.send(RunResponse::IoError(id, err)).await.die_on_err();
                            });
                        }
                    }
                }
                RunRequest::Kill(id) => {
                    if let Some(tx) = self.spawned.remove(id) {
                        let _ = tx.send(Kill);
                    }
                }
            }
        }

        Ok(())
    }
}

fn run_command(
    cmd: RunCommand,
    mut resp: Sender<RunResponse>,
) -> Result<oneshot::Sender<Kill>, ::tokio::io::Error> {
    let RunCommand {
        name,
        args,
        id,
        working_dir,
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
    eprintln!("PID: {}", pid);

    let mut resp_1 = resp.clone();
    tokio::spawn(async move {
        resp_1
            .send(RunResponse::Started(id, pid))
            .await
            .die_on_err();
    });

    let stdout = child.stdout.take().unwrap();

    let _reader = BufReader::new(stdout).lines();

    let (tx, rx) = oneshot::channel();
    let child = KillableChild {
        killed: false,
        kill_chan: rx,
        child,
    };

    // Ensure the child process is spawned in the runtime so it can
    // make progress on its own while we await for any output.
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::channel;

    use crate::model::program_id;

    #[tokio::test(threaded_scheduler)]
    async fn test_run_then_kill() {
        let (mut tx, rx) = channel(128);
        let (mut spawner, _) = Spawner::new(rx);

        tokio::spawn(async move {
            spawner.run().await.unwrap();
        });

        tx.send(RunRequest::Run(RunCommand {
            id: program_id(0),
            name: "cat".to_string(),
            args: vec![],
            working_dir: None,
        }))
        .await
        .unwrap();

        tx.send(RunRequest::Kill(program_id(0))).await.unwrap();
    }
}
