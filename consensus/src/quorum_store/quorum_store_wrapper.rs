// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::quorum_store::{ProofReturnChannel, QuorumStoreError};
use crate::quorum_store::{counters, quorum_store::QuorumStoreCommand};
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use aptos_metrics_core::monitor;
use aptos_types::transaction::SignedTransaction;
use consensus_types::proof_of_store::LogicalTime;
use consensus_types::{
    common::TransactionSummary,
    proof_of_store::{ProofOfStore, SignedDigestInfo},
    request_response::ConsensusRequest,
};
use futures::{
    channel::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    SinkExt, StreamExt, TryFutureExt,
};
use std::collections::HashMap;
use std::future::Future;

pub struct QuorumStoreWrapper {
    consensus_receiver: Receiver<ConsensusRequest>,
    mempool_sender: Sender<QuorumStoreRequest>,
    mempool_txn_pull_timeout_ms: u64,
    quorum_store_sender: Sender<QuorumStoreCommand>,
    batches: HashMap<SignedDigestInfo, Vec<TransactionSummary>>,
    batch_in_progress: Vec<TransactionSummary>,
    broadcast_queue: Vec<ProofOfStore>, // TODO: vector or not?
    proof_in_progress: Vec<tokio::sync::oneshot::Receiver<Result<ProofOfStore, QuorumStoreError>>>, // TODO: hashmap using some internal key?
}

impl QuorumStoreWrapper {
    pub fn new(
        consensus_receiver: Receiver<ConsensusRequest>,
        mempool_sender: Sender<QuorumStoreRequest>,
        quorum_store_sender: Sender<QuorumStoreCommand>,
        mempool_txn_pull_timeout_ms: u64,
    ) -> Self {
        Self {
            consensus_receiver,
            mempool_sender,
            mempool_txn_pull_timeout_ms,
            quorum_store_sender,
            batches: HashMap::new(),
            batch_in_progress: vec![],
            broadcast_queue: vec![],
            proof_in_progress: vec![],
        }
    }

    async fn pull_internal(
        &self,
        max_size: u64,
        exclude_txns: Vec<TransactionSummary>,
    ) -> Result<Vec<SignedTransaction>, anyhow::Error> {
        let (callback, callback_rcv) = oneshot::channel();
        let msg = QuorumStoreRequest::GetBatchRequest(max_size, exclude_txns, callback);
        self.mempool_sender
            .clone()
            .try_send(msg)
            .map_err(anyhow::Error::from)?;
        // wait for response
        match monitor!(
            "pull_txn",
            timeout(
                Duration::from_millis(self.mempool_txn_pull_timeout_ms),
                callback_rcv
            )
            .await
        ) {
            Err(_) => Err(anyhow::anyhow!(
                "[direct_mempool_quorum_store] did not receive GetBatchResponse on time"
            )),
            Ok(resp) => match resp.map_err(anyhow::Error::from)?? {
                QuorumStoreResponse::GetBatchResponse(txns) => Ok(txns),
                _ => Err(anyhow::anyhow!(
                    "[direct_mempool_quorum_store] did not receive expected GetBatchResponse"
                )),
            },
        }
    }

    async fn handle_scheduled_pull(&mut self) {
        let exclude_txns: Vec<_> = self.batches.values().flatten().cloned().collect();
        // TODO: size and unwrap or not?
        let pulled_txns = self.pull_internal(50, exclude_txns).await.unwrap();
        let summaries = pulled_txns
            .iter()
            .map(|txn| TransactionSummary {
                sender: txn.sender(),
                sequence_number: txn.sequence_number(),
            })
            .collect();
        self.batch_in_progress.append(summaries);

        // TODO: also some timer if there are not enough txns
        if self.batch_in_progress.len() <= 100 {
            self.quorum_store_sender
                .send(QuorumStoreCommand::AppendToBatch(pulled_txns))
                .await;
        } else {
            // TODO: do we really want tokio channels in quorum store?
            let (proof_tx, proof_rx) = tokio::sync::oneshot::channel();
            self.quorum_store_sender
                .send(QuorumStoreCommand::EndBatch(
                    pulled_txns,
                    LogicalTime::new(0, 0), // TODO
                    proof_tx,
                ))
                .await;
            self.proof_in_progress.push(proof_rx);
        }
    }

    async fn handle_proof_completed(&mut self, msg: Result<ProofOfStore, QuorumStoreError>) {
        match msg {
            Ok(proof) => {
                self.broadcast_queue.push(proof);
            }
            Err(_) => {
                // TODO: cast to anyhow?
            }
        }
    }

    pub async fn start(mut self) {
        loop {
            let _timer = counters::MAIN_LOOP.start_timer();
            ::futures::select! {
                msg = self.scheduled_pull.select_next_some() => {
                    self.handle_scheduled_pull().await;
                },
                msg = self.proof_in_progress.last().unwrap() => {
                    self.handle_proof_completed(msg).await;
                }
            }
        }

        todo!()
        // Periodically:
        // 1. Pull from mempool.
        // 2. a. Start a batch with these txns if batch is not active
        //    b. Continue batch with these txns if batch is active
        // 3. Close batch if criteria is met.

        // State needed:
        // 1. txn summaries that are part of all pending batches: map<batch_id, vec<txn>>
        //    - pending batches: batches, including those in progress, that have not yet been cleaned.
        //    - batch_id: needs to include epoch, round info.
        // 2. all completed digests that have not yet been cleaned: map<batch_id, digest>
        //    -- is this really needed? pull_payload filters anyway. maybe all that's needed
        //    is a broadcast queue?
    }
}
