mod cmd;
mod exoscale;
mod options;
mod ssh;

use anyhow::Result;
use clap::Parser;

use crate::cmd::command::Command;
use crate::cmd::create::Create;
use crate::cmd::delete::Delete;
use crate::cmd::init::Init;
use crate::cmd::start::Start;
use crate::cmd::state::State;
use crate::cmd::stop::Stop;

#[derive(Parser)]
enum DevPodProviderExoscale {
    Create(Create),
    Init(Init),
    Delete(Delete),
    Command(Command),
    Start(Start),
    Stop(Stop),
    Status(State),
}

impl DevPodProviderExoscale {
    async fn execute(&self) -> Result<()> {
        match self {
            Self::Create(options) => options.execute().await,
            Self::Delete(options) => options.execute().await,
            Self::Init(options) => options.execute().await,
            Self::Command(options) => options.execute().await,
            Self::Start(options) => options.execute().await,
            Self::Stop(options) => options.execute().await,
            Self::Status(options) => options.execute().await,
        }
    }
}

#[tokio::main]
async fn main() {
    let command = DevPodProviderExoscale::parse();
    if let Err(err) = command.execute().await {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
