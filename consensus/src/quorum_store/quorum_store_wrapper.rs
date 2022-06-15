// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_mempool::QuorumStoreRequest;
use consensus_types::request_response::ConsensusRequest;
use futures::{
    channel::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    StreamExt,
};

pub struct QuorumStoreWrapper {
    consensus_receiver: Receiver<ConsensusRequest>,
    mempool_sender: Sender<QuorumStoreRequest>,
    mempool_txn_pull_timeout_ms: u64,
}

impl QuorumStoreWrapper {
    pub fn new(
        consensus_receiver: Receiver<ConsensusRequest>,
        mempool_sender: Sender<QuorumStoreRequest>,
        mempool_txn_pull_timeout_ms: u64,
    ) -> Self {
        Self {
            consensus_receiver,
            mempool_sender,
            mempool_txn_pull_timeout_ms,
        }
    }

    pub async fn start(mut self) {
        todo!()
    }
}
