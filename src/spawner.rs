use std::future::Future;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::process::{ExitStatus, Stdio};

use std::task::{Context, Poll};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::{
    mpsc::{Receiver},
    oneshot,
};


use crate::model::{ProgramMap, RunCommand, RunRequest};

pub struct Spawner {
    requests: Receiver<RunRequest>,
    spawned: ProgramMap<oneshot::Sender<Kill>>,
}

#[derive(Copy, Clone)]
struct Kill;

struct KillableChild {
    kill_chan: oneshot::Receiver<Kill>,
    child: Child,
}

impl Future for KillableChild {
    type Output = ::tokio::io::Result<ExitStatus>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(_) = Pin::new(&mut self.kill_chan).poll(cx) {
            Poll::Ready(match self.child.kill() {
                Ok(_) => Err(Error::new(ErrorKind::Interrupted, "Killed")),
                Err(err) => Err(err),
            })
        } else {
            Pin::new(&mut self.child).poll(cx)
        }
    }
}

impl Spawner {
    pub fn new(requests: Receiver<RunRequest>) -> Self {
        Self {
            requests,
            spawned: ProgramMap::new(),
        }
    }

    pub async fn run(&mut self) -> Result<(), ::tokio::io::Error> {
        while let Some(req) = self.requests.recv().await {
            match req {
                RunRequest::Run(cmd) => {
                    let id = cmd.id;
                    let jh = run_command(cmd);
                    let _ = self.spawned.insert(id, jh.unwrap());
                }
                RunRequest::Kill(id) => {
                    eprintln!("killing");
                    if let Some(tx) = self.spawned.remove(id) {
                        tx.send(Kill);
                    }
                }
            }
        }

        Ok(())
    }
}

fn run_command(cmd: RunCommand) -> Result<oneshot::Sender<Kill>, ::tokio::io::Error> {
    let RunCommand { name, args, .. } = cmd;
    let mut command = tokio::process::Command::new(name);
    for arg in args.into_iter() {
        command.arg(arg);
    }
    let mut child: Child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let pid = child.id();
    eprintln!("PID: {}", pid);
    let stdout = child.stdout.take().unwrap();

    let _reader = BufReader::new(stdout).lines();

    let (tx, rx) = oneshot::channel();
    let child = KillableChild {
        kill_chan: rx,
        child,
    };

    // Ensure the child process is spawned in the runtime so it can
    // make progress on its own while we await for any output.
    let _join_handle = tokio::spawn(async move {
        let status = child.await;
        eprintln!("PID: {}, status: {:?}", pid, status);
    });

    Ok(tx)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use tokio::sync::mpsc::channel;
    use tokio::time::delay_for;

    use crate::model::programId;

    #[tokio::test(threaded_scheduler)]
    async fn test_run_then_kill() {
        let (mut tx, rx) = channel(128);
        let mut spawner = Spawner::new(rx);

        tokio::spawn(async move {
            spawner.run().await.unwrap();
        });

        tx.send(RunRequest::Run(RunCommand {
            id: programId(0),
            name: "cat".to_string(),
            args: vec![],
        }))
        .await
        .unwrap();

        tx.send(RunRequest::Kill(programId(0))).await.unwrap();

        delay_for(Duration::from_millis(100000)).await;
    }
}
