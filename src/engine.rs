use anyhow::Ok;

use crate::exchange::RestClient;
use crate::data::OrderReq;

#[derive(Clone)]
pub struct Engine<C: RestClient> {
    pub client: C,
    pub paper: bool,
    pub last: Option<String>
}

impl<C: RestClient> Engine<C> {
    pub fn new(client: C, paper: bool) -> Self {
        Self { client, paper, last: None }
    }

    pub async fn handle(&mut self, req: &OrderReq) -> anyhow::Result<()> {
        if let Some(id) = &self.last {
            if self.paper {
                tracing::info!("Cancelling paper order: {:?}", req);
            }
            else {
                tracing::info!("Cancelling live order: {:?}", req);
            }
            self.client.cancel_order(id).await?
        }

        let log = if self.paper {"Paper"} else {"Live"};
        tracing::info!("{} order place here for: {:?}", log, req);
        let new_id = self.client.place_order(req).await?;
        self.last = Some(new_id);
        Ok(())
    }
}
