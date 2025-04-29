use std::{error::Error, sync::mpsc::Receiver};
use lost_metrics_sniffer_stub::{packets::opcodes::Pkt, start_capture};

#[cfg(test)]
use mockall::automock;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[cfg_attr(test, automock)]
pub trait PacketSniffer {
    fn start(&mut self, port: u16, region_file_path: String) -> anyhow::Result<()>;
    async fn recv(&mut self) -> Option<(Pkt, Vec<u8>)>;
}

pub struct PacketSnifferStub {
    tx: Option<UnboundedSender<(Pkt, Vec<u8>)>>,
    rx: Option<UnboundedReceiver<(Pkt, Vec<u8>)>>
}

impl PacketSniffer for PacketSnifferStub {
    fn start(&mut self, port: u16, region_file_path: String) -> anyhow::Result<()> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<(Pkt, Vec<u8>)>();
        Ok(())
    }
    
    async fn recv(&mut self) -> Option<(Pkt, Vec<u8>)> {
        self.rx.as_mut().unwrap().recv().await
    }
}

impl PacketSnifferStub {
    pub fn new() -> Self {
        Self {
            tx: None,
            rx: None
        }
    }

    pub fn get_sender(&self) -> UnboundedSender<(Pkt, Vec<u8>)> {
        self.tx.clone().unwrap()
    }
}