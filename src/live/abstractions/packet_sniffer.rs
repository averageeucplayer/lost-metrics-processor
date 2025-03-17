use std::{error::Error, sync::mpsc::Receiver};
use lost_metrics_sniffer_stub::{packets::opcodes::Pkt, start_capture};

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait ReceiverWrapper {
    fn recv(&self) -> anyhow::Result<(Pkt, Vec<u8>)>;
}

impl ReceiverWrapper for Receiver<(Pkt, Vec<u8>)> {
    fn recv(&self) -> anyhow::Result<(Pkt, Vec<u8>)> {
        let result = self.recv()?;
        anyhow::Ok(result)
    }
}

#[cfg_attr(test, automock)]
pub trait PacketSniffer {
    fn start_capture(&self, port: u16, region_file_path: String) -> anyhow::Result<Box<dyn ReceiverWrapper>>;
}

pub struct PacketSnifferStub {
    
}

impl PacketSniffer for PacketSnifferStub {
    fn start_capture(&self, port: u16, region_file_path: String) -> anyhow::Result<Box<dyn ReceiverWrapper>> {
        let result = start_capture(port, region_file_path)
            .map_err(|e| anyhow::anyhow!("Failed capture: {}", e))?;
        anyhow::Ok(Box::new(result))
    }
}

impl PacketSnifferStub {
    pub fn new() -> Self {
        Self {}
    }
}