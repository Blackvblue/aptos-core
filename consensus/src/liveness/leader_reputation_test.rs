// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::liveness::{
    leader_reputation::{
        ActiveInactiveHeuristic, LeaderReputation, MetadataBackend, ReputationHeuristic,
    },
    proposer_election::{next, ProposerElection},
};

use aptos_types::{
    account_address::AccountAddress, block_metadata::NewBlockEvent,
    validator_signer::ValidatorSigner,
};
use consensus_types::common::{Author, Round};
use itertools::Itertools;

struct MockHistory {
    window_size: usize,
    data: Vec<NewBlockEvent>,
}

impl MockHistory {
    fn new(window_size: usize, data: Vec<NewBlockEvent>) -> Self {
        Self { window_size, data }
    }
}

impl MetadataBackend for MockHistory {
    fn get_block_metadata(&self, _target_round: Round) -> Vec<NewBlockEvent> {
        let start = if self.data.len() > self.window_size {
            self.data.len() - self.window_size
        } else {
            0
        };
        self.data[start..].to_vec()
    }
}

fn create_block(epoch: u64, proposer: Author, voters: Vec<bool>) -> NewBlockEvent {
    NewBlockEvent::new(epoch, 0, voters, proposer, 0)
}

#[test]
fn test_simple_heuristic() {
    let active_weight = 9;
    let inactive_weight = 1;
    let mut proposers = vec![];
    let mut signers = vec![];
    for i in 0..8 {
        let signer = ValidatorSigner::random([i; 32]);
        proposers.push(signer.author());
        signers.push(signer);
    }
    let heuristic = ActiveInactiveHeuristic::new(proposers[0], active_weight, inactive_weight);
    // 1. Window size not enough
    let weights = heuristic.get_weights(0, &proposers, &[]);
    assert_eq!(weights.len(), proposers.len());
    for w in weights {
        assert_eq!(w, inactive_weight);
    }
    // 2. Sliding window with [proposer 0, voters 1, 2], [proposer 0, voters 3]
    let weights = heuristic.get_weights(
        0,
        &proposers,
        &[
            create_block(
                0,
                proposers[0],
                vec![false, true, true, false, false, false, false, false],
            ),
            create_block(
                0,
                proposers[0],
                vec![false, false, false, true, false, false, false, false],
            ),
        ],
    );
    assert_eq!(weights.len(), proposers.len());
    for (i, w) in weights.iter().enumerate() {
        let expected = if i < 4 {
            active_weight
        } else {
            inactive_weight
        };
        assert_eq!(*w, expected);
    }
}

#[test]
fn test_epoch_change() {
    let active_weight = 9;
    let inactive_weight = 1;
    let mut proposers = vec![];
    let mut signers = vec![];
    for i in 0..8 {
        let signer = ValidatorSigner::random([i; 32]);
        proposers.push(signer.author());
        signers.push(signer);
    }
    let heuristic = ActiveInactiveHeuristic::new(proposers[0], active_weight, inactive_weight);
    // History with [proposer 0, voters 1, 2], [proposer 0, voters 3] in current epoch
    let weights = heuristic.get_weights(
        2,
        &proposers,
        &[
            create_block(
                2,
                proposers[0],
                vec![false, true, true, false, false, false, false, false],
            ),
            create_block(
                2,
                proposers[0],
                vec![false, false, false, true, false, false, false, false],
            ),
            create_block(
                1,
                proposers[0],
                vec![false, true, true, true, true, true, true, true],
            ),
            create_block(
                0,
                proposers[0],
                vec![false, true, true, true, true, true, true, true],
            ),
        ],
    );
    assert_eq!(weights.len(), proposers.len());
    for (i, w) in weights.iter().enumerate() {
        let expected = if i < 4 {
            active_weight
        } else {
            inactive_weight
        };
        assert_eq!(*w, expected);
    }
}

#[test]
fn test_api() {
    let active_weight = 9;
    let inactive_weight = 1;
    let proposers: Vec<AccountAddress> =
        (0..5).map(|_| AccountAddress::random()).sorted().collect();
    let history = vec![
        create_block(0, proposers[0], vec![false, true, true, false, false]),
        create_block(0, proposers[0], vec![false, false, false, true, false]),
    ];
    let leader_reputation = LeaderReputation::new(
        0,
        proposers.clone(),
        Box::new(MockHistory::new(1, history)),
        Box::new(ActiveInactiveHeuristic::new(
            proposers[0],
            active_weight,
            inactive_weight,
        )),
        4,
    );
    let round = 42u64;
    // first metadata is ignored because of window size 1
    let expected_weights = vec![
        active_weight,
        inactive_weight,
        inactive_weight,
        active_weight,
        inactive_weight,
    ];
    let sum = expected_weights.iter().fold(0, |mut s, w| {
        s += *w;
        s
    });
    let mut state = round.to_le_bytes().to_vec();
    let chosen_weight = next(&mut state) % sum;
    let mut expected_index = 0usize;
    let mut accu = 0u64;
    for (i, w) in expected_weights.iter().enumerate() {
        accu += *w;
        if accu >= chosen_weight {
            expected_index = i;
        }
    }
    let unexpected_index = (expected_index + 1) % proposers.len();
    let output = leader_reputation.get_valid_proposer(round);
    assert_eq!(output, proposers[expected_index]);
    assert!(leader_reputation.is_valid_proposer(proposers[expected_index], 42));
    assert!(!leader_reputation.is_valid_proposer(proposers[unexpected_index], 42));
}
